//! Projection HTTP routes.
//!
//! Endpoints for managing stakeholder access manifests and computing
//! projected views of entity repos filtered by stakeholder permissions.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::domain::ids::{ContactId, EntityId, WorkspaceId};
use crate::error::AppError;
use crate::git::projection::{
    collect_all_paths, compute_visible_files, redact_json, AccessManifest, StakeholderAccess,
};

// ── Request / Response types ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct EntityQuery {
    #[serde(default = "WorkspaceId::new")]
    pub workspace_id: WorkspaceId,
}

#[derive(Deserialize)]
pub struct ProjectionQuery {
    #[serde(default = "WorkspaceId::new")]
    pub workspace_id: WorkspaceId,
    #[serde(default = "default_branch")]
    pub branch: String,
}

fn default_branch() -> String {
    "main".to_owned()
}

#[derive(Serialize)]
pub struct ProjectionResponse {
    pub contact_id: ContactId,
    pub role: String,
    pub files: HashMap<String, serde_json::Value>,
}

// ── Handlers ────────────────────────────────────────────────────────────

async fn get_access_manifest(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<AccessManifest>, AppError> {
    let workspace_id = query.workspace_id;

    let manifest = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = crate::store::entity_store::EntityStore::open(
                &layout,
                workspace_id,
                entity_id,
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;

            store
                .read_access_manifest("main")
                .map_err(|e| AppError::Internal(e.to_string()))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(manifest))
}

async fn set_stakeholder_access(
    State(state): State<AppState>,
    Path((entity_id, contact_id)): Path<(EntityId, ContactId)>,
    Query(query): Query<EntityQuery>,
    Json(access): Json<StakeholderAccess>,
) -> Result<Json<StakeholderAccess>, AppError> {
    let workspace_id = query.workspace_id;

    let updated = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = crate::store::entity_store::EntityStore::open(
                &layout,
                workspace_id,
                entity_id,
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;

            let mut manifest = store
                .read_access_manifest("main")
                .map_err(|e| AppError::Internal(e.to_string()))?;

            manifest.set_stakeholder(contact_id, access.clone());

            store
                .write_access_manifest(
                    "main",
                    &manifest,
                    &format!("Set access for stakeholder {contact_id}"),
                )
                .map_err(|e| AppError::Internal(e.to_string()))?;

            Ok::<_, AppError>(access)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(updated))
}

async fn remove_stakeholder_access(
    State(state): State<AppState>,
    Path((entity_id, contact_id)): Path<(EntityId, ContactId)>,
    Query(query): Query<EntityQuery>,
) -> Result<StatusCode, AppError> {
    let workspace_id = query.workspace_id;

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = crate::store::entity_store::EntityStore::open(
                &layout,
                workspace_id,
                entity_id,
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;

            let mut manifest = store
                .read_access_manifest("main")
                .map_err(|e| AppError::Internal(e.to_string()))?;

            manifest.remove_stakeholder(&contact_id);

            store
                .write_access_manifest(
                    "main",
                    &manifest,
                    &format!("Remove access for stakeholder {contact_id}"),
                )
                .map_err(|e| AppError::Internal(e.to_string()))?;

            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_projection(
    State(state): State<AppState>,
    Path((entity_id, contact_id)): Path<(EntityId, ContactId)>,
    Query(query): Query<ProjectionQuery>,
) -> Result<Json<ProjectionResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let branch = query.branch;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = crate::store::entity_store::EntityStore::open(
                &layout,
                workspace_id,
                entity_id,
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;

            let manifest = store
                .read_access_manifest(&branch)
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let access = manifest
                .get_stakeholder(&contact_id)
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "no access rules for stakeholder {contact_id}"
                    ))
                })?
                .clone();

            let all_paths = collect_all_paths(store.repo(), &branch)
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let visible = compute_visible_files(&access, &all_paths);

            let mut files = HashMap::new();
            for (path, level) in visible {
                let raw: serde_json::Value = match store.repo().read_json(&branch, path) {
                    Ok(v) => v,
                    Err(_) => continue, // skip non-JSON or unreadable files
                };
                let redacted = redact_json(&raw, level);
                files.insert(path.to_owned(), redacted);
            }

            let role = serde_json::to_value(access.role())
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| format!("{:?}", access.role()));

            Ok::<_, AppError>(ProjectionResponse {
                contact_id,
                role,
                files,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(response))
}

// ── Router ──────────────────────────────────────────────────────────────

pub fn projection_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/entities/{entity_id}/access-manifest",
            get(get_access_manifest),
        )
        .route(
            "/v1/entities/{entity_id}/access-manifest/stakeholders/{contact_id}",
            put(set_stakeholder_access).delete(remove_stakeholder_access),
        )
        .route(
            "/v1/entities/{entity_id}/projection/{contact_id}",
            get(get_projection),
        )
}
