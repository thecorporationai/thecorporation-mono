//! Auth HTTP routes.
//!
//! Endpoints for workspace provisioning, API key management, and token exchange.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::RequireAdmin;
use crate::domain::auth::{
    api_key::generate_api_key,
    claims::{Claims, PrincipalType, encode_token},
    scopes::{Scope, ScopeSet},
    ssh_key::{SshKeyLookup, SshKeyRecord, parse_public_key},
};
use crate::domain::ids::{ApiKeyId, ContactId, EntityId, SshKeyId, WorkspaceId};
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProvisionWorkspaceRequest {
    pub name: String,
    #[serde(default)]
    pub owner_email: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<Scope>,
    /// Scope this key to a specific contact. `null` = workspace-wide.
    #[serde(default)]
    pub contact_id: Option<ContactId>,
    /// Restrict this key to specific entities. `null` = all entities.
    #[serde(default)]
    pub entity_ids: Option<Vec<EntityId>>,
}

fn default_scopes() -> Vec<Scope> {
    vec![]
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TokenExchangeRequest {
    pub api_key: String,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: i64,
}

fn default_ttl() -> i64 {
    3600
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct ProvisionWorkspaceResponse {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub api_key: String,
    pub api_key_id: ApiKeyId,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ApiKeyResponse {
    pub key_id: ApiKeyId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub scopes: Vec<Scope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<ContactId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_ids: Option<Vec<EntityId>>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_key: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TokenExchangeResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// ── Handlers ─────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/workspaces/provision",
    tag = "auth",
    request_body = ProvisionWorkspaceRequest,
    responses(
        (status = 201, description = "Workspace provisioned", body = ProvisionWorkspaceResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
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
        let valkey_client = state.valkey_client.clone();
        let name = req.name.clone();
        move || {
            let ws_store = WorkspaceStore::init(&layout, workspace_id, &name, valkey_client.as_ref())
                .map_err(|e| AppError::Internal(format!("init workspace: {e}")))?;

            // Generate the first API key
            let scopes = ScopeSet::from_vec(vec![Scope::All]);
            let (raw_key, record) =
                generate_api_key(workspace_id, "default".to_owned(), scopes, None, None, None)
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

#[utoipa::path(
    post,
    path = "/v1/api-keys",
    tag = "auth",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created", body = ApiKeyResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_api_key(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyResponse>), AppError> {
    if req.name.is_empty() || req.name.len() > 128 {
        return Err(AppError::BadRequest(
            "API key name must be between 1 and 128 characters".to_owned(),
        ));
    }
    let workspace_id = auth.workspace_id();

    let (raw_key, record) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let name = req.name;
        let scopes = req.scopes;
        let contact_id = req.contact_id;
        let entity_ids = req.entity_ids;
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let scope_set = ScopeSet::from_vec(scopes.clone());
            let (raw_key, record) =
                generate_api_key(workspace_id, name, scope_set, None, contact_id, entity_ids)
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok((
        StatusCode::CREATED,
        Json(ApiKeyResponse {
            key_id: record.key_id(),
            workspace_id: record.workspace_id(),
            name: record.name().to_owned(),
            scopes: record.scopes().to_vec(),
            contact_id: record.contact_id(),
            entity_ids: record.entity_ids().map(|ids| ids.to_vec()),
            created_at: record.created_at().to_rfc3339(),
            raw_key: Some(raw_key),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/v1/api-keys",
    tag = "auth",
    responses(
        (status = 200, description = "List of API keys", body = Vec<ApiKeyResponse>),
    ),
)]
async fn list_api_keys(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<ApiKeyResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let keys = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
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
                            contact_id: record.contact_id(),
                            entity_ids: record.entity_ids().map(|ids| ids.to_vec()),
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(keys))
}

#[utoipa::path(
    delete,
    path = "/v1/api-keys/{key_id}",
    tag = "auth",
    params(("key_id" = ApiKeyId, Path, description = "API key ID to revoke")),
    responses(
        (status = 204, description = "API key revoked"),
        (status = 404, description = "API key not found"),
    ),
)]
async fn revoke_api_key(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(key_id): Path<ApiKeyId>,
) -> Result<StatusCode, AppError> {
    let workspace_id = auth.workspace_id();

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/v1/api-keys/{key_id}/rotate",
    tag = "auth",
    params(("key_id" = ApiKeyId, Path, description = "API key ID to rotate")),
    responses(
        (status = 200, description = "Rotated API key", body = ApiKeyResponse),
        (status = 404, description = "API key not found"),
    ),
)]
async fn rotate_api_key(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(key_id): Path<ApiKeyId>,
) -> Result<Json<ApiKeyResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let (raw_key, new_record) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
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
                .write_json(
                    &old_path,
                    &revoked,
                    &format!("Revoke old key {key_id} for rotation"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            // Generate new key with same scopes and contact/entity restrictions
            let (raw_key, new_record) = generate_api_key(
                workspace_id,
                old_record.name().to_owned(),
                old_record.scopes().clone(),
                None,
                old_record.contact_id(),
                old_record.entity_ids().map(|ids| ids.to_vec()),
            )
            .map_err(|e| AppError::Internal(format!("generate key: {e}")))?;

            let new_id = new_record.key_id();
            let new_path = format!("api-keys/{}.json", new_id);
            ws_store
                .write_json(
                    &new_path,
                    &new_record,
                    &format!("Create rotated key {new_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>((raw_key, new_record))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(ApiKeyResponse {
        key_id: new_record.key_id(),
        workspace_id: new_record.workspace_id(),
        name: new_record.name().to_owned(),
        scopes: new_record.scopes().to_vec(),
        contact_id: new_record.contact_id(),
        entity_ids: new_record.entity_ids().map(|ids| ids.to_vec()),
        created_at: new_record.created_at().to_rfc3339(),
        raw_key: Some(raw_key),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/auth/token-exchange",
    tag = "auth",
    request_body = TokenExchangeRequest,
    responses(
        (status = 200, description = "Token exchange successful", body = TokenExchangeResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Invalid API key"),
    ),
)]
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

    let api_key = req.api_key.clone();
    let ttl = req.ttl_seconds;

    // Verify the API key against all workspace storage (workspace is derived from key record).
    let (workspace_id, scopes, contact_id, entity_ids, entity_id) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            for workspace_id in layout.list_workspace_ids() {
                let ws_store = match WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref()) {
                    Ok(store) => store,
                    Err(_) => continue,
                };
                let key_ids = match ws_store.list_api_key_ids() {
                    Ok(ids) => ids,
                    Err(_) => continue,
                };

                for id in key_ids {
                    if let Ok(record) = ws_store.read_api_key(id) {
                        if !record.is_valid() {
                            continue;
                        }
                        if let Ok(true) = crate::domain::auth::api_key::verify_api_key(
                            &api_key,
                            record.key_hash(),
                        ) {
                            let entity_ids = record.entity_ids().map(|ids| ids.to_vec());
                            let entity_id =
                                entity_ids.as_ref().and_then(|ids| ids.first()).copied();
                            return Ok((
                                record.workspace_id(),
                                record.scopes().to_vec(),
                                record.contact_id(),
                                entity_ids,
                                entity_id,
                            ));
                        }
                    }
                }
            }

            Err(AppError::Unauthorized("invalid API key".to_owned()))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let now = chrono::Utc::now().timestamp();
    let exp = now + ttl;

    let claims = Claims::new(
        workspace_id,
        entity_id,
        contact_id,
        entity_ids,
        PrincipalType::User,
        scopes,
        now,
        exp,
    );

    let token = encode_token(&claims, &state.jwt_secret)
        .map_err(|e| AppError::Internal(format!("token generation failed: {e}")))?;

    Ok(Json(TokenExchangeResponse {
        access_token: token,
        token_type: "Bearer".to_owned(),
        expires_in: ttl,
    }))
}

// ── SSH key management ───────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddSshKeyRequest {
    pub name: String,
    pub public_key: String,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<Scope>,
    #[serde(default)]
    pub entity_ids: Option<Vec<EntityId>>,
    #[serde(default)]
    pub contact_id: Option<ContactId>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SshKeyResponse {
    pub key_id: SshKeyId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub fingerprint: String,
    pub algorithm: String,
    pub scopes: Vec<Scope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_ids: Option<Vec<EntityId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<ContactId>,
    pub created_at: String,
}

#[utoipa::path(
    post,
    path = "/v1/ssh-keys",
    tag = "auth",
    request_body = AddSshKeyRequest,
    responses(
        (status = 201, description = "SSH key added", body = SshKeyResponse),
        (status = 400, description = "Invalid public key"),
    ),
)]
async fn add_ssh_key(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<AddSshKeyRequest>,
) -> Result<(StatusCode, Json<SshKeyResponse>), AppError> {
    if req.name.is_empty() || req.name.len() > 128 {
        return Err(AppError::BadRequest(
            "SSH key name must be between 1 and 128 characters".to_owned(),
        ));
    }

    let (fingerprint, algorithm) = parse_public_key(&req.public_key)
        .map_err(|e| AppError::BadRequest(e))?;

    // Check for duplicate fingerprint
    if state.ssh_key_index.lookup(&fingerprint).is_some() {
        return Err(AppError::Conflict(format!(
            "SSH key with fingerprint {fingerprint} is already registered"
        )));
    }

    let workspace_id = auth.workspace_id();
    let key_id = SshKeyId::new();
    let scopes = ScopeSet::from_vec(req.scopes.clone());

    let record = SshKeyRecord {
        key_id,
        workspace_id,
        name: req.name.clone(),
        public_key_openssh: req.public_key.clone(),
        fingerprint: fingerprint.clone(),
        algorithm: algorithm.clone(),
        scopes: scopes.clone(),
        entity_ids: req.entity_ids.clone(),
        contact_id: req.contact_id,
        created_at: chrono::Utc::now(),
        revoked_at: None,
    };

    let record_clone = record.clone();
    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;
            let path = format!("ssh-keys/{}.json", key_id);
            ws_store
                .write_json(&path, &record_clone, &format!("Add SSH key {key_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    // Update in-memory index
    state.ssh_key_index.insert(
        fingerprint.clone(),
        SshKeyLookup {
            workspace_id,
            key_id,
            scopes,
            entity_ids: req.entity_ids.clone(),
            contact_id: req.contact_id,
        },
    );

    Ok((
        StatusCode::CREATED,
        Json(SshKeyResponse {
            key_id,
            workspace_id,
            name: record.name,
            fingerprint,
            algorithm,
            scopes: record.scopes.to_vec(),
            entity_ids: record.entity_ids,
            contact_id: record.contact_id,
            created_at: record.created_at.to_rfc3339(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/v1/ssh-keys",
    tag = "auth",
    responses(
        (status = 200, description = "List of SSH keys", body = Vec<SshKeyResponse>),
    ),
)]
async fn list_ssh_keys(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<SshKeyResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let keys = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let ids = ws_store
                .list_ssh_key_ids()
                .map_err(|e| AppError::Internal(format!("list keys: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(record) = ws_store.read_ssh_key(id) {
                    if record.is_valid() {
                        results.push(SshKeyResponse {
                            key_id: record.key_id,
                            workspace_id: record.workspace_id,
                            name: record.name,
                            fingerprint: record.fingerprint,
                            algorithm: record.algorithm,
                            scopes: record.scopes.to_vec(),
                            entity_ids: record.entity_ids,
                            contact_id: record.contact_id,
                            created_at: record.created_at.to_rfc3339(),
                        });
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(keys))
}

#[utoipa::path(
    delete,
    path = "/v1/ssh-keys/{key_id}",
    tag = "auth",
    params(("key_id" = SshKeyId, Path, description = "SSH key ID to revoke")),
    responses(
        (status = 204, description = "SSH key revoked"),
        (status = 404, description = "SSH key not found"),
    ),
)]
async fn revoke_ssh_key(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(key_id): Path<SshKeyId>,
) -> Result<StatusCode, AppError> {
    let workspace_id = auth.workspace_id();

    let fingerprint = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let mut record = ws_store
                .read_ssh_key(key_id)
                .map_err(|_| AppError::NotFound(format!("SSH key {} not found", key_id)))?;

            let fingerprint = record.fingerprint.clone();
            record.revoke();

            let path = format!("ssh-keys/{}.json", key_id);
            ws_store
                .write_json(&path, &record, &format!("Revoke SSH key {key_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(fingerprint)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    // Remove from in-memory index
    state.ssh_key_index.remove(&fingerprint);

    Ok(StatusCode::NO_CONTENT)
}

// ── Router ───────────────────────────────────────────────────────────

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/workspaces/provision", post(provision_workspace))
        .route("/v1/api-keys", post(create_api_key).get(list_api_keys))
        .route("/v1/api-keys/{key_id}", delete(revoke_api_key))
        .route("/v1/api-keys/{key_id}/rotate", post(rotate_api_key))
        .route("/v1/auth/token-exchange", post(token_exchange))
        .route("/v1/ssh-keys", post(add_ssh_key).get(list_ssh_keys))
        .route("/v1/ssh-keys/{key_id}", delete(revoke_ssh_key))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        provision_workspace,
        create_api_key,
        list_api_keys,
        revoke_api_key,
        rotate_api_key,
        token_exchange,
        add_ssh_key,
        list_ssh_keys,
        revoke_ssh_key,
    ),
    components(schemas(
        ProvisionWorkspaceRequest,
        ProvisionWorkspaceResponse,
        CreateApiKeyRequest,
        ApiKeyResponse,
        TokenExchangeRequest,
        TokenExchangeResponse,
        AddSshKeyRequest,
        SshKeyResponse,
    ))
)]
pub struct AuthApi;
