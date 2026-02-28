//! Redis Pub/Sub for real-time log streaming.
//!
//! Redis is the short-lived buffer for live streaming and reconnect replay.
//! Durable logs are flushed to disk (agent workspace) at end of execution.

use agent_types::{ExecutionId, LogEntry};
use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;

use crate::error::WorkerError;
use super::keys;

/// Publish a log entry to the Pub/Sub channel and persist to history list.
///
/// `max_entries` controls the LTRIM cap on the history list. Pass 0 to disable.
pub async fn publish_log(
    pool: &Pool,
    execution_id: ExecutionId,
    entry: &LogEntry,
    max_entries: i64,
) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let payload = serde_json::to_string(entry)?;
    let history_key = keys::logs_history(execution_id);

    // Persist to history list (for replay on reconnect)
    conn.rpush::<_, _, ()>(&history_key, &payload).await?;

    // Trim to keep only the last N entries
    if max_entries > 0 {
        conn.ltrim::<_, ()>(&history_key, -max_entries as isize, -1).await?;
    }

    // Publish to live subscribers
    conn.publish::<_, _, ()>(&keys::logs_channel(execution_id), &payload).await?;

    Ok(())
}

/// Get all persisted log entries for an execution (for replay).
pub async fn get_log_history(pool: &Pool, execution_id: ExecutionId) -> Result<Vec<String>, WorkerError> {
    let mut conn = pool.get().await?;
    let entries: Vec<String> = conn.lrange(&keys::logs_history(execution_id), 0, -1).await?;
    Ok(entries)
}
