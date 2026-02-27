use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod domain;
#[allow(dead_code)]
mod error;
#[allow(dead_code)]
mod git;
mod openapi;
#[allow(dead_code)]
mod routes;
#[allow(dead_code)]
mod store;

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn openapi_json() -> Json<Value> {
    Json(openapi::openapi_spec())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let layout = Arc::new(store::RepoLayout::new(PathBuf::from("./data/repos")));

    let jwt_secret: Arc<[u8]> = match std::env::var("JWT_SECRET") {
        Ok(s) if s.len() >= 32 => Arc::from(s.into_bytes()),
        Ok(s) if !s.is_empty() => {
            tracing::warn!("JWT_SECRET is shorter than 32 bytes — consider using a stronger secret");
            Arc::from(s.into_bytes())
        }
        _ => {
            if cfg!(debug_assertions) {
                tracing::warn!("JWT_SECRET not set — using insecure dev-only secret");
                Arc::from(b"dev-secret-do-not-use-in-production".as_slice())
            } else {
                panic!("JWT_SECRET environment variable must be set in release builds");
            }
        }
    };

    let state = routes::AppState { layout, jwt_secret };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/openapi.json", get(openapi_json))
        .merge(routes::formation::formation_routes())
        .merge(routes::equity::equity_routes())
        .merge(routes::governance::governance_routes())
        .merge(routes::treasury::treasury_routes())
        .merge(routes::contacts::contacts_routes())
        .merge(routes::execution::execution_routes())
        .merge(routes::branches::branch_routes())
        .merge(routes::projection::projection_routes())
        .merge(routes::compliance::compliance_routes())
        .merge(routes::auth::auth_routes())
        .merge(routes::agents::agent_routes())
        .merge(routes::billing::billing_routes())
        .merge(routes::admin::admin_routes())
        .merge(routes::webhooks::webhook_routes())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
