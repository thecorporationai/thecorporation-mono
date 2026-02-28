//! Cron scheduler — checks agent schedules and enqueues jobs.
//!
//! Runs as a background tokio task with a 60s tick.
//! Fetches active agents with cron channels from api-rs,
//! checks if any cron schedule matches the current minute,
//! and enqueues jobs with Redis dedup.

use std::sync::Arc;

use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;
use tokio_util::sync::CancellationToken;

use crate::config::WorkerConfig;
use crate::domain::ids::{AgentId, ExecutionId, MessageId, WorkspaceId};
use crate::domain::job::JobPayload;
use crate::error::WorkerError;
use crate::redis::{keys, queue};

/// Run the cron loop (60s tick) until cancelled.
pub async fn run(
    pool: Pool,
    config: Arc<WorkerConfig>,
    shutdown: CancellationToken,
) -> Result<(), WorkerError> {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("cron loop shutting down");
                break;
            }
            _ = interval.tick() => {
                if let Err(e) = tick(&pool, &config).await {
                    tracing::error!(error = %e, "cron tick failed");
                }
            }
        }
    }

    Ok(())
}

/// A minimal agent representation for cron checking.
#[derive(serde::Deserialize)]
struct AgentSummary {
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    workspace_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    channels: Vec<ChannelEntry>,
}

#[derive(serde::Deserialize)]
struct ChannelEntry {
    #[serde(rename = "type", default)]
    channel_type: String,
    #[serde(default)]
    schedule: Option<String>,
}

async fn tick(pool: &Pool, config: &WorkerConfig) -> Result<(), WorkerError> {
    // Fetch active agents from api-rs
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/v1/agents?status=active", config.api_base_url))
        .send()
        .await;

    let agents: Vec<AgentSummary> = match resp {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
        Ok(r) => {
            tracing::debug!(status = %r.status(), "cron: failed to fetch agents");
            return Ok(());
        }
        Err(e) => {
            tracing::debug!(error = %e, "cron: api-rs unreachable");
            return Ok(());
        }
    };

    let now = chrono::Utc::now();

    for agent in &agents {
        let Some(agent_id) = &agent.agent_id else { continue };
        let Some(workspace_id) = &agent.workspace_id else { continue };

        if agent.status.as_deref() != Some("active") {
            continue;
        }

        for channel in &agent.channels {
            if channel.channel_type != "cron" {
                continue;
            }
            let Some(ref schedule) = channel.schedule else { continue };

            if !cron_matches_now(schedule, &now) {
                continue;
            }

            // Dedup: SET NX with 120s TTL
            let schedule_hash = simple_hash(schedule);
            let dedup_key = keys::cron_last_fire(agent_id, &schedule_hash);
            let mut conn = pool.get().await?;
            let acquired: Option<String> = deadpool_redis::redis::cmd("SET")
                .arg(&dedup_key)
                .arg("1")
                .arg("NX")
                .arg("EX")
                .arg(120)
                .query_async(&mut *conn)
                .await?;

            if acquired.is_none() {
                continue; // Already fired this minute
            }

            // Parse IDs
            let agent_uuid: AgentId = match agent_id.parse() {
                Ok(id) => id,
                Err(_) => continue,
            };
            let ws_uuid: WorkspaceId = match workspace_id.parse() {
                Ok(id) => id,
                Err(_) => continue,
            };

            let job = JobPayload::new(
                ExecutionId::new(),
                agent_uuid,
                ws_uuid,
                MessageId::new(), // Cron messages don't have a stored message
            );

            tracing::info!(
                agent_id,
                schedule,
                execution_id = %job.execution_id,
                "cron: enqueueing job"
            );
            queue::enqueue_job(pool, &job).await?;
        }
    }

    Ok(())
}

/// Check if a cron expression matches the current minute.
fn cron_matches_now(expr: &str, now: &chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::{Datelike, Timelike};

    // Parse simple cron: "minute hour day_of_month month day_of_week"
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() < 5 {
        return false;
    }

    let checks = [
        (parts[0], now.minute()),
        (parts[1], now.hour()),
        (parts[2], now.day()),
        (parts[3], now.month()),
        (parts[4], now.weekday().num_days_from_sunday()),
    ];

    checks.iter().all(|(pattern, value)| field_matches(pattern, *value))
}

fn field_matches(pattern: &str, value: u32) -> bool {
    if pattern == "*" {
        return true;
    }
    // Handle */N step values
    if let Some(step) = pattern.strip_prefix("*/") {
        if let Ok(n) = step.parse::<u32>() {
            return n > 0 && value % n == 0;
        }
    }
    // Handle comma-separated values
    for part in pattern.split(',') {
        // Handle ranges (e.g., "1-5")
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(s), Ok(e)) = (start.parse::<u32>(), end.parse::<u32>()) {
                if value >= s && value <= e {
                    return true;
                }
            }
        } else if let Ok(n) = part.parse::<u32>() {
            if value == n {
                return true;
            }
        }
    }
    false
}

fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn cron_every_minute() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 2, 28, 9, 30, 0).unwrap();
        assert!(cron_matches_now("* * * * *", &now));
    }

    #[test]
    fn cron_specific_minute() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 2, 28, 9, 30, 0).unwrap();
        assert!(cron_matches_now("30 9 * * *", &now));
        assert!(!cron_matches_now("31 9 * * *", &now));
    }

    #[test]
    fn cron_step() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 2, 28, 9, 30, 0).unwrap();
        assert!(cron_matches_now("*/5 * * * *", &now));  // 30 % 5 == 0
        assert!(!cron_matches_now("*/7 * * * *", &now));  // 30 % 7 != 0
    }

    #[test]
    fn cron_range() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 2, 28, 9, 30, 0).unwrap();
        assert!(cron_matches_now("25-35 * * * *", &now));
        assert!(!cron_matches_now("31-35 * * * *", &now));
    }

    #[test]
    fn cron_comma_list() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 2, 28, 9, 30, 0).unwrap();
        assert!(cron_matches_now("15,30,45 * * * *", &now));
        assert!(!cron_matches_now("15,45 * * * *", &now));
    }

    #[test]
    fn field_matches_star() {
        assert!(field_matches("*", 0));
        assert!(field_matches("*", 59));
    }

    #[test]
    fn field_matches_exact() {
        assert!(field_matches("5", 5));
        assert!(!field_matches("5", 6));
    }
}
