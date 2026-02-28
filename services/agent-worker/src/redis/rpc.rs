//! Redis-based RPC: request/reply pattern for synchronous confirmation.
//!
//! Flow:
//! 1. api-rs: RPUSH job to queue, then BLPOP on aw:rpc:reply:{job_id} (2s timeout)
//! 2. worker: BLPOP job, validates, RPUSH ack to aw:rpc:reply:{job_id}
//! 3. api-rs: receives ack, returns to HTTP caller
//!
//! All operations are sub-millisecond Redis reads/writes.

use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::domain::job::JobPayload;
use crate::error::WorkerError;
use super::keys;

/// RPC reply from worker to api-rs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcReply {
    pub status: RpcStatus,
    pub execution_id: String,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcStatus {
    /// Job accepted and will be executed.
    Accepted,
    /// Job rejected (e.g. agent locked, budget exceeded).
    Rejected,
    /// Kill command acknowledged.
    Killed,
    /// Execution not found or already finished.
    NotFound,
}

/// Send an RPC reply (worker side).
/// Sets a 30s TTL so stale replies auto-cleanup.
pub async fn send_reply(pool: &Pool, job_id: &str, reply: &RpcReply) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::rpc_reply(job_id);
    let payload = serde_json::to_string(reply)?;
    conn.rpush::<_, _, ()>(&key, &payload).await?;
    conn.expire::<_, ()>(&key, 30).await?;
    Ok(())
}

/// Wait for an RPC reply (api-rs side).
/// Returns None if timeout expires (worker may be down).
pub async fn wait_reply(pool: &Pool, job_id: &str, timeout_secs: f64) -> Result<Option<RpcReply>, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::rpc_reply(job_id);
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
pub async fn enqueue_and_wait(
    pool: &Pool,
    job: &JobPayload,
    timeout_secs: f64,
) -> Result<Option<RpcReply>, WorkerError> {
    let mut conn = pool.get().await?;
    let payload = serde_json::to_string(job)?;
    conn.rpush::<_, _, ()>(keys::QUEUE_JOBS, &payload).await?;
    drop(conn);

    wait_reply(pool, &job.job_id.to_string(), timeout_secs).await
}

/// Kill command sent via pub/sub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillCommand {
    pub execution_id: String,
    pub reply_key: String,
}

/// Publish a kill command (api-rs side).
pub async fn publish_kill(pool: &Pool, execution_id: &str) -> Result<String, WorkerError> {
    let mut conn = pool.get().await?;
    let reply_id = uuid::Uuid::new_v4().to_string();
    let cmd = KillCommand {
        execution_id: execution_id.to_owned(),
        reply_key: keys::rpc_reply(&reply_id),
    };
    let payload = serde_json::to_string(&cmd)?;
    conn.publish::<_, _, ()>(keys::CMD_KILL, &payload).await?;
    Ok(reply_id)
}
