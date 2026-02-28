//! Agent message record (stored as `agents/{agent_id}/messages/{message_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{AgentId, MessageId};

/// A message sent to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    message_id: MessageId,
    agent_id: AgentId,
    content: String,
    metadata: serde_json::Value,
    status: String,
    created_at: DateTime<Utc>,
}

impl AgentMessage {
    pub fn new(
        message_id: MessageId,
        agent_id: AgentId,
        content: String,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            message_id,
            agent_id,
            content,
            metadata,
            status: "queued".to_owned(),
            created_at: Utc::now(),
        }
    }

    pub fn message_id(&self) -> MessageId { self.message_id }
    pub fn agent_id(&self) -> AgentId { self.agent_id }
    pub fn content(&self) -> &str { &self.content }
    pub fn metadata(&self) -> &serde_json::Value { &self.metadata }
    pub fn status(&self) -> &str { &self.status }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
}
