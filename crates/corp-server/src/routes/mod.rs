//! Route assembly for the corporate governance API server.
//!
//! [`router`] builds the complete Axum router, wires all domain route modules
//! together, applies middleware layers, and binds the shared [`AppState`].
//!
//! # Layout
//!
//! - `GET /health`   — liveness check, no auth required.
//! - `/v1/*`         — all authenticated domain API routes.

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub mod admin;
pub mod agents;
pub mod contacts;
pub mod equity;
pub mod execution;
pub mod formation;
pub mod governance;
pub mod health;
pub mod manifest;
pub mod openapi;
pub mod services;
pub mod treasury;
pub mod work_items;

/// Build and return the complete application router.
///
/// All routes are bound to `state` via `.with_state(state)`.
/// - [`TraceLayer`] emits structured `tracing` events for every request /
///   response cycle.
/// - [`CorsLayer::permissive`] allows cross-origin requests (suitable for
///   development; restrict origins for production).
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(openapi_routes())
        .nest("/v1", api_routes())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

/// Assemble all authenticated `/v1` API routes.
fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(formation::routes())
        .merge(equity::routes())
        .merge(governance::routes())
        .merge(treasury::routes())
        .merge(execution::routes())
        .merge(contacts::routes())
        .merge(agents::routes())
        .merge(work_items::routes())
        .merge(services::routes())
        .merge(admin::routes())
        .merge(manifest::routes())
}

/// The OpenAPI spec is served at `/openapi.json` (no auth required).
///
/// It is mounted directly on the top-level router (not under `/v1`) so it
/// is accessible without credentials and tools like Swagger UI can reach it.
pub fn openapi_routes() -> Router<AppState> {
    openapi::routes()
}
