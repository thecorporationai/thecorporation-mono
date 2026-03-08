//! End-to-end governance correctness checks through HTTP routes.

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
    };

    Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(json!({"status": "ok"})) }),
        )
        .merge(api_rs::routes::formation::formation_routes())
        .merge(api_rs::routes::execution::execution_routes())
        .merge(api_rs::routes::governance::governance_routes())
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

async fn create_entity(app: &Router) -> (String, String) {
    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);
    let (status, body) = post_json(
        app,
        "/v1/formations",
        json!({
            "entity_type": "corporation",
            "legal_name": "Correctness Corp",
            "jurisdiction": "Delaware",
            "members": [
                {
                    "name": "Alice Founder",
                    "investor_type": "natural_person",
                    "email": "alice@test.com",
                    "ownership_pct": 60.0,
                    "share_count": 6000,
                    "role": "director"
                },
                {
                    "name": "Bob Cofounder",
                    "investor_type": "natural_person",
                    "email": "bob@test.com",
                    "ownership_pct": 40.0,
                    "share_count": 4000,
                    "role": "member"
                }
            ],
            "authorized_shares": 10000000,
            "par_value": "0.0001",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create entity failed: {body}");
    (
        body["entity_id"].as_str().expect("entity_id").to_owned(),
        token,
    )
}

#[tokio::test]
async fn governance_end_to_end_fail_closed_on_metadata_decode_issue() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let (entity_id, token) = create_entity(&app).await;

    let (status, body) = post_json(
        &app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": "execute_standard_form_agreement",
            "description": "Attempt agreement with malformed metadata",
            "metadata": {
                "laneId": "lane-3.1-nda",
                "templateApproved": "true"
            }
        }),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "create intent failed: {body}");
    let decision = &body["policy_decision"];
    assert_ne!(
        decision["tier"],
        json!("tier_1"),
        "malformed metadata must not remain tier_1: {body}"
    );
    let clause_refs = decision["clause_refs"]
        .as_array()
        .expect("clause_refs array");
    assert!(
        clause_refs
            .iter()
            .any(|v| v == "rule.metadata.decode_failure"),
        "expected decode-failure clause ref in decision: {body}"
    );
}
