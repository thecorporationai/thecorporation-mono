//! Full execution lifecycle: spawn container -> stream logs -> collect result -> cleanup.

use std::sync::Arc;

use bollard::Docker;
use bollard::container::LogsOptions;
use deadpool_redis::Pool;
use futures_util::StreamExt;

use crate::config::WorkerConfig;
use crate::domain::execution::ExecutionResult;
use crate::domain::job::JobPayload;
use crate::domain::log_entry::LogEntry;
use crate::error::WorkerError;
use crate::redis::{lock, pubsub, queue, state};
use super::{sandbox, secrets};

/// Run the full lifecycle for a single execution job.
pub async fn run(
    job: JobPayload,
    raw_payload: &str,
    pool: &Pool,
    docker: &Docker,
    config: &WorkerConfig,
    worker_id: &str,
) -> Result<(), WorkerError> {
    let execution_id = job.execution_id.to_string();
    let agent_id = job.agent_id.to_string();

    let result = run_inner(&job, pool, docker, config, worker_id).await;

    // Always cleanup: release lock, remove from processing
    lock::release(pool, &agent_id, worker_id).await.ok();
    queue::remove_from_processing(pool, worker_id, raw_payload).await.ok();

    if let Err(ref e) = result {
        state::set_failed(pool, &execution_id, &e.to_string()).await.ok();
        pubsub::publish_log(
            pool,
            &execution_id,
            &LogEntry::error(&execution_id, &e.to_string()),
        ).await.ok();
    }

    // Always publish done
    pubsub::publish_log(pool, &execution_id, &LogEntry::done(&execution_id)).await.ok();

    result
}

async fn run_inner(
    job: &JobPayload,
    pool: &Pool,
    docker: &Docker,
    config: &WorkerConfig,
    worker_id: &str,
) -> Result<(), WorkerError> {
    let execution_id = job.execution_id.to_string();
    let agent_id = job.agent_id.to_string();
    let workspace_id = job.workspace_id.to_string();

    tracing::info!(execution_id = %execution_id, agent_id = %agent_id, "starting lifecycle");

    // 1. Fetch resolved agent definition from api-rs (merges parent chain)
    let client = reqwest::Client::new();
    let agent_def: crate::domain::agent::AgentDefinition = client
        .get(format!(
            "{}/v1/agents/{}/resolved?workspace_id={}",
            config.api_base_url, agent_id, workspace_id
        ))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| WorkerError::AgentNotFound(format!("{agent_id}: {e}")))?
        .json()
        .await?;

    // 2. Fetch message from api-rs
    let message: crate::domain::message::InboundMessage = client
        .get(format!(
            "{}/v1/workspaces/{}/agents/{}/messages/{}",
            config.api_base_url, workspace_id, agent_id, job.message_id
        ))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| WorkerError::Internal(format!("fetch message: {e}")))?
        .json()
        .await?;

    // 3. Create opaque tokens for secrets (if any)
    // TODO: fetch decrypted secrets from api-rs secure endpoint
    let token_map = secrets::create_token_map(pool, &execution_id, &std::collections::HashMap::new()).await?;

    // 4. Serialize and rewrite config
    let agent_config_json = serde_json::to_string(&agent_def.sanitize_for_container())?;
    let safe_config_json = secrets::rewrite_secret_refs(&agent_config_json, &token_map);
    let message_json = serde_json::to_string(&message)?;

    // 5. Ensure workspace directory
    sandbox::ensure_workspace_dir(&config.workspace_root, &agent_id)?;

    // 6. Build container config
    let (create_opts, container_config) = sandbox::build_container_config(
        &agent_id,
        &safe_config_json,
        &message_json,
        &execution_id,
        &agent_def,
        config,
    );

    // 7. Create and start container
    let container = docker.create_container(Some(create_opts), container_config).await?;
    let container_id = container.id;
    docker.start_container::<String>(&container_id, None).await?;

    tracing::info!(execution_id = %execution_id, container_id = %container_id, "container started");

    // 8. Update execution state to running
    state::set_running(pool, &execution_id, &container_id).await?;

    // 9. Spawn lock renewal task
    let renew_pool = pool.clone();
    let renew_agent_id = agent_id.clone();
    let renew_worker_id = worker_id.to_owned();
    let renew_token = tokio_util::sync::CancellationToken::new();
    let renew_token_clone = renew_token.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(200));
        loop {
            tokio::select! {
                _ = renew_token_clone.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(e) = lock::renew(&renew_pool, &renew_agent_id, &renew_worker_id).await {
                        tracing::warn!(error = %e, "lock renewal failed");
                        break;
                    }
                }
            }
        }
    });

    // 10. Wait for container exit with timeout
    let timeout = std::time::Duration::from_secs(
        agent_def.sandbox.timeout_seconds.max(config.runtime_timeout_seconds)
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
            // Timeout — kill container
            tracing::warn!(execution_id = %execution_id, "execution timed out, killing container");
            docker.kill_container::<String>(&container_id, None).await.ok();
            state::set_failed(pool, &execution_id, "timeout").await?;
            secrets::revoke_tokens(pool, &execution_id).await?;
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
    if exit_code == 0 {
        let result = parse_result(&stdout, &config.workspace_root, &agent_id, &execution_id);
        state::set_completed(pool, &execution_id, &result).await?;
        tracing::info!(
            execution_id = %execution_id,
            success = result.success,
            turns = result.turns,
            "execution completed"
        );
    } else {
        let reason = format!("container exited with code {exit_code}");
        state::set_failed(pool, &execution_id, &reason).await?;
        tracing::warn!(execution_id = %execution_id, exit_code, "container failed");
    }

    // 14. Revoke tokens
    secrets::revoke_tokens(pool, &execution_id).await?;

    Ok(())
}

/// Parse the execution result from container stdout or fallback to .result.json.
fn parse_result(
    stdout: &str,
    workspace_root: &str,
    agent_id: &str,
    execution_id: &str,
) -> ExecutionResult {
    // Try last non-empty line of stdout as JSON
    if let Some(last_line) = stdout.lines().rev().find(|l| !l.trim().is_empty()) {
        if let Ok(result) = serde_json::from_str::<ExecutionResult>(last_line) {
            return result;
        }
    }

    // Fallback: read .result.json from workspace
    let result_path = format!("{workspace_root}/{agent_id}/.result.json");
    if let Ok(content) = std::fs::read_to_string(&result_path) {
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
