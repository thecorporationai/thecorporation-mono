//! Agent configuration types — tools, MCP servers, skills, channels, budget, sandbox.
//!
//! All struct fields that represent names, commands, or URLs use [`NonEmpty`]
//! to guarantee non-blank values at parse time.  Numeric limits use
//! positive-value validators so that zero-budget or zero-memory configs are
//! rejected at the system boundary rather than silently misbehaving at runtime.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::enums::{HttpMethod, NetworkEgress, Transport};
use crate::validated::{
    CronExpr, NonEmpty, deserialize_positive_f64, deserialize_positive_u32,
    deserialize_positive_u64,
};

// ── ToolSpec ─────────────────────────────────────────────────────────

/// HTTP tool that the agent can call.
///
/// `name` and `url` are [`NonEmpty`] — deserialization of blank values
/// fails with a clear error rather than producing a broken tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: NonEmpty,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub method: HttpMethod,
    pub url: NonEmpty,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub body_schema: serde_json::Value,
}

// ── MCPServerSpec ────────────────────────────────────────────────────

/// MCP server that runs inside the agent container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerSpec {
    pub name: NonEmpty,
    #[serde(default)]
    pub transport: Transport,
    pub command: NonEmpty,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

// ── SkillSpec ────────────────────────────────────────────────────────

/// A composable agent skill (maps to tools / MCP servers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSpec {
    pub name: NonEmpty,
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

// ── ChannelConfig ────────────────────────────────────────────────────

/// Inbound channel configuration — a tagged enum so that each variant
/// carries only the fields it needs.
///
/// The key invariant: a `Cron` channel *always* has a valid schedule.
/// This is unrepresentable with the old struct approach (schedule was
/// `Option<String>`, allowing a cron channel with no schedule).
///
/// Wire format is the same as before — internally tagged on `"type"`:
/// ```json
/// {"type": "cron", "schedule": "*/5 * * * *"}
/// {"type": "email", "address": "bot@acme.com"}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelConfig {
    Email {
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        webhook_secret: Option<String>,
    },
    Webhook {
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        webhook_secret: Option<String>,
    },
    Cron {
        schedule: CronExpr,
    },
    Manual,
}

impl ChannelConfig {
    /// Returns the channel type as a string slug.
    pub fn channel_type_str(&self) -> &'static str {
        match self {
            Self::Email { .. } => "email",
            Self::Webhook { .. } => "webhook",
            Self::Cron { .. } => "cron",
            Self::Manual => "manual",
        }
    }

    /// Returns true if this is a cron channel.
    pub fn is_cron(&self) -> bool {
        matches!(self, Self::Cron { .. })
    }

    /// Extract the schedule from a Cron channel. Returns `None` for other types.
    pub fn schedule(&self) -> Option<&CronExpr> {
        match self {
            Self::Cron { schedule } => Some(schedule),
            _ => None,
        }
    }
}

// ── BudgetConfig ─────────────────────────────────────────────────────

/// Execution budget limits.
///
/// All limits are validated positive on deserialization — a budget of
/// zero turns or zero tokens is nonsensical and rejected at parse time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    #[serde(default = "default_max_turns", deserialize_with = "deserialize_positive_u32")]
    pub max_turns: u32,
    #[serde(default = "default_max_tokens", deserialize_with = "deserialize_positive_u64")]
    pub max_tokens: u64,
    #[serde(default = "default_max_monthly_cost_cents", deserialize_with = "deserialize_positive_u64")]
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

// ── SandboxConfig ────────────────────────────────────────────────────

/// Per-agent sandbox (container) configuration.
///
/// Numeric resource limits are validated positive — a container with
/// zero memory or zero CPU cannot start, so we reject at parse time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default = "default_memory_mb", deserialize_with = "deserialize_positive_u64")]
    pub memory_mb: u64,
    #[serde(default = "default_cpu_limit", deserialize_with = "deserialize_positive_f64")]
    pub cpu_limit: f64,
    #[serde(default = "default_timeout", deserialize_with = "deserialize_positive_u64")]
    pub timeout_seconds: u64,
    #[serde(default = "default_disk_mb", deserialize_with = "deserialize_positive_u64")]
    pub disk_mb: u64,
    #[serde(default)]
    pub network_egress: NetworkEgress,
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
            network_egress: NetworkEgress::default(),
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
fn default_runtimes() -> Vec<String> { vec!["python".to_owned()] }

// ── AgentSkill (api-rs compat) ───────────────────────────────────────

/// A skill that an agent can perform (used in api-rs Agent struct).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    pub name: NonEmpty,
    pub description: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::ChannelType;

    #[test]
    fn tool_spec_defaults() {
        let json = r#"{"name": "check", "url": "http://example.com"}"#;
        let tool: ToolSpec = serde_json::from_str(json).unwrap();
        assert_eq!(tool.method, HttpMethod::Get);
        assert!(tool.headers.is_empty());
        assert!(tool.description.is_none());
    }

    #[test]
    fn tool_spec_rejects_empty_name() {
        let json = r#"{"name": "", "url": "http://example.com"}"#;
        assert!(serde_json::from_str::<ToolSpec>(json).is_err());
    }

    #[test]
    fn tool_spec_rejects_empty_url() {
        let json = r#"{"name": "check", "url": ""}"#;
        assert!(serde_json::from_str::<ToolSpec>(json).is_err());
    }

    #[test]
    fn tool_spec_with_method() {
        let json = r#"{"name": "submit", "url": "http://example.com", "method": "POST"}"#;
        let tool: ToolSpec = serde_json::from_str(json).unwrap();
        assert_eq!(tool.method, HttpMethod::Post);
    }

    #[test]
    fn sandbox_config_defaults() {
        let config = SandboxConfig::default();
        assert_eq!(config.memory_mb, 512);
        assert_eq!(config.cpu_limit, 0.5);
        assert_eq!(config.timeout_seconds, 300);
        assert!(config.enable_code_execution);
        assert_eq!(config.network_egress, NetworkEgress::Restricted);
    }

    #[test]
    fn sandbox_config_rejects_zero_memory() {
        let json = r#"{"memory_mb": 0}"#;
        assert!(serde_json::from_str::<SandboxConfig>(json).is_err());
    }

    #[test]
    fn sandbox_config_rejects_zero_timeout() {
        let json = r#"{"timeout_seconds": 0}"#;
        assert!(serde_json::from_str::<SandboxConfig>(json).is_err());
    }

    #[test]
    fn sandbox_config_rejects_zero_cpu() {
        let json = r#"{"cpu_limit": 0.0}"#;
        assert!(serde_json::from_str::<SandboxConfig>(json).is_err());
    }

    #[test]
    fn budget_config_defaults() {
        let budget = BudgetConfig::default();
        assert_eq!(budget.max_turns, 20);
        assert_eq!(budget.max_tokens, 100_000);
    }

    #[test]
    fn budget_config_rejects_zero_turns() {
        let json = r#"{"max_turns": 0}"#;
        assert!(serde_json::from_str::<BudgetConfig>(json).is_err());
    }

    #[test]
    fn budget_config_rejects_zero_tokens() {
        let json = r#"{"max_tokens": 0}"#;
        assert!(serde_json::from_str::<BudgetConfig>(json).is_err());
    }

    #[test]
    fn channel_cron_requires_schedule() {
        // Missing schedule → parse failure
        let json = r#"{"type": "cron"}"#;
        assert!(serde_json::from_str::<ChannelConfig>(json).is_err());
    }

    #[test]
    fn channel_cron_rejects_invalid_schedule() {
        // Too few fields → CronExpr parse failure
        let json = r#"{"type": "cron", "schedule": "* *"}"#;
        assert!(serde_json::from_str::<ChannelConfig>(json).is_err());
    }

    #[test]
    fn channel_cron_valid() {
        let json = r#"{"type": "cron", "schedule": "*/5 * * * *"}"#;
        let ch: ChannelConfig = serde_json::from_str(json).unwrap();
        assert!(ch.is_cron());
        assert_eq!(ch.schedule().unwrap().as_str(), "*/5 * * * *");
    }

    #[test]
    fn channel_email() {
        let json = r#"{"type": "email", "address": "bot@acme.com"}"#;
        let ch: ChannelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(ch.channel_type_str(), "email");
        assert!(!ch.is_cron());
    }

    #[test]
    fn channel_manual_minimal() {
        let json = r#"{"type": "manual"}"#;
        let ch: ChannelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(ch.channel_type_str(), "manual");
    }

    #[test]
    fn channel_webhook_with_secret() {
        let json = r#"{"type": "webhook", "address": "https://hook.example.com", "webhook_secret": "s3cr3t"}"#;
        let ch: ChannelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(ch.channel_type_str(), "webhook");
    }

    #[test]
    fn channel_roundtrip() {
        let ch = ChannelConfig::Cron {
            schedule: CronExpr::parse("*/5 * * * *").unwrap(),
        };
        let json = serde_json::to_string(&ch).unwrap();
        let parsed: ChannelConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_cron());
        assert_eq!(parsed.schedule().unwrap().as_str(), "*/5 * * * *");
    }

    #[test]
    fn mcp_server_spec() {
        let json = r#"{"name": "sqlite", "transport": "stdio", "command": "sqlite-mcp", "args": ["--db", "/data/test.db"]}"#;
        let spec: MCPServerSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.name.as_str(), "sqlite");
        assert_eq!(spec.transport, Transport::Stdio);
    }

    #[test]
    fn mcp_server_rejects_empty_command() {
        let json = r#"{"name": "sqlite", "command": ""}"#;
        assert!(serde_json::from_str::<MCPServerSpec>(json).is_err());
    }

    #[test]
    fn agent_skill_rejects_empty_name() {
        let json = r#"{"name": "", "description": "does stuff"}"#;
        assert!(serde_json::from_str::<AgentSkill>(json).is_err());
    }

    // Verify backward compat: old-style JSON with extra null fields still parses
    #[test]
    fn channel_email_with_extra_nulls() {
        let json = r#"{"type": "email", "address": "bot@acme.com", "schedule": null, "webhook_secret": null}"#;
        let ch: ChannelConfig = serde_json::from_str(json).unwrap();
        assert_eq!(ch.channel_type_str(), "email");
    }

    // ── ChannelType compat ──────────────────────────────────────────

    #[test]
    fn channel_type_matches_enum() {
        // Ensure the tagged enum variant names match ChannelType string forms
        assert_eq!(ChannelType::Cron.as_str(), "cron");
        assert_eq!(ChannelType::Email.as_str(), "email");
        assert_eq!(ChannelType::Webhook.as_str(), "webhook");
        assert_eq!(ChannelType::Manual.as_str(), "manual");
    }
}
