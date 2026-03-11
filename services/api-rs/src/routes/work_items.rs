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
use crate::auth::{RequireExecutionRead, RequireExecutionWrite};
use crate::domain::ids::{EntityId, WorkItemId, WorkspaceId};
use crate::domain::work_items::types::WorkItemStatus;
use crate::domain::work_items::work_item::WorkItem;
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Helpers ─────────────────────────────────────────────────────────

fn open_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<EntityStore<'a>, AppError> {
    EntityStore::open(layout, workspace_id, entity_id).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {} not found", entity_id))
        }
        other => AppError::Internal(other.to_string()),
    })
}

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
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimWorkItemRequest {
    pub claimed_by: String,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CompleteWorkItemRequest {
    pub completed_by: String,
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
    pub claimed_at: Option<String>,
    pub claim_ttl_seconds: Option<u64>,
    pub status: WorkItemStatus,
    pub effective_status: WorkItemStatus,
    pub completed_at: Option<String>,
    pub completed_by: Option<String>,
    pub result: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub created_by: Option<String>,
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
        claimed_at: w.claimed_at().map(|dt| dt.to_rfc3339()),
        claim_ttl_seconds: w.claim_ttl_seconds(),
        status: w.status(),
        effective_status: w.effective_status(now),
        completed_at: w.completed_at().map(|dt| dt.to_rfc3339()),
        completed_by: w.completed_by().map(|s| s.to_owned()),
        result: w.result().map(|s| s.to_owned()),
        metadata: w.metadata().clone(),
        created_at: w.created_at().to_rfc3339(),
        created_by: w.created_by().map(|s| s.to_owned()),
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
    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title cannot be empty".to_owned()));
    }
    if req
        .deadline
        .is_some_and(|deadline| deadline < Utc::now().date_naive() - chrono::Duration::days(365))
    {
        return Err(AppError::BadRequest(
            "deadline cannot be more than one year in the past".to_owned(),
        ));
    }
    let category = validate_category(&req.category)?;

    let work_item = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let category = category.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let work_item_id = WorkItemId::new();
            let metadata = req
                .metadata
                .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
            let work_item = WorkItem::new(
                work_item_id,
                entity_id,
                req.title,
                req.description.unwrap_or_default(),
                category,
                req.deadline,
                req.asap,
                metadata,
                req.created_by,
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
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

    let items = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = match EntityStore::open(&layout, workspace_id, entity_id) {
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
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

    let work_item = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            store
                .read::<WorkItem>("main", work_item_id)
                .map_err(|_| AppError::NotFound(format!("work item {} not found", work_item_id)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

    let work_item = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut w = store
                .read::<WorkItem>("main", work_item_id)
                .map_err(|_| AppError::NotFound(format!("work item {} not found", work_item_id)))?;

            // Auto-release expired claims before attempting to claim
            w.auto_release_expired_claim(Utc::now());

            w.claim(req.claimed_by, req.ttl_seconds)?;

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
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

    let work_item = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut w = store
                .read::<WorkItem>("main", work_item_id)
                .map_err(|_| AppError::NotFound(format!("work item {} not found", work_item_id)))?;
            w.auto_release_expired_claim(Utc::now());
            if let Some(claimed_by) = w.claimed_by()
                && claimed_by != req.completed_by
            {
                return Err(AppError::Conflict(format!(
                    "work item is claimed by {} and cannot be completed by {}",
                    claimed_by, req.completed_by
                )));
            }

            w.complete(req.completed_by, req.result)?;

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
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

    let work_item = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
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
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

    let work_item = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
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
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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
