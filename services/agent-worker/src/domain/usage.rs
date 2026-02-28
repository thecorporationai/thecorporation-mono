//! Token/cost usage tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Usage event recorded after an execution completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub agent_id: String,
    pub execution_id: String,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub tool_calls_count: u32,
    #[serde(default)]
    pub duration_seconds: f64,
    #[serde(default)]
    pub cost_cents: u64,
    #[serde(default)]
    pub recorded_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = UsageEvent {
            agent_id: "agt_1".to_owned(),
            execution_id: "exec_1".to_owned(),
            input_tokens: 1500,
            output_tokens: 800,
            model: "anthropic/claude-sonnet-4-6".to_owned(),
            tool_calls_count: 3,
            duration_seconds: 12.0,
            cost_cents: 15,
            recorded_at: Some(Utc::now()),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: UsageEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.cost_cents, 15);
    }
}
