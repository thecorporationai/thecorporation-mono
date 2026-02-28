//! Execution state and result.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of an agent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Timeout | Self::Cancelled)
    }
}

/// Result collected from a finished execution container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub final_response: Option<String>,
    #[serde(default)]
    pub tool_calls_count: u32,
    #[serde(default)]
    pub turns: u32,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub duration_seconds: f64,
}

/// Full execution record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub id: String,
    pub agent_id: String,
    pub message_id: String,
    pub status: ExecutionStatus,
    #[serde(default)]
    pub container_id: Option<String>,
    #[serde(default)]
    pub result: Option<ExecutionResult>,
    #[serde(default)]
    pub transcript: Vec<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_serde() {
        let s = ExecutionStatus::Running;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"running\"");
        let parsed: ExecutionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, s);
    }

    #[test]
    fn is_terminal() {
        assert!(!ExecutionStatus::Queued.is_terminal());
        assert!(!ExecutionStatus::Running.is_terminal());
        assert!(ExecutionStatus::Completed.is_terminal());
        assert!(ExecutionStatus::Failed.is_terminal());
        assert!(ExecutionStatus::Timeout.is_terminal());
        assert!(ExecutionStatus::Cancelled.is_terminal());
    }

    #[test]
    fn execution_result_defaults() {
        let json = r#"{}"#;
        let result: ExecutionResult = serde_json::from_str(json).unwrap();
        assert!(!result.success);
        assert_eq!(result.turns, 0);
        assert_eq!(result.input_tokens, 0);
    }

    #[test]
    fn execution_result_roundtrip() {
        let result = ExecutionResult {
            success: true,
            reason: None,
            final_response: Some("Done.".to_owned()),
            tool_calls_count: 3,
            turns: 5,
            input_tokens: 1500,
            output_tokens: 800,
            duration_seconds: 12.5,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ExecutionResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.turns, 5);
    }
}
