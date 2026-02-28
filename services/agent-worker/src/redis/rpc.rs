//! Redis-based RPC: request/reply pattern for synchronous confirmation.
//!
//! Flow:
//! 1. api-rs: RPUSH job to queue, then BLPOP on aw:rpc:reply:{job_id} (2s timeout)
//! 2. worker: BLPOP job, validates, RPUSH ack to aw:rpc:reply:{job_id}
//! 3. api-rs: receives ack, returns to HTTP caller

use agent_types::{ExecutionId, JobPayload, KillCommand, RpcReply};
use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;

use crate::error::WorkerError;
use super::keys;

/// Send an RPC reply (worker side).
/// Sets a 30s TTL so stale replies auto-cleanup.
pub async fn send_reply(pool: &Pool, reply_id: &str, reply: &RpcReply) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::rpc_reply(reply_id);
    let payload = serde_json::to_string(reply)?;
    conn.rpush::<_, _, ()>(&key, &payload).await?;
    conn.expire::<_, ()>(&key, 30).await?;
    Ok(())
}

/// Wait for an RPC reply (api-rs side).
/// Returns None if timeout expires (worker may be down).
pub async fn wait_reply(pool: &Pool, reply_id: &str, timeout_secs: f64) -> Result<Option<RpcReply>, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::rpc_reply(reply_id);
    let result: Option<(String, String)> = deadpool_redis::redis::cmd("BLPOP")
        .arg(&key)
        .arg(timeout_secs)
        .query_async(&mut *conn)
        .await?;

    match result {
        Some((_key, payload)) => {
            let reply: RpcReply = serde_json::from_str(&payload)?;
            Ok(Some(reply))
        }
        None => Ok(None),
    }
}

/// Enqueue a job and wait for worker acknowledgment (api-rs side).
/// Pass `max_depth = 0` to disable the queue limit.
pub async fn enqueue_and_wait(
    pool: &Pool,
    job: &JobPayload,
    timeout_secs: f64,
    max_depth: u64,
) -> Result<Option<RpcReply>, WorkerError> {
    if max_depth > 0 {
        let current = super::queue::queue_len(pool).await?;
        if current >= max_depth {
            return Err(WorkerError::QueueFull { current, max: max_depth });
        }
    }
    let mut conn = pool.get().await?;
    let payload = serde_json::to_string(job)?;
    conn.rpush::<_, _, ()>(keys::QUEUE_JOBS, &payload).await?;
    drop(conn);

    wait_reply(pool, &job.job_id.to_string(), timeout_secs).await
}

/// Publish a kill command (api-rs side). Returns the reply_id for wait_reply.
pub async fn publish_kill(pool: &Pool, execution_id: ExecutionId) -> Result<String, WorkerError> {
    let mut conn = pool.get().await?;
    let reply_id = uuid::Uuid::new_v4().to_string();
    let cmd = KillCommand {
        execution_id,
        reply_id: reply_id.clone(),
    };
    let payload = serde_json::to_string(&cmd)?;
    conn.publish::<_, _, ()>(keys::CMD_KILL, &payload).await?;
    Ok(reply_id)
}
