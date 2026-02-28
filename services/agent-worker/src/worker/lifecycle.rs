//! Full execution lifecycle: spawn container -> stream logs -> collect result -> cleanup.

use bollard::Docker;
use bollard::container::LogsOptions;
use deadpool_redis::Pool;
use futures_util::StreamExt;

use agent_types::{
    AgentDefinition, AgentId, ExecutionId, ExecutionResult, InboundMessage, JobPayload, LogEntry,
};
use crate::config::WorkerConfig;
use crate::error::WorkerError;
use crate::redis::{lock, pubsub, queue, state};
use super::{sandbox, secrets, workspace_git};

/// Outcome of a successful container run (exit 0 or non-zero).
/// Timeout and infra errors are still `Err(WorkerError)`.
struct RunOutcome {
    exit_code: i64,
    /// Parsed execution result (only present when exit_code == 0).
    result: Option<ExecutionResult>,
    /// Model name from the resolved agent definition.
    model: String,
}

/// Run the full lifecycle for a single execution job.
///
/// The outer `run` guarantees cleanup (lock release, processing removal,
/// token revocation, TTL setting, done log) regardless of success/failure.
pub async fn run(
    job: JobPayload,
    raw_payload: &str,
    pool: &Pool,
    docker: &Docker,
    http_client: &reqwest::Client,
    config: &WorkerConfig,
    worker_id: &str,
) -> Result<(), WorkerError> {
    let execution_id = job.execution_id;
    let agent_id = job.agent_id;

    let result = run_inner(&job, pool, docker, http_client, config, worker_id).await;

    // ── Always cleanup ──────────────────────────────────────────────
    lock::release(pool, agent_id, worker_id).await.ok();
    queue::remove_from_processing(pool, worker_id, raw_payload).await.ok();
    secrets::revoke_tokens(pool, execution_id).await.ok();

    let max_log = config.max_log_entries_redis;

    if let Err(ref e) = result {
        state::set_failed(pool, execution_id, &e.to_string()).await.ok();
        pubsub::publish_log(
            pool,
            execution_id,
            &LogEntry::error(execution_id, &e.to_string()),
            max_log,
        ).await.ok();
    }

    // Flush logs from Redis to disk before they expire
    flush_logs_to_disk(pool, config, agent_id, execution_id).await;

    // Read proxy-accumulated usage (best-effort)
    let proxy_usage = match state::get_proxy_usage(pool, execution_id).await {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(execution_id = %execution_id, error = %e, "failed to read proxy usage");
            None
        }
    };

    // Commit workspace changes to git (best-effort)
    {
        let ws_root = config.workspace_root.clone();
        let aid = agent_id.to_string();
        let eid = execution_id.to_string();

        let (status, summary) = build_commit_summary(&result, config, proxy_usage.as_ref());

        let commit_result = tokio::task::spawn_blocking(move || {
            let repo = workspace_git::init_or_open(&ws_root, &aid)?;
            workspace_git::commit_execution(&repo, &eid, status, &summary)
        }).await;
        match commit_result {
            Ok(Ok(Some(oid))) => tracing::debug!(execution_id = %execution_id, oid = %oid, "committed workspace"),
            Ok(Ok(None)) => tracing::debug!(execution_id = %execution_id, "no workspace changes to commit"),
            Ok(Err(e)) => tracing::warn!(execution_id = %execution_id, error = %e, "failed to commit workspace"),
            Err(e) => tracing::warn!(execution_id = %execution_id, error = %e, "workspace commit task panicked"),
        }
    }

    // Set TTLs on state + log history so they expire after 7 days
    state::set_cleanup_ttls(pool, execution_id).await.ok();

    // Always publish done
    pubsub::publish_log(pool, execution_id, &LogEntry::done(execution_id), max_log).await.ok();

    result.map(|_| ())
}

async fn run_inner(
    job: &JobPayload,
    pool: &Pool,
    docker: &Docker,
    http_client: &reqwest::Client,
    config: &WorkerConfig,
    worker_id: &str,
) -> Result<RunOutcome, WorkerError> {
    let execution_id = job.execution_id;
    let agent_id = job.agent_id;
    let workspace_id = job.workspace_id;

    tracing::info!(execution_id = %execution_id, agent_id = %agent_id, "starting lifecycle");

    // 1. Fetch resolved agent definition from api-rs (merges parent chain)
    let agent_def: AgentDefinition = http_client
        .get(format!(
            "{}/v1/agents/{}/resolved?workspace_id={}",
            config.api_base_url, agent_id, workspace_id
        ))
        .send()
        .await?
        .error_for_status()
        .map_err(|_| WorkerError::AgentNotFound(agent_id))?
        .json()
        .await?;

    // 2. Fetch message from api-rs (or synthesize for cron)
    let message: InboundMessage = if let Some(message_id) = job.message_id {
        http_client
            .get(format!(
                "{}/v1/workspaces/{}/agents/{}/messages/{}",
                config.api_base_url, workspace_id, agent_id, message_id
            ))
            .send()
            .await?
            .error_for_status()
            .map_err(|e| WorkerError::Internal(format!("fetch message: {e}")))?
            .json()
            .await?
    } else {
        InboundMessage::cron_trigger(agent_id)
    };

    // 3. Create opaque tokens for secrets (if any)
    let token_map = secrets::create_token_map(pool, execution_id, &std::collections::HashMap::new()).await?;

    // 4. Serialize and rewrite config
    let agent_config_json = serde_json::to_string(
        &agent_def.sanitize_for_container().map_err(|e| WorkerError::Json(e))?
    )?;
    let safe_config_json = secrets::rewrite_secret_refs(&agent_config_json, &token_map);
    let message_json = serde_json::to_string(&message)?;

    // 5. Ensure workspace directory
    let agent_id_str = agent_id.to_string();
    sandbox::ensure_workspace_dir(&config.workspace_root, &agent_id_str)?;

    // 5b. Ensure workspace is a git repo (best-effort)
    {
        let ws_root = config.workspace_root.clone();
        let aid = agent_id_str.clone();
        match tokio::task::spawn_blocking(move || workspace_git::init_or_open(&ws_root, &aid)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => tracing::warn!(agent_id = %agent_id, error = %e, "failed to init workspace git repo"),
            Err(e) => tracing::warn!(agent_id = %agent_id, error = %e, "workspace git init task panicked"),
        }
    }

    // 6. Build container config
    let (create_opts, container_config) = sandbox::build_container_config(
        &agent_id_str,
        &safe_config_json,
        &message_json,
        &execution_id.to_string(),
        &agent_def,
        config,
    );

    // 7. Create and start container
    let container = docker.create_container(Some(create_opts), container_config).await?;
    let container_id = container.id;
    docker.start_container::<String>(&container_id, None).await?;

    tracing::info!(execution_id = %execution_id, container_id = %container_id, "container started");

    // 8. Update execution state to running
    state::set_running(pool, execution_id, &container_id).await?;

    // 9. Spawn lock renewal task
    let renew_pool = pool.clone();
    let renew_worker_id = worker_id.to_owned();
    let renew_token = tokio_util::sync::CancellationToken::new();
    let renew_token_clone = renew_token.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(200));
        interval.tick().await; // consume the immediate first tick
        loop {
            tokio::select! {
                _ = renew_token_clone.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(e) = lock::renew(&renew_pool, agent_id, &renew_worker_id).await {
                        tracing::warn!(error = %e, "lock renewal failed");
                        break;
                    }
                }
            }
        }
    });

    // 10. Wait for container exit with timeout.
    //     Use .min() to enforce the worker's cap — an agent cannot exceed it.
    let timeout = std::time::Duration::from_secs(
        agent_def.sandbox.timeout_seconds.min(config.runtime_timeout_seconds)
    );
    let wait_result = tokio::time::timeout(
        timeout,
        docker.wait_container::<String>(&container_id, None).collect::<Vec<_>>(),
    ).await;

    // Cancel lock renewal
    renew_token.cancel();

    let exit_code = match wait_result {
        Ok(results) => {
            results.last()
                .and_then(|r| r.as_ref().ok())
                .map(|r| r.status_code)
                .unwrap_or(-1)
        }
        Err(_) => {
            // Timeout — kill container. Don't call set_failed here;
            // the outer `run` handles it via the returned error.
            tracing::warn!(execution_id = %execution_id, "execution timed out, killing container");
            docker.kill_container::<String>(&container_id, None).await.ok();
            docker.remove_container(&container_id, None).await.ok();
            return Err(WorkerError::Timeout(execution_id));
        }
    };

    // 11. Collect stdout (last line should be JSON result)
    let logs: Vec<_> = docker.logs::<String>(
        &container_id,
        Some(LogsOptions {
            stdout: true,
            stderr: false,
            ..Default::default()
        }),
    ).collect().await;

    let stdout: String = logs.into_iter()
        .filter_map(|r| r.ok())
        .map(|chunk| chunk.to_string())
        .collect();

    // 12. Remove container
    docker.remove_container(&container_id, None).await.ok();

    // 13. Parse result
    let model = agent_def.model.clone();
    if exit_code == 0 {
        let result = parse_result(&stdout, &config.workspace_root, &agent_id_str, execution_id).await;
        state::set_completed(pool, execution_id, &result).await?;
        tracing::info!(
            execution_id = %execution_id,
            success = result.success,
            turns = result.turns,
            "execution completed"
        );
        Ok(RunOutcome { exit_code, result: Some(result), model })
    } else {
        let reason = format!("container exited with code {exit_code}");
        state::set_failed(pool, execution_id, &reason).await?;
        tracing::warn!(execution_id = %execution_id, exit_code, "container failed");
        Ok(RunOutcome { exit_code, result: None, model })
    }
}

/// Build a rich commit summary from the execution outcome.
///
/// Returns `(status, summary_body)` where status is a `&'static str` for the
/// first line and summary_body contains the metadata lines.
///
/// When proxy usage is available, it is preferred over container-reported usage
/// since it captures the actual upstream API calls. Per-model breakdowns are
/// shown when multiple models were used.
fn build_commit_summary(
    result: &Result<RunOutcome, WorkerError>,
    config: &WorkerConfig,
    proxy_usage: Option<&state::ProxyUsage>,
) -> (&'static str, String) {
    match result {
        Ok(outcome) => {
            let status = if outcome.exit_code == 0 { "completed" } else { "failed" };
            let mut lines = vec![format!("exit_code: {}", outcome.exit_code)];

            if let Some(ref r) = outcome.result {
                lines.push(format!("duration: {:.1}s", r.duration_seconds));
                lines.push(format!("turns: {}", r.turns));
            }

            if let Some(pu) = proxy_usage {
                // Per-model breakdown from proxy-accumulated usage
                for mu in &pu.models {
                    lines.push(format!(
                        "{}: {} in / {} out / ${:.4}",
                        mu.model, mu.prompt_tokens, mu.completion_tokens, mu.cost
                    ));
                }
                lines.push(format!("total_cost: ${:.4}", pu.total_cost));
                lines.push("usage_source: proxy".to_owned());
            } else {
                // Fallback to container-reported usage
                lines.push(format!("model: {}", outcome.model));
                if let Some(ref r) = outcome.result {
                    lines.push(format!("input_tokens: {}", r.input_tokens));
                    lines.push(format!("output_tokens: {}", r.output_tokens));
                    if let Some(cost) = config.cost_for_model(&outcome.model, r.input_tokens, r.output_tokens) {
                        lines.push(format!("cost: ${cost:.4}"));
                    }
                }
                lines.push("usage_source: container".to_owned());
            }

            (status, lines.join("\n"))
        }
        Err(e) => {
            let status = if matches!(e, WorkerError::Timeout(_)) { "timeout" } else { "error" };
            (status, e.to_string())
        }
    }
}

/// Parse the execution result from container stdout or fallback to .result.json.
/// Uses async I/O to avoid blocking the tokio runtime thread.
async fn parse_result(
    stdout: &str,
    workspace_root: &str,
    agent_id: &str,
    _execution_id: ExecutionId,
) -> ExecutionResult {
    // Try last non-empty line of stdout as JSON
    if let Some(last_line) = stdout.lines().rev().find(|l| !l.trim().is_empty()) {
        if let Ok(result) = serde_json::from_str::<ExecutionResult>(last_line) {
            return result;
        }
    }

    // Fallback: read .result.json from workspace (async I/O)
    let result_path = format!("{workspace_root}/{agent_id}/.result.json");
    if let Ok(content) = tokio::fs::read_to_string(&result_path).await {
        if let Ok(result) = serde_json::from_str::<ExecutionResult>(&content) {
            return result;
        }
    }

    // Final fallback
    ExecutionResult {
        success: false,
        reason: Some("could not parse execution result".to_owned()),
        final_response: if stdout.is_empty() { None } else { Some(stdout.to_owned()) },
        tool_calls_count: 0,
        turns: 0,
        input_tokens: 0,
        output_tokens: 0,
        duration_seconds: 0.0,
    }
}

/// Flush log entries from Redis history to a JSONL file in the agent workspace.
///
/// Writes to `{workspace_root}/{agent_id}/logs/{execution_id}.jsonl`.
/// Best-effort: failures are logged but don't fail the execution.
async fn flush_logs_to_disk(
    pool: &Pool,
    config: &WorkerConfig,
    agent_id: AgentId,
    execution_id: ExecutionId,
) {
    let entries = match pubsub::get_log_history(pool, execution_id).await {
        Ok(e) if !e.is_empty() => e,
        Ok(_) => return,
        Err(e) => {
            tracing::warn!(execution_id = %execution_id, error = %e, "failed to read log history for disk flush");
            return;
        }
    };

    let logs_dir = format!("{}/{}/logs", config.workspace_root, agent_id);
    if let Err(e) = tokio::fs::create_dir_all(&logs_dir).await {
        tracing::warn!(error = %e, "failed to create logs dir: {logs_dir}");
        return;
    }

    let log_path = format!("{logs_dir}/{execution_id}.jsonl");
    let content = entries.join("\n") + "\n";
    if let Err(e) = tokio::fs::write(&log_path, content.as_bytes()).await {
        tracing::warn!(execution_id = %execution_id, error = %e, "failed to flush logs to disk");
    } else {
        tracing::debug!(execution_id = %execution_id, entries = entries.len(), "flushed logs to {log_path}");
    }
}
