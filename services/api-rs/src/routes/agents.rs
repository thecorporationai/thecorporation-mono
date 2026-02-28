//! Agent management HTTP routes.
//!
//! Endpoints for creating, listing, updating agents, adding skills, and messaging.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::{RequireAdmin, RequireInternalWorker};
use crate::domain::agents::{
    agent::Agent,
    resolve,
    types::{
        AgentSkill, AgentStatus, BudgetConfig, ChannelConfig, MCPServerSpec, NonEmpty,
        SandboxConfig, ToolSpec,
    },
};
use crate::domain::auth::claims::{Claims, PrincipalType, encode_token};
use crate::domain::auth::scopes::{Scope, ScopeSet};
use crate::domain::ids::{AgentId, EntityId, ExecutionId, MessageId, WorkspaceId};
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;
use agent_types::{AgentDefinition, ChannelType, InboundMessage, JobPayload, RpcReply, RpcStatus};

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateAgentRequest {
    pub name: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub entity_id: Option<EntityId>,
    #[serde(default)]
    pub parent_agent_id: Option<AgentId>,
    #[serde(default)]
    pub scopes: Vec<Scope>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAgentRequest {
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
    #[serde(default)]
    pub tools: Option<Vec<ToolSpec>>,
    #[serde(default)]
    pub mcp_servers: Option<Vec<MCPServerSpec>>,
    #[serde(default)]
    pub channels: Option<Vec<ChannelConfig>>,
    #[serde(default)]
    pub budget: Option<BudgetConfig>,
    #[serde(default)]
    pub sandbox: Option<SandboxConfig>,
    #[serde(default)]
    pub parent_agent_id: Option<AgentId>,
    #[serde(default)]
    pub scopes: Option<Vec<Scope>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddSkillRequest {
    /// Parsed at deserialization — empty names are rejected by `NonEmpty`.
    pub name: NonEmpty,
    pub description: String,
    #[serde(default)]
    pub parameters: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendMessageRequest {
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
    pub tools: Vec<ToolSpec>,
    pub mcp_servers: Vec<MCPServerSpec>,
    pub channels: Vec<ChannelConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<BudgetConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxConfig>,
    pub status: AgentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_agent_id: Option<AgentId>,
    pub email_address: Option<String>,
    pub webhook_url: Option<String>,
    pub scopes: Vec<Scope>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub agent_id: AgentId,
    pub message_id: MessageId,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_id: Option<ExecutionId>,
    pub message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerWorkspaceQuery {
    pub workspace_id: WorkspaceId,
}

#[derive(Serialize)]
pub struct InternalChannelResponse {
    #[serde(rename = "type")]
    pub channel_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
}

#[derive(Serialize)]
pub struct InternalCronAgentResponse {
    pub agent_id: String,
    pub workspace_id: String,
    pub status: String,
    pub channels: Vec<InternalChannelResponse>,
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
        tools: a.tools().to_vec(),
        mcp_servers: a.mcp_servers().to_vec(),
        channels: a.channels().to_vec(),
        budget: a.budget().cloned(),
        sandbox: a.sandbox().cloned(),
        status: a.status(),
        parent_agent_id: a.parent_agent_id(),
        email_address: a.email_address().map(|s| s.to_owned()),
        webhook_url: a.webhook_url().map(|s| s.to_owned()),
        scopes: a.scopes().to_vec(),
        created_at: a.created_at().to_rfc3339(),
    }
}

fn agent_to_definition(a: &Agent) -> Result<AgentDefinition, AppError> {
    Ok(AgentDefinition {
        id: a.agent_id(),
        workspace_id: Some(a.workspace_id()),
        entity_id: a.entity_id().map(|id| id.to_string()),
        name: NonEmpty::parse(a.name().to_owned())
            .map_err(|e| AppError::Internal(format!("invalid agent name: {e}")))?,
        status: a.status(),
        system_prompt: a.system_prompt().unwrap_or_default().to_owned(),
        model: a
            .model()
            .unwrap_or("anthropic/claude-sonnet-4-6")
            .to_owned(),
        tools: a.tools().to_vec(),
        skills: a
            .skills()
            .iter()
            .map(|s| agent_types::SkillSpec {
                name: s.name.clone(),
                description: s.description.clone(),
                instructions: String::new(),
                tools: Vec::new(),
                mcp_server: None,
                enabled: true,
            })
            .collect(),
        mcp_servers: a.mcp_servers().to_vec(),
        channels: a.channels().to_vec(),
        budget: a.budget().cloned().unwrap_or_default(),
        sandbox: a.sandbox().cloned().unwrap_or_default(),
        parent_agent_id: a.parent_agent_id(),
        email_address: a.email_address().map(|s| s.to_owned()),
        webhook_url: a.webhook_url().map(|s| s.to_owned()),
        created_at: Some(a.created_at()),
        updated_at: Some(a.created_at()),
    })
}

fn budget_month_key(agent_id: AgentId) -> String {
    let month = chrono::Utc::now().format("%Y-%m");
    format!("aw:budget:agent:{agent_id}:{month}")
}

async fn enforce_monthly_budget(
    redis: &deadpool_redis::Pool,
    agent_id: AgentId,
    budget_cents: u64,
) -> Result<(), AppError> {
    use deadpool_redis::redis::AsyncCommands;

    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis pool: {e}")))?;
    let key = budget_month_key(agent_id);
    let spent_raw: Option<i64> = conn
        .get(&key)
        .await
        .map_err(|e| AppError::Internal(format!("redis get: {e}")))?;
    let spent_cents = spent_raw.unwrap_or(0).max(0) as u64;
    if spent_cents >= budget_cents {
        return Err(AppError::Conflict(format!(
            "Monthly budget exceeded ({spent_cents} >= {budget_cents} cents)"
        )));
    }
    Ok(())
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn create_agent(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<AgentResponse>), AppError> {
    if req.name.is_empty() || req.name.len() > 256 {
        return Err(AppError::BadRequest(
            "agent name must be between 1 and 256 characters".to_owned(),
        ));
    }
    let workspace_id = auth.workspace_id();

    let agent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            // Validate parent_agent_id if provided
            if let Some(parent_id) = req.parent_agent_id {
                validate_parent(&ws_store, parent_id, None)?;
            }

            let agent_id = AgentId::new();
            let mut agent = Agent::new(
                agent_id,
                workspace_id,
                req.name,
                req.system_prompt,
                req.model,
                req.entity_id,
            );
            agent.set_parent_agent_id(req.parent_agent_id);
            if !req.scopes.is_empty() {
                agent.set_scopes(ScopeSet::from_vec(req.scopes));
            }

            let path = format!("agents/{}.json", agent_id);
            ws_store
                .write_json(&path, &agent, &format!("Create agent {agent_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(agent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok((StatusCode::CREATED, Json(agent_to_response(&agent))))
}

async fn list_agents(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<AgentResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(agents))
}

async fn update_agent(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<AgentResponse>, AppError> {
    let workspace_id = auth.workspace_id();

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
            if let Some(tools) = req.tools {
                agent.set_tools(tools);
            }
            if let Some(mcp_servers) = req.mcp_servers {
                agent.set_mcp_servers(mcp_servers);
            }
            if let Some(channels) = req.channels {
                agent.set_channels(channels);
            }
            if req.budget.is_some() {
                agent.set_budget(req.budget);
            }
            if req.sandbox.is_some() {
                agent.set_sandbox(req.sandbox);
            }
            if let Some(parent_id) = req.parent_agent_id {
                validate_parent(&ws_store, parent_id, Some(agent_id))?;
                agent.set_parent_agent_id(Some(parent_id));
            }
            if let Some(scopes) = req.scopes {
                agent.set_scopes(ScopeSet::from_vec(scopes));
            }

            ws_store
                .write_json(&path, &agent, &format!("Update agent {agent_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(agent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(agent_to_response(&agent)))
}

async fn add_agent_skill(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Json(req): Json<AddSkillRequest>,
) -> Result<Json<AgentResponse>, AppError> {
    let workspace_id = auth.workspace_id();

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(agent_to_response(&agent)))
}

async fn send_agent_message(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    if req.message.is_empty() {
        return Err(AppError::BadRequest("message cannot be empty".to_owned()));
    }
    let workspace_id = auth.workspace_id();

    let (agent_status, budget_limit_cents) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;
            let agent_path = format!("agents/{}.json", agent_id);
            let agent: Agent = ws_store
                .read_json(&agent_path)
                .map_err(|_| AppError::NotFound(format!("agent {} not found", agent_id)))?;
            let budget_limit_cents = agent
                .budget()
                .cloned()
                .unwrap_or_default()
                .max_monthly_cost_cents;
            Ok::<_, AppError>((agent.status(), budget_limit_cents))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    if agent_status != AgentStatus::Active {
        return Err(AppError::Conflict(format!(
            "agent {} is {}",
            agent_id,
            agent_status.as_str()
        )));
    }

    if let Some(redis) = &state.redis {
        enforce_monthly_budget(redis, agent_id, budget_limit_cents).await?;
    }

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

            if _agent.status() != AgentStatus::Active {
                return Err(AppError::Conflict(format!(
                    "agent {} is {}",
                    agent_id,
                    _agent.status().as_str()
                )));
            }

            // Store the message
            let message_id = MessageId::new();
            let msg = crate::domain::agents::message::AgentMessage::new(
                message_id,
                agent_id,
                message_text,
                metadata,
            );

            let msg_path = format!("agents/{}/messages/{}.json", agent_id, message_id);
            ws_store
                .write_json(
                    &msg_path,
                    &msg,
                    &format!("Message {message_id} to agent {agent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(msg)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    // Enqueue to Redis for worker dispatch (if Redis is configured)
    let execution_id = if let Some(ref redis) = state.redis {
        match enqueue_execution(
            redis,
            workspace_id,
            agent_id,
            msg.message_id(),
            state.max_queue_depth,
        )
        .await
        {
            Ok(exec_id) => Some(exec_id),
            Err(e @ AppError::ServiceUnavailable(_)) => return Err(e),
            Err(e) => {
                tracing::error!(error = ?e, "failed to enqueue execution");
                None
            }
        }
    } else {
        None
    };

    Ok(Json(MessageResponse {
        agent_id,
        message_id: msg.message_id(),
        status: if execution_id.is_some() {
            "accepted".to_owned()
        } else {
            msg.status().to_owned()
        },
        execution_id,
        message: format!("Message {} sent to agent {}", msg.message_id(), agent_id),
    }))
}

async fn get_agent_message_internal(
    _worker: RequireInternalWorker,
    State(state): State<AppState>,
    Path((agent_id, message_id)): Path<(AgentId, MessageId)>,
    Query(query): Query<WorkerWorkspaceQuery>,
) -> Result<Json<InboundMessage>, AppError> {
    let workspace_id = query.workspace_id;

    let msg = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let agent_path = format!("agents/{}.json", agent_id);
            let _agent: Agent = ws_store
                .read_json(&agent_path)
                .map_err(|_| AppError::NotFound(format!("agent {} not found", agent_id)))?;

            let msg_path = format!("agents/{}/messages/{}.json", agent_id, message_id);
            let msg: crate::domain::agents::message::AgentMessage =
                ws_store
                    .read_json(&msg_path)
                    .map_err(|_| AppError::NotFound(format!("message {} not found", message_id)))?;

            Ok::<_, AppError>(msg)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let channel_metadata = msg
        .metadata()
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    Ok(Json(InboundMessage {
        id: msg.message_id(),
        agent_id: msg.agent_id(),
        channel: ChannelType::Manual,
        sender: None,
        subject: None,
        body: msg.content().to_owned(),
        attachments: Vec::new(),
        channel_metadata,
        received_at: Some(msg.created_at()),
    }))
}

/// Enqueue an execution job to Redis and wait for worker acknowledgment.
async fn enqueue_execution(
    redis: &deadpool_redis::Pool,
    workspace_id: WorkspaceId,
    agent_id: AgentId,
    message_id: MessageId,
    max_queue_depth: u64,
) -> Result<ExecutionId, AppError> {
    use deadpool_redis::redis::AsyncCommands;

    // Check queue depth before enqueuing
    if max_queue_depth > 0 {
        let mut conn = redis
            .get()
            .await
            .map_err(|e| AppError::Internal(format!("redis pool: {e}")))?;
        let current: u64 = conn
            .llen("aw:queue:jobs")
            .await
            .map_err(|e| AppError::Internal(format!("redis llen: {e}")))?;
        if current >= max_queue_depth {
            return Err(AppError::ServiceUnavailable(format!(
                "execution queue is full ({current}/{max_queue_depth})"
            )));
        }
    }

    let execution_id = ExecutionId::new();
    let job = JobPayload::new(execution_id, agent_id, workspace_id, message_id);
    let reply_key = format!("aw:rpc:reply:{}", job.job_id());

    let job_json = serde_json::to_string(&job)
        .map_err(|e| AppError::Internal(format!("serialize job: {e}")))?;

    // Init execution state
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis pool: {e}")))?;

    let exec_key = format!("aw:exec:{execution_id}");
    conn.hset_multiple::<_, _, _, ()>(
        &exec_key,
        &[
            ("status", "queued"),
            ("agent_id", &agent_id.to_string()),
            ("workspace_id", &workspace_id.to_string()),
            ("message_id", &message_id.to_string()),
            ("created_at", &chrono::Utc::now().to_rfc3339()),
        ],
    )
    .await
    .map_err(|e| AppError::Internal(format!("redis hset: {e}")))?;

    // Enqueue job
    conn.rpush::<_, _, ()>("aw:queue:jobs", &job_json)
        .await
        .map_err(|e| AppError::Internal(format!("redis rpush: {e}")))?;

    drop(conn);

    // Wait for worker acknowledgment (2s timeout)
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis pool: {e}")))?;

    let result: Option<(String, String)> = deadpool_redis::redis::cmd("BLPOP")
        .arg(&reply_key)
        .arg(2.0_f64)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::Internal(format!("redis blpop: {e}")))?;

    match result {
        Some((_key, payload)) => {
            // Worker acknowledged — check if accepted or rejected
            if let Ok(reply) = serde_json::from_str::<RpcReply>(&payload) {
                if reply.status == RpcStatus::Rejected {
                    let reason = reply.message.as_deref().unwrap_or("rejected by worker");
                    return Err(AppError::Conflict(reason.to_owned()));
                }
            }
            Ok(execution_id)
        }
        None => {
            // No worker responded — job is queued but unconfirmed
            tracing::warn!(execution_id = %execution_id, "no worker ack within timeout");
            Ok(execution_id)
        }
    }
}

// ── Validation helpers ────────────────────────────────────────────────

/// Validate that a parent agent exists and that setting it won't create
/// a cycle or exceed the maximum chain depth.
fn validate_parent(
    ws_store: &WorkspaceStore,
    parent_id: AgentId,
    self_id: Option<AgentId>,
) -> Result<(), AppError> {
    // Parent must exist
    let parent_path = format!("agents/{}.json", parent_id);
    let _parent: Agent = ws_store
        .read_json(&parent_path)
        .map_err(|_| AppError::BadRequest(format!("parent agent {} not found", parent_id)))?;

    // Check that setting this parent won't create a cycle.
    // Walk the parent's chain; if we encounter self_id, it's a cycle.
    if let Some(self_id) = self_id {
        if parent_id == self_id {
            return Err(AppError::BadRequest(
                "agent cannot be its own parent".to_owned(),
            ));
        }
        // Walk the parent chain to detect cycles and check depth
        let chain = resolve::walk_parent_chain(ws_store, parent_id)?;
        for ancestor in &chain {
            if ancestor.agent_id() == self_id {
                return Err(AppError::BadRequest(
                    "setting this parent would create a cycle".to_owned(),
                ));
            }
        }
        // chain length + 1 (for the child itself) must not exceed max depth
        if chain.len() >= 5 {
            return Err(AppError::BadRequest(
                "parent chain would exceed maximum depth of 5".to_owned(),
            ));
        }
    }

    Ok(())
}

// ── Resolved agent endpoint ──────────────────────────────────────────

async fn get_resolved_agent(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
) -> Result<Json<AgentResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let agent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            resolve::resolve_agent(&ws_store, agent_id)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(agent_to_response(&agent)))
}

async fn get_resolved_agent_internal(
    _worker: RequireInternalWorker,
    State(state): State<AppState>,
    Path(agent_id): Path<AgentId>,
    Query(query): Query<WorkerWorkspaceQuery>,
) -> Result<Json<AgentDefinition>, AppError> {
    let workspace_id = query.workspace_id;

    let agent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            resolve::resolve_agent(&ws_store, agent_id)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(agent_to_definition(&agent)?))
}

async fn list_active_agents_internal(
    _worker: RequireInternalWorker,
    State(state): State<AppState>,
) -> Result<Json<Vec<InternalCronAgentResponse>>, AppError> {
    let results = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let mut out = Vec::new();

            for workspace_id in layout.list_workspace_ids() {
                let ws_store = match WorkspaceStore::open(&layout, workspace_id) {
                    Ok(store) => store,
                    Err(_) => continue,
                };

                let ids: Vec<AgentId> = match ws_store.list_ids_in_dir_pub("agents") {
                    Ok(ids) => ids,
                    Err(_) => continue,
                };

                for id in ids {
                    let path = format!("agents/{}.json", id);
                    let Ok(agent) = ws_store.read_json::<Agent>(&path) else {
                        continue;
                    };
                    if agent.status() != AgentStatus::Active {
                        continue;
                    }
                    let channels = agent
                        .channels()
                        .iter()
                        .map(|ch| InternalChannelResponse {
                            channel_type: ch.channel_type_str().to_owned(),
                            schedule: ch.schedule().map(|s| s.as_str().to_owned()),
                        })
                        .collect::<Vec<_>>();
                    out.push(InternalCronAgentResponse {
                        agent_id: agent.agent_id().to_string(),
                        workspace_id: workspace_id.to_string(),
                        status: agent.status().as_str().to_owned(),
                        channels,
                    });
                }
            }

            Ok::<_, AppError>(out)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(results))
}

// ── Agent token minting ──────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MintAgentTokenRequest {
    pub workspace_id: WorkspaceId,
    pub agent_id: AgentId,
    #[serde(default)]
    pub ttl_seconds: Option<i64>,
}

#[derive(Serialize)]
pub struct MintAgentTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scopes: Vec<Scope>,
}

async fn mint_agent_token(
    _worker: RequireInternalWorker,
    State(state): State<AppState>,
    Json(req): Json<MintAgentTokenRequest>,
) -> Result<Json<MintAgentTokenResponse>, AppError> {
    let ttl = req.ttl_seconds.unwrap_or(3600).clamp(1, 86400);

    let agent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let workspace_id = req.workspace_id;
        let agent_id = req.agent_id;
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            resolve::resolve_agent(&ws_store, agent_id)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    if agent.scopes().is_empty() {
        return Err(AppError::BadRequest(
            "agent has no API scopes configured".to_owned(),
        ));
    }

    let now = chrono::Utc::now().timestamp();
    let claims = Claims::new(
        req.workspace_id,
        agent.entity_id(),
        None,
        None,
        PrincipalType::Agent,
        agent.scopes().to_vec(),
        now,
        now + ttl,
    );

    let token = encode_token(&claims, &state.jwt_secret)?;
    let scopes = agent.scopes().to_vec();

    Ok(Json(MintAgentTokenResponse {
        access_token: token,
        token_type: "Bearer".to_owned(),
        expires_in: ttl,
        scopes,
    }))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn agent_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/agents", post(create_agent).get(list_agents))
        .route("/v1/agents/{agent_id}", patch(update_agent))
        .route("/v1/agents/{agent_id}/resolved", get(get_resolved_agent))
        .route("/v1/agents/{agent_id}/skills", post(add_agent_skill))
        .route("/v1/agents/{agent_id}/messages", post(send_agent_message))
        .route(
            "/v1/agents/{agent_id}/messages/{message_id}",
            get(get_agent_message_internal),
        )
        .route(
            "/v1/internal/agents/{agent_id}/resolved",
            get(get_resolved_agent_internal),
        )
        .route(
            "/v1/internal/agents/active",
            get(list_active_agents_internal),
        )
        .route("/v1/internal/agent-token", post(mint_agent_token))
}
