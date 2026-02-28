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

/// An HTTP tool the agent can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_method")]
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub body_schema: serde_json::Value,
}

fn default_method() -> String {
    "GET".to_owned()
}

/// An MCP server running inside the agent container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerSpec {
    pub name: String,
    #[serde(default = "default_transport")]
    pub transport: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

fn default_transport() -> String {
    "stdio".to_owned()
}

/// Inbound message channel configuration.
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

fn default_max_turns() -> u32 {
    20
}
fn default_max_tokens() -> u64 {
    100_000
}
fn default_max_monthly_cost_cents() -> u64 {
    10_000
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

/// Container resource limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u64,
    #[serde(default = "default_cpu_limit")]
    pub cpu_limit: f64,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_disk_mb")]
    pub disk_mb: u64,
    #[serde(default = "default_network_egress")]
    pub network_egress: String,
    #[serde(default)]
    pub egress_allowlist: Vec<String>,
}

fn default_memory_mb() -> u64 {
    512
}
fn default_cpu_limit() -> f64 {
    0.5
}
fn default_timeout_seconds() -> u64 {
    300
}
fn default_disk_mb() -> u64 {
    1024
}
fn default_network_egress() -> String {
    "restricted".to_owned()
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            memory_mb: default_memory_mb(),
            cpu_limit: default_cpu_limit(),
            timeout_seconds: default_timeout_seconds(),
            disk_mb: default_disk_mb(),
            network_egress: default_network_egress(),
            egress_allowlist: Vec::new(),
        }
    }
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

    #[test]
    fn tool_spec_defaults() {
        let json = r#"{"name":"fetch","url":"https://example.com"}"#;
        let tool: ToolSpec = serde_json::from_str(json).unwrap();
        assert_eq!(tool.method, "GET");
        assert!(tool.headers.is_empty());
    }

    #[test]
    fn budget_config_defaults() {
        let budget = BudgetConfig::default();
        assert_eq!(budget.max_turns, 20);
        assert_eq!(budget.max_tokens, 100_000);
    }

    #[test]
    fn sandbox_config_defaults() {
        let sandbox = SandboxConfig::default();
        assert_eq!(sandbox.memory_mb, 512);
        assert_eq!(sandbox.cpu_limit, 0.5);
        assert_eq!(sandbox.network_egress, "restricted");
    }

    #[test]
    fn channel_config_cron() {
        let json = r#"{"type":"cron","schedule":"*/5 * * * *"}"#;
        let ch: ChannelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(ch.channel_type, "cron");
        assert_eq!(ch.schedule.as_deref(), Some("*/5 * * * *"));
    }

    #[test]
    fn mcp_server_spec() {
        let spec = MCPServerSpec {
            name: "sqlite".to_owned(),
            transport: "stdio".to_owned(),
            command: "sqlite-mcp".to_owned(),
            args: vec!["--db".to_owned(), "/data/test.db".to_owned()],
            env: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: MCPServerSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "sqlite");
        assert_eq!(parsed.args.len(), 2);
    }
}
