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
use axum::http::{header, Method};
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
pub mod next_steps;
pub mod openapi;
pub mod services;
pub mod treasury;
pub mod validation;
pub mod work_items;

/// Build and return the complete application router.
///
/// All routes are bound to `state` via `.with_state(state)`.
/// - [`TraceLayer`] emits structured `tracing` events for every request /
///   response cycle.
/// - CORS origins are read from `CORP_CORS_ORIGINS` (comma-separated).
///   Set to `*` for development. Defaults to production domains if unset.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(openapi_routes())
        .nest("/v1", api_routes())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
}

/// Build a CORS layer from the `CORP_CORS_ORIGINS` environment variable.
///
/// - Unset or empty: defaults to production origins.
/// - `*`: permissive (development only).
/// - Otherwise: comma-separated list of allowed origins.
pub fn cors_layer() -> CorsLayer {
    let origins_str = std::env::var("CORP_CORS_ORIGINS").unwrap_or_default();

    if origins_str.trim() == "*" {
        return CorsLayer::permissive();
    }

    let origins: Vec<axum::http::HeaderValue> = if origins_str.is_empty() {
        vec![
            "https://www.thecorporation.ai".parse().unwrap(),
            "https://humans.thecorporation.ai".parse().unwrap(),
            "https://docs.thecorporation.ai".parse().unwrap(),
            "https://api.thecorporation.ai".parse().unwrap(),
        ]
    } else {
        origins_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600))
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
        .merge(next_steps::routes())
        .merge(manifest::routes())
}

/// The OpenAPI spec is served at `/openapi.json` (no auth required).
///
/// It is mounted directly on the top-level router (not under `/v1`) so it
/// is accessible without credentials and tools like Swagger UI can reach it.
pub fn openapi_routes() -> Router<AppState> {
    openapi::routes()
}
