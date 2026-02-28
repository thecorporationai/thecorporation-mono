//! Job queue operations using Redis lists.
//!
//! Uses BLMOVE for atomic pop-and-move to the processing list,
//! eliminating the window where a job could be lost on crash.

use agent_types::JobPayload;
use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;

use crate::error::WorkerError;
use super::keys;

/// Get the current queue length.
pub async fn queue_len(pool: &Pool) -> Result<u64, WorkerError> {
    let mut conn = pool.get().await?;
    let len: u64 = conn.llen(keys::QUEUE_JOBS).await?;
    Ok(len)
}

/// Push a job onto the queue, rejecting if the queue is at capacity.
pub async fn enqueue_job(pool: &Pool, job: &JobPayload, max_depth: u64) -> Result<(), WorkerError> {
    if max_depth > 0 {
        let current = queue_len(pool).await?;
        if current >= max_depth {
            return Err(WorkerError::QueueFull { current, max: max_depth });
        }
    }
    let mut conn = pool.get().await?;
    let payload = serde_json::to_string(job)?;
    conn.rpush::<_, _, ()>(keys::QUEUE_JOBS, &payload).await?;
    Ok(())
}

/// Atomically pop a job from the queue and push to the processing list.
///
/// Uses BLMOVE (Redis >= 6.2) so the job is always in exactly one list.
/// Returns `None` if the timeout expires with no job.
pub async fn dequeue_to_processing(
    pool: &Pool,
    worker_id: &str,
    timeout_secs: f64,
) -> Result<Option<(String, JobPayload)>, WorkerError> {
    let mut conn = pool.get().await?;
    let processing = keys::queue_processing(worker_id);
    let result: Option<String> = deadpool_redis::redis::cmd("BLMOVE")
        .arg(keys::QUEUE_JOBS)
        .arg(&processing)
        .arg("LEFT")
        .arg("RIGHT")
        .arg(timeout_secs)
        .query_async(&mut *conn)
        .await?;

    match result {
        Some(raw) => {
            let job: JobPayload = serde_json::from_str(&raw)?;
            Ok(Some((raw, job)))
        }
        None => Ok(None),
    }
}

/// Move a rejected job from processing back to the queue.
///
/// Uses a pipeline to send both commands in one round trip.
pub async fn reject_to_queue(pool: &Pool, worker_id: &str, raw_payload: &str) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let processing = keys::queue_processing(worker_id);
    deadpool_redis::redis::cmd("RPUSH")
        .arg(keys::QUEUE_JOBS)
        .arg(raw_payload)
        .query_async::<()>(&mut *conn)
        .await?;
    conn.lrem::<_, _, ()>(&processing, 1, raw_payload).await?;
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
