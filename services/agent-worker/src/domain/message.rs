//! Inbound message — what triggers an agent execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A message sent to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub id: String,
    pub agent_id: String,
    pub channel: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_message() {
        let json = r#"{"id": "msg_abc", "agent_id": "agt_123", "channel": "manual", "body": "Hello"}"#;
        let msg: InboundMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.channel, "manual");
        assert_eq!(msg.body, "Hello");
        assert!(msg.sender.is_none());
    }

    #[test]
    fn full_message_roundtrip() {
        let msg = InboundMessage {
            id: "msg_test".to_owned(),
            agent_id: "agt_test".to_owned(),
            channel: "webhook".to_owned(),
            sender: Some("user@example.com".to_owned()),
            subject: Some("Test".to_owned()),
            body: "Do something".to_owned(),
            attachments: vec![serde_json::json!({"name": "file.pdf"})],
            channel_metadata: HashMap::new(),
            received_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: InboundMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "msg_test");
        assert_eq!(parsed.attachments.len(), 1);
    }
}
