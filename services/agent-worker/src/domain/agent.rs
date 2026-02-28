//! Agent definition — the full spec for a configured agent.
//!
//! Ported from Python `AgentDefinition` in services/agents/agents/models.py.
//! This is what the worker receives from api-rs and passes to the Pi container.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ids::{AgentId, WorkspaceId};

/// HTTP tool that the agent can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
    #[serde(default)]
    pub body_schema: Option<serde_json::Value>,
}

fn default_method() -> String {
    "GET".to_owned()
}

/// MCP server that runs inside the agent container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerSpec {
    pub name: String,
    #[serde(default = "default_transport")]
    pub transport: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_transport() -> String {
    "stdio".to_owned()
}

/// A composable agent skill (maps to tools/MCP servers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub mcp_server: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Inbound channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    #[serde(rename = "type")]
    pub channel_type: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub schedule: Option<String>,
    #[serde(default)]
    pub webhook_secret: Option<String>,
}

/// Execution budget limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u64,
    #[serde(default = "default_max_monthly_cost_cents")]
    pub max_monthly_cost_cents: u64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_turns: default_max_turns(),
            max_tokens: default_max_tokens(),
            max_monthly_cost_cents: default_max_monthly_cost_cents(),
        }
    }
}

fn default_max_turns() -> u32 { 20 }
fn default_max_tokens() -> u64 { 100_000 }
fn default_max_monthly_cost_cents() -> u64 { 10_000 }

/// Per-agent sandbox (container) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u64,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f64,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_disk_mb")]
    pub disk_mb: u64,
    #[serde(default = "default_network_egress")]
    pub network_egress: String,
    #[serde(default)]
    pub egress_allowlist: Vec<String>,
    #[serde(default = "default_runtimes")]
    pub runtimes: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default = "default_true")]
    pub enable_code_execution: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_mb: default_memory_mb(),
            cpu_limit: default_cpu_limit(),
            timeout_seconds: default_timeout(),
            disk_mb: default_disk_mb(),
            network_egress: default_network_egress(),
            egress_allowlist: Vec::new(),
            runtimes: default_runtimes(),
            packages: Vec::new(),
            enable_code_execution: true,
        }
    }
}

fn default_memory_mb() -> u64 { 512 }
fn default_cpu_limit() -> f64 { 0.5 }
fn default_timeout() -> u64 { 300 }
fn default_disk_mb() -> u64 { 1024 }
fn default_network_egress() -> String { "restricted".to_owned() }
fn default_runtimes() -> Vec<String> { vec!["python".to_owned()] }

/// Full agent definition — everything needed to run an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
    pub entity_id: Option<String>,
    pub name: String,
    #[serde(default = "default_status")]
    pub status: String,
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
    pub parent_agent_id: Option<String>,
    #[serde(default)]
    pub email_address: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

fn default_status() -> String { "active".to_owned() }
fn default_model() -> String { "anthropic/claude-sonnet-4-6".to_owned() }

/// Sanitized agent config for injection into containers.
/// Strips secrets and workspace_id.
impl AgentDefinition {
    pub fn sanitize_for_container(&self) -> serde_json::Value {
        let mut val = serde_json::to_value(self).unwrap_or_default();
        if let Some(obj) = val.as_object_mut() {
            obj.remove("_secrets");
            // workspace_id stays — Pi needs it for corp extension config
        }
        val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_agent_deserialize() {
        let json = r#"{"id": "agt_abc123", "name": "Test Agent"}"#;
        let agent: AgentDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.model, "anthropic/claude-sonnet-4-6");
        assert_eq!(agent.budget.max_turns, 20);
        assert_eq!(agent.sandbox.memory_mb, 512);
    }

    #[test]
    fn full_agent_roundtrip() {
        let agent = AgentDefinition {
            id: "agt_test".to_owned(),
            workspace_id: "ws_123".to_owned(),
            entity_id: None,
            parent_agent_id: None,
            name: "Corp Agent".to_owned(),
            status: "active".to_owned(),
            system_prompt: "You are a helpful agent.".to_owned(),
            model: "anthropic/claude-sonnet-4-6".to_owned(),
            tools: vec![ToolSpec {
                name: "get_entities".to_owned(),
                description: "List entities".to_owned(),
                method: "GET".to_owned(),
                url: "http://api/v1/entities".to_owned(),
                headers: HashMap::new(),
                parameters: None,
                body_schema: None,
            }],
            skills: Vec::new(),
            mcp_servers: Vec::new(),
            channels: vec![ChannelConfig {
                channel_type: "cron".to_owned(),
                address: None,
                schedule: Some("0 9 * * *".to_owned()),
                webhook_secret: None,
            }],
            budget: BudgetConfig::default(),
            sandbox: SandboxConfig::default(),
            email_address: Some("abc@agents.thecorporation.app".to_owned()),
            webhook_url: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&agent).unwrap();
        let parsed: AgentDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "agt_test");
        assert_eq!(parsed.tools.len(), 1);
        assert_eq!(parsed.channels.len(), 1);
    }

    #[test]
    fn tool_spec_defaults() {
        let json = r#"{"name": "check", "url": "http://example.com"}"#;
        let tool: ToolSpec = serde_json::from_str(json).unwrap();
        assert_eq!(tool.method, "GET");
        assert!(tool.headers.is_empty());
    }

    #[test]
    fn sandbox_config_defaults() {
        let config = SandboxConfig::default();
        assert_eq!(config.memory_mb, 512);
        assert_eq!(config.cpu_limit, 0.5);
        assert_eq!(config.timeout_seconds, 300);
        assert!(config.enable_code_execution);
    }
}
