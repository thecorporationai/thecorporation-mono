//! Agents domain — AI agents and their skills.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, EntityId, WorkspaceId};

// ── AgentStatus ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Inactive,
}

// ── AgentSkill ────────────────────────────────────────────────────────────────

/// A discrete capability or tool available to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkill {
    pub name: String,
    pub description: String,
    pub instructions: Option<String>,
}

// ── Agent ─────────────────────────────────────────────────────────────────────

/// An AI agent configured for a workspace, optionally scoped to an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub agent_id: AgentId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    /// When set, this agent operates in the context of a specific legal entity.
    pub entity_id: Option<EntityId>,
    pub skills: Vec<AgentSkill>,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
}

impl Agent {
    /// Create a new active agent with no skills.
    pub fn new(
        workspace_id: WorkspaceId,
        name: impl Into<String>,
        entity_id: Option<EntityId>,
    ) -> Self {
        Self {
            agent_id: AgentId::new(),
            workspace_id,
            name: name.into(),
            system_prompt: None,
            model: None,
            entity_id,
            skills: Vec::new(),
            status: AgentStatus::Active,
            created_at: Utc::now(),
        }
    }

    /// Append a skill to this agent's skill list.
    pub fn add_skill(&mut self, skill: AgentSkill) {
        self.skills.push(skill);
    }

    /// Remove a skill by name. Returns an error message if no skill with that
    /// name exists.
    pub fn remove_skill(&mut self, name: &str) -> Result<(), String> {
        let pos = self
            .skills
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| format!("skill '{}' not found", name))?;
        self.skills.remove(pos);
        Ok(())
    }

    /// Transition `Active → Inactive`.
    pub fn pause(&mut self) {
        self.status = AgentStatus::Inactive;
    }

    /// Transition `Inactive → Active`.
    pub fn resume(&mut self) {
        self.status = AgentStatus::Active;
    }

    /// Update the agent's display name.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Set or replace the model identifier (e.g. `"claude-sonnet-4-5"`).
    pub fn set_model(&mut self, model: Option<String>) {
        self.model = model;
    }

    /// Set or replace the system prompt.
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.system_prompt = prompt;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent() -> Agent {
        Agent::new(WorkspaceId::new(), "Corp Agent", None)
    }

    fn make_skill(name: &str) -> AgentSkill {
        AgentSkill {
            name: name.into(),
            description: format!("{name} skill"),
            instructions: None,
        }
    }

    // ── new() ─────────────────────────────────────────────────────────────────

    #[test]
    fn new_agent_is_active() {
        let agent = make_agent();
        assert_eq!(agent.status, AgentStatus::Active);
    }

    #[test]
    fn new_agent_has_no_skills() {
        let agent = make_agent();
        assert!(agent.skills.is_empty());
    }

    #[test]
    fn new_agent_stores_name() {
        let agent = make_agent();
        assert_eq!(agent.name, "Corp Agent");
    }

    #[test]
    fn new_agent_no_entity_id() {
        let agent = make_agent();
        assert!(agent.entity_id.is_none());
    }

    #[test]
    fn new_agent_with_entity_id() {
        let eid = EntityId::new();
        let agent = Agent::new(WorkspaceId::new(), "Agent With Entity", Some(eid));
        assert_eq!(agent.entity_id, Some(eid));
    }

    #[test]
    fn new_agent_no_system_prompt() {
        let agent = make_agent();
        assert!(agent.system_prompt.is_none());
    }

    #[test]
    fn new_agent_no_model() {
        let agent = make_agent();
        assert!(agent.model.is_none());
    }

    // ── add_skill() ───────────────────────────────────────────────────────────

    #[test]
    fn add_skill_appends() {
        let mut agent = make_agent();
        agent.add_skill(make_skill("filing"));
        assert_eq!(agent.skills.len(), 1);
        assert_eq!(agent.skills[0].name, "filing");
    }

    #[test]
    fn add_multiple_skills() {
        let mut agent = make_agent();
        agent.add_skill(make_skill("research"));
        agent.add_skill(make_skill("drafting"));
        agent.add_skill(make_skill("review"));
        assert_eq!(agent.skills.len(), 3);
    }

    #[test]
    fn agent_skill_with_instructions() {
        let skill = AgentSkill {
            name: "filing".into(),
            description: "File documents".into(),
            instructions: Some("Follow SEC guidelines".into()),
        };
        assert_eq!(skill.instructions.as_deref(), Some("Follow SEC guidelines"));
    }

    #[test]
    fn agent_skill_without_instructions() {
        let skill = make_skill("review");
        assert!(skill.instructions.is_none());
    }

    // ── pause() / resume() ───────────────────────────────────────────────────

    #[test]
    fn pause_active_agent() {
        let mut agent = make_agent();
        agent.pause();
        assert_eq!(agent.status, AgentStatus::Inactive);
    }

    #[test]
    fn pause_already_inactive_is_idempotent() {
        let mut agent = make_agent();
        agent.pause();
        agent.pause(); // should not panic
        assert_eq!(agent.status, AgentStatus::Inactive);
    }

    #[test]
    fn resume_inactive_agent() {
        let mut agent = make_agent();
        agent.pause();
        agent.resume();
        assert_eq!(agent.status, AgentStatus::Active);
    }

    #[test]
    fn resume_already_active_is_idempotent() {
        let mut agent = make_agent();
        agent.resume(); // already active, should not panic
        assert_eq!(agent.status, AgentStatus::Active);
    }

    #[test]
    fn pause_then_resume_cycle() {
        let mut agent = make_agent();
        agent.pause();
        assert_eq!(agent.status, AgentStatus::Inactive);
        agent.resume();
        assert_eq!(agent.status, AgentStatus::Active);
        agent.pause();
        assert_eq!(agent.status, AgentStatus::Inactive);
    }

    // ── set_name() / set_model() / set_system_prompt() ───────────────────────

    #[test]
    fn set_name_updates_field() {
        let mut agent = make_agent();
        agent.set_name("New Agent Name");
        assert_eq!(agent.name, "New Agent Name");
    }

    #[test]
    fn set_model_updates_field() {
        let mut agent = make_agent();
        agent.set_model(Some("claude-sonnet-4-5".into()));
        assert_eq!(agent.model.as_deref(), Some("claude-sonnet-4-5"));
    }

    #[test]
    fn set_model_to_none() {
        let mut agent = make_agent();
        agent.set_model(Some("old-model".into()));
        agent.set_model(None);
        assert!(agent.model.is_none());
    }

    #[test]
    fn set_system_prompt_updates_field() {
        let mut agent = make_agent();
        agent.set_system_prompt(Some("You are a corporate AI agent.".into()));
        assert_eq!(agent.system_prompt.as_deref(), Some("You are a corporate AI agent."));
    }

    #[test]
    fn set_system_prompt_to_none() {
        let mut agent = make_agent();
        agent.set_system_prompt(Some("prompt".into()));
        agent.set_system_prompt(None);
        assert!(agent.system_prompt.is_none());
    }

    // ── AgentStatus serde roundtrips ──────────────────────────────────────────

    #[test]
    fn agent_status_serde_roundtrip() {
        for status in [AgentStatus::Active, AgentStatus::Inactive] {
            let s = serde_json::to_string(&status).unwrap();
            let de: AgentStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, status);
        }
    }

    #[test]
    fn agent_status_serde_values() {
        assert_eq!(serde_json::to_string(&AgentStatus::Active).unwrap(), r#""active""#);
        assert_eq!(serde_json::to_string(&AgentStatus::Inactive).unwrap(), r#""inactive""#);
    }

    #[test]
    fn agent_ids_are_unique() {
        let a = make_agent();
        let b = make_agent();
        assert_ne!(a.agent_id, b.agent_id);
    }

    #[test]
    fn agent_skill_serde_roundtrip() {
        let skill = AgentSkill {
            name: "research".into(),
            description: "Conduct research".into(),
            instructions: Some("Be thorough".into()),
        };
        let json = serde_json::to_string(&skill).unwrap();
        let de: AgentSkill = serde_json::from_str(&json).unwrap();
        assert_eq!(de.name, "research");
        assert_eq!(de.instructions.as_deref(), Some("Be thorough"));
    }
}
