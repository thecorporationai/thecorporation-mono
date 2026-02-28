//! Centralized Redis key format constants.
//!
//! All keys are prefixed with `aw:` (agent-worker) to avoid collisions.

use agent_types::{AgentId, ExecutionId};

/// Main job queue (LIST).
pub const QUEUE_JOBS: &str = "aw:queue:jobs";

/// Per-worker in-flight jobs for crash recovery (LIST).
pub fn queue_processing(worker_id: &str) -> String {
    format!("aw:queue:processing:{worker_id}")
}

/// Per-agent distributed lock (STRING).
pub fn lock_agent(agent_id: AgentId) -> String {
    format!("aw:lock:agent:{agent_id}")
}

/// Execution state (HASH).
pub fn exec_state(execution_id: ExecutionId) -> String {
    format!("aw:exec:{execution_id}")
}

/// Execution result JSON (STRING, 7d TTL).
pub fn exec_result(execution_id: ExecutionId) -> String {
    format!("aw:exec:{execution_id}:result")
}

/// Idempotency key (STRING, 24h TTL).
pub fn idempotency(key: &str) -> String {
    format!("aw:idem:{key}")
}

/// Log Pub/Sub channel.
pub fn logs_channel(execution_id: ExecutionId) -> String {
    format!("aw:logs:{execution_id}")
}

/// Log history list (for replay on reconnect).
pub fn logs_history(execution_id: ExecutionId) -> String {
    format!("aw:logs:{execution_id}:history")
}

/// Opaque token map for an execution (HASH).
pub fn tokens(execution_id: ExecutionId) -> String {
    format!("aw:tokens:{execution_id}")
}

/// Reverse token lookup (STRING).
pub fn token_reverse(opaque_token: &str) -> String {
    format!("aw:tokens:reverse:{opaque_token}")
}

/// Cron dedup flag (STRING, 120s TTL).
pub fn cron_last_fire(agent_id: AgentId, schedule_hash: &str) -> String {
    format!("aw:cron:last_fire:{agent_id}:{schedule_hash}")
}

/// Worker heartbeat (STRING, 30s TTL).
pub fn worker_heartbeat(worker_id: &str) -> String {
    format!("aw:worker:{worker_id}")
}

/// RPC reply key (LIST, short-lived). Caller BLPOPs, worker RPUSHes the ack.
pub fn rpc_reply(reply_id: &str) -> String {
    format!("aw:rpc:reply:{reply_id}")
}

/// Proxy-accumulated LLM usage (HASH, 7d TTL).
pub fn usage(execution_id: ExecutionId) -> String {
    format!("aw:usage:{execution_id}")
}

/// Kill command channel (PUB/SUB). api-rs publishes, workers subscribe.
pub const CMD_KILL: &str = "aw:cmd:kill";
