//! Agent route handlers — workspace-scoped AI agents.
//!
//! Agents are workspace-scoped (no `entity_id` in the path).  They are stored
//! under a sentinel entity ID (`AGENTS_ENTITY`) in the workspace's entity store.
//!
//! ## Route map
//!
//! | Method | Path | Scope |
//! |--------|------|-------|
//! | GET    | `/agents` | `AgentsRead` |
//! | POST   | `/agents` | `AgentsWrite` |
//! | GET    | `/agents/{agent_id}` | `AgentsRead` |
//! | PATCH  | `/agents/{agent_id}` | `AgentsWrite` |
//! | DELETE | `/agents/{agent_id}` | `AgentsWrite` |
//! | POST   | `/agents/{agent_id}/skills` | `AgentsWrite` |
//! | DELETE | `/agents/{agent_id}/skills/{name}` | `AgentsWrite` |
//! | POST   | `/agents/{agent_id}/pause` | `AgentsWrite` |
//! | POST   | `/agents/{agent_id}/resume` | `AgentsWrite` |

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;
use corp_auth::{RequireAgentsRead, RequireAgentsWrite};
use corp_core::agents::{Agent, AgentSkill};
use corp_core::ids::{AgentId, EntityId};

// ── Sentinel entity ID for workspace-scoped agent storage ─────────────────────
//
// All agents in a workspace are stored under a fixed, reserved entity UUID so
// that we can re-use EntityStore without requiring a real legal entity.
//
// Value: `00000000-0000-0000-0000-000000000001`
const AGENTS_ENTITY_STR: &str = "00000000-0000-0000-0000-000000000001";

fn agents_entity_id() -> EntityId {
    AGENTS_ENTITY_STR
        .parse()
        .expect("agents sentinel UUID is valid")
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/agents", get(list_agents).post(create_agent))
        .route(
            "/agents/{agent_id}",
            get(get_agent).patch(update_agent).delete(delete_agent),
        )
        .route("/agents/{agent_id}/skills", post(add_skill))
        .route("/agents/{agent_id}/skills/{name}", delete(remove_skill))
        .route("/agents/{agent_id}/pause", post(pause_agent))
        .route("/agents/{agent_id}/resume", post(resume_agent))
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub entity_id: Option<EntityId>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddSkillRequest {
    pub name: String,
    pub description: String,
    pub instructions: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Open (or lazily initialise) the agents entity store.
///
/// Uses the sentinel entity ID.  If the store does not yet exist for this
/// workspace it is created automatically.
async fn open_agents_store(
    state: &AppState,
    workspace_id: corp_core::ids::WorkspaceId,
) -> Result<corp_storage::entity_store::EntityStore, AppError> {
    let entity_id = agents_entity_id();
    match state.open_entity_store(workspace_id, entity_id).await {
        Ok(store) => Ok(store),
        Err(AppError::NotFound(_)) => {
            // First use — initialise the agents store for this workspace.
            state.init_entity_store(workspace_id, entity_id).await
        }
        Err(e) => Err(e),
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_agents(
    RequireAgentsRead(principal): RequireAgentsRead,
    State(state): State<AppState>,
) -> Result<Json<Vec<Agent>>, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    let agents = store
        .read_all::<Agent>("main")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(agents))
}

async fn create_agent(
    RequireAgentsWrite(principal): RequireAgentsWrite,
    State(state): State<AppState>,
    Json(body): Json<CreateAgentRequest>,
) -> Result<Json<Agent>, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    let mut agent = Agent::new(principal.workspace_id, body.name, body.entity_id);
    agent.set_system_prompt(body.system_prompt);
    agent.set_model(body.model);
    store
        .write::<Agent>(&agent, agent.agent_id, "main", "create agent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(agent))
}

async fn get_agent(
    RequireAgentsRead(principal): RequireAgentsRead,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
) -> Result<Json<Agent>, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    let agent = store.read::<Agent>(agent_id, "main").await.map_err(|e| {
        use corp_storage::error::StorageError;
        match e {
            StorageError::NotFound(_) => {
                AppError::NotFound(format!("agent {} not found", agent_id))
            }
            other => AppError::Storage(other),
        }
    })?;
    Ok(Json(agent))
}

async fn update_agent(
    RequireAgentsWrite(principal): RequireAgentsWrite,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Json(body): Json<UpdateAgentRequest>,
) -> Result<Json<Agent>, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    let mut agent = store
        .read::<Agent>(agent_id, "main")
        .await
        .map_err(AppError::Storage)?;

    if let Some(name) = body.name {
        agent.set_name(name);
    }
    if let Some(prompt) = body.system_prompt {
        agent.set_system_prompt(Some(prompt));
    }
    if let Some(model) = body.model {
        agent.set_model(Some(model));
    }

    store
        .write::<Agent>(&agent, agent_id, "main", "update agent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(agent))
}

async fn delete_agent(
    RequireAgentsWrite(principal): RequireAgentsWrite,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
) -> Result<StatusCode, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    store
        .delete::<Agent>(agent_id, "main", "delete agent")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::NotFound(format!("agent {} not found", agent_id))
                }
                other => AppError::Storage(other),
            }
        })?;
    Ok(StatusCode::NO_CONTENT)
}

async fn add_skill(
    RequireAgentsWrite(principal): RequireAgentsWrite,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Json(body): Json<AddSkillRequest>,
) -> Result<Json<Agent>, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    let mut agent = store
        .read::<Agent>(agent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    let skill = AgentSkill {
        name: body.name,
        description: body.description,
        instructions: body.instructions,
    };
    agent.add_skill(skill);
    store
        .write::<Agent>(&agent, agent_id, "main", "add skill to agent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(agent))
}

async fn remove_skill(
    RequireAgentsWrite(principal): RequireAgentsWrite,
    State(state): State<AppState>,
    Path((agent_id, name)): Path<(AgentId, String)>,
) -> Result<Json<Agent>, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    let mut agent = store
        .read::<Agent>(agent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    agent.remove_skill(&name).map_err(AppError::NotFound)?;
    store
        .write::<Agent>(&agent, agent_id, "main", "remove skill from agent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(agent))
}

async fn pause_agent(
    RequireAgentsWrite(principal): RequireAgentsWrite,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
) -> Result<Json<Agent>, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    let mut agent = store
        .read::<Agent>(agent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    agent.pause();
    store
        .write::<Agent>(&agent, agent_id, "main", "pause agent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(agent))
}

async fn resume_agent(
    RequireAgentsWrite(principal): RequireAgentsWrite,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
) -> Result<Json<Agent>, AppError> {
    let store = open_agents_store(&state, principal.workspace_id).await?;
    let mut agent = store
        .read::<Agent>(agent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    agent.resume();
    store
        .write::<Agent>(&agent, agent_id, "main", "resume agent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(agent))
}
