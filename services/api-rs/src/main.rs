#![allow(dead_code)]
#![allow(clippy::inconsistent_digit_grouping)]

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

mod config;
mod domain;
mod error;
mod git;
mod openapi;
mod routes;
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

    let commit_signer = match std::env::var("COMMIT_SIGNING_KEY") {
        Ok(pem) if !pem.is_empty() => {
            // Replace literal \n with actual newlines (env vars often encode PEM this way)
            let pem = pem.replace("\\n", "\n");
            match git::signing::CommitSigner::from_pem(&pem) {
                Ok(signer) => {
                    tracing::info!(
                        fingerprint = %signer.public_key_fingerprint(),
                        "commit signing enabled"
                    );
                    Some(Arc::new(signer))
                }
                Err(e) => {
                    tracing::error!("failed to load COMMIT_SIGNING_KEY: {e}");
                    panic!("COMMIT_SIGNING_KEY is set but invalid: {e}");
                }
            }
        }
        _ => {
            tracing::info!("COMMIT_SIGNING_KEY not set — commits will be unsigned");
            None
        }
    };

    // Optional Redis pool for agent execution queue
    let redis = match std::env::var("REDIS_URL") {
        Ok(url) if !url.is_empty() => {
            let cfg = deadpool_redis::Config::from_url(&url);
            match cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1)) {
                Ok(pool) => {
                    tracing::info!("redis pool connected");
                    Some(pool)
                }
                Err(e) => {
                    tracing::warn!("failed to create redis pool: {e} — agent dispatch disabled");
                    None
                }
            }
        }
        _ => {
            tracing::info!("REDIS_URL not set — agent dispatch disabled");
            None
        }
    };

    // Optional Fernet key for encrypting secrets at rest in workspace repos
    let secrets_fernet = match std::env::var("SECRETS_MASTER_KEY") {
        Ok(key) if !key.is_empty() => match fernet::Fernet::new(&key) {
            Some(f) => {
                tracing::info!("secrets encryption enabled");
                Some(Arc::new(f))
            }
            None => {
                tracing::error!("SECRETS_MASTER_KEY is not a valid Fernet key — secrets encryption disabled");
                None
            }
        },
        _ => {
            tracing::info!("SECRETS_MASTER_KEY not set — secrets encryption disabled");
            None
        }
    };

    let state = routes::AppState {
        layout,
        jwt_secret,
        commit_signer,
        redis,
        secrets_fernet,
    };

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
        .merge(routes::compliance::compliance_routes())
        .merge(routes::auth::auth_routes())
        .merge(routes::agents::agent_routes())
        .merge(routes::agent_executions::execution_routes())
        .merge(routes::secrets_proxy::secrets_routes())
        .merge(routes::secret_proxies::secret_proxy_routes())
        .merge(routes::billing::billing_routes())
        .merge(routes::admin::admin_routes())
        .merge(routes::webhooks::webhook_routes())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
