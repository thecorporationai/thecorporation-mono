//! Log entries published to Redis Pub/Sub for real-time streaming.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::enums::LogLevel;
use crate::ids::ExecutionId;

/// A log entry emitted during execution, streamed via Redis Pub/Sub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub execution_id: ExecutionId,
    #[serde(default)]
    pub level: LogLevel,
    pub event: String,
    #[serde(default)]
    pub data: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<DateTime<Utc>>,
}

impl LogEntry {
    /// Create a "done" log entry signaling execution is complete.
    pub fn done(execution_id: ExecutionId) -> Self {
        Self {
            execution_id,
            level: LogLevel::Info,
            event: "done".to_owned(),
            data: HashMap::new(),
            timestamp: Some(Utc::now()),
        }
    }

    /// Create an "error" log entry.
    pub fn error(execution_id: ExecutionId, message: &str) -> Self {
        let mut data = HashMap::new();
        data.insert("message".to_owned(), serde_json::Value::String(message.to_owned()));
        Self {
            execution_id,
            level: LogLevel::Error,
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
        let entry = LogEntry::done(ExecutionId::new());
        assert_eq!(entry.event, "done");
        assert_eq!(entry.level, LogLevel::Info);
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"done\""));
    }

    #[test]
    fn error_entry() {
        let entry = LogEntry::error(ExecutionId::new(), "container crashed");
        assert_eq!(entry.event, "error");
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.data["message"], "container crashed");
    }
}
