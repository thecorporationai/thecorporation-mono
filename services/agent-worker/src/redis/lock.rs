//! Distributed per-agent lock using Redis SET NX PX + Lua CAS release.

use agent_types::AgentId;
use deadpool_redis::Pool;
use deadpool_redis::redis::Script;

use crate::error::WorkerError;
use super::keys;

/// Default lock TTL in milliseconds (10 minutes).
const LOCK_TTL_MS: u64 = 600_000;

/// Lua script for safe lock release (CAS: only delete if value matches).
const RELEASE_SCRIPT: &str = r#"
if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("del", KEYS[1])
else
    return 0
end
"#;

/// Lua script for lock renewal (extend TTL only if we still hold it).
const RENEW_SCRIPT: &str = r#"
if redis.call("get", KEYS[1]) == ARGV[1] then
    return redis.call("pexpire", KEYS[1], ARGV[2])
else
    return 0
end
"#;

/// Attempt to acquire a per-agent lock.
/// Returns `true` if the lock was acquired.
pub async fn acquire(pool: &Pool, agent_id: AgentId, worker_id: &str) -> Result<bool, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::lock_agent(agent_id);
    let result: Option<String> = deadpool_redis::redis::cmd("SET")
        .arg(&key)
        .arg(worker_id)
        .arg("NX")
        .arg("PX")
        .arg(LOCK_TTL_MS)
        .query_async(&mut *conn)
        .await?;
    Ok(result.is_some())
}

/// Release the per-agent lock (only if we still own it).
pub async fn release(pool: &Pool, agent_id: AgentId, worker_id: &str) -> Result<bool, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::lock_agent(agent_id);
    let script = Script::new(RELEASE_SCRIPT);
    let result: i64 = script
        .key(&key)
        .arg(worker_id)
        .invoke_async(&mut *conn)
        .await?;
    Ok(result == 1)
}

/// Renew the lock TTL (only if we still own it).
pub async fn renew(pool: &Pool, agent_id: AgentId, worker_id: &str) -> Result<bool, WorkerError> {
    let mut conn = pool.get().await?;
    let key = keys::lock_agent(agent_id);
    let script = Script::new(RENEW_SCRIPT);
    let result: i64 = script
        .key(&key)
        .arg(worker_id)
        .arg(LOCK_TTL_MS)
        .invoke_async(&mut *conn)
        .await?;
    Ok(result == 1)
}
