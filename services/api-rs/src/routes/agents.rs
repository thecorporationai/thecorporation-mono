//! Agent management HTTP routes.
//!
//! Endpoints for creating, listing, updating agents, adding skills, and messaging.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::domain::agents::{
    agent::Agent,
    types::{AgentSkill, AgentStatus},
};
use crate::domain::ids::{AgentId, EntityId, WorkspaceId};
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateAgentRequest {
    pub workspace_id: WorkspaceId,
    pub name: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub entity_id: Option<EntityId>,
}

#[derive(Deserialize)]
pub struct UpdateAgentRequest {
    pub workspace_id: WorkspaceId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub status: Option<AgentStatus>,
    #[serde(default)]
    pub webhook_url: Option<String>,
}

#[derive(Deserialize)]
pub struct AddSkillRequest {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub workspace_id: WorkspaceId,
    pub message: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct AgentResponse {
    pub agent_id: AgentId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub entity_id: Option<EntityId>,
    pub skills: Vec<AgentSkill>,
    pub status: AgentStatus,
    pub email_address: Option<String>,
    pub webhook_url: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub agent_id: AgentId,
    pub status: String,
    pub message: String,
}

fn agent_to_response(a: &Agent) -> AgentResponse {
    AgentResponse {
        agent_id: a.agent_id(),
        workspace_id: a.workspace_id(),
        name: a.name().to_owned(),
        system_prompt: a.system_prompt().map(|s| s.to_owned()),
        model: a.model().map(|s| s.to_owned()),
        entity_id: a.entity_id(),
        skills: a.skills().to_vec(),
        status: a.status(),
        email_address: a.email_address().map(|s| s.to_owned()),
        webhook_url: a.webhook_url().map(|s| s.to_owned()),
        created_at: a.created_at().to_rfc3339(),
    }
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn create_agent(
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<AgentResponse>), AppError> {
    if req.name.is_empty() || req.name.len() > 256 {
        return Err(AppError::BadRequest(
            "agent name must be between 1 and 256 characters".to_owned(),
        ));
    }
    let workspace_id = req.workspace_id;

    let agent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let agent_id = AgentId::new();
            let agent = Agent::new(
                agent_id,
                workspace_id,
                req.name,
                req.system_prompt,
                req.model,
                req.entity_id,
            );

            let path = format!("agents/{}.json", agent_id);
            ws_store
                .write_json(&path, &agent, &format!("Create agent {agent_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(agent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok((StatusCode::CREATED, Json(agent_to_response(&agent))))
}

#[derive(Deserialize)]
pub struct WorkspaceQuery {
    pub workspace_id: WorkspaceId,
}

async fn list_agents(
    State(state): State<AppState>,
    Query(query): Query<WorkspaceQuery>,
) -> Result<Json<Vec<AgentResponse>>, AppError> {
    let workspace_id = query.workspace_id;
    let agents = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let ids: Vec<AgentId> = ws_store
                .list_ids_in_dir_pub("agents")
                .map_err(|e| AppError::Internal(format!("list agents: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let path = format!("agents/{}.json", id);
                if let Ok(agent) = ws_store.read_json::<Agent>(&path) {
                    results.push(agent_to_response(&agent));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(agents))
}

async fn update_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<AgentResponse>, AppError> {
    let workspace_id = req.workspace_id;

    let agent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let path = format!("agents/{}.json", agent_id);
            let mut agent: Agent = ws_store
                .read_json(&path)
                .map_err(|_| AppError::NotFound(format!("agent {} not found", agent_id)))?;

            if let Some(name) = req.name {
                agent.set_name(name);
            }
            if req.system_prompt.is_some() {
                agent.set_system_prompt(req.system_prompt);
            }
            if req.model.is_some() {
                agent.set_model(req.model);
            }
            if let Some(status) = req.status {
                agent.set_status(status);
            }
            if req.webhook_url.is_some() {
                agent.set_webhook_url(req.webhook_url);
            }

            ws_store
                .write_json(&path, &agent, &format!("Update agent {agent_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(agent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(agent_to_response(&agent)))
}

async fn add_agent_skill(
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Json(req): Json<AddSkillRequest>,
) -> Result<Json<AgentResponse>, AppError> {
    let workspace_id = req.workspace_id;

    let agent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let path = format!("agents/{}.json", agent_id);
            let mut agent: Agent = ws_store
                .read_json(&path)
                .map_err(|_| AppError::NotFound(format!("agent {} not found", agent_id)))?;

            agent.add_skill(AgentSkill {
                name: req.name,
                description: req.description,
                parameters: req.parameters,
            });

            ws_store
                .write_json(&path, &agent, &format!("Add skill to agent {agent_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(agent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(agent_to_response(&agent)))
}

async fn send_agent_message(
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    if req.message.is_empty() {
        return Err(AppError::BadRequest("message cannot be empty".to_owned()));
    }
    let workspace_id = req.workspace_id;

    let msg = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let message_text = req.message.clone();
        let metadata = req.metadata.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            // Verify agent exists
            let agent_path = format!("agents/{}.json", agent_id);
            let _agent: Agent = ws_store
                .read_json(&agent_path)
                .map_err(|_| AppError::NotFound(format!("agent {} not found", agent_id)))?;

            // Store the message
            let message_id = crate::domain::ids::MessageId::new();
            let msg = crate::domain::agents::message::AgentMessage::new(
                message_id,
                agent_id,
                message_text,
                metadata,
            );

            let msg_path = format!("agents/{}/messages/{}.json", agent_id, message_id);
            ws_store
                .write_json(&msg_path, &msg, &format!("Message {message_id} to agent {agent_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(msg)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(MessageResponse {
        agent_id,
        status: msg.status().to_owned(),
        message: format!("Message {} queued for agent {}", msg.message_id(), agent_id),
    }))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn agent_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/agents", post(create_agent).get(list_agents))
        .route("/v1/agents/{agent_id}", patch(update_agent))
        .route("/v1/agents/{agent_id}/skills", post(add_agent_skill))
        .route("/v1/agents/{agent_id}/messages", post(send_agent_message))
}
