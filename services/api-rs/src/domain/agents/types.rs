//! Agent domain types.

use serde::{Deserialize, Serialize};

/// Status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Paused,
    Disabled,
}

/// A skill that an agent can perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_status_serde() {
        let status = AgentStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");
        let parsed: AgentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn agent_skill_serde() {
        let skill = AgentSkill {
            name: "file_formation".to_owned(),
            description: "File entity formation documents".to_owned(),
            parameters: serde_json::json!({}),
        };
        let json = serde_json::to_string(&skill).unwrap();
        let parsed: AgentSkill = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "file_formation");
    }
}
