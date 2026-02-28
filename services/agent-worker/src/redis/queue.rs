//! Job queue operations using Redis lists.
//!
//! Implements the reliable-queue pattern:
//! - Enqueue: RPUSH to main queue
//! - Dequeue: BLPOP from main queue
//! - Processing: RPUSH to per-worker processing list
//! - Completion: LREM from processing list

use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;

use crate::domain::job::JobPayload;
use crate::error::WorkerError;
use super::keys;

/// Push a job onto the queue.
pub async fn enqueue_job(pool: &Pool, job: &JobPayload) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let payload = serde_json::to_string(job)?;
    conn.rpush::<_, _, ()>(keys::QUEUE_JOBS, &payload).await?;
    Ok(())
}

/// Blocking pop a job from the queue.
/// Returns `None` if the timeout expires with no job.
pub async fn dequeue_job(pool: &Pool, timeout_secs: f64) -> Result<Option<(String, JobPayload)>, WorkerError> {
    let mut conn = pool.get().await?;
    let result: Option<(String, String)> = deadpool_redis::redis::cmd("BLPOP")
        .arg(keys::QUEUE_JOBS)
        .arg(timeout_secs)
        .query_async(&mut *conn)
        .await?;

    match result {
        Some((_key, payload)) => {
            let job: JobPayload = serde_json::from_str(&payload)?;
            Ok(Some((payload, job)))
        }
        None => Ok(None),
    }
}

/// Move a job to the per-worker processing list (for crash recovery).
pub async fn mark_processing(pool: &Pool, worker_id: &str, raw_payload: &str) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    conn.rpush::<_, _, ()>(&keys::queue_processing(worker_id), raw_payload).await?;
    Ok(())
}

/// Remove a job from the processing list after successful completion.
pub async fn remove_from_processing(pool: &Pool, worker_id: &str, raw_payload: &str) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    conn.lrem::<_, _, ()>(&keys::queue_processing(worker_id), 1, raw_payload).await?;
    Ok(())
}

/// Get all jobs in the processing list (for crash recovery on startup).
pub async fn list_processing(pool: &Pool, worker_id: &str) -> Result<Vec<String>, WorkerError> {
    let mut conn = pool.get().await?;
    let items: Vec<String> = conn.lrange(&keys::queue_processing(worker_id), 0, -1).await?;
    Ok(items)
}
