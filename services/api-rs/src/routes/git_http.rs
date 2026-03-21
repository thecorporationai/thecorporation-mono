//! Git HTTP smart protocol routes.
//!
//! Implements the stateless-rpc protocol that `git clone` and `git push` use
//! when talking to an HTTP remote. Auth is via the existing Bearer token / API
//! key system with `GitRead` and `GitWrite` scopes.
//!
//! **Requires `STORAGE_BACKEND=valkey`**. The native protocol handlers talk
//! directly to the corp_store Valkey backend — no subprocess, no bare repos
//! on disk, single source of truth.
//!
//! Endpoints:
//! - `GET  /git/{workspace_id}/{repo}.git/info/refs?service=git-upload-pack`
//! - `POST /git/{workspace_id}/{repo}.git/git-upload-pack`
//! - `POST /git/{workspace_id}/{repo}.git/git-receive-pack`

use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::Response,
    routing::{get, post},
};
use serde::Deserialize;

use super::AppState;
use crate::auth::Principal;
use crate::domain::auth::scopes::Scope;
use crate::error::AppError;
use crate::git::native_transport;
use crate::git::protocol::{GitService, parse_repo_path, parse_service_param};

#[derive(Deserialize)]
struct InfoRefsQuery {
    service: String,
}

/// `GET /git/{workspace_id}/{repo}.git/info/refs?service=git-upload-pack`
///
/// Returns the ref advertisement for the requested service.
async fn info_refs(
    auth: Principal,
    State(state): State<AppState>,
    Path((workspace_id_str, repo_dotgit)): Path<(String, String)>,
    Query(params): Query<InfoRefsQuery>,
) -> Result<Response<Body>, AppError> {
    let service = parse_service_param(&params.service)?;
    let (workspace_id, entity_id) = parse_repo_ids(&workspace_id_str, &repo_dotgit)?;
    check_git_auth(&auth, workspace_id, entity_id, service)?;
    require_valkey(&state)?;

    let ws = workspace_id.to_string();
    let ent = entity_id.to_string();
    let service_name = params.service.clone();

    let git_output = tokio::task::spawn_blocking({
        let valkey_client = state.valkey_client.clone().unwrap();
        move || {
            let mut con = valkey_client
                .get_connection()
                .map_err(|e| AppError::Internal(format!("valkey connection: {e}")))?;
            native_transport::info_refs(&mut con, &ws, &ent, service)
                .map_err(AppError::from)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    // Build response: service announcement pkt-line + flush + ref advertisement
    let service_line = format!("# service={}\n", service_name);
    let pkt_len = service_line.len() + 4;
    let pkt = format!("{pkt_len:04x}{service_line}");

    let mut body = pkt.into_bytes();
    body.extend_from_slice(b"0000"); // flush-pkt
    body.extend_from_slice(&git_output);

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", service.advertisement_content_type())
        .header("Cache-Control", "no-cache")
        .body(Body::from(body))
        .unwrap())
}

/// `POST /git/{workspace_id}/{repo}.git/git-upload-pack`
///
/// Handles the fetch/clone data exchange.
async fn upload_pack(
    auth: Principal,
    State(state): State<AppState>,
    Path((workspace_id_str, repo_dotgit)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> Result<Response<Body>, AppError> {
    let (workspace_id, entity_id) = parse_repo_ids(&workspace_id_str, &repo_dotgit)?;
    check_git_auth(&auth, workspace_id, entity_id, GitService::UploadPack)?;
    require_valkey(&state)?;

    let ws = workspace_id.to_string();
    let ent = entity_id.to_string();

    let response_body = tokio::task::spawn_blocking({
        let valkey_client = state.valkey_client.clone().unwrap();
        move || {
            let mut con = valkey_client
                .get_connection()
                .map_err(|e| AppError::Internal(format!("valkey connection: {e}")))?;
            native_transport::upload_pack(&mut con, &ws, &ent, &body)
                .map_err(AppError::from)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    Ok(Response::builder()
        .status(200)
        .header(
            "Content-Type",
            GitService::UploadPack.result_content_type(),
        )
        .header("Cache-Control", "no-cache")
        .body(Body::from(response_body))
        .unwrap())
}

/// `POST /git/{workspace_id}/{repo}.git/git-receive-pack`
///
/// Handles the push data exchange.
async fn receive_pack(
    auth: Principal,
    State(state): State<AppState>,
    Path((workspace_id_str, repo_dotgit)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> Result<Response<Body>, AppError> {
    let (workspace_id, entity_id) = parse_repo_ids(&workspace_id_str, &repo_dotgit)?;
    check_git_auth(&auth, workspace_id, entity_id, GitService::ReceivePack)?;
    require_valkey(&state)?;

    let ws = workspace_id.to_string();
    let ent = entity_id.to_string();

    let response_body = tokio::task::spawn_blocking({
        let valkey_client = state.valkey_client.clone().unwrap();
        move || {
            let mut con = valkey_client
                .get_connection()
                .map_err(|e| AppError::Internal(format!("valkey connection: {e}")))?;
            native_transport::receive_pack(&mut con, &ws, &ent, &body)
                .map_err(AppError::from)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join: {e}")))??;

    Ok(Response::builder()
        .status(200)
        .header(
            "Content-Type",
            GitService::ReceivePack.result_content_type(),
        )
        .header("Cache-Control", "no-cache")
        .body(Body::from(response_body))
        .unwrap())
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Parse workspace_id and entity_id from the URL path segments.
fn parse_repo_ids(
    workspace_id_str: &str,
    repo_segment: &str,
) -> Result<(crate::domain::ids::WorkspaceId, crate::domain::ids::EntityId), AppError> {
    let path = format!("{workspace_id_str}/{repo_segment}");
    parse_repo_path(&path).map_err(AppError::from)
}

/// Verify the principal has access to the requested repo and service.
fn check_git_auth(
    auth: &Principal,
    workspace_id: crate::domain::ids::WorkspaceId,
    entity_id: crate::domain::ids::EntityId,
    service: GitService,
) -> Result<(), AppError> {
    if auth.workspace_id() != workspace_id {
        return Err(AppError::Forbidden("workspace mismatch".to_owned()));
    }

    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let required = match service {
        GitService::UploadPack => Scope::GitRead,
        GitService::ReceivePack => Scope::GitWrite,
    };
    if !auth.scopes().has(required) {
        return Err(AppError::Forbidden(format!(
            "insufficient scopes: required {}",
            required
        )));
    }

    Ok(())
}

/// Ensure the Valkey backend is available (git server requires it).
fn require_valkey(state: &AppState) -> Result<(), AppError> {
    if state.valkey_client.is_none() {
        return Err(AppError::ServiceUnavailable(
            "git server requires STORAGE_BACKEND=valkey".to_owned(),
        ));
    }
    Ok(())
}

// ── Router ──────────────────────────────────────────────────────────

pub fn git_http_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/git/{workspace_id}/{repo}/info/refs",
            get(info_refs),
        )
        .route(
            "/git/{workspace_id}/{repo}/git-upload-pack",
            post(upload_pack),
        )
        .route(
            "/git/{workspace_id}/{repo}/git-receive-pack",
            post(receive_pack),
        )
}
