//! Job payload — serialized into the Redis queue.
//!
//! `JobPayload` is a tagged enum, so the trigger source is part of the type:
//! - `Message` always carries `message_id`
//! - `Cron` can never carry `message_id`
//!
//! This encodes parse-time invariants directly in the wire type.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ids::{AgentId, ExecutionId, MessageId, WorkspaceId};

/// A job on the Redis queue, representing an execution to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "trigger", rename_all = "snake_case")]
pub enum JobPayload {
    Message {
        job_id: Uuid,
        execution_id: ExecutionId,
        agent_id: AgentId,
        workspace_id: WorkspaceId,
        message_id: MessageId,
        #[serde(default)]
        idempotency_key: Option<String>,
        enqueued_at: DateTime<Utc>,
    },
    Cron {
        job_id: Uuid,
        execution_id: ExecutionId,
        agent_id: AgentId,
        workspace_id: WorkspaceId,
        #[serde(default)]
        idempotency_key: Option<String>,
        enqueued_at: DateTime<Utc>,
    },
}

impl JobPayload {
    /// Create a job for a message-triggered execution.
    pub fn new(
        execution_id: ExecutionId,
        agent_id: AgentId,
        workspace_id: WorkspaceId,
        message_id: MessageId,
    ) -> Self {
        Self::Message {
            job_id: Uuid::new_v4(),
            execution_id,
            agent_id,
            workspace_id,
            message_id,
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
        Self::Cron {
            job_id: Uuid::new_v4(),
            execution_id,
            agent_id,
            workspace_id,
            idempotency_key: None,
            enqueued_at: Utc::now(),
        }
    }

    pub fn job_id(&self) -> Uuid {
        match self {
            Self::Message { job_id, .. } | Self::Cron { job_id, .. } => *job_id,
        }
    }

    pub fn execution_id(&self) -> ExecutionId {
        match self {
            Self::Message { execution_id, .. } | Self::Cron { execution_id, .. } => *execution_id,
        }
    }

    pub fn agent_id(&self) -> AgentId {
        match self {
            Self::Message { agent_id, .. } | Self::Cron { agent_id, .. } => *agent_id,
        }
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        match self {
            Self::Message { workspace_id, .. } | Self::Cron { workspace_id, .. } => *workspace_id,
        }
    }

    pub fn message_id(&self) -> Option<MessageId> {
        match self {
            Self::Message { message_id, .. } => Some(*message_id),
            Self::Cron { .. } => None,
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
        assert!(job.message_id().is_some());
        let json = serde_json::to_string(&job).unwrap();
        let parsed: JobPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.job_id(), job.job_id());
        assert_eq!(parsed.execution_id(), job.execution_id());
    }

    #[test]
    fn cron_job_has_no_message_id() {
        let job = JobPayload::cron(
            ExecutionId::new(),
            AgentId::new(),
            WorkspaceId::new(),
        );
        assert!(job.message_id().is_none());
        let json = serde_json::to_string(&job).unwrap();
        let parsed: JobPayload = serde_json::from_str(&json).unwrap();
        assert!(parsed.message_id().is_none());
    }
}
