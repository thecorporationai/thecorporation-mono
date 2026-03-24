//! `corp agents` — AI agent management.

use serde_json::json;

use crate::output;
use super::Context;

// ── AgentsCommand ─────────────────────────────────────────────────────────────

#[derive(clap::Subcommand)]
#[command(long_about = "Manage AI agents. Agents can be scoped to entities and given skills to automate tasks.")]
pub enum AgentsCommand {
    /// List agents in the workspace
    List,

    /// Show an agent
    Show {
        /// Agent ID (from `corp agents list`)
        agent_ref: String,
    },

    /// Create a new agent
    Create {
        #[arg(long, help = "Agent display name")]
        name: String,

        #[arg(long, help = "System prompt defining agent behavior")]
        prompt: Option<String>,

        #[arg(long, help = "LLM model (e.g. claude-sonnet-4-6)")]
        model: Option<String>,

        #[arg(long, help = "Entity to scope agent to (omit for workspace-scoped)")]
        entity_id: Option<String>,
    },

    /// Update an agent
    Update {
        /// Agent ID (from `corp agents list`)
        agent_ref: String,

        /// New display name
        #[arg(long)]
        name: Option<String>,

        #[arg(long, help = "System prompt defining agent behavior")]
        prompt: Option<String>,

        #[arg(long, help = "LLM model (e.g. claude-sonnet-4-6)")]
        model: Option<String>,
    },

    /// Add a skill to an agent
    AddSkill {
        /// Agent ID (from `corp agents list`)
        agent_ref: String,

        #[arg(long, help = "Skill name (e.g. document-review)")]
        name: String,

        #[arg(long, help = "What the skill does")]
        description: String,

        #[arg(long, help = "Detailed execution instructions")]
        instructions: Option<String>,
    },

    /// Remove a skill from an agent
    RemoveSkill {
        /// Agent ID (from `corp agents list`)
        agent_ref: String,

        /// Skill name to remove
        #[arg(long)]
        name: String,
    },

    /// Pause an agent — stops processing new tasks
    Pause {
        /// Agent ID (from `corp agents list`)
        agent_ref: String,
    },

    /// Resume a paused agent
    Resume {
        /// Agent ID (from `corp agents list`)
        agent_ref: String,
    },

    /// Permanently delete an agent
    Delete {
        /// Agent ID (from `corp agents list`)
        agent_ref: String,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: AgentsCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();

    match cmd {
        AgentsCommand::List => {
            let value = ctx.client.get("/v1/agents").await?;
            output::print_value(&value, mode);
        }

        AgentsCommand::Show { agent_ref } => {
            let path = format!("/v1/agents/{agent_ref}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        AgentsCommand::Create { name, prompt, model, entity_id } => {
            let body = json!({
                "name": name,
                "system_prompt": prompt,
                "model": model,
                "entity_id": entity_id,
            });
            let value = ctx.client.post("/v1/agents", &body).await?;
            output::print_value(&value, mode);
            output::print_success("Agent created.", mode);
        }

        AgentsCommand::Update { agent_ref, name, prompt, model } => {
            let path = format!("/v1/agents/{agent_ref}");
            let mut patch = serde_json::Map::new();
            if let Some(n) = name { patch.insert("name".into(), json!(n)); }
            if let Some(p) = prompt { patch.insert("system_prompt".into(), json!(p)); }
            if let Some(m) = model { patch.insert("model".into(), json!(m)); }
            let value = ctx.client.patch(&path, &serde_json::Value::Object(patch)).await?;
            output::print_value(&value, mode);
            output::print_success("Agent updated.", mode);
        }

        AgentsCommand::AddSkill { agent_ref, name, description, instructions } => {
            let path = format!("/v1/agents/{agent_ref}/skills");
            let body = json!({
                "name": name,
                "description": description,
                "instructions": instructions,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Skill added.", mode);
        }

        AgentsCommand::RemoveSkill { agent_ref, name } => {
            let path = format!("/v1/agents/{agent_ref}/skills/{name}");
            let value = ctx.client.delete(&path).await?;
            output::print_value(&value, mode);
            output::print_success("Skill removed.", mode);
        }

        AgentsCommand::Pause { agent_ref } => {
            let path = format!("/v1/agents/{agent_ref}/pause");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Agent paused.", mode);
        }

        AgentsCommand::Resume { agent_ref } => {
            let path = format!("/v1/agents/{agent_ref}/resume");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Agent resumed.", mode);
        }

        AgentsCommand::Delete { agent_ref } => {
            let path = format!("/v1/agents/{agent_ref}");
            let value = ctx.client.delete(&path).await?;
            output::print_value(&value, mode);
            output::print_success("Agent deleted.", mode);
        }
    }

    Ok(())
}
