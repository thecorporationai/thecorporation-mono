//! Work Items HTTP routes.
//!
//! Long-term coordination items stored in entity repos. Agents can claim,
//! complete, and release work items with optional TTL-based auto-expiry.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::AppState;
use super::validation::{require_non_empty_trimmed, validate_max_len, validate_not_too_far_past};
use crate::auth::{RequireExecutionRead, RequireExecutionWrite};
use crate::domain::agents::{agent::Agent, types::AgentStatus};
use crate::domain::contacts::contact::Contact;
use crate::domain::ids::{AgentId, ContactId, EntityId, WorkItemId, WorkspaceId};
use crate::domain::work_items::types::WorkItemStatus;
use crate::domain::work_items::work_item::{WorkItem, WorkItemActor, WorkItemActorType};
use crate::error::AppError;
use crate::store::entity_store::EntityStore;
use crate::store::workspace_store::WorkspaceStore;

// ── Helpers ─────────────────────────────────────────────────────────

fn validate_category(category: &str) -> Result<String, AppError> {
    let trimmed = category.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest("category cannot be empty".to_owned()));
    }
    if trimmed.len() > 64 {
        return Err(AppError::BadRequest(
            "category must be at most 64 characters".to_owned(),
        ));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err(AppError::BadRequest(
            "category must use only letters, numbers, '_' or '-'".to_owned(),
        ));
    }
    Ok(trimmed.to_owned())
}

fn validate_title(title: &str) -> Result<String, AppError> {
    let trimmed = require_non_empty_trimmed(title, "title")?;
    validate_max_len(&trimmed, "title", 1000)?;
    if trimmed.contains('<')
        || trimmed.contains('>')
        || trimmed.contains("{{")
        || trimmed.contains("}}")
        || trimmed.chars().any(|ch| ch == '\n' || ch == '\r')
    {
        return Err(AppError::BadRequest(
            "title cannot contain markup, template syntax, or newlines".to_owned(),
        ));
    }
    Ok(trimmed)
}

fn resolve_contact_actor(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    raw: &str,
    field: &str,
) -> Result<WorkItemActor, AppError> {
    let trimmed = require_non_empty_trimmed(raw, field)?;
    validate_max_len(&trimmed, field, 256)?;

    if let Ok(contact_id) = trimmed.parse::<ContactId>() {
        let contact = store.read::<Contact>("main", contact_id).map_err(|_| {
            AppError::BadRequest(format!("{field} must reference an existing entity contact"))
        })?;
        if contact.entity_id() != entity_id {
            return Err(AppError::BadRequest(format!(
                "{field} must reference a contact on the same entity"
            )));
        }
        return Ok(WorkItemActor::new(
            WorkItemActorType::Contact,
            contact.contact_id().to_string(),
            contact.name().to_owned(),
        ));
    }

    let mut matches = Vec::new();
    for contact_id in store.list_ids::<Contact>("main").unwrap_or_default() {
        let contact = match store.read::<Contact>("main", contact_id) {
            Ok(contact) => contact,
            Err(_) => continue,
        };
        if contact.entity_id() != entity_id {
            continue;
        }
        let matches_name = contact.name().eq_ignore_ascii_case(trimmed.as_str());
        let matches_email = contact
            .email()
            .is_some_and(|email| email.eq_ignore_ascii_case(trimmed.as_str()));
        if matches_name || matches_email {
            matches.push(WorkItemActor::new(
                WorkItemActorType::Contact,
                contact.contact_id().to_string(),
                contact.name().to_owned(),
            ));
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(AppError::BadRequest(format!(
            "{field} must reference an existing entity contact"
        ))),
        _ => Err(AppError::Conflict(format!(
            "{field} matches multiple entity contacts"
        ))),
    }
}

fn validate_agent_actor(
    agent: &Agent,
    entity_id: EntityId,
    field: &str,
) -> Result<(), AppError> {
    if agent.status() == AgentStatus::Disabled {
        return Err(AppError::BadRequest(format!(
            "{field} must reference an active or paused workspace agent"
        )));
    }
    if let Some(agent_entity_id) = agent.entity_id()
        && agent_entity_id != entity_id
    {
        return Err(AppError::BadRequest(format!(
            "{field} must reference an agent bound to the same entity or the workspace"
        )));
    }
    Ok(())
}

fn load_agent_actor_by_id(
    workspace_store: &WorkspaceStore<'_>,
    entity_id: EntityId,
    agent_id: AgentId,
    field: &str,
) -> Result<WorkItemActor, AppError> {
    let path = format!("agents/{}.json", agent_id);
    let agent = workspace_store.read_json::<Agent>(&path).map_err(|_| {
        AppError::BadRequest(format!("{field} must reference an existing workspace agent"))
    })?;
    validate_agent_actor(&agent, entity_id, field)?;
    Ok(WorkItemActor::new(
        WorkItemActorType::Agent,
        agent.agent_id().to_string(),
        agent.name().to_owned(),
    ))
}

fn lookup_agent_actor(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    raw: &str,
    field: &str,
    valkey_client: Option<&redis::Client>,
) -> Result<Option<WorkItemActor>, AppError> {
    let trimmed = require_non_empty_trimmed(raw, field)?;
    validate_max_len(&trimmed, field, 256)?;

    let workspace_store = WorkspaceStore::open(layout, workspace_id, valkey_client).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("workspace {} not found", workspace_id))
        }
        other => AppError::Internal(other.to_string()),
    })?;

    if let Ok(agent_id) = trimmed.parse::<AgentId>() {
        return load_agent_actor_by_id(&workspace_store, entity_id, agent_id, field).map(Some);
    }

    let mut matches = Vec::new();
    for agent_id in workspace_store
        .list_ids_in_dir_pub::<AgentId>("agents")
        .unwrap_or_default()
    {
        let path = format!("agents/{}.json", agent_id);
        let agent = match workspace_store.read_json::<Agent>(&path) {
            Ok(agent) => agent,
            Err(_) => continue,
        };
        if !agent.name().eq_ignore_ascii_case(trimmed.as_str()) {
            continue;
        }
        validate_agent_actor(&agent, entity_id, field)?;
        matches.push(WorkItemActor::new(
            WorkItemActorType::Agent,
            agent.agent_id().to_string(),
            agent.name().to_owned(),
        ));
    }

    match matches.len() {
        0 => Ok(None),
        1 => Ok(Some(matches.remove(0))),
        _ => Err(AppError::Conflict(format!(
            "{field} matches multiple workspace agents"
        ))),
    }
}

fn resolve_legacy_actor(
    layout: &crate::store::RepoLayout,
    store: &EntityStore<'_>,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    raw: &str,
    field: &str,
    valkey_client: Option<&redis::Client>,
) -> Result<WorkItemActor, AppError> {
    let contact_match = match resolve_contact_actor(store, entity_id, raw, field) {
        Ok(actor) => Some(actor),
        Err(AppError::BadRequest(_)) => None,
        Err(err) => return Err(err),
    };
    let agent_match = lookup_agent_actor(layout, workspace_id, entity_id, raw, field, valkey_client)?;

    match (contact_match, agent_match) {
        (Some(contact), None) => Ok(contact),
        (None, Some(agent)) => Ok(agent),
        (Some(_), Some(_)) => Err(AppError::Conflict(format!(
            "{field} is ambiguous between a contact and an agent; pass an explicit typed actor"
        ))),
        (None, None) => Err(AppError::BadRequest(format!(
            "{field} must reference an existing entity contact or workspace agent"
        ))),
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemActorTypeValue {
    Contact,
    Agent,
}

impl From<WorkItemActorTypeValue> for WorkItemActorType {
    fn from(value: WorkItemActorTypeValue) -> Self {
        match value {
            WorkItemActorTypeValue::Contact => WorkItemActorType::Contact,
            WorkItemActorTypeValue::Agent => WorkItemActorType::Agent,
        }
    }
}

impl From<WorkItemActorType> for WorkItemActorTypeValue {
    fn from(value: WorkItemActorType) -> Self {
        match value {
            WorkItemActorType::Contact => WorkItemActorTypeValue::Contact,
            WorkItemActorType::Agent => WorkItemActorTypeValue::Agent,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkItemActorRefRequest {
    pub actor_type: WorkItemActorTypeValue,
    pub actor_id: String,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct WorkItemActorResponse {
    pub actor_type: WorkItemActorTypeValue,
    pub actor_id: String,
    pub label: String,
}

fn resolve_explicit_actor(
    layout: &crate::store::RepoLayout,
    store: &EntityStore<'_>,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    actor: &WorkItemActorRefRequest,
    field: &str,
    valkey_client: Option<&redis::Client>,
) -> Result<WorkItemActor, AppError> {
    let actor_id = require_non_empty_trimmed(&actor.actor_id, &format!("{field}.actor_id"))?;
    validate_max_len(&actor_id, &format!("{field}.actor_id"), 256)?;

    match actor.actor_type {
        WorkItemActorTypeValue::Contact => {
            let contact_id = actor_id.parse::<ContactId>().map_err(|_| {
                AppError::BadRequest(format!("{field}.actor_id must be a contact UUID"))
            })?;
            let contact = store.read::<Contact>("main", contact_id).map_err(|_| {
                AppError::BadRequest(format!("{field} must reference an existing entity contact"))
            })?;
            if contact.entity_id() != entity_id {
                return Err(AppError::BadRequest(format!(
                    "{field} must reference a contact on the same entity"
                )));
            }
            Ok(WorkItemActor::new(
                WorkItemActorType::Contact,
                contact.contact_id().to_string(),
                contact.name().to_owned(),
            ))
        }
        WorkItemActorTypeValue::Agent => {
            let agent_id = actor_id.parse::<AgentId>().map_err(|_| {
                AppError::BadRequest(format!("{field}.actor_id must be an agent UUID"))
            })?;
            let workspace_store = WorkspaceStore::open(layout, workspace_id, valkey_client).map_err(|e| match e {
                crate::git::error::GitStorageError::RepoNotFound(_) => {
                    AppError::NotFound(format!("workspace {} not found", workspace_id))
                }
                other => AppError::Internal(other.to_string()),
            })?;
            load_agent_actor_by_id(&workspace_store, entity_id, agent_id, field)
        }
    }
}

fn resolve_actor_input(
    layout: &crate::store::RepoLayout,
    store: &EntityStore<'_>,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    raw: Option<&str>,
    actor: Option<&WorkItemActorRefRequest>,
    field: &str,
    required: bool,
    valkey_client: Option<&redis::Client>,
) -> Result<Option<WorkItemActor>, AppError> {
    if raw.is_some() && actor.is_some() {
        return Err(AppError::BadRequest(format!(
            "{field} cannot include both a legacy string reference and a typed actor"
        )));
    }
    if let Some(actor) = actor {
        return resolve_explicit_actor(layout, store, workspace_id, entity_id, actor, field, valkey_client)
            .map(Some);
    }
    if let Some(raw) = raw {
        return resolve_legacy_actor(layout, store, workspace_id, entity_id, raw, field, valkey_client).map(Some);
    }
    if required {
        return Err(AppError::BadRequest(format!("{field} is required")));
    }
    Ok(None)
}

fn actor_to_response(actor: &WorkItemActor) -> WorkItemActorResponse {
    WorkItemActorResponse {
        actor_type: actor.actor_type().into(),
        actor_id: actor.actor_id().to_owned(),
        label: actor.label().to_owned(),
    }
}

// ── Request types ───────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateWorkItemRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub category: String,
    #[serde(default)]
    pub deadline: Option<NaiveDate>,
    #[serde(default)]
    pub asap: bool,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub created_by_actor: Option<WorkItemActorRefRequest>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimWorkItemRequest {
    #[serde(default)]
    pub claimed_by: Option<String>,
    #[serde(default)]
    pub claimed_by_actor: Option<WorkItemActorRefRequest>,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CompleteWorkItemRequest {
    #[serde(default)]
    pub completed_by: Option<String>,
    #[serde(default)]
    pub completed_by_actor: Option<WorkItemActorRefRequest>,
    #[serde(default)]
    pub result: Option<String>,
}

// ── Response type ───────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkItemResponse {
    pub work_item_id: WorkItemId,
    pub entity_id: EntityId,
    pub title: String,
    pub description: String,
    pub category: String,
    pub deadline: Option<NaiveDate>,
    pub asap: bool,
    pub claimed_by: Option<String>,
    pub claimed_by_actor: Option<WorkItemActorResponse>,
    pub claimed_at: Option<String>,
    pub claim_ttl_seconds: Option<u64>,
    pub status: WorkItemStatus,
    pub effective_status: WorkItemStatus,
    pub completed_at: Option<String>,
    pub completed_by: Option<String>,
    pub completed_by_actor: Option<WorkItemActorResponse>,
    pub result: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub created_by: Option<String>,
    pub created_by_actor: Option<WorkItemActorResponse>,
}

fn work_item_to_response(w: &WorkItem) -> WorkItemResponse {
    let now = Utc::now();
    WorkItemResponse {
        work_item_id: w.work_item_id(),
        entity_id: w.entity_id(),
        title: w.title().to_owned(),
        description: w.description().to_owned(),
        category: w.category().to_owned(),
        deadline: w.deadline(),
        asap: w.asap(),
        claimed_by: w.claimed_by().map(|s| s.to_owned()),
        claimed_by_actor: w.claimed_by_actor().map(actor_to_response),
        claimed_at: w.claimed_at().map(|dt| dt.to_rfc3339()),
        claim_ttl_seconds: w.claim_ttl_seconds(),
        status: w.status(),
        effective_status: w.effective_status(now),
        completed_at: w.completed_at().map(|dt| dt.to_rfc3339()),
        completed_by: w.completed_by().map(|s| s.to_owned()),
        completed_by_actor: w.completed_by_actor().map(actor_to_response),
        result: w.result().map(|s| s.to_owned()),
        metadata: w.metadata().clone(),
        created_at: w.created_at().to_rfc3339(),
        created_by: w.created_by().map(|s| s.to_owned()),
        created_by_actor: w.created_by_actor().map(actor_to_response),
    }
}

// ── Query params ────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListWorkItemsQuery {
    #[serde(default)]
    pub status: Option<WorkItemStatus>,
    #[serde(default)]
    pub category: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/entities/{entity_id}/work-items",
    tag = "work_items",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = CreateWorkItemRequest,
    responses(
        (status = 200, description = "Work item created", body = WorkItemResponse),
    ),
)]
async fn create_work_item(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<CreateWorkItemRequest>,
) -> Result<Json<WorkItemResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    state.enforce_creation_rate_limit("work_items.create", workspace_id, 60, 60)?;
    let title = validate_title(&req.title)?;
    if let Some(deadline) = req.deadline {
        validate_not_too_far_past("deadline", deadline, 365)?;
    }
    let category = validate_category(&req.category)?;

    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let category = category.clone();
    let title = title.clone();
    let work_item = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = super::shared::open_entity_store(layout, workspace_id, entity_id, entity_scope.as_deref(), valkey, s3)?;
        let created_by = resolve_actor_input(
            layout,
            &store,
            workspace_id,
            entity_id,
            req.created_by.as_deref(),
            req.created_by_actor.as_ref(),
            "created_by",
            false,
            valkey,
        )?;
        let work_item_id = WorkItemId::new();
        let metadata = req
            .metadata
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        let work_item = WorkItem::new(
            work_item_id,
            entity_id,
            title,
            req.description.unwrap_or_default(),
            category,
            req.deadline,
            req.asap,
            metadata,
            created_by,
        );

        let path = format!("workitems/{}.json", work_item_id);
        store
            .write_json(
                "main",
                &path,
                &work_item,
                &format!("Create work item {work_item_id}"),
            )
            .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

        Ok::<_, AppError>(work_item)
    })
    .await?;

    Ok(Json(work_item_to_response(&work_item)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/work-items",
    tag = "work_items",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        ListWorkItemsQuery,
    ),
    responses(
        (status = 200, description = "List of work items", body = Vec<WorkItemResponse>),
    ),
)]
async fn list_work_items(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    axum::extract::Query(query): axum::extract::Query<ListWorkItemsQuery>,
) -> Result<Json<Vec<WorkItemResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let items = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = match EntityStore::open(layout, workspace_id, entity_id, valkey, s3) {
            Ok(s) => s,
            Err(crate::git::error::GitStorageError::RepoNotFound(_)) => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(AppError::Internal(e.to_string())),
        };
        let ids = store
            .list_ids::<WorkItem>("main")
            .map_err(|e| AppError::Internal(format!("list work items: {e}")))?;

        let now = Utc::now();
        let mut results = Vec::new();
        for id in ids {
            let w = store
                .read::<WorkItem>("main", id)
                .map_err(|e| AppError::Internal(format!("read work item {id}: {e}")))?;

            // Filter by effective status if requested
            if let Some(ref status_filter) = query.status {
                if w.effective_status(now) != *status_filter {
                    continue;
                }
            }
            if let Some(ref cat_filter) = query.category {
                if w.category() != cat_filter.as_str() {
                    continue;
                }
            }

            results.push(work_item_to_response(&w));
        }
        Ok::<_, AppError>(results)
    })
    .await?;

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/work-items/{work_item_id}",
    tag = "work_items",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        ("work_item_id" = WorkItemId, Path, description = "Work Item ID"),
    ),
    responses(
        (status = 200, description = "Work item details", body = WorkItemResponse),
        (status = 404, description = "Work item not found"),
    ),
)]
async fn get_work_item(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path((entity_id, work_item_id)): Path<(EntityId, WorkItemId)>,
) -> Result<Json<WorkItemResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let work_item = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = super::shared::open_entity_store(layout, workspace_id, entity_id, entity_scope.as_deref(), valkey, s3)?;
        store
            .read::<WorkItem>("main", work_item_id)
            .map_err(|_| AppError::NotFound(format!("work item {} not found", work_item_id)))
    })
    .await?;

    Ok(Json(work_item_to_response(&work_item)))
}

#[utoipa::path(
    post,
    path = "/v1/entities/{entity_id}/work-items/{work_item_id}/claim",
    tag = "work_items",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        ("work_item_id" = WorkItemId, Path, description = "Work Item ID"),
    ),
    request_body = ClaimWorkItemRequest,
    responses(
        (status = 200, description = "Work item claimed", body = WorkItemResponse),
        (status = 404, description = "Work item not found"),
        (status = 422, description = "Invalid state transition"),
    ),
)]
async fn claim_work_item(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, work_item_id)): Path<(EntityId, WorkItemId)>,
    Json(req): Json<ClaimWorkItemRequest>,
) -> Result<Json<WorkItemResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let work_item = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = super::shared::open_entity_store(layout, workspace_id, entity_id, entity_scope.as_deref(), valkey, s3)?;
        let claimed_by = resolve_actor_input(
            layout,
            &store,
            workspace_id,
            entity_id,
            req.claimed_by.as_deref(),
            req.claimed_by_actor.as_ref(),
            "claimed_by",
            true,
            valkey,
        )?
        .expect("required actor should exist");
        let mut w = store
            .read::<WorkItem>("main", work_item_id)
            .map_err(|_| AppError::NotFound(format!("work item {} not found", work_item_id)))?;

        // Auto-release expired claims before attempting to claim
        w.auto_release_expired_claim(Utc::now());

        w.claim(claimed_by, req.ttl_seconds)?;

        let path = format!("workitems/{}.json", work_item_id);
        store
            .write_json(
                "main",
                &path,
                &w,
                &format!("Claim work item {work_item_id}"),
            )
            .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

        Ok::<_, AppError>(w)
    })
    .await?;

    Ok(Json(work_item_to_response(&work_item)))
}

#[utoipa::path(
    post,
    path = "/v1/entities/{entity_id}/work-items/{work_item_id}/complete",
    tag = "work_items",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        ("work_item_id" = WorkItemId, Path, description = "Work Item ID"),
    ),
    request_body = CompleteWorkItemRequest,
    responses(
        (status = 200, description = "Work item completed", body = WorkItemResponse),
        (status = 404, description = "Work item not found"),
        (status = 422, description = "Invalid state transition"),
    ),
)]
async fn complete_work_item(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, work_item_id)): Path<(EntityId, WorkItemId)>,
    Json(req): Json<CompleteWorkItemRequest>,
) -> Result<Json<WorkItemResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let work_item = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = super::shared::open_entity_store(layout, workspace_id, entity_id, entity_scope.as_deref(), valkey, s3)?;
        let completed_by = resolve_actor_input(
            layout,
            &store,
            workspace_id,
            entity_id,
            req.completed_by.as_deref(),
            req.completed_by_actor.as_ref(),
            "completed_by",
            true,
            valkey,
        )?
        .expect("required actor should exist");
        let mut w = store
            .read::<WorkItem>("main", work_item_id)
            .map_err(|_| AppError::NotFound(format!("work item {} not found", work_item_id)))?;
        w.auto_release_expired_claim(Utc::now());
        if let Some(claimed_by) = w.claimed_by()
            && !w.is_claimed_by_actor(&completed_by)
        {
            return Err(AppError::Conflict(format!(
                "work item is claimed by {} and cannot be completed by {}",
                claimed_by,
                completed_by.label()
            )));
        }

        w.complete(completed_by, req.result)?;

        let path = format!("workitems/{}.json", work_item_id);
        store
            .write_json(
                "main",
                &path,
                &w,
                &format!("Complete work item {work_item_id}"),
            )
            .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

        Ok::<_, AppError>(w)
    })
    .await?;

    Ok(Json(work_item_to_response(&work_item)))
}

#[utoipa::path(
    post,
    path = "/v1/entities/{entity_id}/work-items/{work_item_id}/release",
    tag = "work_items",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        ("work_item_id" = WorkItemId, Path, description = "Work Item ID"),
    ),
    responses(
        (status = 200, description = "Claim released", body = WorkItemResponse),
        (status = 404, description = "Work item not found"),
        (status = 422, description = "Work item is not claimed"),
    ),
)]
async fn release_work_item(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, work_item_id)): Path<(EntityId, WorkItemId)>,
) -> Result<Json<WorkItemResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let work_item = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = super::shared::open_entity_store(layout, workspace_id, entity_id, entity_scope.as_deref(), valkey, s3)?;
        let mut w = store
            .read::<WorkItem>("main", work_item_id)
            .map_err(|_| AppError::NotFound(format!("work item {} not found", work_item_id)))?;

        w.release_claim()?;

        let path = format!("workitems/{}.json", work_item_id);
        store
            .write_json(
                "main",
                &path,
                &w,
                &format!("Release claim on work item {work_item_id}"),
            )
            .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

        Ok::<_, AppError>(w)
    })
    .await?;

    Ok(Json(work_item_to_response(&work_item)))
}

#[utoipa::path(
    post,
    path = "/v1/entities/{entity_id}/work-items/{work_item_id}/cancel",
    tag = "work_items",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        ("work_item_id" = WorkItemId, Path, description = "Work Item ID"),
    ),
    responses(
        (status = 200, description = "Work item cancelled", body = WorkItemResponse),
        (status = 404, description = "Work item not found"),
        (status = 422, description = "Invalid state transition"),
    ),
)]
async fn cancel_work_item(
    RequireExecutionWrite(auth): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, work_item_id)): Path<(EntityId, WorkItemId)>,
) -> Result<Json<WorkItemResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let work_item = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = super::shared::open_entity_store(layout, workspace_id, entity_id, entity_scope.as_deref(), valkey, s3)?;
        let mut w = store
            .read::<WorkItem>("main", work_item_id)
            .map_err(|_| AppError::NotFound(format!("work item {} not found", work_item_id)))?;

        w.cancel()?;

        let path = format!("workitems/{}.json", work_item_id);
        store
            .write_json(
                "main",
                &path,
                &w,
                &format!("Cancel work item {work_item_id}"),
            )
            .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

        Ok::<_, AppError>(w)
    })
    .await?;

    Ok(Json(work_item_to_response(&work_item)))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn work_items_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/entities/{entity_id}/work-items",
            post(create_work_item).get(list_work_items),
        )
        .route(
            "/v1/entities/{entity_id}/work-items/{work_item_id}",
            get(get_work_item),
        )
        .route(
            "/v1/entities/{entity_id}/work-items/{work_item_id}/claim",
            post(claim_work_item),
        )
        .route(
            "/v1/entities/{entity_id}/work-items/{work_item_id}/complete",
            post(complete_work_item),
        )
        .route(
            "/v1/entities/{entity_id}/work-items/{work_item_id}/release",
            post(release_work_item),
        )
        .route(
            "/v1/entities/{entity_id}/work-items/{work_item_id}/cancel",
            post(cancel_work_item),
        )
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create_work_item,
        list_work_items,
        get_work_item,
        claim_work_item,
        complete_work_item,
        release_work_item,
        cancel_work_item,
    ),
    components(schemas(
        CreateWorkItemRequest,
        ClaimWorkItemRequest,
        CompleteWorkItemRequest,
        WorkItemResponse,
        WorkItemStatus,
    ))
)]
pub struct WorkItemsApi;
