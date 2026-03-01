//! Agent domain types — re-exported from the shared `agent-types` crate.
//!
//! All wire-format types are defined once in `crates/agent-types` and shared
//! with agent-worker.  This module re-exports them so the rest of api-rs can
//! continue to `use super::types::*` unchanged.

pub use agent_types::{
    AgentSkill, AgentStatus, BudgetConfig, ChannelConfig, MCPServerSpec, NonEmpty, SandboxConfig,
    ToolSpec,
};

// HttpMethod and CronExpr are used in test code
#[allow(unused_imports)]
pub use agent_types::{CronExpr, HttpMethod};

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
            name: NonEmpty::parse("file_formation").unwrap(),
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
        assert_eq!(tool.method, HttpMethod::Get);
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
        assert_eq!(
            sandbox.network_egress,
            agent_types::NetworkEgress::Restricted
        );
    }

    #[test]
    fn channel_config_cron() {
        let json = r#"{"type":"cron","schedule":"*/5 * * * *"}"#;
        let ch: ChannelConfig = serde_json::from_str(json).unwrap();
        assert!(ch.is_cron());
        assert_eq!(ch.schedule().unwrap().as_str(), "*/5 * * * *");
    }

    #[test]
    fn mcp_server_spec() {
        let json = r#"{"name": "sqlite", "transport": "stdio", "command": "sqlite-mcp", "args": ["--db", "/data/test.db"]}"#;
        let parsed: MCPServerSpec = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.name, "sqlite");
        assert_eq!(parsed.args.len(), 2);
    }
}
