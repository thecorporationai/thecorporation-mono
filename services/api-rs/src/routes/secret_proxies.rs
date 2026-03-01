//! Secret proxy management routes.
//!
//! Secrets are stored encrypted (Fernet) in workspace git repos at:
//! `secrets/<proxy_name>/config.json` and `secrets/<proxy_name>/secrets.json`.
//!
//! When `url` is `"self"`, secrets are resolved locally by decrypting from git.
//! Otherwise, the worker forwards resolution to the external URL.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::AppState;
use crate::auth::{RequireAdmin, RequireInternalWorker};
use crate::domain::agents::secret_proxy::{EncryptedSecrets, SecretProxyConfig};
use crate::domain::ids::WorkspaceId;
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProxyRequest {
    pub name: String,
    /// `"self"` for local encrypted secrets, or an external URL.
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetSecretsRequest {
    /// Key-value pairs. Values are plaintext — the server encrypts before storing.
    pub secrets: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyPathParams {
    pub workspace_id: WorkspaceId,
    pub proxy_name: String,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ProxyResponse {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: String,
    pub secret_count: usize,
}

#[derive(Serialize)]
pub struct SecretNamesResponse {
    pub proxy_name: String,
    pub names: Vec<String>,
}

// ── Internal resolve (called by worker) ──────────────────────────────

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveSecretsRequest {
    pub workspace_id: WorkspaceId,
    pub proxy_name: String,
    /// Which secret keys to resolve. If empty, resolve all.
    #[serde(default)]
    pub keys: Vec<String>,
}

#[derive(Serialize)]
pub struct ResolveSecretsResponse {
    pub proxy_name: String,
    pub url: String,
    pub values: HashMap<String, String>,
}

// ── Helpers ──────────────────────────────────────────────────────────

fn validate_proxy_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::BadRequest(
            "proxy name must be 1-64 characters".to_owned(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::BadRequest(
            "proxy name must contain only alphanumeric, hyphens, or underscores".to_owned(),
        ));
    }
    Ok(())
}

fn config_path(name: &str) -> String {
    format!("secrets/{name}/config.json")
}

fn secrets_path(name: &str) -> String {
    format!("secrets/{name}/secrets.json")
}

fn encrypt_value(fernet: &fernet::Fernet, plaintext: &str) -> String {
    fernet.encrypt(plaintext.as_bytes())
}

fn decrypt_value(fernet: &fernet::Fernet, token: &str) -> Result<String, AppError> {
    let bytes = fernet
        .decrypt(token)
        .map_err(|_| AppError::Internal("failed to decrypt secret".to_owned()))?;
    String::from_utf8(bytes)
        .map_err(|_| AppError::Internal("decrypted secret is not valid UTF-8".to_owned()))
}

fn require_fernet(state: &AppState) -> Result<&fernet::Fernet, AppError> {
    state
        .secrets_fernet
        .as_deref()
        .ok_or_else(|| AppError::Internal("SECRETS_MASTER_KEY not configured".to_owned()))
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn create_proxy(
    RequireAdmin(auth): RequireAdmin,
    Path(workspace_id): Path<WorkspaceId>,
    State(state): State<AppState>,
    Json(req): Json<CreateProxyRequest>,
) -> Result<(StatusCode, Json<ProxyResponse>), AppError> {
    if auth.workspace_id() != workspace_id {
        return Err(AppError::Forbidden("workspace access denied".to_owned()));
    }
    validate_proxy_name(&req.name)?;
    let name = req.name.clone();

    let config = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            // Check if proxy already exists
            if ws_store
                .path_exists(&config_path(&req.name))
                .unwrap_or(false)
            {
                return Err(AppError::Conflict(format!(
                    "secret proxy '{}' already exists",
                    req.name
                )));
            }

            let config = SecretProxyConfig::new(req.name.clone(), req.url, req.description);
            let empty_secrets = EncryptedSecrets::default();

            // Atomic commit: config + empty secrets
            use crate::git::commit::{FileWrite, commit_files};
            let files = vec![
                FileWrite::json(config_path(&req.name), &config)
                    .map_err(|e| AppError::Internal(format!("serialize config: {e}")))?,
                FileWrite::json(secrets_path(&req.name), &empty_secrets)
                    .map_err(|e| AppError::Internal(format!("serialize secrets: {e}")))?,
            ];
            commit_files(
                ws_store.repo(),
                "main",
                &format!("Create secret proxy '{}'", req.name),
                &files,
                None,
            )
            .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(config)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    Ok((
        StatusCode::CREATED,
        Json(ProxyResponse {
            name,
            url: config.url,
            description: config.description,
            created_at: config.created_at,
            secret_count: 0,
        }),
    ))
}

async fn list_proxies(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<Vec<ProxyResponse>>, AppError> {
    if auth.workspace_id() != workspace_id {
        return Err(AppError::Forbidden("workspace access denied".to_owned()));
    }

    let proxies = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let names = ws_store
                .list_names_in_dir("secrets")
                .map_err(|e| AppError::Internal(format!("list secrets dir: {e}")))?;

            let mut results = Vec::new();
            for name in names {
                let config: SecretProxyConfig = match ws_store.read_json(&config_path(&name)) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let secrets: EncryptedSecrets =
                    ws_store.read_json(&secrets_path(&name)).unwrap_or_default();
                results.push(ProxyResponse {
                    name: config.name,
                    url: config.url,
                    description: config.description,
                    created_at: config.created_at,
                    secret_count: secrets.entries.len(),
                });
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    Ok(Json(proxies))
}

async fn get_proxy(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(params): Path<ProxyPathParams>,
) -> Result<Json<ProxyResponse>, AppError> {
    if auth.workspace_id() != params.workspace_id {
        return Err(AppError::Forbidden("workspace access denied".to_owned()));
    }

    let proxy = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, params.workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let config: SecretProxyConfig = ws_store
                .read_json(&config_path(&params.proxy_name))
                .map_err(|_| {
                    AppError::NotFound(format!("secret proxy '{}' not found", params.proxy_name))
                })?;
            let secrets: EncryptedSecrets = ws_store
                .read_json(&secrets_path(&params.proxy_name))
                .unwrap_or_default();

            Ok::<_, AppError>(ProxyResponse {
                name: config.name,
                url: config.url,
                description: config.description,
                created_at: config.created_at,
                secret_count: secrets.entries.len(),
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    Ok(Json(proxy))
}

async fn set_secrets(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(params): Path<ProxyPathParams>,
    Json(req): Json<SetSecretsRequest>,
) -> Result<Json<SecretNamesResponse>, AppError> {
    if auth.workspace_id() != params.workspace_id {
        return Err(AppError::Forbidden("workspace access denied".to_owned()));
    }

    let fernet = require_fernet(&state)?;
    let fernet = fernet.clone();
    let proxy_name = params.proxy_name.clone();

    let names = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, params.workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            // Verify proxy exists
            let _config: SecretProxyConfig = ws_store
                .read_json(&config_path(&params.proxy_name))
                .map_err(|_| {
                AppError::NotFound(format!("secret proxy '{}' not found", params.proxy_name))
            })?;

            // Read existing secrets and merge
            let mut secrets: EncryptedSecrets = ws_store
                .read_json(&secrets_path(&params.proxy_name))
                .unwrap_or_default();

            for (key, value) in &req.secrets {
                let encrypted = encrypt_value(&fernet, value);
                secrets.entries.insert(key.clone(), encrypted);
            }

            let names: Vec<String> = secrets.entries.keys().cloned().collect();

            ws_store
                .write_json(
                    &secrets_path(&params.proxy_name),
                    &secrets,
                    &format!("Update secrets for proxy '{}'", params.proxy_name),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(names)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    Ok(Json(SecretNamesResponse { proxy_name, names }))
}

async fn list_secret_names(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(params): Path<ProxyPathParams>,
) -> Result<Json<SecretNamesResponse>, AppError> {
    if auth.workspace_id() != params.workspace_id {
        return Err(AppError::Forbidden("workspace access denied".to_owned()));
    }

    let proxy_name = params.proxy_name.clone();

    let names = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, params.workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            // Verify proxy exists
            let _config: SecretProxyConfig = ws_store
                .read_json(&config_path(&params.proxy_name))
                .map_err(|_| {
                AppError::NotFound(format!("secret proxy '{}' not found", params.proxy_name))
            })?;

            let secrets: EncryptedSecrets = ws_store
                .read_json(&secrets_path(&params.proxy_name))
                .unwrap_or_default();

            Ok::<_, AppError>(secrets.entries.keys().cloned().collect::<Vec<_>>())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    Ok(Json(SecretNamesResponse { proxy_name, names }))
}

/// Internal endpoint: resolve encrypted secrets for agent execution.
///
/// Called by the worker to get plaintext secret values for opaque token creation.
/// For `"self"` proxies, decrypts from git. For external proxies, returns the URL
/// so the worker can forward requests there.
async fn resolve_secrets(
    _worker: RequireInternalWorker,
    State(state): State<AppState>,
    Json(req): Json<ResolveSecretsRequest>,
) -> Result<Json<ResolveSecretsResponse>, AppError> {
    let fernet = require_fernet(&state)?;
    let fernet = fernet.clone();

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, req.workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let config: SecretProxyConfig = ws_store
                .read_json(&config_path(&req.proxy_name))
                .map_err(|_| {
                    AppError::NotFound(format!("secret proxy '{}' not found", req.proxy_name))
                })?;

            if !config.is_self() {
                // External proxy — return the URL, no values
                return Ok(ResolveSecretsResponse {
                    proxy_name: req.proxy_name,
                    url: config.url,
                    values: HashMap::new(),
                });
            }

            // Self proxy — decrypt from git
            let secrets: EncryptedSecrets = ws_store
                .read_json(&secrets_path(&req.proxy_name))
                .unwrap_or_default();

            let mut values = HashMap::new();
            for (key, encrypted) in &secrets.entries {
                // If specific keys requested, only decrypt those
                if !req.keys.is_empty() && !req.keys.contains(key) {
                    continue;
                }
                match decrypt_value(&fernet, encrypted) {
                    Ok(plaintext) => {
                        values.insert(key.clone(), plaintext);
                    }
                    Err(e) => {
                        tracing::error!(key, error = ?e, "failed to decrypt secret");
                    }
                }
            }

            Ok::<_, AppError>(ResolveSecretsResponse {
                proxy_name: req.proxy_name,
                url: "self".to_owned(),
                values,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    Ok(Json(result))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn secret_proxy_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/workspaces/{workspace_id}/secret-proxies",
            post(create_proxy).get(list_proxies),
        )
        .route(
            "/v1/workspaces/{workspace_id}/secret-proxies/{proxy_name}",
            get(get_proxy),
        )
        .route(
            "/v1/workspaces/{workspace_id}/secret-proxies/{proxy_name}/secrets",
            put(set_secrets).get(list_secret_names),
        )
        .route("/v1/internal/resolve-secrets", post(resolve_secrets))
}
