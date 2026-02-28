//! Execution state stored in Redis HASHes.

use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;
use std::collections::HashMap;

use crate::domain::execution::{ExecutionResult, ExecutionStatus};
use crate::error::WorkerError;
use super::keys;

/// Set execution status and optional fields.
pub async fn set_status(
    pool: &Pool,
    execution_id: &str,
    status: ExecutionStatus,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    conn.hset::<_, _, _, ()>(&key, "status", status.as_str()).await?;
    Ok(())
}

/// Set execution to running with container ID.
pub async fn set_running(
    pool: &Pool,
    execution_id: &str,
    container_id: &str,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    conn.hset_multiple::<_, _, _, ()>(&key, &[
        ("status", "running"),
        ("container_id", container_id),
        ("started_at", &chrono::Utc::now().to_rfc3339()),
    ]).await?;
    Ok(())
}

/// Mark execution as completed and store result.
pub async fn set_completed(
    pool: &Pool,
    execution_id: &str,
    result: &ExecutionResult,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let state_key = keys::exec_state(execution_id);
    let result_key = keys::exec_result(execution_id);
    let result_json = serde_json::to_string(result)?;

    conn.hset_multiple::<_, _, _, ()>(&state_key, &[
        ("status", "completed"),
        ("completed_at", &chrono::Utc::now().to_rfc3339()),
    ]).await?;

    // Store result with 7-day TTL
    conn.set_ex::<_, _, ()>(&result_key, &result_json, 604_800).await?;

    Ok(())
}

/// Mark execution as failed.
pub async fn set_failed(
    pool: &Pool,
    execution_id: &str,
    reason: &str,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    conn.hset_multiple::<_, _, _, ()>(&key, &[
        ("status", "failed"),
        ("reason", reason),
        ("completed_at", &chrono::Utc::now().to_rfc3339()),
    ]).await?;
    Ok(())
}

/// Get execution state as a HashMap.
pub async fn get_state(
    pool: &Pool,
    execution_id: &str,
) -> Result<HashMap<String, String>, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    let fields: HashMap<String, String> = conn.hgetall(&key).await?;
    Ok(fields)
}

/// Get execution result JSON.
pub async fn get_result(
    pool: &Pool,
    execution_id: &str,
) -> Result<Option<ExecutionResult>, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_result(execution_id);
    let json: Option<String> = conn.get(&key).await?;
    match json {
        Some(s) => Ok(Some(serde_json::from_str(&s)?)),
        None => Ok(None),
    }
}

/// Initialize execution state when enqueued.
pub async fn init_queued(
    pool: &Pool,
    execution_id: &str,
    agent_id: &str,
    message_id: &str,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::exec_state(execution_id);
    conn.hset_multiple::<_, _, _, ()>(&key, &[
        ("status", "queued"),
        ("agent_id", agent_id),
        ("message_id", message_id),
        ("created_at", &chrono::Utc::now().to_rfc3339()),
    ]).await?;
    Ok(())
}
