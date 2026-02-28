//! Execution state stored in Redis HASHes.
//!
//! All state keys get a 7-day TTL to prevent unbounded Redis memory growth.

use std::collections::HashMap;

use agent_types::{AgentId, ExecutionId, ExecutionStatus, MessageId, WorkspaceId};
use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;

use crate::error::WorkerError;
use super::keys;

/// TTL for execution state and result: 7 days.
const STATE_TTL_SECS: u64 = 7 * 24 * 3600;

/// TTL for log history in Redis: 24 hours.
/// Durable logs are flushed to disk; Redis copy is only for live replay.
const LOG_TTL_SECS: u64 = 24 * 3600;

/// Typed execution state parsed from the Redis hash.
#[derive(Debug, Clone)]
pub struct ExecutionState {
    pub status: ExecutionStatus,
    pub container_id: Option<String>,
    pub reason: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl ExecutionState {
    /// Parse from a raw Redis hash.
    fn from_hash(map: &HashMap<String, String>) -> Result<Self, WorkerError> {
        let status_str = map.get("status")
            .ok_or_else(|| WorkerError::Internal("execution state missing 'status' field".to_owned()))?;
        let status: ExecutionStatus = status_str.parse()
            .map_err(|_| WorkerError::Internal(format!("invalid execution status: {status_str}")))?;
        Ok(Self {
            status,
            container_id: map.get("container_id").filter(|s| !s.is_empty()).cloned(),
            reason: map.get("reason").cloned(),
            created_at: map.get("created_at").cloned(),
            started_at: map.get("started_at").cloned(),
            completed_at: map.get("completed_at").cloned(),
        })
    }
}

/// Set execution status.
pub async fn set_status(
    pool: &Pool,
    execution_id: ExecutionId,
    status: ExecutionStatus,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    conn.hset::<_, _, _, ()>(&key, "status", status.as_str()).await?;
    conn.expire::<_, ()>(&key, STATE_TTL_SECS as i64).await?;
    Ok(())
}

/// Set execution to running with container ID (atomic pipeline).
pub async fn set_running(
    pool: &Pool,
    execution_id: ExecutionId,
    container_id: &str,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    let started_at = chrono::Utc::now().to_rfc3339();
    deadpool_redis::redis::pipe()
        .hset(&key, "status", ExecutionStatus::Running.as_str()).ignore()
        .hset(&key, "container_id", container_id).ignore()
        .hset(&key, "started_at", &started_at).ignore()
        .expire(&key, STATE_TTL_SECS as i64).ignore()
        .query_async::<()>(&mut *conn)
        .await?;
    Ok(())
}

/// Mark execution as completed and store result (atomic pipeline).
pub async fn set_completed(
    pool: &Pool,
    execution_id: ExecutionId,
    result: &agent_types::ExecutionResult,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let state_key = keys::exec_state(execution_id);
    let result_key = keys::exec_result(execution_id);
    let result_json = serde_json::to_string(result)?;
    let completed_at = chrono::Utc::now().to_rfc3339();

    deadpool_redis::redis::pipe()
        .hset(&state_key, "status", ExecutionStatus::Completed.as_str()).ignore()
        .hset(&state_key, "completed_at", &completed_at).ignore()
        .expire(&state_key, STATE_TTL_SECS as i64).ignore()
        .set_ex(&result_key, &result_json, STATE_TTL_SECS).ignore()
        .query_async::<()>(&mut *conn)
        .await?;

    Ok(())
}

/// Mark execution as failed (atomic pipeline).
pub async fn set_failed(
    pool: &Pool,
    execution_id: ExecutionId,
    reason: &str,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    let completed_at = chrono::Utc::now().to_rfc3339();
    deadpool_redis::redis::pipe()
        .hset(&key, "status", ExecutionStatus::Failed.as_str()).ignore()
        .hset(&key, "reason", reason).ignore()
        .hset(&key, "completed_at", &completed_at).ignore()
        .expire(&key, STATE_TTL_SECS as i64).ignore()
        .query_async::<()>(&mut *conn)
        .await?;
    Ok(())
}

/// Get typed execution state.
pub async fn get_state(
    pool: &Pool,
    execution_id: ExecutionId,
) -> Result<ExecutionState, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    let fields: HashMap<String, String> = conn.hgetall(&key).await?;
    if fields.is_empty() {
        return Err(WorkerError::ExecutionNotFound(execution_id));
    }
    ExecutionState::from_hash(&fields)
}

/// Get execution result JSON.
pub async fn get_result(
    pool: &Pool,
    execution_id: ExecutionId,
) -> Result<Option<agent_types::ExecutionResult>, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_result(execution_id);
    let json: Option<String> = conn.get(&key).await?;
    match json {
        Some(s) => Ok(Some(serde_json::from_str(&s)?)),
        None => Ok(None),
    }
}

/// Initialize execution state when enqueued (atomic pipeline).
pub async fn init_queued(
    pool: &Pool,
    execution_id: ExecutionId,
    agent_id: AgentId,
    workspace_id: WorkspaceId,
    message_id: Option<MessageId>,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    let created_at = chrono::Utc::now().to_rfc3339();
    let agent_str = agent_id.to_string();
    let workspace_str = workspace_id.to_string();

    let mut pipe = deadpool_redis::redis::pipe();
    pipe.hset(&key, "status", ExecutionStatus::Queued.as_str()).ignore()
        .hset(&key, "agent_id", &agent_str).ignore()
        .hset(&key, "workspace_id", &workspace_str).ignore()
        .hset(&key, "created_at", &created_at).ignore();
    if let Some(mid) = message_id {
        pipe.hset(&key, "message_id", mid.to_string()).ignore();
    }
    pipe.expire(&key, STATE_TTL_SECS as i64).ignore()
        .query_async::<()>(&mut *conn)
        .await?;

    Ok(())
}

/// Set cleanup TTLs on execution state and log history after execution ends.
///
/// Execution state/result gets 7-day TTL; log history gets 24-hour TTL since
/// durable logs are flushed to disk in the agent workspace.
pub async fn set_cleanup_ttls(pool: &Pool, execution_id: ExecutionId) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    conn.expire::<_, ()>(&keys::exec_state(execution_id), STATE_TTL_SECS as i64).await?;
    conn.expire::<_, ()>(&keys::logs_history(execution_id), LOG_TTL_SECS as i64).await?;
    conn.expire::<_, ()>(&keys::usage(execution_id), STATE_TTL_SECS as i64).await?;
    Ok(())
}

/// Per-model LLM usage accumulated by the proxy.
#[derive(Debug, Clone)]
pub struct ModelUsage {
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost: f64,
}

/// Aggregate LLM usage accumulated by the proxy for an execution.
#[derive(Debug, Clone)]
pub struct ProxyUsage {
    pub total_cost: f64,
    pub request_count: u64,
    pub models: Vec<ModelUsage>,
}

/// Read proxy-accumulated usage from Redis.
///
/// Returns `None` if no usage has been recorded (key doesn't exist).
pub async fn get_proxy_usage(
    pool: &Pool,
    execution_id: ExecutionId,
) -> Result<Option<ProxyUsage>, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::usage(execution_id);
    let fields: HashMap<String, String> = conn.hgetall(&key).await?;
    if fields.is_empty() {
        return Ok(None);
    }

    let total_cost: f64 = fields
        .get("total_cost")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let request_count: u64 = fields
        .get("request_count")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // Parse per-model entries from fields like "model:{name}:prompt_tokens"
    let mut model_map: HashMap<String, ModelUsage> = HashMap::new();
    for (field, value) in &fields {
        if let Some(rest) = field.strip_prefix("model:") {
            if let Some((model_name, metric)) = rest.rsplit_once(':') {
                let entry = model_map.entry(model_name.to_owned()).or_insert_with(|| ModelUsage {
                    model: model_name.to_owned(),
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    cost: 0.0,
                });
                match metric {
                    "prompt_tokens" => entry.prompt_tokens = value.parse().unwrap_or(0),
                    "completion_tokens" => entry.completion_tokens = value.parse().unwrap_or(0),
                    "cost" => entry.cost = value.parse().unwrap_or(0.0),
                    _ => {}
                }
            }
        }
    }

    let mut models: Vec<ModelUsage> = model_map.into_values().collect();
    models.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));

    Ok(Some(ProxyUsage {
        total_cost,
        request_count,
        models,
    }))
}
