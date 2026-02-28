//! Agent hierarchy resolution.
//!
//! Agents can have a `parent_agent_id` forming a chain of inheritance.
//! Resolution walks the chain and merges config at read time so that
//! changes to a parent cascade instantly to all children.

use std::collections::HashSet;

use crate::domain::ids::AgentId;
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;

use super::agent::Agent;
use super::types::{AgentSkill, MCPServerSpec, ToolSpec};

/// Maximum parent chain depth (prevents runaway recursion).
const MAX_DEPTH: usize = 5;

/// Resolve an agent by walking its parent chain and merging configs.
///
/// The returned `Agent` has the child's identity (id, name, status, etc.)
/// but with inherited tools, mcp_servers, skills, system_prompt, model,
/// budget, and sandbox from ancestors.
pub fn resolve_agent(ws_store: &WorkspaceStore, agent_id: AgentId) -> Result<Agent, AppError> {
    let chain = walk_parent_chain(ws_store, agent_id)?;

    // chain[0] is the requested agent, chain[last] is the root ancestor.
    // Merge from root down so that each level overrides its parent.
    let mut iter = chain.into_iter().rev();
    let mut merged = iter.next().expect("chain is never empty");

    for child in iter {
        merged = merge_agents(&merged, &child);
    }

    Ok(merged)
}

/// Walk up the parent chain starting from `agent_id`.
///
/// Returns a vec where index 0 is the requested agent and the last
/// element is the root ancestor (the one with no parent).
pub fn walk_parent_chain(
    ws_store: &WorkspaceStore,
    agent_id: AgentId,
) -> Result<Vec<Agent>, AppError> {
    let mut chain = Vec::new();
    let mut visited = HashSet::new();
    let mut current_id = agent_id;

    loop {
        if !visited.insert(current_id) {
            return Err(AppError::BadRequest(format!(
                "cycle detected in agent parent chain at {}",
                current_id
            )));
        }

        if chain.len() >= MAX_DEPTH {
            return Err(AppError::BadRequest(format!(
                "agent parent chain exceeds maximum depth of {}",
                MAX_DEPTH
            )));
        }

        let path = format!("agents/{}.json", current_id);
        let agent: Agent = ws_store
            .read_json(&path)
            .map_err(|_| AppError::NotFound(format!("agent {} not found", current_id)))?;

        let parent = agent.parent_agent_id();
        chain.push(agent);

        match parent {
            Some(pid) => current_id = pid,
            None => break,
        }
    }

    Ok(chain)
}

/// Merge a parent agent's config into a child agent.
///
/// The child's identity fields (id, name, status, workspace_id, etc.)
/// are preserved. Inherited fields use the merge strategies from the plan.
pub fn merge_agents(parent: &Agent, child: &Agent) -> Agent {
    // Start with a clone of the child so identity fields are correct.
    let mut merged = child.clone();

    // system_prompt: concatenate parent + child
    let prompt = match (parent.system_prompt(), child.system_prompt()) {
        (Some(p), Some(c)) => Some(format!("{}\n\n---\n\n{}", p, c)),
        (Some(p), None) => Some(p.to_owned()),
        (None, Some(c)) => Some(c.to_owned()),
        (None, None) => None,
    };
    merged.set_system_prompt(prompt);

    // model: child overrides, else inherit
    if child.model().is_none() {
        merged.set_model(parent.model().map(|s| s.to_owned()));
    }

    // tools: union by name, child overrides
    merged.set_tools(merge_by_name(parent.tools(), child.tools()));

    // mcp_servers: union by name, child overrides
    merged.set_mcp_servers(merge_by_name(parent.mcp_servers(), child.mcp_servers()));

    // skills: union by name, child overrides
    merged.set_skills(merge_by_name(parent.skills(), child.skills()));

    // channels: child only (not inherited) — already set from clone

    // budget: child overrides if set, else inherit
    if child.budget().is_none() {
        merged.set_budget(parent.budget().cloned());
    }

    // sandbox: child overrides if set, else inherit
    if child.sandbox().is_none() {
        merged.set_sandbox(parent.sandbox().cloned());
    }

    // scopes: union (child adds to parent, never subtracts)
    merged.set_scopes(parent.scopes().union(child.scopes()));

    merged
}

/// Trait for types that have a name field, used for union-by-name merging.
pub trait HasName {
    fn name(&self) -> &str;
}

impl HasName for ToolSpec {
    fn name(&self) -> &str {
        &self.name
    }
}

impl HasName for MCPServerSpec {
    fn name(&self) -> &str {
        &self.name
    }
}

impl HasName for AgentSkill {
    fn name(&self) -> &str {
        &self.name
    }
}

/// Merge two slices by name. Items from `child` override items with the
/// same name from `parent`. Items in `parent` that are not in `child`
/// are appended.
fn merge_by_name<T: HasName + Clone>(parent: &[T], child: &[T]) -> Vec<T> {
    let child_names: HashSet<&str> = child.iter().map(|item| item.name()).collect();
    let mut result: Vec<T> = child.to_vec();

    for item in parent {
        if !child_names.contains(item.name()) {
            result.push(item.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::types::BudgetConfig;
    use crate::domain::ids::WorkspaceId;

    fn make_agent(name: &str) -> Agent {
        Agent::new(
            AgentId::new(),
            WorkspaceId::new(),
            name.to_owned(),
            None,
            None,
            None,
        )
    }

    #[test]
    fn merge_system_prompt_concatenation() {
        let mut parent = make_agent("parent");
        parent.set_system_prompt(Some("You are a corporate agent.".to_owned()));

        let mut child = make_agent("child");
        child.set_system_prompt(Some("Focus on finance.".to_owned()));

        let merged = merge_agents(&parent, &child);
        let prompt = merged.system_prompt().unwrap();
        assert!(prompt.contains("You are a corporate agent."));
        assert!(prompt.contains("---"));
        assert!(prompt.contains("Focus on finance."));
    }

    #[test]
    fn merge_system_prompt_parent_only() {
        let mut parent = make_agent("parent");
        parent.set_system_prompt(Some("Parent prompt.".to_owned()));
        let child = make_agent("child");

        let merged = merge_agents(&parent, &child);
        assert_eq!(merged.system_prompt(), Some("Parent prompt."));
    }

    #[test]
    fn merge_model_child_overrides() {
        let mut parent = make_agent("parent");
        parent.set_model(Some("gpt-4".to_owned()));

        let mut child = make_agent("child");
        child.set_model(Some("claude-sonnet-4-6".to_owned()));

        let merged = merge_agents(&parent, &child);
        assert_eq!(merged.model(), Some("claude-sonnet-4-6"));
    }

    #[test]
    fn merge_model_inherits_from_parent() {
        let mut parent = make_agent("parent");
        parent.set_model(Some("gpt-4".to_owned()));
        let child = make_agent("child");

        let merged = merge_agents(&parent, &child);
        assert_eq!(merged.model(), Some("gpt-4"));
    }

    #[test]
    fn merge_tools_union_by_name() {
        let mut parent = make_agent("parent");
        use crate::domain::agents::types::{HttpMethod, NonEmpty};

        parent.set_tools(vec![
            ToolSpec {
                name: NonEmpty::parse("fetch").unwrap(),
                description: Some("Parent fetch".to_owned()),
                method: HttpMethod::Get,
                url: NonEmpty::parse("http://parent/fetch").unwrap(),
                headers: Default::default(),
                parameters: serde_json::json!({}),
                body_schema: serde_json::json!({}),
            },
            ToolSpec {
                name: NonEmpty::parse("search").unwrap(),
                description: Some("Parent search".to_owned()),
                method: HttpMethod::Get,
                url: NonEmpty::parse("http://parent/search").unwrap(),
                headers: Default::default(),
                parameters: serde_json::json!({}),
                body_schema: serde_json::json!({}),
            },
        ]);

        let mut child = make_agent("child");
        child.set_tools(vec![ToolSpec {
            name: NonEmpty::parse("fetch").unwrap(),
            description: Some("Child fetch override".to_owned()),
            method: HttpMethod::Post,
            url: NonEmpty::parse("http://child/fetch").unwrap(),
            headers: Default::default(),
            parameters: serde_json::json!({}),
            body_schema: serde_json::json!({}),
        }]);

        let merged = merge_agents(&parent, &child);
        assert_eq!(merged.tools().len(), 2);

        let fetch = merged.tools().iter().find(|t| t.name == "fetch").unwrap();
        assert_eq!(fetch.url, "http://child/fetch"); // child overrides
        assert_eq!(fetch.method, HttpMethod::Post);

        let search = merged.tools().iter().find(|t| t.name == "search").unwrap();
        assert_eq!(search.url, "http://parent/search"); // inherited
    }

    #[test]
    fn merge_budget_child_overrides() {
        let mut parent = make_agent("parent");
        parent.set_budget(Some(BudgetConfig {
            max_turns: 50,
            ..Default::default()
        }));

        let mut child = make_agent("child");
        child.set_budget(Some(BudgetConfig {
            max_turns: 10,
            ..Default::default()
        }));

        let merged = merge_agents(&parent, &child);
        assert_eq!(merged.budget().unwrap().max_turns, 10);
    }

    #[test]
    fn merge_budget_inherits_from_parent() {
        let mut parent = make_agent("parent");
        parent.set_budget(Some(BudgetConfig {
            max_turns: 50,
            ..Default::default()
        }));
        let child = make_agent("child");

        let merged = merge_agents(&parent, &child);
        assert_eq!(merged.budget().unwrap().max_turns, 50);
    }

    #[test]
    fn merge_channels_not_inherited() {
        use crate::domain::agents::types::ChannelConfig;

        let mut parent = make_agent("parent");
        parent.set_channels(vec![ChannelConfig::Email {
            address: Some("parent@example.com".to_owned()),
            webhook_secret: None,
        }]);

        let child = make_agent("child");
        let merged = merge_agents(&parent, &child);
        assert!(merged.channels().is_empty());
    }

    #[test]
    fn merge_preserves_child_identity() {
        let parent = make_agent("Parent Agent");
        let child = make_agent("Child Agent");

        let merged = merge_agents(&parent, &child);
        assert_eq!(merged.name(), "Child Agent");
        assert_eq!(merged.agent_id(), child.agent_id());
    }

    #[test]
    fn merge_scopes_union() {
        use crate::domain::auth::scopes::{Scope, ScopeSet};

        let mut parent = make_agent("parent");
        parent.set_scopes(ScopeSet::from_vec(vec![
            Scope::FormationRead,
            Scope::EquityRead,
        ]));

        let mut child = make_agent("child");
        child.set_scopes(ScopeSet::from_vec(vec![Scope::EquityRead, Scope::Admin]));

        let merged = merge_agents(&parent, &child);
        assert!(merged.scopes().has(Scope::FormationRead));
        assert!(merged.scopes().has(Scope::EquityRead));
        assert!(merged.scopes().has(Scope::Admin));
        assert!(!merged.scopes().has(Scope::TreasuryRead));
    }

    #[test]
    fn merge_by_name_works() {
        use crate::domain::agents::types::NonEmpty;

        let parent = vec![
            AgentSkill {
                name: NonEmpty::parse("a").unwrap(),
                description: "parent a".to_owned(),
                parameters: serde_json::json!({}),
            },
            AgentSkill {
                name: NonEmpty::parse("b").unwrap(),
                description: "parent b".to_owned(),
                parameters: serde_json::json!({}),
            },
        ];
        let child = vec![AgentSkill {
            name: NonEmpty::parse("b").unwrap(),
            description: "child b".to_owned(),
            parameters: serde_json::json!({}),
        }];

        let result = merge_by_name(&parent, &child);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.iter().find(|s| s.name == "b").unwrap().description,
            "child b"
        );
        assert_eq!(
            result.iter().find(|s| s.name == "a").unwrap().description,
            "parent a"
        );
    }
}
