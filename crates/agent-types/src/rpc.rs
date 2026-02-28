//! Redis-based RPC types: request/reply for synchronous confirmation.

use serde::{Deserialize, Serialize};

use crate::ids::ExecutionId;

/// RPC reply from worker to api-rs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcReply {
    pub status: RpcStatus,
    pub execution_id: ExecutionId,
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

/// Kill command sent via pub/sub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillCommand {
    pub execution_id: ExecutionId,
    /// UUID of the reply channel (not the full Redis key).
    pub reply_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_status_serde() {
        let s = RpcStatus::Accepted;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"accepted\"");
        let parsed: RpcStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, s);
    }

    #[test]
    fn rpc_reply_roundtrip() {
        let reply = RpcReply {
            status: RpcStatus::Accepted,
            execution_id: ExecutionId::new(),
            message: None,
        };
        let json = serde_json::to_string(&reply).unwrap();
        let parsed: RpcReply = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, RpcStatus::Accepted);
    }

    #[test]
    fn kill_command_roundtrip() {
        let cmd = KillCommand {
            execution_id: ExecutionId::new(),
            reply_id: uuid::Uuid::new_v4().to_string(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: KillCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.execution_id, cmd.execution_id);
    }
}
