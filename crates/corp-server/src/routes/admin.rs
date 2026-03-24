//! Admin route handlers — workspace management, entity listing, and API key lifecycle.
//!
//! All routes require the `Admin` scope.
//!
//! ## Route map
//!
//! | Method | Path | Scope |
//! |--------|------|-------|
//! | GET    | `/workspaces` | `Admin` |
//! | GET    | `/workspaces/{workspace_id}/entities` | `Admin` |
//! | GET    | `/api-keys` | `Admin` |
//! | POST   | `/api-keys` | `Admin` |
//! | POST   | `/api-keys/{key_id}/revoke` | `Admin` |

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use corp_auth::{ApiKeyManager, RequireAdmin};
use corp_core::ids::{ApiKeyId, EntityId, WorkspaceId};
use corp_storage::workspace_store::ApiKeyRecord;

use crate::error::AppError;
use crate::state::{AppState, StorageBackend};

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workspaces", get(list_workspaces))
        .route(
            "/workspaces/{workspace_id}/entities",
            get(list_workspace_entities),
        )
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/{key_id}/revoke", post(revoke_api_key))
}

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    /// Human-readable label for this key (e.g. `"CI/CD deploy key"`).
    pub name: String,
    /// Capability scopes granted by this key (kebab-case strings).
    pub scopes: Vec<String>,
    /// Optionally restrict the key to a single entity.
    pub entity_id: Option<EntityId>,
}

/// Response for `POST /api-keys`.
///
/// The `raw_key` is **only ever returned once**.  The caller is responsible
/// for storing it securely; subsequent reads will only show the key ID and
/// metadata, not the secret.
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub key_id: ApiKeyId,
    /// The raw secret — shown exactly once.
    pub raw_key: String,
    pub name: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceSummary {
    pub workspace_id: WorkspaceId,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `GET /workspaces` — list all workspaces known to this deployment.
///
/// Enumerates the workspace directories under `DATA_ROOT`.  Each immediate
/// subdirectory whose name is a valid `WorkspaceId` UUID is returned.
async fn list_workspaces(
    RequireAdmin(_principal): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkspaceSummary>>, AppError> {
    let workspace_ids: Vec<WorkspaceId> = match &state.storage_backend {
        StorageBackend::Git => {
            let data_dir = state.data_dir.clone();
            tokio::task::spawn_blocking(move || -> std::io::Result<Vec<WorkspaceId>> {
                let mut ids = Vec::new();
                let Ok(rd) = std::fs::read_dir(&data_dir) else {
                    return Ok(ids);
                };
                for entry in rd.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let name = entry.file_name();
                    let s = name.to_string_lossy();
                    if let Ok(id) = s.parse::<WorkspaceId>() {
                        ids.push(id);
                    }
                }
                Ok(ids)
            })
            .await
            .map_err(|e| AppError::Internal(format!("spawn_blocking: {}", e)))?
            .map_err(|e| AppError::Internal(format!("readdir: {}", e)))?
        }
        StorageBackend::Kv { redis_url, .. } => {
            let mut con = crate::state::kv_connection_manager(redis_url).await?;
            let ids_str = corp_storage::kv::list_workspaces(&mut con)
                .await
                .map_err(AppError::Storage)?;
            ids_str
                .into_iter()
                .filter_map(|s| s.parse::<WorkspaceId>().ok())
                .collect()
        }
    };

    let workspaces = workspace_ids
        .into_iter()
        .map(|workspace_id| WorkspaceSummary { workspace_id })
        .collect();

    Ok(Json(workspaces))
}

/// `GET /workspaces/{workspace_id}/entities` — list all entities in a workspace.
async fn list_workspace_entities(
    RequireAdmin(_principal): RequireAdmin,
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<Vec<EntityId>>, AppError> {
    let workspace_store = state.open_workspace_store(workspace_id).await?;
    let entity_ids = workspace_store
        .list_entity_ids()
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(entity_ids))
}

/// `GET /api-keys` — list all API keys in the caller's workspace.
///
/// Returns metadata only; the raw key secret is never returned after creation.
/// Soft-deleted keys are excluded.
async fn list_api_keys(
    RequireAdmin(principal): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<ApiKeyRecord>>, AppError> {
    let workspace_store = state.open_workspace_store(principal.workspace_id).await?;
    let key_ids = workspace_store
        .list_api_key_ids()
        .await
        .map_err(AppError::Storage)?;

    let mut keys = Vec::with_capacity(key_ids.len());
    for key_id in key_ids {
        match workspace_store.read_api_key(key_id).await {
            Ok(record) if !record.deleted => keys.push(record),
            Ok(_) => {} // skip soft-deleted keys
            Err(e) => return Err(AppError::Storage(e)),
        }
    }

    Ok(Json(keys))
}

/// `POST /api-keys` — generate a new API key.
///
/// The raw secret is included in the response exactly once.  The caller must
/// store it securely; it cannot be retrieved again.
async fn create_api_key(
    RequireAdmin(principal): RequireAdmin,
    State(state): State<AppState>,
    Json(body): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    let workspace_store = state.open_workspace_store(principal.workspace_id).await?;

    let (raw_key, key_hash) = ApiKeyManager::generate();

    let record = ApiKeyRecord::new(
        body.name.clone(),
        key_hash,
        body.scopes.clone(),
        body.entity_id,
    );

    let key_id = record.key_id;
    workspace_store
        .write_api_key(&record)
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(CreateApiKeyResponse {
        key_id,
        raw_key,
        name: body.name,
        scopes: body.scopes,
    }))
}

/// `POST /api-keys/{key_id}/revoke` — soft-delete an API key.
///
/// Sets `deleted = true` on the stored record.  The key can no longer be used
/// to authenticate, and it is excluded from the `list_api_keys` response.
async fn revoke_api_key(
    RequireAdmin(principal): RequireAdmin,
    State(state): State<AppState>,
    Path(key_id): Path<ApiKeyId>,
) -> Result<Json<ApiKeyRecord>, AppError> {
    let workspace_store = state.open_workspace_store(principal.workspace_id).await?;

    workspace_store.delete_api_key(key_id).await.map_err(|e| {
        use corp_storage::error::StorageError;
        match e {
            StorageError::NotFound(_) => {
                AppError::NotFound(format!("api key {} not found", key_id))
            }
            other => AppError::Storage(other),
        }
    })?;

    // Return the updated (now deleted) record.
    let updated = workspace_store
        .read_api_key(key_id)
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(updated))
}
