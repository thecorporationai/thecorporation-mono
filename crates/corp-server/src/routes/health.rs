//! Health check endpoint.
//!
//! `GET /health` is intentionally unauthenticated so load-balancers and
//! container orchestrators can use it as a liveness probe without credentials.

use axum::routing::get;
use axum::{Json, Router};

use crate::state::AppState;

/// Build the health sub-router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/status", get(status_check))
}

/// `GET /health` — server liveness probe.
///
/// Returns `200 OK` with `{ "status": "ok" }` when the server process is alive.
/// Does not check downstream dependencies (database, storage) — use a separate
/// readiness probe for that.
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// `GET /v1/status` — API status summary (authenticated alias of `/health`).
///
/// Returns the server version and a brief status message.  Used by the CLI
/// `corp status` command.
async fn status_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "service": "corp-server",
    }))
}
