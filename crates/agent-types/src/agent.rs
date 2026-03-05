//! Agent definition — the full spec for a configured agent.
//!
//! This is the wire type between api-rs (producer) and agent-worker (consumer).
//! api-rs serializes a resolved agent as this struct; the worker deserializes it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{
    BudgetConfig, ChannelConfig, MCPServerSpec, SandboxConfig, SkillSpec, ToolSpec,
};
use crate::enums::AgentStatus;
use crate::ids::{AgentId, WorkspaceId};
use crate::validated::NonEmpty;

/// Full agent definition — everything needed to run an agent.
///
/// `name` is [`NonEmpty`] — agents must have a non-blank name.  This is
/// enforced at deserialization time so downstream code never needs to check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentDefinition {
    pub id: AgentId,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
    #[serde(default)]
    pub entity_id: Option<String>,
    pub name: NonEmpty,
    #[serde(default)]
    pub status: AgentStatus,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    #[serde(default)]
    pub skills: Vec<SkillSpec>,
    #[serde(default)]
    pub mcp_servers: Vec<MCPServerSpec>,
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    #[serde(default)]
    pub budget: BudgetConfig,
    #[serde(default)]
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub parent_agent_id: Option<AgentId>,
    #[serde(default)]
    pub email_address: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Active
    }
}

fn default_model() -> String {
    "anthropic/claude-sonnet-4-6".to_owned()
}

impl AgentDefinition {
    /// Sanitized copy for injection into containers.
    /// Returns a `Result` instead of silently swallowing serialization failures.
    pub fn sanitize_for_container(&self) -> Result<serde_json::Value, serde_json::Error> {
        let mut val = serde_json::to_value(self)?;
        if let Some(obj) = val.as_object_mut() {
            obj.remove("_secrets");
        }
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_agent_deserialize() {
        let id = AgentId::new();
        let json = format!(r#"{{"id": "{id}", "name": "Test Agent"}}"#);
        let agent: AgentDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(agent.name.as_str(), "Test Agent");
        assert_eq!(agent.model, "anthropic/claude-sonnet-4-6");
        assert_eq!(agent.budget.max_turns, 20);
        assert_eq!(agent.sandbox.memory_mb, 512);
        assert_eq!(agent.status, AgentStatus::Active);
    }

    #[test]
    fn agent_rejects_empty_name() {
        let id = AgentId::new();
        let json = format!(r#"{{"id": "{id}", "name": ""}}"#);
        assert!(serde_json::from_str::<AgentDefinition>(&json).is_err());
    }

    #[test]
    fn agent_rejects_blank_name() {
        let id = AgentId::new();
        let json = format!(r#"{{"id": "{id}", "name": "   "}}"#);
        assert!(serde_json::from_str::<AgentDefinition>(&json).is_err());
    }

    #[test]
    fn full_agent_roundtrip() {
        let agent = AgentDefinition {
            id: AgentId::new(),
            workspace_id: Some(WorkspaceId::new()),
            entity_id: None,
            parent_agent_id: None,
            name: NonEmpty::parse("Corp Agent").unwrap(),
            status: AgentStatus::Active,
            system_prompt: "You are a helpful agent.".to_owned(),
            model: "anthropic/claude-sonnet-4-6".to_owned(),
            tools: Vec::new(),
            skills: Vec::new(),
            mcp_servers: Vec::new(),
            channels: Vec::new(),
            budget: BudgetConfig::default(),
            sandbox: SandboxConfig::default(),
            email_address: None,
            webhook_url: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&agent).unwrap();
        let parsed: AgentDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, agent.id);
        assert_eq!(parsed.name.as_str(), "Corp Agent");
    }

    #[test]
    fn sanitize_returns_result() {
        let agent = AgentDefinition {
            id: AgentId::new(),
            workspace_id: None,
            entity_id: None,
            parent_agent_id: None,
            name: NonEmpty::parse("Test").unwrap(),
            status: AgentStatus::Active,
            system_prompt: String::new(),
            model: default_model(),
            tools: Vec::new(),
            skills: Vec::new(),
            mcp_servers: Vec::new(),
            channels: Vec::new(),
            budget: BudgetConfig::default(),
            sandbox: SandboxConfig::default(),
            email_address: None,
            webhook_url: None,
            created_at: None,
            updated_at: None,
        };
        let val = agent.sanitize_for_container().unwrap();
        assert!(val.is_object());
    }
}
