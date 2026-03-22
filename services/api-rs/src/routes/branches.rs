//! Branch management HTTP routes.
//!
//! Endpoints for creating, listing, merging, and deleting branches on entity
//! repos. Also provides the `BranchTarget` extractor that reads the
//! `X-Corp-Branch` header (defaulting to `"main"`).

use axum::{
    Json, Router,
    extract::{FromRequestParts, Path, Query, State},
    http::{StatusCode, request::Parts},
    routing::{delete, post},
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::{RequireBranchCreate, RequireBranchDelete, RequireBranchMerge};
use crate::error::AppError;
use crate::git::branch_name::{BranchName, BranchNameError};
use crate::store::entity_store::MergeOutcome;

// ── BranchTarget extractor ──────────────────────────────────────────────

/// Extracts the target branch from the `X-Corp-Branch` header.
///
/// If the header is absent, defaults to `"main"`.
/// Validates the branch name against git branch naming rules.
pub struct BranchTarget(BranchName);

impl BranchTarget {
    /// Returns the branch name as a string slice.
    pub fn name(&self) -> &str {
        self.0.as_str()
    }

    /// Returns a reference to the inner `BranchName`.
    pub fn branch(&self) -> &BranchName {
        &self.0
    }

    /// Consumes the extractor and returns the inner branch name string.
    pub fn into_inner(self) -> String {
        self.0.into_inner()
    }
}

impl<S> FromRequestParts<S> for BranchTarget
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let raw = parts
            .headers
            .get("X-Corp-Branch")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("main");
        let branch = BranchName::new(raw)
            .map_err(|e: BranchNameError| (StatusCode::BAD_REQUEST, e.to_string()))?;
        Ok(BranchTarget(branch))
    }
}

// ── Request / Response types ────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateBranchRequest {
    name: BranchName,
    #[serde(default = "default_branch")]
    from: BranchName,
}

impl CreateBranchRequest {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn from(&self) -> &str {
        self.from.as_str()
    }
}

fn default_branch() -> BranchName {
    BranchName::main()
}

#[derive(Serialize, utoipa::ToSchema)]
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

#[derive(Serialize, utoipa::ToSchema)]
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct MergeBranchRequest {
    #[serde(default = "default_branch")]
    into: BranchName,
    #[serde(default = "default_squash")]
    squash: bool,
}

fn default_squash() -> bool {
    true
}

impl MergeBranchRequest {
    pub fn target_branch(&self) -> &str {
        self.into.as_str()
    }

    pub fn squash(&self) -> bool {
        self.squash
    }
}

#[derive(Serialize, utoipa::ToSchema)]
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

#[utoipa::path(
    post,
    path = "/v1/branches",
    tag = "branches",
    request_body = CreateBranchRequest,
    responses(
        (status = 201, description = "Branch created", body = CreateBranchResponse),
        (status = 400, description = "Invalid branch name"),
    ),
)]
async fn create_branch(
    RequireBranchCreate(auth): RequireBranchCreate,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
    Json(req): Json<CreateBranchRequest>,
) -> Result<(StatusCode, Json<CreateBranchResponse>), AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let name = req.name.clone();
    let from = req.from.clone();
    let info = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = crate::store::entity_store::EntityStore::open(
            layout,
            workspace_id,
            entity_id,
            valkey,
            s3,
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

        store
            .create_branch(name.as_str(), from.as_str())
            .map_err(AppError::from)
    })
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateBranchResponse {
            branch: info.name,
            base_commit: info.head_oid,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/v1/branches",
    tag = "branches",
    responses(
        (status = 200, description = "List of branches", body = Vec<BranchListEntry>),
    ),
)]
async fn list_branches(
    RequireBranchCreate(auth): RequireBranchCreate,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<BranchListEntry>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let branches = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = crate::store::entity_store::EntityStore::open(
            layout,
            workspace_id,
            entity_id,
            valkey,
            s3,
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

        store.list_branches().map_err(AppError::from)
    })
    .await?;

    let entries = branches
        .into_iter()
        .map(|b| BranchListEntry {
            name: b.name,
            head_oid: b.head_oid,
        })
        .collect();

    Ok(Json(entries))
}

#[utoipa::path(
    post,
    path = "/v1/branches/{name}/merge",
    tag = "branches",
    params(
        ("name" = String, Path, description = "Branch name to merge"),
    ),
    request_body = MergeBranchRequest,
    responses(
        (status = 200, description = "Merge result", body = MergeBranchResponse),
        (status = 400, description = "Invalid branch name"),
    ),
)]
async fn merge_branch(
    RequireBranchMerge(auth): RequireBranchMerge,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<super::EntityIdQuery>,
    Json(req): Json<MergeBranchRequest>,
) -> Result<Json<MergeBranchResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let source = BranchName::new(name).map_err(|e| AppError::BadRequest(e.to_string()))?;

    let squash = req.squash;
    let target = req.into.clone();
    let result = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = crate::store::entity_store::EntityStore::open(
            layout,
            workspace_id,
            entity_id,
            valkey,
            s3,
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

        store
            .merge_branch(source.as_str(), target.as_str(), squash)
            .map_err(AppError::from)
    })
    .await?;

    let response = match result {
        MergeOutcome::FastForward { oid } => MergeBranchResponse {
            merged: true,
            strategy: "fast_forward".to_owned(),
            commit: Some(oid),
        },
        MergeOutcome::AlreadyUpToDate => MergeBranchResponse {
            merged: true,
            strategy: "already_up_to_date".to_owned(),
            commit: None,
        },
        MergeOutcome::ThreeWayMerge { oid } => MergeBranchResponse {
            merged: true,
            strategy: "three_way".to_owned(),
            commit: Some(oid),
        },
        MergeOutcome::Squash { oid } => MergeBranchResponse {
            merged: true,
            strategy: "squash".to_owned(),
            commit: Some(oid),
        },
    };

    Ok(Json(response))
}

#[utoipa::path(
    delete,
    path = "/v1/branches/{name}",
    tag = "branches",
    params(
        ("name" = String, Path, description = "Branch name to delete"),
    ),
    responses(
        (status = 204, description = "Branch deleted"),
        (status = 400, description = "Invalid branch name"),
    ),
)]
async fn delete_branch_handler(
    RequireBranchDelete(auth): RequireBranchDelete,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<StatusCode, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    let branch = BranchName::new(name).map_err(|e| AppError::BadRequest(e.to_string()))?;

    super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let store = crate::store::entity_store::EntityStore::open(
            layout,
            workspace_id,
            entity_id,
            valkey,
            s3,
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

        store
            .delete_branch(branch.as_str())
            .map_err(AppError::from)
    })
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/v1/branches/{name}/prune",
    tag = "branches",
    params(
        ("name" = String, Path, description = "Branch name to prune"),
    ),
    responses(
        (status = 204, description = "Branch pruned"),
        (status = 400, description = "Invalid branch name"),
    ),
)]
/// Prune a branch (POST alternative to DELETE for clients that don't support DELETE).
async fn prune_branch(
    RequireBranchDelete(auth): RequireBranchDelete,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<StatusCode, AppError> {
    delete_branch_handler(
        RequireBranchDelete(auth),
        State(state),
        Path(name),
        Query(query),
    )
    .await
}

// ── Router ──────────────────────────────────────────────────────────────

pub fn branch_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/branches", post(create_branch).get(list_branches))
        .route("/v1/branches/{name}/merge", post(merge_branch))
        .route("/v1/branches/{name}/prune", post(prune_branch))
        .route("/v1/branches/{name}", delete(delete_branch_handler))
}

// ── OpenAPI ─────────────────────────────────────────────────────────────

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create_branch,
        list_branches,
        merge_branch,
        delete_branch_handler,
        prune_branch,
    ),
    components(schemas(
        CreateBranchRequest,
        CreateBranchResponse,
        BranchListEntry,
        MergeBranchRequest,
        MergeBranchResponse,
    ))
)]
pub struct BranchesApi;

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Request};

    #[tokio::test]
    async fn branch_target_defaults_to_main() {
        let mut parts = request_parts_without_header();
        let target = BranchTarget::from_request_parts(&mut parts, &())
            .await
            .unwrap();
        assert_eq!(target.name(), "main");
    }

    #[tokio::test]
    async fn branch_target_reads_header() {
        let mut parts = request_parts_with_header("feature/equity-grants");
        let target = BranchTarget::from_request_parts(&mut parts, &())
            .await
            .unwrap();
        assert_eq!(target.name(), "feature/equity-grants");
    }

    #[tokio::test]
    async fn branch_target_into_inner() {
        let mut parts = request_parts_with_header("dev");
        let target = BranchTarget::from_request_parts(&mut parts, &())
            .await
            .unwrap();
        assert_eq!(target.into_inner(), "dev");
    }

    #[tokio::test]
    async fn branch_target_rejects_invalid() {
        let mut parts = request_parts_with_header("my branch");
        let result = BranchTarget::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
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
