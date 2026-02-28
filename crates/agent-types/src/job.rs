//! Job payload — serialized into the Redis queue.
//!
//! The struct is `#[non_exhaustive]` so that external crates cannot
//! construct a `JobPayload` via struct literal — they must use `new()`
//! or `cron()`.  This enforces the invariant that message-triggered jobs
//! always have `message_id: Some(...)` and cron jobs always have `None`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ids::{AgentId, ExecutionId, MessageId, WorkspaceId};

/// A job on the Redis queue, representing an execution to run.
///
/// # Construction
///
/// Use [`JobPayload::new()`] for message-triggered jobs (guarantees
/// `message_id` is `Some`) or [`JobPayload::cron()`] for scheduled
/// jobs (guarantees `message_id` is `None`).  Direct struct-literal
/// construction is prevented by `#[non_exhaustive]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct JobPayload {
    pub job_id: Uuid,
    pub execution_id: ExecutionId,
    pub agent_id: AgentId,
    pub workspace_id: WorkspaceId,
    /// `None` for cron-triggered jobs (no stored message in api-rs).
    #[serde(default)]
    pub message_id: Option<MessageId>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    pub enqueued_at: DateTime<Utc>,
}

impl JobPayload {
    /// Create a job for a message-triggered execution.
    pub fn new(
        execution_id: ExecutionId,
        agent_id: AgentId,
        workspace_id: WorkspaceId,
        message_id: MessageId,
    ) -> Self {
        Self {
            job_id: Uuid::new_v4(),
            execution_id,
            agent_id,
            workspace_id,
            message_id: Some(message_id),
            idempotency_key: None,
            enqueued_at: Utc::now(),
        }
    }

    /// Create a job for a cron-triggered execution (no stored message).
    pub fn cron(
        execution_id: ExecutionId,
        agent_id: AgentId,
        workspace_id: WorkspaceId,
    ) -> Self {
        Self {
            job_id: Uuid::new_v4(),
            execution_id,
            agent_id,
            workspace_id,
            message_id: None,
            idempotency_key: None,
            enqueued_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_job_roundtrip() {
        let job = JobPayload::new(
            ExecutionId::new(),
            AgentId::new(),
            WorkspaceId::new(),
            MessageId::new(),
        );
        assert!(job.message_id.is_some());
        let json = serde_json::to_string(&job).unwrap();
        let parsed: JobPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.job_id, job.job_id);
        assert_eq!(parsed.execution_id, job.execution_id);
    }

    #[test]
    fn cron_job_has_no_message_id() {
        let job = JobPayload::cron(
            ExecutionId::new(),
            AgentId::new(),
            WorkspaceId::new(),
        );
        assert!(job.message_id.is_none());
        let json = serde_json::to_string(&job).unwrap();
        let parsed: JobPayload = serde_json::from_str(&json).unwrap();
        assert!(parsed.message_id.is_none());
    }
}
