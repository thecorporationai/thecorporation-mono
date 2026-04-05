//! Work items route handlers — open tasks claimed and completed by agents or humans.
//!
//! ## Route map
//!
//! | Method | Path | Scope |
//! |--------|------|-------|
//! | GET    | `/entities/{entity_id}/work-items` | `WorkItemsRead` |
//! | POST   | `/entities/{entity_id}/work-items` | `WorkItemsWrite` |
//! | GET    | `/entities/{entity_id}/work-items/{item_id}` | `WorkItemsRead` |
//! | POST   | `/entities/{entity_id}/work-items/{item_id}/claim` | `WorkItemsWrite` |
//! | POST   | `/entities/{entity_id}/work-items/{item_id}/release` | `WorkItemsWrite` |
//! | POST   | `/entities/{entity_id}/work-items/{item_id}/complete` | `WorkItemsWrite` |
//! | POST   | `/entities/{entity_id}/work-items/{item_id}/cancel` | `WorkItemsWrite` |

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::Deserialize;

use corp_auth::{RequireWorkItemsRead, RequireWorkItemsWrite};
use corp_core::ids::{EntityId, WorkItemId};
use corp_core::work_items::WorkItem;

use crate::error::AppError;
use crate::state::AppState;

// ── StoredEntity impl ─────────────────────────────────────────────────────────

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/entities/{entity_id}/work-items",
            get(list_work_items).post(create_work_item),
        )
        .route(
            "/entities/{entity_id}/work-items/{item_id}",
            get(get_work_item),
        )
        .route(
            "/entities/{entity_id}/work-items/{item_id}/claim",
            post(claim_work_item),
        )
        .route(
            "/entities/{entity_id}/work-items/{item_id}/release",
            post(release_work_item),
        )
        .route(
            "/entities/{entity_id}/work-items/{item_id}/complete",
            post(complete_work_item),
        )
        .route(
            "/entities/{entity_id}/work-items/{item_id}/cancel",
            post(cancel_work_item),
        )
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateWorkItemRequest {
    pub title: String,
    pub description: String,
    pub category: String,
    pub deadline: Option<NaiveDate>,
    #[serde(default)]
    pub asap: bool,
}

#[derive(Debug, Deserialize)]
pub struct ClaimWorkItemRequest {
    /// Identifier of the claimant (agent ID, user ID, or display name).
    pub claimed_by: String,
    /// Optional TTL in seconds before the claim automatically expires.
    pub claim_ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteWorkItemRequest {
    pub completed_by: String,
    pub result: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_work_items(
    RequireWorkItemsRead(principal): RequireWorkItemsRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<WorkItem>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let items = store
        .read_all::<WorkItem>("main")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(items))
}

async fn create_work_item(
    RequireWorkItemsWrite(principal): RequireWorkItemsWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateWorkItemRequest>,
) -> Result<Json<WorkItem>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let item = WorkItem::new(
        entity_id,
        body.title,
        body.description,
        body.category,
        body.deadline,
        body.asap,
    );
    store
        .write::<WorkItem>(&item, item.work_item_id, "main", "create work item")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(item))
}

async fn get_work_item(
    RequireWorkItemsRead(principal): RequireWorkItemsRead,
    State(state): State<AppState>,
    Path((entity_id, item_id)): Path<(EntityId, WorkItemId)>,
) -> Result<Json<WorkItem>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let item = store.read::<WorkItem>(item_id, "main").await.map_err(|e| {
        use corp_storage::error::StorageError;
        match e {
            StorageError::NotFound(_) => {
                AppError::NotFound(format!("work item {} not found", item_id))
            }
            other => AppError::Storage(other),
        }
    })?;
    Ok(Json(item))
}

async fn claim_work_item(
    RequireWorkItemsWrite(principal): RequireWorkItemsWrite,
    State(state): State<AppState>,
    Path((entity_id, item_id)): Path<(EntityId, WorkItemId)>,
    Json(body): Json<ClaimWorkItemRequest>,
) -> Result<Json<WorkItem>, AppError> {
    if body.claimed_by.trim().is_empty() {
        return Err(AppError::BadRequest("claimed_by must not be empty".into()));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut item = store
        .read::<WorkItem>(item_id, "main")
        .await
        .map_err(AppError::Storage)?;
    item.claim(&body.claimed_by)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    item.claim_ttl_seconds = body.claim_ttl_seconds;
    store
        .write::<WorkItem>(&item, item_id, "main", "claim work item")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(item))
}

async fn release_work_item(
    RequireWorkItemsWrite(principal): RequireWorkItemsWrite,
    State(state): State<AppState>,
    Path((entity_id, item_id)): Path<(EntityId, WorkItemId)>,
) -> Result<Json<WorkItem>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut item = store
        .read::<WorkItem>(item_id, "main")
        .await
        .map_err(AppError::Storage)?;
    item.release_claim()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<WorkItem>(&item, item_id, "main", "release work item claim")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(item))
}

async fn complete_work_item(
    RequireWorkItemsWrite(principal): RequireWorkItemsWrite,
    State(state): State<AppState>,
    Path((entity_id, item_id)): Path<(EntityId, WorkItemId)>,
    Json(body): Json<CompleteWorkItemRequest>,
) -> Result<Json<WorkItem>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut item = store
        .read::<WorkItem>(item_id, "main")
        .await
        .map_err(AppError::Storage)?;
    item.complete(&body.completed_by, body.result)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<WorkItem>(&item, item_id, "main", "complete work item")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(item))
}

async fn cancel_work_item(
    RequireWorkItemsWrite(principal): RequireWorkItemsWrite,
    State(state): State<AppState>,
    Path((entity_id, item_id)): Path<(EntityId, WorkItemId)>,
) -> Result<Json<WorkItem>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut item = store
        .read::<WorkItem>(item_id, "main")
        .await
        .map_err(AppError::Storage)?;
    item.cancel()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<WorkItem>(&item, item_id, "main", "cancel work item")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(item))
}
