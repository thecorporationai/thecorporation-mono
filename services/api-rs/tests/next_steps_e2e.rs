//! Integration tests for next-steps endpoints.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

use api_rs::domain::auth::claims::{Claims, PrincipalType, encode_token};
use api_rs::domain::auth::scopes::Scope;
use api_rs::domain::ids::WorkspaceId;

const TEST_SECRET: &[u8] = b"test-secret-for-integration-tests";

fn make_token(ws_id: WorkspaceId) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims::new(
        ws_id,
        None,
        None,
        None,
        PrincipalType::User,
        vec![Scope::All],
        now,
        now + 3600,
    );
    encode_token(&claims, TEST_SECRET).expect("encode token")
}

fn build_app(tmp: &TempDir) -> Router {
    unsafe { std::env::set_var("JWT_SECRET", "test-secret-for-integration-tests") };
    unsafe { std::env::set_var("TERMINAL_AUTH_SECRET", "chat-ws-test-secret") };
    unsafe { std::env::set_var("INTERNAL_WORKER_TOKEN", "internal-worker-test-token") };

    let layout = Arc::new(api_rs::store::RepoLayout::new(tmp.path().to_path_buf()));
    let state = api_rs::routes::AppState {
        layout,
        jwt_secret: Arc::from(b"test-secret-for-integration-tests".as_slice()),
        commit_signer: None,
        redis: None,
        secrets_fernet: None,
        max_queue_depth: 1000,
        http_client: reqwest::Client::new(),
        llm_upstream_url: "http://localhost:0".to_owned(),
        model_pricing: HashMap::new(),
        creation_rate_limiter: Arc::new(api_rs::routes::CreationRateLimiter::default()),
        storage_backend: api_rs::store::StorageBackendKind::Git,
        valkey_client: None,
        ssh_key_index: Arc::new(api_rs::domain::auth::ssh_key::SshKeyIndex::empty()),
        s3_backend: None,
        startup_time: std::time::Instant::now(),
    };

    Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(json!({"status": "ok"})) }),
        )
        .merge(api_rs::routes::formation::formation_routes())
        .merge(api_rs::routes::next_steps::next_steps_routes())
        .with_state(state)
}

async fn post_json(app: &Router, path: &str, body: Value, token: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::to_vec(&body).expect("serialize body"),
        ))
        .expect("build request");
    let response = app.clone().oneshot(req).await.expect("oneshot");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn get_json(app: &Router, path: &str, token: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::GET)
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("build request");
    let response = app.clone().oneshot(req).await.expect("oneshot");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

/// Test 1: workspace with no entities returns an empty next-steps response.
#[tokio::test]
async fn workspace_next_steps_empty_workspace() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);

    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);

    let (status, body) = get_json(
        &app,
        &format!("/v1/workspaces/{ws_id}/next-steps"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "workspace next-steps failed: {body}");
    assert!(body["top"].is_null(), "expected top to be null for empty workspace, got: {}", body["top"]);
    assert_eq!(
        body["backlog"].as_array().expect("backlog array").len(),
        0,
        "expected empty backlog, got: {}",
        body["backlog"]
    );
    let summary = &body["summary"];
    assert_eq!(summary["critical"], 0, "critical should be 0");
    assert_eq!(summary["high"], 0, "high should be 0");
    assert_eq!(summary["medium"], 0, "medium should be 0");
    assert_eq!(summary["low"], 0, "low should be 0");
}

/// Test 2: entity in pending formation status surfaces a critical formation step.
#[tokio::test]
async fn entity_next_steps_pending_formation() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);

    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);

    // Create a pending entity via the formation endpoint.
    let (status, body) = post_json(
        &app,
        "/v1/formations",
        json!({
            "entity_type": "corporation",
            "legal_name": "Next Steps Test Corp",
            "jurisdiction": "Delaware",
            "registered_agent_name": "Delaware Registered Agent Co.",
            "registered_agent_address": "1209 Orange St, Wilmington, DE 19801",
            "company_address": {
                "street": "2261 Market St",
                "city": "San Francisco",
                "state": "CA",
                "zip": "94114"
            },
            "members": [
                {
                    "name": "Alice Founder",
                    "investor_type": "natural_person",
                    "email": "alice@nextsteps.test",
                    "ownership_pct": 100.0,
                    "share_count": 10000,
                    "role": "director",
                    "officer_title": "ceo",
                    "is_incorporator": true,
                    "address": {
                        "street": "2261 Market St",
                        "city": "San Francisco",
                        "state": "CA",
                        "zip": "94114"
                    }
                }
            ],
            "authorized_shares": 10000000,
            "par_value": "0.0001"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create formation failed: {body}");
    let entity_id = body["entity_id"].as_str().expect("entity_id").to_owned();

    // The newly created entity starts as Pending — query next steps.
    let (status, ns) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/next-steps"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "entity next-steps failed: {ns}");

    let top = &ns["top"];
    assert!(!top.is_null(), "expected top to be non-null for new entity, got: {ns}");

    // One-shot formation creates entity in DocumentsGenerated state.
    // Top recommendation should be signing documents (not activate, since docs aren't signed yet).
    let category = top["category"].as_str().unwrap_or("");
    let command = top["command"].as_str().unwrap_or("");
    assert!(
        category == "documents" || category == "formation",
        "expected top.category to be \"documents\" or \"formation\", got: {category}"
    );
    assert!(
        command.contains("signing-link") || command.contains("finalize") || command.contains("activate"),
        "expected actionable command, got: {command}"
    );
}
