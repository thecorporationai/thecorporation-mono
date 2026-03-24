//! CLI command manifest endpoint.
//!
//! `GET /cli/manifest` returns the full command registry as JSON so web
//! terminals and other consumers can discover available commands, their API
//! paths, and form fields at runtime.

use axum::routing::get;
use axum::{Json, Router};
use corp_core::command_registry::{build_manifest, CommandManifest};

use crate::state::AppState;

/// Build the manifest sub-router.
pub fn routes() -> Router<AppState> {
    Router::new().route("/cli/manifest", get(manifest))
}

/// `GET /cli/manifest` — return the complete command manifest.
async fn manifest() -> Json<CommandManifest> {
    Json(build_manifest())
}
