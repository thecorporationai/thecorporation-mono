//! Cron scheduler — checks agent schedules and enqueues jobs.
//!
//! Runs as a background tokio task with a 60s tick.
//! Uses SipHash for stable schedule hashing across Rust versions.

use std::sync::Arc;

use deadpool_redis::Pool;
use siphasher::sip::SipHasher13;
use tokio_util::sync::CancellationToken;

use agent_types::{AgentId, ExecutionId, JobPayload, WorkspaceId};
use crate::config::WorkerConfig;
use crate::error::WorkerError;
use crate::redis::{keys, queue, state};

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
        let Some(agent_id_str) = &agent.agent_id else { continue };
        let Some(workspace_id_str) = &agent.workspace_id else { continue };

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
            let agent_uuid: AgentId = match agent_id_str.parse() {
                Ok(id) => id,
                Err(_) => continue,
            };
            let schedule_hash = stable_hash(schedule);
            let dedup_key = keys::cron_last_fire(agent_uuid, &schedule_hash);
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

            let ws_uuid: WorkspaceId = match workspace_id_str.parse() {
                Ok(id) => id,
                Err(_) => continue,
            };

            let execution_id = ExecutionId::new();
            let job = JobPayload::cron(execution_id, agent_uuid, ws_uuid);

            // Initialize execution state for cron jobs (api-rs does it for message jobs)
            state::init_queued(pool, execution_id, agent_uuid, None).await?;

            tracing::info!(
                agent_id = agent_id_str.as_str(),
                schedule,
                execution_id = %job.execution_id,
                "cron: enqueueing job"
            );
            queue::enqueue_job(pool, &job, config.max_queue_depth).await?;
        }
    }

    Ok(())
}

/// Check if a cron expression matches the current minute.
fn cron_matches_now(expr: &str, now: &chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::{Datelike, Timelike};

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
    if let Some(step) = pattern.strip_prefix("*/") {
        if let Ok(n) = step.parse::<u32>() {
            return n > 0 && value % n == 0;
        }
    }
    for part in pattern.split(',') {
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

/// Stable hash using SipHash-1-3 with a fixed key.
/// Unlike `DefaultHasher`, output is deterministic across Rust versions.
fn stable_hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = SipHasher13::new_with_keys(0xdead_beef_cafe_babe, 0x0123_4567_89ab_cdef);
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
        assert!(cron_matches_now("*/5 * * * *", &now));
        assert!(!cron_matches_now("*/7 * * * *", &now));
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

    #[test]
    fn stable_hash_deterministic() {
        let a = stable_hash("*/5 * * * *");
        let b = stable_hash("*/5 * * * *");
        assert_eq!(a, b);
        // Different input → different hash
        let c = stable_hash("*/10 * * * *");
        assert_ne!(a, c);
    }
}
