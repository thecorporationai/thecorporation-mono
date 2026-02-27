pub mod admin;
pub mod agents;
pub mod auth;
pub mod billing;
pub mod branches;
pub mod compliance;
pub mod contacts;
pub mod equity;
pub mod execution;
pub mod formation;
pub mod governance;
pub mod projection;
pub mod treasury;
pub mod webhooks;

use std::sync::Arc;

use crate::domain::ids::{EntityId, WorkspaceId};
use crate::store::RepoLayout;

/// Query params requiring both workspace and entity identification.
#[derive(serde::Deserialize)]
pub struct WorkspaceEntityQuery {
    pub workspace_id: WorkspaceId,
    pub entity_id: EntityId,
}

/// Shared application state, passed to all route handlers via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub layout: Arc<RepoLayout>,
    /// Shared secret used to sign and verify JWTs.
    /// Loaded once from `JWT_SECRET` env var at startup.
    pub jwt_secret: Arc<[u8]>,
}
