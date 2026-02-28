//! Agent record (stored as `agents/{agent_id}.json` in workspace repo).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{
    AgentSkill, AgentStatus, BudgetConfig, ChannelConfig, MCPServerSpec, SandboxConfig, ToolSpec,
};
use crate::domain::auth::scopes::ScopeSet;
use crate::domain::ids::{AgentId, EntityId, WorkspaceId};

/// An AI agent associated with a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    agent_id: AgentId,
    workspace_id: WorkspaceId,
    name: String,
    #[serde(default)]
    system_prompt: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    entity_id: Option<EntityId>,
    #[serde(default)]
    skills: Vec<AgentSkill>,
    status: AgentStatus,
    #[serde(default)]
    tools: Vec<ToolSpec>,
    #[serde(default)]
    mcp_servers: Vec<MCPServerSpec>,
    #[serde(default)]
    channels: Vec<ChannelConfig>,
    #[serde(default)]
    budget: Option<BudgetConfig>,
    #[serde(default)]
    sandbox: Option<SandboxConfig>,
    #[serde(default)]
    parent_agent_id: Option<AgentId>,
    #[serde(default)]
    email_address: Option<String>,
    #[serde(default)]
    webhook_url: Option<String>,
    #[serde(default)]
    scopes: ScopeSet,
    created_at: DateTime<Utc>,
}

impl Agent {
    pub fn new(
        agent_id: AgentId,
        workspace_id: WorkspaceId,
        name: String,
        system_prompt: Option<String>,
        model: Option<String>,
        entity_id: Option<EntityId>,
    ) -> Self {
        Self {
            agent_id,
            workspace_id,
            name,
            system_prompt,
            model,
            entity_id,
            skills: Vec::new(),
            tools: Vec::new(),
            mcp_servers: Vec::new(),
            channels: Vec::new(),
            budget: None,
            sandbox: None,
            status: AgentStatus::Active,
            parent_agent_id: None,
            email_address: None,
            webhook_url: None,
            scopes: ScopeSet::empty(),
            created_at: Utc::now(),
        }
    }

    pub fn add_skill(&mut self, skill: AgentSkill) {
        self.skills.push(skill);
    }

    pub fn set_skills(&mut self, skills: Vec<AgentSkill>) {
        self.skills = skills;
    }

    pub fn set_status(&mut self, status: AgentStatus) {
        self.status = status;
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.system_prompt = prompt;
    }

    pub fn set_model(&mut self, model: Option<String>) {
        self.model = model;
    }

    pub fn set_webhook_url(&mut self, url: Option<String>) {
        self.webhook_url = url;
    }

    pub fn set_tools(&mut self, tools: Vec<ToolSpec>) {
        self.tools = tools;
    }

    pub fn set_mcp_servers(&mut self, servers: Vec<MCPServerSpec>) {
        self.mcp_servers = servers;
    }

    pub fn set_channels(&mut self, channels: Vec<ChannelConfig>) {
        self.channels = channels;
    }

    pub fn set_budget(&mut self, budget: Option<BudgetConfig>) {
        self.budget = budget;
    }

    pub fn set_sandbox(&mut self, sandbox: Option<SandboxConfig>) {
        self.sandbox = sandbox;
    }

    pub fn set_parent_agent_id(&mut self, parent_agent_id: Option<AgentId>) {
        self.parent_agent_id = parent_agent_id;
    }

    pub fn set_scopes(&mut self, scopes: ScopeSet) {
        self.scopes = scopes;
    }

    // Accessors
    pub fn agent_id(&self) -> AgentId {
        self.agent_id
    }
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }
    pub fn entity_id(&self) -> Option<EntityId> {
        self.entity_id
    }
    pub fn skills(&self) -> &[AgentSkill] {
        &self.skills
    }
    pub fn tools(&self) -> &[ToolSpec] {
        &self.tools
    }
    pub fn mcp_servers(&self) -> &[MCPServerSpec] {
        &self.mcp_servers
    }
    pub fn channels(&self) -> &[ChannelConfig] {
        &self.channels
    }
    pub fn budget(&self) -> Option<&BudgetConfig> {
        self.budget.as_ref()
    }
    pub fn sandbox(&self) -> Option<&SandboxConfig> {
        self.sandbox.as_ref()
    }
    pub fn status(&self) -> AgentStatus {
        self.status
    }
    pub fn parent_agent_id(&self) -> Option<AgentId> {
        self.parent_agent_id
    }
    pub fn email_address(&self) -> Option<&str> {
        self.email_address.as_deref()
    }
    pub fn webhook_url(&self) -> Option<&str> {
        self.webhook_url.as_deref()
    }
    pub fn scopes(&self) -> &ScopeSet {
        &self.scopes
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent() -> Agent {
        Agent::new(
            AgentId::new(),
            WorkspaceId::new(),
            "Test Agent".to_owned(),
            Some("You are a helpful corporate agent.".to_owned()),
            Some("claude-sonnet-4-6".to_owned()),
            None,
        )
    }

    #[test]
    fn new_agent_is_active() {
        let a = make_agent();
        assert_eq!(a.status(), AgentStatus::Active);
        assert_eq!(a.name(), "Test Agent");
        assert!(a.skills().is_empty());
    }

    #[test]
    fn add_skill() {
        use agent_types::NonEmpty;
        let mut a = make_agent();
        a.add_skill(AgentSkill {
            name: NonEmpty::parse("formation").unwrap(),
            description: "File formations".to_owned(),
            parameters: serde_json::json!({}),
        });
        assert_eq!(a.skills().len(), 1);
        assert_eq!(a.skills()[0].name, "formation");
    }

    #[test]
    fn serde_roundtrip() {
        use agent_types::NonEmpty;
        let mut a = make_agent();
        a.add_skill(AgentSkill {
            name: NonEmpty::parse("equity").unwrap(),
            description: "Manage equity".to_owned(),
            parameters: serde_json::json!({}),
        });

        let json = serde_json::to_string(&a).unwrap();
        let parsed: Agent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.agent_id(), a.agent_id());
        assert_eq!(parsed.name(), "Test Agent");
        assert_eq!(parsed.skills().len(), 1);
    }
}
