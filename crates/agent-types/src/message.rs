//! Inbound message — what triggers an agent execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::enums::ChannelType;
use crate::ids::{AgentId, MessageId};

/// A message sent to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub id: MessageId,
    pub agent_id: AgentId,
    pub channel: ChannelType,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub attachments: Vec<serde_json::Value>,
    #[serde(default)]
    pub channel_metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub received_at: Option<DateTime<Utc>>,
}

impl InboundMessage {
    /// Create a synthetic cron-triggered message (no stored message in api-rs).
    pub fn cron_trigger(agent_id: AgentId) -> Self {
        Self {
            id: MessageId::new(),
            agent_id,
            channel: ChannelType::Cron,
            sender: None,
            subject: Some("Scheduled execution".to_owned()),
            body: String::new(),
            attachments: Vec::new(),
            channel_metadata: HashMap::new(),
            received_at: Some(Utc::now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_message() {
        let agent_id = AgentId::new();
        let msg_id = MessageId::new();
        let json = format!(
            r#"{{"id": "{msg_id}", "agent_id": "{agent_id}", "channel": "manual", "body": "Hello"}}"#
        );
        let msg: InboundMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.channel, ChannelType::Manual);
        assert_eq!(msg.body, "Hello");
        assert!(msg.sender.is_none());
    }

    #[test]
    fn cron_trigger() {
        let msg = InboundMessage::cron_trigger(AgentId::new());
        assert_eq!(msg.channel, ChannelType::Cron);
        assert!(msg.received_at.is_some());
    }

    #[test]
    fn full_message_roundtrip() {
        let msg = InboundMessage {
            id: MessageId::new(),
            agent_id: AgentId::new(),
            channel: ChannelType::Webhook,
            sender: Some("user@example.com".to_owned()),
            subject: Some("Test".to_owned()),
            body: "Do something".to_owned(),
            attachments: vec![serde_json::json!({"name": "file.pdf"})],
            channel_metadata: HashMap::new(),
            received_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: InboundMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, msg.id);
        assert_eq!(parsed.attachments.len(), 1);
    }
}
