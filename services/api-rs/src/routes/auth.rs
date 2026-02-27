//! Auth HTTP routes.
//!
//! Endpoints for workspace provisioning, API key management, and token exchange.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::domain::auth::{
    api_key::generate_api_key,
    claims::{encode_token, Claims},
    scopes::{Scope, ScopeSet},
};
use crate::domain::ids::{ApiKeyId, WorkspaceId};
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProvisionWorkspaceRequest {
    pub name: String,
    #[serde(default)]
    pub owner_email: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    pub workspace_id: WorkspaceId,
    pub name: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<Scope>,
}

fn default_scopes() -> Vec<Scope> {
    vec![Scope::All]
}

#[derive(Deserialize)]
pub struct TokenExchangeRequest {
    pub api_key: String,
    pub workspace_id: WorkspaceId,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: i64,
}

fn default_ttl() -> i64 {
    3600
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ProvisionWorkspaceResponse {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub api_key: String,
    pub api_key_id: ApiKeyId,
}

#[derive(Serialize)]
pub struct ApiKeyResponse {
    pub key_id: ApiKeyId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub scopes: Vec<Scope>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_key: Option<String>,
}

#[derive(Serialize)]
pub struct TokenExchangeResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn provision_workspace(
    State(state): State<AppState>,
    Json(req): Json<ProvisionWorkspaceRequest>,
) -> Result<(StatusCode, Json<ProvisionWorkspaceResponse>), AppError> {
    if req.name.is_empty() || req.name.len() > 256 {
        return Err(AppError::BadRequest(
            "workspace name must be between 1 and 256 characters".to_owned(),
        ));
    }
    let workspace_id = WorkspaceId::new();

    let (raw_key, key_id) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let name = req.name.clone();
        move || {
            let ws_store = WorkspaceStore::init(&layout, workspace_id, &name)
                .map_err(|e| AppError::Internal(format!("init workspace: {e}")))?;

            // Generate the first API key
            let scopes = ScopeSet::from_vec(vec![Scope::All]);
            let (raw_key, record) = generate_api_key(workspace_id, "default".to_owned(), scopes, None)
                .map_err(|e| AppError::Internal(format!("generate key: {e}")))?;

            let key_id = record.key_id();
            let path = format!("api-keys/{}.json", key_id);
            ws_store
                .write_json(&path, &record, &format!("Create initial API key {key_id}"))
                .map_err(|e| AppError::Internal(format!("commit key: {e}")))?;

            Ok::<_, AppError>((raw_key, key_id))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok((
        StatusCode::CREATED,
        Json(ProvisionWorkspaceResponse {
            workspace_id,
            name: req.name,
            api_key: raw_key,
            api_key_id: key_id,
        }),
    ))
}

async fn create_api_key(
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>), AppError> {
    if req.name.is_empty() || req.name.len() > 128 {
        return Err(AppError::BadRequest(
            "API key name must be between 1 and 128 characters".to_owned(),
        ));
    }
    let workspace_id = req.workspace_id;

    let (raw_key, record) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let name = req.name;
        let scopes = req.scopes;
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let scope_set = ScopeSet::from_vec(scopes.clone());
            let (raw_key, record) = generate_api_key(workspace_id, name, scope_set, None)
                .map_err(|e| AppError::Internal(format!("generate key: {e}")))?;

            let key_id = record.key_id();
            let path = format!("api-keys/{}.json", key_id);
            ws_store
                .write_json(&path, &record, &format!("Create API key {key_id}"))
                .map_err(|e| AppError::Internal(format!("commit key: {e}")))?;

            Ok::<_, AppError>((raw_key, record))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok((
        StatusCode::CREATED,
        Json(ApiKeyResponse {
            key_id: record.key_id(),
            workspace_id: record.workspace_id(),
            name: record.name().to_owned(),
            scopes: record.scopes().to_vec(),
            created_at: record.created_at().to_rfc3339(),
            raw_key: Some(raw_key),
        }),
    ))
}

async fn list_api_keys(
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<Vec<ApiKeyResponse>>, AppError> {
    let keys = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let ids = ws_store
                .list_api_key_ids()
                .map_err(|e| AppError::Internal(format!("list keys: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(record) = ws_store.read_api_key(id) {
                    if record.is_valid() {
                        results.push(ApiKeyResponse {
                            key_id: record.key_id(),
                            workspace_id: record.workspace_id(),
                            name: record.name().to_owned(),
                            scopes: record.scopes().to_vec(),
                            created_at: record.created_at().to_rfc3339(),
                            raw_key: None,
                        });
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(keys))
}

async fn revoke_api_key(
    State(state): State<AppState>,
    Path((workspace_id, key_id)): Path<(WorkspaceId, ApiKeyId)>,
) -> Result<StatusCode, AppError> {
    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let mut record = ws_store
                .read_api_key(key_id)
                .map_err(|_| AppError::NotFound(format!("API key {} not found", key_id)))?;

            record.revoke();

            let path = format!("api-keys/{}.json", key_id);
            ws_store
                .write_json(&path, &record, &format!("Revoke API key {key_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(StatusCode::NO_CONTENT)
}

async fn rotate_api_key(
    State(state): State<AppState>,
    Path((workspace_id, key_id)): Path<(WorkspaceId, ApiKeyId)>,
) -> Result<Json<ApiKeyResponse>, AppError> {
    let (raw_key, new_record) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            // Read old key to get its metadata
            let old_record = ws_store
                .read_api_key(key_id)
                .map_err(|_| AppError::NotFound(format!("API key {} not found", key_id)))?;

            // Revoke old key
            let mut revoked = old_record.clone();
            revoked.revoke();
            let old_path = format!("api-keys/{}.json", key_id);
            ws_store
                .write_json(&old_path, &revoked, &format!("Revoke old key {key_id} for rotation"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            // Generate new key with same scopes
            let (raw_key, new_record) = generate_api_key(
                workspace_id,
                old_record.name().to_owned(),
                old_record.scopes().clone(),
                None,
            )
            .map_err(|e| AppError::Internal(format!("generate key: {e}")))?;

            let new_id = new_record.key_id();
            let new_path = format!("api-keys/{}.json", new_id);
            ws_store
                .write_json(&new_path, &new_record, &format!("Create rotated key {new_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>((raw_key, new_record))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(ApiKeyResponse {
        key_id: new_record.key_id(),
        workspace_id: new_record.workspace_id(),
        name: new_record.name().to_owned(),
        scopes: new_record.scopes().to_vec(),
        created_at: new_record.created_at().to_rfc3339(),
        raw_key: Some(raw_key),
    }))
}

async fn token_exchange(
    State(state): State<AppState>,
    Json(req): Json<TokenExchangeRequest>,
) -> Result<Json<TokenExchangeResponse>, AppError> {
    if !req.api_key.starts_with("sk_") {
        return Err(AppError::Unauthorized("invalid API key format".to_owned()));
    }
    if req.ttl_seconds < 60 || req.ttl_seconds > 86_400 {
        return Err(AppError::BadRequest(
            "ttl_seconds must be between 60 and 86400".to_owned(),
        ));
    }

    let workspace_id = req.workspace_id;
    let api_key = req.api_key.clone();
    let ttl = req.ttl_seconds;

    // Verify the API key against workspace storage
    let scopes = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|_| AppError::Unauthorized("workspace not found".to_owned()))?;

            let key_ids = ws_store
                .list_api_key_ids()
                .map_err(|e| AppError::Internal(format!("list keys: {e}")))?;

            for id in key_ids {
                if let Ok(record) = ws_store.read_api_key(id) {
                    if !record.is_valid() {
                        continue;
                    }
                    if let Ok(true) = crate::domain::auth::api_key::verify_api_key(&api_key, record.key_hash()) {
                        return Ok(record.scopes().to_vec());
                    }
                }
            }

            Err(AppError::Unauthorized("invalid API key".to_owned()))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let now = chrono::Utc::now().timestamp();
    let exp = now + ttl;

    let claims = Claims::new(workspace_id, None, scopes, now, exp);

    let token = encode_token(&claims, &state.jwt_secret)
        .map_err(|e| AppError::Internal(format!("token generation failed: {e}")))?;

    Ok(Json(TokenExchangeResponse {
        access_token: token,
        token_type: "Bearer".to_owned(),
        expires_in: ttl,
    }))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/workspaces/provision", post(provision_workspace))
        .route("/v1/api-keys", post(create_api_key))
        .route("/v1/api-keys/{workspace_id}", get(list_api_keys))
        .route(
            "/v1/api-keys/{workspace_id}/{key_id}",
            delete(revoke_api_key),
        )
        .route(
            "/v1/api-keys/{workspace_id}/{key_id}/rotate",
            post(rotate_api_key),
        )
        .route("/v1/auth/token-exchange", post(token_exchange))
}
