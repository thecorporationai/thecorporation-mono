#![allow(dead_code)]
#![allow(clippy::inconsistent_digit_grouping)]

use axum::http::HeaderValue;
use axum::response::Response;
use axum::{Json, Router, middleware, routing::get};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

mod auth;
mod config;
mod domain;
mod error;
mod git;
mod openapi;
mod routes;
mod store;
mod validate;

#[derive(Parser)]
#[command(name = "api-rs", version, about = "Corporate API server")]
struct Cli {
    /// Skip data validation on server boot.
    #[arg(long)]
    skip_validation: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Validate all stored data by deserializing every file and reporting errors.
    Validate {
        /// Path to the data/repos directory.
        #[arg(long, default_value = "./data/repos")]
        data_dir: PathBuf,
    },
    /// Write the OpenAPI specification JSON to stdout.
    DumpOpenApi,
    /// Generate governance markdown docs bundle from Rust.
    GenerateGovernanceDocs {
        /// Governance profile to generate.
        #[arg(long, value_enum)]
        entity_type: GovernanceEntityTypeArg,
        /// Output directory for generated docs and manifest.
        #[arg(long)]
        out_dir: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GovernanceEntityTypeArg {
    Corporation,
    Llc,
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi::openapi_spec())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Validate { data_dir }) => {
            let code = validate::run(data_dir);
            std::process::exit(code);
        }
        Some(Command::DumpOpenApi) => {
            let spec = openapi::openapi_spec();
            println!("{}", serde_json::to_string_pretty(&spec).unwrap());
        }
        Some(Command::GenerateGovernanceDocs {
            entity_type,
            out_dir,
        }) => {
            let profile = match entity_type {
                GovernanceEntityTypeArg::Corporation => {
                    domain::governance::doc_generator::GovernanceDocEntityType::Corporation
                }
                GovernanceEntityTypeArg::Llc => {
                    domain::governance::doc_generator::GovernanceDocEntityType::Llc
                }
            };

            match domain::governance::doc_generator::generate_bundle(profile, &out_dir) {
                Ok(manifest) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&manifest)
                            .unwrap_or_else(|_| "{\"status\":\"ok\"}".to_owned())
                    );
                }
                Err(err) => {
                    eprintln!("governance docs generation failed: {err:#}");
                    std::process::exit(1);
                }
            }
        }
        None => {
            run_server(cli.skip_validation).await;
        }
    }
}

async fn run_server(skip_validation: bool) {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let data_dir = PathBuf::from("./data/repos");

    if skip_validation {
        tracing::warn!("--skip-validation set — skipping startup data validation");
    } else if data_dir.exists() {
        tracing::info!("validating stored data before startup…");
        let code = validate::run(data_dir.clone());
        if code != 0 {
            panic!(
                "data validation failed — fix the errors above or pass --skip-validation to bypass"
            );
        }
        tracing::info!("data validation passed");
    }

    let layout = Arc::new(store::RepoLayout::new(data_dir));

    let jwt_secret: Arc<[u8]> = match std::env::var("JWT_SECRET") {
        Ok(s) if s.len() >= 32 => Arc::from(s.into_bytes()),
        Ok(s) if !s.is_empty() => {
            tracing::warn!(
                "JWT_SECRET is shorter than 32 bytes — consider using a stronger secret"
            );
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

    // Fernet key for encrypting secrets at rest in workspace repos
    let secrets_fernet = match std::env::var("SECRETS_MASTER_KEY") {
        Ok(key) if !key.is_empty() => match fernet::Fernet::new(&key) {
            Some(f) => {
                tracing::info!("secrets encryption enabled");
                Some(Arc::new(f))
            }
            None => {
                panic!("SECRETS_MASTER_KEY is not a valid Fernet key");
            }
        },
        _ => {
            if cfg!(debug_assertions) {
                tracing::warn!(
                    "SECRETS_MASTER_KEY not set — secrets encryption disabled (dev only)"
                );
                None
            } else {
                panic!("SECRETS_MASTER_KEY must be set in release builds");
            }
        }
    };

    let max_queue_depth: u64 = std::env::var("MAX_QUEUE_DEPTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);

    // HTTP client for LLM proxy (5-min timeout for long LLM calls)
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .expect("failed to build HTTP client");

    let llm_upstream_url = std::env::var("LLM_UPSTREAM_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_owned());

    let model_pricing = {
        let mut m = HashMap::new();
        m.insert(
            "anthropic/claude-sonnet-4-6".to_owned(),
            routes::ModelPricing {
                input: 300,
                output: 1500,
            },
        );
        m.insert(
            "anthropic/claude-haiku-4-5".to_owned(),
            routes::ModelPricing {
                input: 80,
                output: 400,
            },
        );
        m.insert(
            "openai/gpt-4o".to_owned(),
            routes::ModelPricing {
                input: 250,
                output: 1000,
            },
        );
        m.insert(
            "openai/gpt-4o-mini".to_owned(),
            routes::ModelPricing {
                input: 15,
                output: 60,
            },
        );
        m
    };

    let internal_worker_token = std::env::var("INTERNAL_WORKER_TOKEN").unwrap_or_default();
    if internal_worker_token.trim().is_empty() {
        panic!("INTERNAL_WORKER_TOKEN must be set");
    }

    let state = routes::AppState {
        layout,
        jwt_secret,
        commit_signer,
        redis,
        secrets_fernet,
        max_queue_depth,
        http_client,
        llm_upstream_url,
        model_pricing,
        creation_rate_limiter: Arc::new(routes::CreationRateLimiter::default()),
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
        .merge(routes::llm_proxy::llm_proxy_routes())
        .merge(routes::references::references_routes())
        .merge(routes::secret_proxies::secret_proxy_routes())
        .merge(routes::work_items::work_items_routes())
        .merge(routes::services::services_routes())
        .merge(routes::admin::admin_routes())
        .merge(routes::admin::admin_billing_routes())
        .with_state(state)
        .layer(middleware::map_response(security_headers));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| {
            eprintln!("ERROR: cannot bind to {addr}: {e}");
            std::process::exit(1);
        })
        .unwrap();
    axum::serve(listener, app)
        .await
        .map_err(|e| {
            eprintln!("ERROR: server error: {e}");
            std::process::exit(1);
        })
        .unwrap();
}

async fn security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert("x-xss-protection", HeaderValue::from_static("0"));
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}
