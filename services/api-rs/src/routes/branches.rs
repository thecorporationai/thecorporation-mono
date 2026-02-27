//! Branch management HTTP routes.
//!
//! Endpoints for creating, listing, merging, and deleting branches on entity
//! repos. Also provides the `BranchTarget` extractor that reads the
//! `X-Corp-Branch` header (defaulting to `"main"`).

use axum::{
    extract::{FromRequestParts, Path, Query, State},
    http::{request::Parts, StatusCode},
    routing::{delete, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::{AppState, WorkspaceEntityQuery};
use crate::error::AppError;
use crate::git::merge::MergeResult;

// ── BranchTarget extractor ──────────────────────────────────────────────

/// Extracts the target branch from the `X-Corp-Branch` header.
///
/// If the header is absent, defaults to `"main"`.
pub struct BranchTarget(String);

impl BranchTarget {
    /// Returns the branch name.
    pub fn name(&self) -> &str {
        &self.0
    }

    /// Consumes the extractor and returns the inner branch name.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<S> FromRequestParts<S> for BranchTarget
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let branch = parts
            .headers
            .get("X-Corp-Branch")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("main")
            .to_owned();
        Ok(BranchTarget(branch))
    }
}

// ── Request / Response types ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateBranchRequest {
    name: String,
    #[serde(default = "default_branch")]
    from: String,
}

impl CreateBranchRequest {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn from(&self) -> &str {
        &self.from
    }
}

fn default_branch() -> String {
    "main".to_owned()
}

#[derive(Serialize)]
pub struct CreateBranchResponse {
    branch: String,
    base_commit: String,
}

impl CreateBranchResponse {
    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn base_commit(&self) -> &str {
        &self.base_commit
    }
}

#[derive(Serialize)]
pub struct BranchListEntry {
    name: String,
    head_oid: String,
}

impl BranchListEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn head_oid(&self) -> &str {
        &self.head_oid
    }
}

#[derive(Deserialize)]
pub struct MergeBranchRequest {
    #[serde(default = "default_branch")]
    into: String,
}

impl MergeBranchRequest {
    pub fn target_branch(&self) -> &str {
        &self.into
    }
}

#[derive(Serialize)]
pub struct MergeBranchResponse {
    merged: bool,
    strategy: String,
    commit: Option<String>,
}

impl MergeBranchResponse {
    pub fn merged(&self) -> bool {
        self.merged
    }

    pub fn strategy(&self) -> &str {
        &self.strategy
    }

    pub fn commit(&self) -> Option<&str> {
        self.commit.as_deref()
    }
}

// ── Handlers ────────────────────────────────────────────────────────────

async fn create_branch(
    State(state): State<AppState>,
    Query(query): Query<WorkspaceEntityQuery>,
    Json(req): Json<CreateBranchRequest>,
) -> Result<(StatusCode, Json<CreateBranchResponse>), AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let info = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let name = req.name.clone();
        let from = req.from.clone();
        move || {
            let store = crate::store::entity_store::EntityStore::open(
                &layout,
                workspace_id,
                entity_id,
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;

            crate::git::branch::create_branch(store.repo(), &name, &from)
                .map_err(AppError::from)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok((
        StatusCode::CREATED,
        Json(CreateBranchResponse {
            branch: info.name,
            base_commit: info.head_oid.to_string(),
        }),
    ))
}

async fn list_branches(
    State(state): State<AppState>,
    Query(query): Query<WorkspaceEntityQuery>,
) -> Result<Json<Vec<BranchListEntry>>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let branches = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = crate::store::entity_store::EntityStore::open(
                &layout,
                workspace_id,
                entity_id,
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;

            crate::git::branch::list_branches(store.repo()).map_err(AppError::from)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let entries = branches
        .into_iter()
        .map(|b| BranchListEntry {
            name: b.name,
            head_oid: b.head_oid.to_string(),
        })
        .collect();

    Ok(Json(entries))
}

async fn merge_branch(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<WorkspaceEntityQuery>,
    Json(req): Json<MergeBranchRequest>,
) -> Result<Json<MergeBranchResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let source = name.clone();
        let target = req.into.clone();
        move || {
            let store = crate::store::entity_store::EntityStore::open(
                &layout,
                workspace_id,
                entity_id,
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;

            crate::git::merge::merge_branch(store.repo(), &source, &target, None)
                .map_err(AppError::from)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let response = match result {
        MergeResult::FastForward { new_oid } => MergeBranchResponse {
            merged: true,
            strategy: "fast_forward".to_owned(),
            commit: Some(new_oid.to_string()),
        },
        MergeResult::AlreadyUpToDate => MergeBranchResponse {
            merged: true,
            strategy: "already_up_to_date".to_owned(),
            commit: None,
        },
        MergeResult::ThreeWayMerge { new_oid } => MergeBranchResponse {
            merged: true,
            strategy: "three_way".to_owned(),
            commit: Some(new_oid.to_string()),
        },
    };

    Ok(Json(response))
}

async fn delete_branch_handler(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<WorkspaceEntityQuery>,
) -> Result<StatusCode, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = crate::store::entity_store::EntityStore::open(
                &layout,
                workspace_id,
                entity_id,
            )
            .map_err(|e| AppError::Internal(e.to_string()))?;

            crate::git::branch::delete_branch(store.repo(), &name)
                .map_err(AppError::from)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(StatusCode::NO_CONTENT)
}

/// Prune a branch (POST alternative to DELETE for clients that don't support DELETE).
async fn prune_branch(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<WorkspaceEntityQuery>,
) -> Result<StatusCode, AppError> {
    delete_branch_handler(State(state), Path(name), Query(query)).await
}

// ── Router ──────────────────────────────────────────────────────────────

pub fn branch_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/branches", post(create_branch).get(list_branches))
        .route("/v1/branches/{name}/merge", post(merge_branch))
        .route("/v1/branches/{name}/prune", post(prune_branch))
        .route("/v1/branches/{name}", delete(delete_branch_handler))
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Request};

    #[tokio::test]
    async fn branch_target_defaults_to_main() {
        let mut parts = request_parts_without_header();
        let target = BranchTarget::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(target.name(), "main");
    }

    #[tokio::test]
    async fn branch_target_reads_header() {
        let mut parts = request_parts_with_header("feature/equity-grants");
        let target = BranchTarget::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(target.name(), "feature/equity-grants");
    }

    #[tokio::test]
    async fn branch_target_into_inner() {
        let mut parts = request_parts_with_header("dev");
        let target = BranchTarget::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(target.into_inner(), "dev");
    }

    fn request_parts_without_header() -> Parts {
        let (parts, _body) = Request::builder()
            .uri("/v1/branches")
            .body(())
            .unwrap()
            .into_parts();
        parts
    }

    fn request_parts_with_header(branch: &str) -> Parts {
        let (parts, _body) = Request::builder()
            .uri("/v1/branches")
            .header("X-Corp-Branch", HeaderValue::from_str(branch).unwrap())
            .body(())
            .unwrap()
            .into_parts();
        parts
    }
}
