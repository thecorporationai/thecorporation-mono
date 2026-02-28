//! Log entries published to Redis Pub/Sub for real-time streaming.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A log entry emitted during execution, streamed via Redis Pub/Sub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub execution_id: String,
    #[serde(default = "default_level")]
    pub level: String,
    pub event: String,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
}

fn default_level() -> String {
    "info".to_owned()
}

impl LogEntry {
    /// Create a "done" log entry signaling execution is complete.
    pub fn done(execution_id: &str) -> Self {
        Self {
            execution_id: execution_id.to_owned(),
            level: "info".to_owned(),
            event: "done".to_owned(),
            data: HashMap::new(),
            timestamp: Some(Utc::now()),
        }
    }

    /// Create an "error" log entry.
    pub fn error(execution_id: &str, message: &str) -> Self {
        let mut data = HashMap::new();
        data.insert("message".to_owned(), serde_json::Value::String(message.to_owned()));
        Self {
            execution_id: execution_id.to_owned(),
            level: "error".to_owned(),
            event: "error".to_owned(),
            data,
            timestamp: Some(Utc::now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_entry() {
        let entry = LogEntry::done("exec_123");
        assert_eq!(entry.event, "done");
        assert_eq!(entry.level, "info");
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"done\""));
    }

    #[test]
    fn error_entry() {
        let entry = LogEntry::error("exec_123", "container crashed");
        assert_eq!(entry.event, "error");
        assert_eq!(entry.data["message"], "container crashed");
    }
}
