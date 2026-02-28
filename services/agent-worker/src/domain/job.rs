//! Job payload — serialized into the Redis queue.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ids::{AgentId, ExecutionId, MessageId, WorkspaceId};

/// A job on the Redis queue, representing an execution to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPayload {
    pub job_id: Uuid,
    pub execution_id: ExecutionId,
    pub agent_id: AgentId,
    pub workspace_id: WorkspaceId,
    pub message_id: MessageId,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    pub enqueued_at: DateTime<Utc>,
}

impl JobPayload {
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
            message_id,
            idempotency_key: None,
            enqueued_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let job = JobPayload::new(
            ExecutionId::new(),
            AgentId::new(),
            WorkspaceId::new(),
            MessageId::new(),
        );
        let json = serde_json::to_string(&job).unwrap();
        let parsed: JobPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.job_id, job.job_id);
        assert_eq!(parsed.execution_id, job.execution_id);
    }
}
