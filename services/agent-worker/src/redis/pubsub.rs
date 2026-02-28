//! Redis Pub/Sub for real-time log streaming.

use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;

use crate::domain::log_entry::LogEntry;
use crate::error::WorkerError;
use super::keys;

/// Publish a log entry to the Pub/Sub channel and persist to history list.
pub async fn publish_log(pool: &Pool, execution_id: &str, entry: &LogEntry) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let payload = serde_json::to_string(entry)?;

    // Persist to history list (for replay on reconnect)
    conn.rpush::<_, _, ()>(&keys::logs_history(execution_id), &payload).await?;

    // Publish to live subscribers
    conn.publish::<_, _, ()>(&keys::logs_channel(execution_id), &payload).await?;

    Ok(())
}

/// Get all persisted log entries for an execution (for replay).
pub async fn get_log_history(pool: &Pool, execution_id: &str) -> Result<Vec<String>, WorkerError> {
    let mut conn = pool.get().await?;
    let entries: Vec<String> = conn.lrange(&keys::logs_history(execution_id), 0, -1).await?;
    Ok(entries)
}
