//! Integration tests exercising the full HTTP API lifecycle.
//!
//! Each test constructs the Axum router with a temporary data directory,
//! then sends requests directly via `tower::ServiceExt::oneshot` — no
//! actual TCP listener is needed.

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt; // oneshot

use api_rs::domain::auth::claims::{Claims, PrincipalType, encode_token};
use api_rs::domain::auth::scopes::Scope;
use api_rs::domain::equity::conversion_execution::ConversionExecution;
use api_rs::domain::equity::round::{EquityRound, EquityRoundStatus};
use api_rs::domain::execution::intent::Intent;
use api_rs::domain::execution::types::IntentStatus;
use api_rs::domain::ids::{ConversionExecutionId, EntityId, EquityRoundId, IntentId, WorkspaceId};
use api_rs::store::entity_store::EntityStore;

// ── Helpers ──────────────────────────────────────────────────────────────

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
    // Ensure JWT_SECRET is set for the auth middleware
    unsafe { std::env::set_var("JWT_SECRET", "test-secret-for-integration-tests") };
    unsafe { std::env::set_var("TERMINAL_AUTH_SECRET", "chat-ws-test-secret") };

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
        model_pricing: std::collections::HashMap::new(),
    };

    Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(json!({"status": "ok"})) }),
        )
        .route(
            "/v1/openapi.json",
            get(|| async { axum::Json(api_rs::openapi::openapi_spec()) }),
        )
        .merge(api_rs::routes::formation::formation_routes())
        .merge(api_rs::routes::equity::equity_routes())
        .merge(api_rs::routes::governance::governance_routes())
        .merge(api_rs::routes::treasury::treasury_routes())
        .merge(api_rs::routes::contacts::contacts_routes())
        .merge(api_rs::routes::execution::execution_routes())
        .merge(api_rs::routes::branches::branch_routes())
        .merge(api_rs::routes::compliance::compliance_routes())
        .merge(api_rs::routes::auth::auth_routes())
        .merge(api_rs::routes::agents::agent_routes())
        .merge(api_rs::routes::billing::billing_routes())
        .merge(api_rs::routes::admin::admin_routes())
        .merge(api_rs::routes::webhooks::webhook_routes())
        .with_state(state)
}

async fn post_json(app: &Router, path: &str, body: Value, token: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn post_json_no_auth(app: &Router, path: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn get_json(app: &Router, path: &str, token: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::GET)
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn delete_req(app: &Router, path: &str, token: &str) -> StatusCode {
    let req = Request::builder()
        .method(Method::DELETE)
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    response.status()
}

/// Helper: create an entity and return (workspace_id, entity_id, token).
async fn create_entity(app: &Router) -> (WorkspaceId, String, String) {
    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);
    let (status, body) = post_json(
        app,
        "/v1/formations",
        json!({
            "entity_type": "corporation",
            "legal_name": "Test Corp",
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
    assert_eq!(status, StatusCode::OK, "create_formation failed: {body}");
    let entity_id = body["entity_id"].as_str().unwrap().to_owned();
    (ws_id, entity_id, token)
}

async fn create_round_with_terms(app: &Router, entity_id: &str, token: &str) -> (String, String) {
    let (status, legal_entity) = post_json(
        app,
        "/v1/equity/entities",
        json!({
            "entity_id": entity_id,
            "linked_entity_id": entity_id,
            "name": "Validation Corp",
            "role": "operating",
        }),
        token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create legal entity failed: {legal_entity}"
    );
    let issuer_legal_entity_id = legal_entity["legal_entity_id"].as_str().unwrap();

    let (status, target_instrument) = post_json(
        app,
        "/v1/equity/instruments",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "symbol": "SERIES_A",
            "kind": "preferred_equity",
            "terms": {},
        }),
        token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create target instrument failed: {target_instrument}"
    );
    let target_instrument_id = target_instrument["instrument_id"].as_str().unwrap();

    let (status, round) = post_json(
        app,
        "/v1/equity/rounds",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "name": "Series A",
            "round_price_cents": 100_i64,
            "target_raise_cents": 10000000_i64,
            "conversion_target_instrument_id": target_instrument_id,
            "metadata": {},
        }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create round failed: {round}");
    let round_id = round["round_id"].as_str().unwrap();

    let (status, apply_terms) = post_json(
        app,
        &format!("/v1/equity/rounds/{round_id}/apply-terms"),
        json!({
            "entity_id": entity_id,
            "anti_dilution_method": "none",
            "conversion_precedence": ["safe"],
            "protective_provisions": {},
        }),
        token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "apply round terms failed: {apply_terms}"
    );

    (issuer_legal_entity_id.to_owned(), round_id.to_owned())
}

async fn create_resolution_for_body(
    app: &Router,
    entity_id: &str,
    token: &str,
    body_type: &str,
    vote_values: &[&str],
) -> (String, String) {
    let e_query = format!("entity_id={entity_id}");

    let (status, body) = post_json(
        app,
        "/v1/governance-bodies",
        json!({
            "entity_id": entity_id,
            "body_type": body_type,
            "name": format!("{} body", body_type),
            "quorum_rule": "majority",
            "voting_method": "per_capita",
        }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create governance body: {body}");
    let body_id = body["body_id"].as_str().unwrap();

    let mut contact_ids = Vec::new();
    let mut seat_ids = Vec::new();
    for (idx, _) in vote_values.iter().enumerate() {
        let (status, contact) = post_json(
            app,
            "/v1/contacts",
            json!({
                "entity_id": entity_id,
                "contact_type": "individual",
                "name": format!("Director {idx}"),
                "email": format!("director{idx}@example.com"),
                "category": "board_member",
            }),
            token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "create contact {idx}: {contact}");
        let contact_id = contact["contact_id"].as_str().unwrap().to_owned();
        contact_ids.push(contact_id.clone());

        let role = if idx == 0 { "chair" } else { "member" };
        let (status, seat) = post_json(
            app,
            &format!("/v1/governance-bodies/{body_id}/seats?{e_query}"),
            json!({
                "holder_id": contact_id,
                "role": role,
                "voting_power": 1,
            }),
            token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "create seat {idx}: {seat}");
        seat_ids.push(seat["seat_id"].as_str().unwrap().to_owned());
    }

    let meeting_type = if body_type == "board_of_directors" {
        "board_meeting"
    } else {
        "member_meeting"
    };
    let (status, meeting) = post_json(
        app,
        "/v1/meetings",
        json!({
            "entity_id": entity_id,
            "body_id": body_id,
            "meeting_type": meeting_type,
            "title": "Approval Meeting",
            "scheduled_date": "2026-03-15",
            "location": "Virtual",
            "notice_days": 0,
            "agenda_item_titles": ["Approve round"],
        }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "schedule meeting: {meeting}");
    let meeting_id = meeting["meeting_id"].as_str().unwrap();

    let (status, notice) = post_json(
        app,
        &format!("/v1/meetings/{meeting_id}/notice?{e_query}"),
        json!({}),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "notice meeting: {notice}");

    let (status, convene) = post_json(
        app,
        &format!("/v1/meetings/{meeting_id}/convene?{e_query}"),
        json!({
            "present_seat_ids": seat_ids,
        }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "convene meeting: {convene}");

    let (status, agenda_items) = get_json(
        app,
        &format!("/v1/meetings/{meeting_id}/agenda-items?{e_query}"),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list agenda items: {agenda_items}");
    let agenda_item_id = agenda_items.as_array().unwrap()[0]["agenda_item_id"]
        .as_str()
        .unwrap();

    for (idx, vote_value) in vote_values.iter().enumerate() {
        let (status, vote) = post_json(
            app,
            &format!("/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/vote?{e_query}"),
            json!({
                "voter_id": contact_ids[idx],
                "vote_value": vote_value,
            }),
            token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "cast vote {idx}: {vote}");
    }

    let (status, resolution) = post_json(
        app,
        &format!("/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/resolution?{e_query}"),
        json!({
            "resolution_text": "Resolved: approve financing round.",
        }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "compute resolution: {resolution}");

    (
        meeting_id.to_owned(),
        resolution["resolution_id"].as_str().unwrap().to_owned(),
    )
}

async fn create_authorized_round_intent(
    app: &Router,
    entity_id: &str,
    token: &str,
    intent_type: &str,
    metadata_round_id: &str,
) -> String {
    let (status, intent) = post_json(
        app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": intent_type,
            "authority_tier": "tier_2",
            "description": format!("{} test intent", intent_type),
            "metadata": {"round_id": metadata_round_id},
        }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create intent: {intent}");
    let intent_id = intent["intent_id"].as_str().unwrap().to_owned();
    let e_query = format!("entity_id={entity_id}");

    let (status, evaluate) = post_json(
        app,
        &format!("/v1/intents/{intent_id}/evaluate?{e_query}"),
        json!({}),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "evaluate intent: {evaluate}");

    let (status, authorize) = post_json(
        app,
        &format!("/v1/intents/{intent_id}/authorize?{e_query}"),
        json!({}),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "authorize intent: {authorize}");

    intent_id
}

// ── 1. Health check ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_health() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let token = make_token(WorkspaceId::new());
    let (status, body) = get_json(&app, "/health", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_public_chat_session_mints_ws_token() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);

    let (status, body) = post_json_no_auth(
        &app,
        "/v1/chat/session",
        json!({ "email": "founder@example.com" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create chat session: {body}");
    let ws_token = body["ws_token"].as_str().unwrap_or_default();
    assert!(
        ws_token.contains('.'),
        "ws_token should contain payload.signature"
    );
    assert!(body["expires_at"].is_string());

    let mut parts = ws_token.split('.');
    let payload_b64 = parts.next().unwrap_or_default();
    let sig_b64 = parts.next().unwrap_or_default();
    assert!(!payload_b64.is_empty());
    assert!(!sig_b64.is_empty());

    let payload = URL_SAFE_NO_PAD.decode(payload_b64).expect("decode payload");
    let payload_json: Value = serde_json::from_slice(&payload).expect("parse payload");
    assert_eq!(payload_json["email"], "founder@example.com");
    assert!(payload_json["workspace_id"].is_string());
    assert!(
        payload_json["api_key"]
            .as_str()
            .unwrap_or_default()
            .starts_with("sk_")
    );
    assert!(payload_json["exp"].is_number());
}

// ── 2. Formation lifecycle ───────────────────────────────────────────────

#[tokio::test]
async fn test_formation_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);

    // 1. Create entity
    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);
    let (status, body) = post_json(
        &app,
        "/v1/formations",
        json!({
            "entity_type": "corporation",
            "legal_name": "FormCo Inc.",
            "jurisdiction": "Delaware",
            "members": [
                {
                    "name": "Alice Founder",
                    "investor_type": "natural_person",
                    "email": "alice@formco.com",
                    "ownership_pct": 100.0,
                    "share_count": 10000,
                    "role": "director"
                }
            ],
            "authorized_shares": 10000000,
            "par_value": "0.0001",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create formation: {body}");
    let entity_id = body["entity_id"].as_str().unwrap();
    let formation_status = body["formation_status"].as_str().unwrap();
    assert_eq!(formation_status, "documents_generated");
    let doc_ids: Vec<String> = body["document_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert!(!doc_ids.is_empty(), "should have at least one document");

    // 2. GET formation status
    let (status, body) = get_json(&app, &format!("/v1/formations/{entity_id}"), &token).await;
    assert_eq!(status, StatusCode::OK, "get formation: {body}");
    assert_eq!(body["legal_name"], "FormCo Inc.");
    assert_eq!(body["entity_type"], "corporation");
    assert_eq!(body["jurisdiction"], "Delaware");

    // 3. List documents
    let (status, body) = get_json(
        &app,
        &format!("/v1/formations/{entity_id}/documents"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list documents: {body}");
    let docs = body.as_array().unwrap();
    assert!(!docs.is_empty());

    // 4. Sign each document
    for doc_id in &doc_ids {
        let (status, body) = post_json(
            &app,
            &format!("/v1/documents/{doc_id}/sign?entity_id={entity_id}"),
            json!({
                "signer_name": "Alice Founder",
                "signer_role": "Incorporator",
                "signer_email": "alice@formco.com",
                "signature_text": "Alice Founder"
            }),
            &token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "sign document {doc_id}: {body}");
        assert!(body["signature_id"].as_str().is_some());
        assert_eq!(body["document_id"], doc_id.as_str());
    }

    // 5. Verify we can read a signed document
    let doc_id = &doc_ids[0];
    let (status, body) = get_json(
        &app,
        &format!("/v1/documents/{doc_id}?entity_id={entity_id}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get document: {body}");
    assert!(!body["signatures"].as_array().unwrap().is_empty());
    assert_eq!(body["entity_id"], entity_id);

    // 6. Mark all documents signed (DocumentsGenerated -> DocumentsSigned)
    let (status, body) = post_json(
        &app,
        &format!("/v1/formations/{entity_id}/mark-documents-signed"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "mark documents signed: {body}");
    assert_eq!(body["formation_status"], "documents_signed");

    // 7. Submit filing (DocumentsSigned -> FilingSubmitted)
    let (status, body) = post_json(
        &app,
        &format!("/v1/formations/{entity_id}/submit-filing"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "submit filing: {body}");
    assert_eq!(body["formation_status"], "filing_submitted");

    // 8. Confirm filing (FilingSubmitted -> Filed)
    let (status, body) = post_json(
        &app,
        &format!("/v1/formations/{entity_id}/filing-confirmation"),
        json!({
            "external_filing_id": "DE-STATE-12345",
            "receipt_reference": "RCPT-98765"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "confirm filing: {body}");
    assert_eq!(body["formation_status"], "filed");

    // 9. Apply for EIN (Filed -> EinApplied)
    let (status, body) = post_json(
        &app,
        &format!("/v1/formations/{entity_id}/apply-ein"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "apply ein: {body}");
    assert_eq!(body["formation_status"], "ein_applied");

    // 10. Confirm EIN (EinApplied -> Active)
    let (status, body) = post_json(
        &app,
        &format!("/v1/formations/{entity_id}/ein-confirmation"),
        json!({
            "ein": "12-3456789"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "confirm ein: {body}");
    assert_eq!(body["formation_status"], "active");

    // 11. Verify final formation status is active
    let (status, body) = get_json(&app, &format!("/v1/formations/{entity_id}"), &token).await;
    assert_eq!(status, StatusCode::OK, "get active formation: {body}");
    assert_eq!(body["formation_status"], "active");
}

// ── 3. Equity lifecycle ──────────────────────────────────────────────────

#[tokio::test]
async fn test_equity_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;

    // 1. Create operating legal entity linked to formation entity.
    let (status, legal_entity) = post_json(
        &app,
        "/v1/equity/entities",
        json!({
            "entity_id": entity_id,
            "linked_entity_id": entity_id,
            "name": "Test Corp, Inc.",
            "role": "operating",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create legal entity: {legal_entity}"
    );
    let issuer_legal_entity_id = legal_entity["legal_entity_id"].as_str().unwrap();

    // 2. Create founder and SAFE investor contacts + holders.
    let (status, founder_contact) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Alice Founder",
            "email": "alice@founder.com",
            "category": "employee",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create founder contact: {founder_contact}"
    );
    let founder_contact_id = founder_contact["contact_id"].as_str().unwrap();

    let (status, founder_holder) = post_json(
        &app,
        "/v1/equity/holders",
        json!({
            "entity_id": entity_id,
            "contact_id": founder_contact_id,
            "name": "Alice Founder",
            "holder_type": "individual",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create founder holder: {founder_holder}"
    );
    let founder_holder_id = founder_holder["holder_id"].as_str().unwrap();

    let (status, investor_contact) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "organization",
            "name": "Seed Ventures",
            "email": "ops@seedventures.com",
            "category": "investor",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create investor contact: {investor_contact}"
    );
    let investor_contact_id = investor_contact["contact_id"].as_str().unwrap();

    let (status, investor_holder) = post_json(
        &app,
        "/v1/equity/holders",
        json!({
            "entity_id": entity_id,
            "contact_id": investor_contact_id,
            "name": "Seed Ventures",
            "holder_type": "fund",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create investor holder: {investor_holder}"
    );
    let investor_holder_id = investor_holder["holder_id"].as_str().unwrap();

    // 3. Create instruments for common, SAFE, and Series A preferred.
    let (status, common) = post_json(
        &app,
        "/v1/equity/instruments",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "symbol": "COMMON",
            "kind": "common_equity",
            "authorized_units": 10000000_i64,
            "issue_price_cents": 1_i64,
            "terms": {},
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create common instrument: {common}");
    let common_instrument_id = common["instrument_id"].as_str().unwrap();

    let (status, safe) = post_json(
        &app,
        "/v1/equity/instruments",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "symbol": "SAFE-PM",
            "kind": "safe",
            "terms": {
                "discount_bps": 2000,
                "cap_price_cents": 80
            },
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create safe instrument: {safe}");
    let safe_instrument_id = safe["instrument_id"].as_str().unwrap();

    let (status, series_a) = post_json(
        &app,
        "/v1/equity/instruments",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "symbol": "SERIES_A",
            "kind": "preferred_equity",
            "authorized_units": 5000000_i64,
            "issue_price_cents": 100_i64,
            "terms": {},
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create preferred instrument: {series_a}"
    );
    let series_a_instrument_id = series_a["instrument_id"].as_str().unwrap();

    // 4. Seed positions.
    let (status, founder_position) = post_json(
        &app,
        "/v1/equity/positions/adjust",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "holder_id": founder_holder_id,
            "instrument_id": common_instrument_id,
            "quantity_delta": 8000000_i64,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create founder position: {founder_position}"
    );

    let (status, safe_position) = post_json(
        &app,
        "/v1/equity/positions/adjust",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "holder_id": investor_holder_id,
            "instrument_id": safe_instrument_id,
            "quantity_delta": 0_i64,
            "principal_delta_cents": 2000000_i64,
            "source_reference": "SAFE-2026-001",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create safe position: {safe_position}"
    );

    // 5. Create round and apply terms.
    let (status, round) = post_json(
        &app,
        "/v1/equity/rounds",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "name": "Series A",
            "pre_money_cents": 800000000_i64,
            "round_price_cents": 100_i64,
            "target_raise_cents": 100000000_i64,
            "conversion_target_instrument_id": series_a_instrument_id,
            "metadata": {"lead": "OpenAlpha Capital"},
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create round: {round}");
    let round_id = round["round_id"].as_str().unwrap();

    let (status, rules) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/apply-terms"),
        json!({
            "entity_id": entity_id,
            "anti_dilution_method": "none",
            "conversion_precedence": ["safe"],
            "protective_provisions": {},
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "apply round terms: {rules}");
    assert!(rules["rule_set_id"].as_str().is_some());

    // 6. Set up board meeting + passed resolution for board approval gating.
    let (status, director_one) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Director One",
            "category": "board_member",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create director one: {director_one}"
    );
    let director_one_id = director_one["contact_id"].as_str().unwrap();

    let (status, director_two) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Director Two",
            "category": "board_member",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create director two: {director_two}"
    );
    let director_two_id = director_two["contact_id"].as_str().unwrap();

    let (status, gov_body) = post_json(
        &app,
        "/v1/governance-bodies",
        json!({
            "entity_id": entity_id,
            "body_type": "board_of_directors",
            "name": "Board of Directors",
            "quorum_rule": "majority",
            "voting_method": "per_capita",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create board body: {gov_body}");
    let body_id = gov_body["body_id"].as_str().unwrap();
    let e_query = format!("entity_id={entity_id}");

    let (status, seat_one) = post_json(
        &app,
        &format!("/v1/governance-bodies/{body_id}/seats?{e_query}"),
        json!({
            "holder_id": director_one_id,
            "role": "chair",
            "voting_power": 1,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create seat one: {seat_one}");
    let seat_one_id = seat_one["seat_id"].as_str().unwrap();

    let (status, seat_two) = post_json(
        &app,
        &format!("/v1/governance-bodies/{body_id}/seats?{e_query}"),
        json!({
            "holder_id": director_two_id,
            "role": "member",
            "voting_power": 1,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create seat two: {seat_two}");
    let seat_two_id = seat_two["seat_id"].as_str().unwrap();

    let (status, meeting) = post_json(
        &app,
        "/v1/meetings",
        json!({
            "entity_id": entity_id,
            "body_id": body_id,
            "meeting_type": "board_meeting",
            "title": "Series A Approval Meeting",
            "scheduled_date": "2026-03-15",
            "location": "Virtual",
            "notice_days": 0,
            "agenda_item_titles": ["Approve Series A round"],
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "schedule approval meeting: {meeting}"
    );
    let meeting_id = meeting["meeting_id"].as_str().unwrap();

    let (status, notice) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/notice?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "notice meeting: {notice}");

    let (status, convene) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/convene?{e_query}"),
        json!({
            "present_seat_ids": [seat_one_id, seat_two_id]
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "convene meeting: {convene}");

    let (status, agenda_items) = get_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items?{e_query}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list agenda items: {agenda_items}");
    let agenda_item_id = agenda_items.as_array().unwrap()[0]["agenda_item_id"]
        .as_str()
        .unwrap();

    let (status, vote_one) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/vote?{e_query}"),
        json!({
            "voter_id": director_one_id,
            "vote_value": "for",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "cast vote one: {vote_one}");

    let (status, vote_two) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/vote?{e_query}"),
        json!({
            "voter_id": director_two_id,
            "vote_value": "for",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "cast vote two: {vote_two}");

    let (status, resolution) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/resolution?{e_query}"),
        json!({
            "resolution_text": "Resolved: approve Series A financing",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "compute resolution: {resolution}");
    assert_eq!(resolution["passed"], true);
    let resolution_id = resolution["resolution_id"].as_str().unwrap();

    let (status, round_after_board_approval) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/board-approve"),
        json!({
            "entity_id": entity_id,
            "meeting_id": meeting_id,
            "resolution_id": resolution_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "board approve round: {round_after_board_approval}"
    );
    assert_eq!(round_after_board_approval["status"], "board_approved");
    assert_eq!(
        round_after_board_approval["board_approval_resolution_id"],
        resolution_id
    );

    // 7. Accept round with an authorized intent.
    let (status, accept_intent) = post_json(
        &app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": "equity.round.accept",
            "authority_tier": "tier_2",
            "description": "Accept approved Series A round",
            "metadata": {"round_id": round_id},
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create accept intent: {accept_intent}"
    );
    let accept_intent_id = accept_intent["intent_id"].as_str().unwrap();

    let (status, _) = post_json(
        &app,
        &format!("/v1/intents/{accept_intent_id}/evaluate?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = post_json(
        &app,
        &format!("/v1/intents/{accept_intent_id}/authorize?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, accepted_round) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/accept"),
        json!({
            "entity_id": entity_id,
            "intent_id": accept_intent_id,
            "accepted_by_contact_id": founder_contact_id,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "accept round: {accepted_round}");
    assert_eq!(accepted_round["status"], "accepted");

    let (status, preview) = post_json(
        &app,
        "/v1/equity/conversions/preview",
        json!({
            "entity_id": entity_id,
            "round_id": round_id,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "preview conversion: {preview}");
    assert_eq!(preview["round_id"], round_id);
    assert_eq!(preview["target_instrument_id"], series_a_instrument_id);
    assert!(!preview["lines"].as_array().unwrap().is_empty());
    assert!(preview["total_new_units"].as_i64().unwrap_or(0) > 0);

    // 8. Execute conversion with authorized execute intent.
    let (status, execute_intent) = post_json(
        &app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": "equity.round.execute_conversion",
            "authority_tier": "tier_2",
            "description": "Execute Series A conversion",
            "metadata": {"round_id": round_id},
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create execute intent: {execute_intent}"
    );
    let execute_intent_id = execute_intent["intent_id"].as_str().unwrap();

    let (status, _) = post_json(
        &app,
        &format!("/v1/intents/{execute_intent_id}/evaluate?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = post_json(
        &app,
        &format!("/v1/intents/{execute_intent_id}/authorize?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, execution) = post_json(
        &app,
        "/v1/equity/conversions/execute",
        json!({
            "entity_id": entity_id,
            "round_id": round_id,
            "intent_id": execute_intent_id,
            "source_reference": "series-a-close-2026-02-01",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "execute conversion: {execution}");
    assert!(execution["conversion_execution_id"].as_str().is_some());
    assert!(
        execution["total_new_units"].as_i64().unwrap_or(0)
            >= preview["total_new_units"].as_i64().unwrap_or(0)
    );

    // 9. Read cap table in as-converted basis.
    let (status, cap_table) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/cap-table?basis=as_converted"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get cap table: {cap_table}");
    assert_eq!(cap_table["entity_id"], entity_id);
    assert_eq!(cap_table["issuer_legal_entity_id"], issuer_legal_entity_id);
    assert_eq!(cap_table["basis"], "as_converted");
    assert!(cap_table["total_units"].as_i64().unwrap_or(0) >= 8020000_i64);
    assert!(cap_table["holders"].as_array().unwrap().len() >= 2);

    // 10. Persisted state checks: round closed, intents executed, conversion record stored.
    let layout = api_rs::store::RepoLayout::new(tmp.path().to_path_buf());
    let parsed_entity_id: EntityId = entity_id.parse().unwrap();
    let store = EntityStore::open(&layout, ws_id, parsed_entity_id).unwrap();

    let parsed_round_id: EquityRoundId = round_id.parse().unwrap();
    let persisted_round = store.read::<EquityRound>("main", parsed_round_id).unwrap();
    assert_eq!(persisted_round.status(), EquityRoundStatus::Closed);

    let parsed_accept_intent_id: IntentId = accept_intent_id.parse().unwrap();
    let persisted_accept_intent = store
        .read::<Intent>("main", parsed_accept_intent_id)
        .unwrap();
    assert_eq!(persisted_accept_intent.status(), IntentStatus::Executed);

    let parsed_execute_intent_id: IntentId = execute_intent_id.parse().unwrap();
    let persisted_execute_intent = store
        .read::<Intent>("main", parsed_execute_intent_id)
        .unwrap();
    assert_eq!(persisted_execute_intent.status(), IntentStatus::Executed);

    let parsed_conversion_id: ConversionExecutionId = execution["conversion_execution_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let persisted_conversion = store
        .read::<ConversionExecution>("main", parsed_conversion_id)
        .unwrap();
    assert_eq!(persisted_conversion.equity_round_id(), parsed_round_id);
}

#[tokio::test]
async fn test_cap_table_100_investors() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;

    let (status, legal_entity) = post_json(
        &app,
        "/v1/equity/entities",
        json!({
            "entity_id": entity_id,
            "linked_entity_id": entity_id,
            "name": "OpenAI HoldCo",
            "role": "operating",
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create legal entity: {legal_entity}"
    );
    let issuer_legal_entity_id = legal_entity["legal_entity_id"].as_str().unwrap();

    let (status, common) = post_json(
        &app,
        "/v1/equity/instruments",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "symbol": "COMMON",
            "kind": "common_equity",
            "authorized_units": 50000000_i64,
            "terms": {
                "example_structure": "openai_hybrid_like_control_plus_economic_entities"
            },
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create common instrument: {common}");
    let common_instrument_id = common["instrument_id"].as_str().unwrap();

    let mut expected_total: i64 = 0;
    for i in 0..100_i64 {
        let investor_name = format!("Investor {i:03}");
        let investor_email = format!("investor{i:03}@example.com");

        let (status, contact) = post_json(
            &app,
            "/v1/contacts",
            json!({
                "entity_id": entity_id,
                "contact_type": "organization",
                "name": investor_name,
                "email": investor_email,
                "category": "investor",
            }),
            &token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "create contact {i}: {contact}");
        let contact_id = contact["contact_id"].as_str().unwrap();

        let (status, holder) = post_json(
            &app,
            "/v1/equity/holders",
            json!({
                "entity_id": entity_id,
                "contact_id": contact_id,
                "name": format!("Investor {i:03}"),
                "holder_type": "fund",
            }),
            &token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "create holder {i}: {holder}");
        let holder_id = holder["holder_id"].as_str().unwrap();

        let shares = 1000_i64 + i;
        expected_total += shares;
        let (status, position) = post_json(
            &app,
            "/v1/equity/positions/adjust",
            json!({
                "entity_id": entity_id,
                "issuer_legal_entity_id": issuer_legal_entity_id,
                "holder_id": holder_id,
                "instrument_id": common_instrument_id,
                "quantity_delta": shares,
                "source_reference": format!("seed-allocation-{i:03}"),
            }),
            &token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "create position {i}: {position}");
    }

    let (status, cap_table) =
        get_json(&app, &format!("/v1/entities/{entity_id}/cap-table"), &token).await;
    assert_eq!(status, StatusCode::OK, "get cap table: {cap_table}");
    assert_eq!(cap_table["entity_id"], entity_id);
    assert_eq!(cap_table["issuer_legal_entity_id"], issuer_legal_entity_id);
    assert_eq!(
        cap_table["total_units"].as_i64().unwrap_or(-1),
        expected_total
    );
    assert_eq!(cap_table["holders"].as_array().unwrap().len(), 100);
}

// ── 4. Governance lifecycle ──────────────────────────────────────────────

#[tokio::test]
async fn test_governance_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;

    // 1. Create contacts for board members
    let (_, c1) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Director One",
            "category": "board_member",
        }),
        &token,
    )
    .await;
    let contact_id_1 = c1["contact_id"].as_str().unwrap().to_owned();

    let (_, c2) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Director Two",
            "category": "board_member",
        }),
        &token,
    )
    .await;
    let contact_id_2 = c2["contact_id"].as_str().unwrap().to_owned();

    // 2. Create governance body (board of directors)
    let (status, body) = post_json(
        &app,
        "/v1/governance-bodies",
        json!({
            "entity_id": entity_id,
            "body_type": "board_of_directors",
            "name": "Board of Directors",
            "quorum_rule": "majority",
            "voting_method": "per_capita",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create governance body: {body}");
    let body_id = body["body_id"].as_str().unwrap();
    assert_eq!(body["name"], "Board of Directors");

    // 3. Create seats
    let e_query = format!("entity_id={entity_id}");
    let (status, seat1) = post_json(
        &app,
        &format!("/v1/governance-bodies/{body_id}/seats?{e_query}"),
        json!({
            "holder_id": contact_id_1,
            "role": "chair",
            "voting_power": 1,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create seat 1: {seat1}");
    let seat_id_1 = seat1["seat_id"].as_str().unwrap().to_owned();

    let (status, seat2) = post_json(
        &app,
        &format!("/v1/governance-bodies/{body_id}/seats?{e_query}"),
        json!({
            "holder_id": contact_id_2,
            "role": "member",
            "voting_power": 1,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create seat 2: {seat2}");
    let seat_id_2 = seat2["seat_id"].as_str().unwrap().to_owned();

    // 4. Schedule meeting
    let (status, meeting_body) = post_json(
        &app,
        "/v1/meetings",
        json!({
            "entity_id": entity_id,
            "body_id": body_id,
            "meeting_type": "board_meeting",
            "title": "Q1 Board Meeting",
            "scheduled_date": "2026-03-15",
            "location": "Virtual",
            "notice_days": 10,
            "agenda_item_titles": ["Approve budget", "Elect officers"],
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "schedule meeting: {meeting_body}");
    let meeting_id = meeting_body["meeting_id"].as_str().unwrap();
    assert_eq!(meeting_body["title"], "Q1 Board Meeting");
    assert_eq!(meeting_body["status"], "draft");
    assert_eq!(meeting_body["agenda_item_ids"].as_array().unwrap().len(), 2);

    // 5. Send notice
    let (status, body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/notice?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "send notice: {body}");
    assert_eq!(body["status"], "noticed");

    // 6. Convene meeting (both directors present)
    let (status, body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/convene?{e_query}"),
        json!({
            "present_seat_ids": [seat_id_1, seat_id_2],
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "convene meeting: {body}");
    assert_eq!(body["status"], "convened");
    assert_eq!(body["quorum_met"], "met");

    // 7. List agenda items and grab the first item ID
    let (status, agenda) = get_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items?{e_query}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list agenda items: {agenda}");
    let agenda_items = agenda.as_array().unwrap();
    assert_eq!(agenda_items.len(), 2);
    let agenda_item_id = agenda_items[0]["agenda_item_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // 8. Cast votes on the first agenda item
    let (status, vote_body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/vote?{e_query}"),
        json!({
            "voter_id": contact_id_1,
            "vote_value": "for"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "cast vote 1: {vote_body}");

    let (status, vote_body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/vote?{e_query}"),
        json!({
            "voter_id": contact_id_2,
            "vote_value": "for"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "cast vote 2: {vote_body}");

    // 9. Compute resolution from votes
    let (status, resolution_body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/resolution?{e_query}"),
        json!({
            "resolution_text": "Resolved: approve FY budget."
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "compute resolution: {resolution_body}"
    );
    assert_eq!(resolution_body["passed"], true);

    // 10. Adjourn meeting
    let (status, body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/adjourn?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "adjourn meeting: {body}");
    assert_eq!(body["status"], "adjourned");
}

// ── 5. Treasury lifecycle ────────────────────────────────────────────────

#[tokio::test]
async fn test_treasury_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;
    let e_query = format!("entity_id={entity_id}");

    // 1. Create GL accounts (Cash and Revenue)
    let (status, cash_acct) = post_json(
        &app,
        "/v1/treasury/accounts",
        json!({
            "entity_id": entity_id,
            "account_code": "Cash",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create cash account: {cash_acct}");
    let cash_id = cash_acct["account_id"].as_str().unwrap();
    assert_eq!(cash_acct["account_name"], "Cash");

    let (status, rev_acct) = post_json(
        &app,
        "/v1/treasury/accounts",
        json!({
            "entity_id": entity_id,
            "account_code": "Revenue",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create revenue account: {rev_acct}");
    let rev_id = rev_acct["account_id"].as_str().unwrap();

    // 2. List accounts
    let (status, accounts) =
        get_json(&app, &format!("/v1/entities/{entity_id}/accounts"), &token).await;
    assert_eq!(status, StatusCode::OK, "list accounts: {accounts}");
    assert_eq!(accounts.as_array().unwrap().len(), 2);

    // 3. Create balanced journal entry (debit cash, credit revenue)
    let (status, je) = post_json(
        &app,
        "/v1/treasury/journal-entries",
        json!({
            "entity_id": entity_id,
            "description": "Client payment received",
            "effective_date": "2026-02-01",
            "lines": [
                { "account_id": cash_id, "side": "debit", "amount_cents": 100000 },
                { "account_id": rev_id, "side": "credit", "amount_cents": 100000 },
            ],
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create journal entry: {je}");
    let je_id = je["journal_entry_id"].as_str().unwrap();
    assert_eq!(je["total_debits_cents"], 100000);
    assert_eq!(je["total_credits_cents"], 100000);
    assert_eq!(je["status"], "draft");

    // 4. Post journal entry
    let (status, je) = post_json(
        &app,
        &format!("/v1/journal-entries/{je_id}/post?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "post journal entry: {je}");
    assert_eq!(je["status"], "posted");

    // 5. Create invoice
    let (status, inv) = post_json(
        &app,
        "/v1/treasury/invoices",
        json!({
            "entity_id": entity_id,
            "customer_name": "Acme Corp",
            "amount_cents": 250000,
            "description": "Consulting services - Jan 2026",
            "due_date": "2026-03-01",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create invoice: {inv}");
    let inv_id = inv["invoice_id"].as_str().unwrap();
    assert_eq!(inv["status"], "draft");

    // 6. Send invoice
    let (status, inv) = post_json(
        &app,
        &format!("/v1/invoices/{inv_id}/send?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "send invoice: {inv}");
    assert_eq!(inv["status"], "sent");

    // 7. Mark invoice paid
    let (status, inv) = post_json(
        &app,
        &format!("/v1/invoices/{inv_id}/mark-paid?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "mark invoice paid: {inv}");
    assert_eq!(inv["status"], "paid");

    // 8. Create bank account
    let (status, ba) = post_json(
        &app,
        "/v1/treasury/bank-accounts",
        json!({
            "entity_id": entity_id,
            "bank_name": "First National Bank",
            "account_type": "checking",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create bank account: {ba}");
    let ba_id = ba["bank_account_id"].as_str().unwrap();
    assert_eq!(ba["status"], "pending_review");

    // 9. Activate bank account
    let (status, ba) = post_json(
        &app,
        &format!("/v1/bank-accounts/{ba_id}/activate?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "activate bank account: {ba}");
    assert_eq!(ba["status"], "active");
}

// ── 6. Execution lifecycle ───────────────────────────────────────────────

#[tokio::test]
async fn test_execution_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;
    let e_query = format!("entity_id={entity_id}");

    // 1. Create intent
    let (status, intent) = post_json(
        &app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": "hire_employee",
            "authority_tier": "tier_1",
            "description": "Hire new engineer",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create intent: {intent}");
    let intent_id = intent["intent_id"].as_str().unwrap();
    assert_eq!(intent["status"], "pending");

    // 2. Evaluate intent
    let (status, intent) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/evaluate?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "evaluate intent: {intent}");
    assert_eq!(intent["status"], "evaluated");

    // 3. Authorize intent
    let (status, intent) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/authorize?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "authorize intent: {intent}");
    assert_eq!(intent["status"], "authorized");

    // 4. Execute intent
    let (status, intent) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/execute?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "execute intent: {intent}");
    assert_eq!(intent["status"], "executed");

    // 5. Create obligation
    let (status, ob) = post_json(
        &app,
        "/v1/execution/obligations",
        json!({
            "entity_id": entity_id,
            "intent_id": intent_id,
            "obligation_type": "file_tax_return",
            "assignee_type": "internal",
            "description": "File Q1 tax return",
            "due_date": "2026-04-15",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create obligation: {ob}");
    let ob_id = ob["obligation_id"].as_str().unwrap();
    assert_eq!(ob["status"], "required");

    // 6. Fulfill obligation
    let (status, ob) = post_json(
        &app,
        &format!("/v1/obligations/{ob_id}/fulfill?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "fulfill obligation: {ob}");
    assert_eq!(ob["status"], "fulfilled");
}

// ── 7. Contacts ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_contacts() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;

    // 1. Create contacts
    let (status, c1) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Jane Attorney",
            "email": "jane@lawfirm.com",
            "category": "law_firm",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create contact 1: {c1}");
    assert_eq!(c1["name"], "Jane Attorney");
    assert_eq!(c1["contact_type"], "individual");

    let (status, c2) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "organization",
            "name": "Big Accounting LLP",
            "email": "info@bigacct.com",
            "category": "accounting_firm",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create contact 2: {c2}");

    // 2. List contacts
    let (status, contacts) =
        get_json(&app, &format!("/v1/entities/{entity_id}/contacts"), &token).await;
    assert_eq!(status, StatusCode::OK, "list contacts: {contacts}");
    let arr = contacts.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

// ── 8. Branch lifecycle ──────────────────────────────────────────────────

#[tokio::test]
async fn test_branch_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;
    let e_query = format!("entity_id={entity_id}");

    // 1. Create branch
    let (status, body) = post_json(
        &app,
        &format!("/v1/branches?{e_query}"),
        json!({
            "name": "feature/test-branch",
            "from": "main",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create branch: {body}");
    assert_eq!(body["branch"], "feature/test-branch");
    assert!(body["base_commit"].as_str().is_some());

    // 2. List branches
    let (status, body) = get_json(&app, &format!("/v1/branches?{e_query}"), &token).await;
    assert_eq!(status, StatusCode::OK, "list branches: {body}");
    let branches = body.as_array().unwrap();
    let branch_names: Vec<&str> = branches
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert!(branch_names.contains(&"main"));
    assert!(branch_names.contains(&"feature/test-branch"));

    // 3. Merge branch to main
    let (status, body) = post_json(
        &app,
        &format!("/v1/branches/feature%2Ftest-branch/merge?{e_query}"),
        json!({ "into": "main" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "merge branch: {body}");
    assert_eq!(body["merged"], true);

    // 4. Delete branch
    let status = delete_req(
        &app,
        &format!("/v1/branches/feature%2Ftest-branch?{e_query}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "delete branch");

    // 5. Verify branch is gone
    let (status, body) = get_json(&app, &format!("/v1/branches?{e_query}"), &token).await;
    assert_eq!(status, StatusCode::OK);
    let branch_names: Vec<&str> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert!(!branch_names.contains(&"feature/test-branch"));
}

// ── 9. Error cases ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_not_found_errors() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);

    let fake_id = uuid::Uuid::new_v4().to_string();
    let token = make_token(WorkspaceId::new());

    // GET formation for nonexistent entity
    let (status, body) = get_json(&app, &format!("/v1/formations/{fake_id}"), &token).await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::UNPROCESSABLE_ENTITY,
        "expected 404 or 422 for missing entity, got {status}: {body}"
    );

    // GET cap table for nonexistent entity
    let (status, body) = get_json(&app, &format!("/v1/entities/{fake_id}/cap-table"), &token).await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::INTERNAL_SERVER_ERROR,
        "expected 404 or 500 for missing cap table, got {status}: {body}"
    );
}

#[tokio::test]
async fn test_unbalanced_journal_entry_rejected() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;

    // Create two accounts
    let (_, cash_acct) = post_json(
        &app,
        "/v1/treasury/accounts",
        json!({
            "entity_id": entity_id,
            "account_code": "Cash",
        }),
        &token,
    )
    .await;
    let cash_id = cash_acct["account_id"].as_str().unwrap();

    let (_, rev_acct) = post_json(
        &app,
        "/v1/treasury/accounts",
        json!({
            "entity_id": entity_id,
            "account_code": "Revenue",
        }),
        &token,
    )
    .await;
    let rev_id = rev_acct["account_id"].as_str().unwrap();

    // Try to create unbalanced journal entry
    let (status, body) = post_json(
        &app,
        "/v1/treasury/journal-entries",
        json!({
            "entity_id": entity_id,
            "description": "Unbalanced entry",
            "effective_date": "2026-02-01",
            "lines": [
                { "account_id": cash_id, "side": "debit", "amount_cents": 100000 },
                { "account_id": rev_id, "side": "credit", "amount_cents": 50000 },
            ],
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unbalanced JE should be 422: {body}"
    );
}

#[tokio::test]
async fn test_conversion_preview_requires_round_terms() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;

    let (_, legal_entity) = post_json(
        &app,
        "/v1/equity/entities",
        json!({
            "entity_id": entity_id,
            "linked_entity_id": entity_id,
            "name": "Validation Corp",
            "role": "operating",
        }),
        &token,
    )
    .await;
    let issuer_legal_entity_id = legal_entity["legal_entity_id"].as_str().unwrap();

    let (_, target_instrument) = post_json(
        &app,
        "/v1/equity/instruments",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "symbol": "SERIES_A",
            "kind": "preferred_equity",
            "terms": {},
        }),
        &token,
    )
    .await;
    let target_instrument_id = target_instrument["instrument_id"].as_str().unwrap();

    let (_, round) = post_json(
        &app,
        "/v1/equity/rounds",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "name": "Series A",
            "round_price_cents": 100_i64,
            "target_raise_cents": 10000000_i64,
            "conversion_target_instrument_id": target_instrument_id,
            "metadata": {},
        }),
        &token,
    )
    .await;
    let round_id = round["round_id"].as_str().unwrap();

    // Preview conversion without round terms applied should fail.
    let (status, body) = post_json(
        &app,
        "/v1/equity/conversions/preview",
        json!({
            "entity_id": entity_id,
            "round_id": round_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "preview without terms should be 400: {body}"
    );
}

#[tokio::test]
async fn test_execute_conversion_requires_round_acceptance() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;
    let e_query = format!("entity_id={entity_id}");

    let (_, legal_entity) = post_json(
        &app,
        "/v1/equity/entities",
        json!({
            "entity_id": entity_id,
            "linked_entity_id": entity_id,
            "name": "Acceptance Validation Corp",
            "role": "operating",
        }),
        &token,
    )
    .await;
    let issuer_legal_entity_id = legal_entity["legal_entity_id"].as_str().unwrap();

    let (_, target_instrument) = post_json(
        &app,
        "/v1/equity/instruments",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "symbol": "SERIES_A",
            "kind": "preferred_equity",
            "terms": {},
        }),
        &token,
    )
    .await;
    let target_instrument_id = target_instrument["instrument_id"].as_str().unwrap();

    let (_, round) = post_json(
        &app,
        "/v1/equity/rounds",
        json!({
            "entity_id": entity_id,
            "issuer_legal_entity_id": issuer_legal_entity_id,
            "name": "Series A",
            "round_price_cents": 100_i64,
            "target_raise_cents": 10000000_i64,
            "conversion_target_instrument_id": target_instrument_id,
            "metadata": {},
        }),
        &token,
    )
    .await;
    let round_id = round["round_id"].as_str().unwrap();

    let (status, _) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/apply-terms"),
        json!({
            "entity_id": entity_id,
            "anti_dilution_method": "none",
            "conversion_precedence": ["safe"],
            "protective_provisions": {},
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, execute_intent) = post_json(
        &app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": "equity.round.execute_conversion",
            "authority_tier": "tier_2",
            "description": "Execute Series A conversion",
            "metadata": {"round_id": round_id},
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let execute_intent_id = execute_intent["intent_id"].as_str().unwrap();

    let (status, _) = post_json(
        &app,
        &format!("/v1/intents/{execute_intent_id}/evaluate?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = post_json(
        &app,
        &format!("/v1/intents/{execute_intent_id}/authorize?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Execute conversion without board approval/acceptance should fail.
    let (status, body) = post_json(
        &app,
        "/v1/equity/conversions/execute",
        json!({
            "entity_id": entity_id,
            "round_id": round_id,
            "intent_id": execute_intent_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "execute before round acceptance should be 422: {body}"
    );
}

#[tokio::test]
async fn test_board_approve_round_validation_guards() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (_ws_id, entity_id, token) = create_entity(&app).await;
    let (_issuer_legal_entity_id, round_id) =
        create_round_with_terms(&app, &entity_id, &token).await;

    let (llc_meeting_id, llc_resolution_id) =
        create_resolution_for_body(&app, &entity_id, &token, "llc_member_vote", &["for"]).await;

    let (status, non_board_body) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/board-approve"),
        json!({
            "entity_id": entity_id,
            "meeting_id": llc_meeting_id,
            "resolution_id": llc_resolution_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "board approval with non-board body should fail: {non_board_body}"
    );

    let (failed_meeting_id, failed_resolution_id) = create_resolution_for_body(
        &app,
        &entity_id,
        &token,
        "board_of_directors",
        &["for", "against"],
    )
    .await;

    let (status, failed_resolution) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/board-approve"),
        json!({
            "entity_id": entity_id,
            "meeting_id": failed_meeting_id,
            "resolution_id": failed_resolution_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "board approval with failed resolution should fail: {failed_resolution}"
    );

    let (meeting_id, _resolution_id) =
        create_resolution_for_body(&app, &entity_id, &token, "board_of_directors", &["for"]).await;
    let missing_resolution_id = "00000000-0000-0000-0000-000000000001";
    let (status, missing_resolution) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/board-approve"),
        json!({
            "entity_id": entity_id,
            "meeting_id": meeting_id,
            "resolution_id": missing_resolution_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "board approval with missing resolution should be 404: {missing_resolution}"
    );
}

#[tokio::test]
async fn test_accept_round_requires_board_approval_and_valid_intent() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (_ws_id, entity_id, token) = create_entity(&app).await;
    let (_issuer_legal_entity_id, round_id) =
        create_round_with_terms(&app, &entity_id, &token).await;

    let authorized_accept_intent_id =
        create_authorized_round_intent(&app, &entity_id, &token, "equity.round.accept", &round_id)
            .await;

    let (status, pre_approval_accept) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/accept"),
        json!({
            "entity_id": entity_id,
            "intent_id": authorized_accept_intent_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "accept before board approval should fail: {pre_approval_accept}"
    );

    let (meeting_id, resolution_id) =
        create_resolution_for_body(&app, &entity_id, &token, "board_of_directors", &["for"]).await;
    let (status, board_approved_round) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/board-approve"),
        json!({
            "entity_id": entity_id,
            "meeting_id": meeting_id,
            "resolution_id": resolution_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "board approval should succeed: {board_approved_round}"
    );

    let (status, draft_intent) = post_json(
        &app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": "equity.round.accept",
            "authority_tier": "tier_2",
            "description": "Draft accept intent",
            "metadata": {"round_id": round_id},
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create draft intent: {draft_intent}"
    );
    let draft_intent_id = draft_intent["intent_id"].as_str().unwrap();

    let (status, unauthorized_accept) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/accept"),
        json!({
            "entity_id": entity_id,
            "intent_id": draft_intent_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "accept with non-authorized intent should fail: {unauthorized_accept}"
    );

    let wrong_type_intent_id = create_authorized_round_intent(
        &app,
        &entity_id,
        &token,
        "equity.round.execute_conversion",
        &round_id,
    )
    .await;
    let (status, wrong_type_accept) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/accept"),
        json!({
            "entity_id": entity_id,
            "intent_id": wrong_type_intent_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "accept with wrong intent type should fail: {wrong_type_accept}"
    );

    let wrong_round_accept_intent_id = create_authorized_round_intent(
        &app,
        &entity_id,
        &token,
        "equity.round.accept",
        "00000000-0000-0000-0000-000000000002",
    )
    .await;
    let (status, wrong_round_accept) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/accept"),
        json!({
            "entity_id": entity_id,
            "intent_id": wrong_round_accept_intent_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "accept with wrong round metadata should fail: {wrong_round_accept}"
    );
}

#[tokio::test]
async fn test_execute_conversion_requires_execute_intent_type() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (_ws_id, entity_id, token) = create_entity(&app).await;
    let (_issuer_legal_entity_id, round_id) =
        create_round_with_terms(&app, &entity_id, &token).await;

    let (meeting_id, resolution_id) =
        create_resolution_for_body(&app, &entity_id, &token, "board_of_directors", &["for"]).await;
    let (status, board_approved_round) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/board-approve"),
        json!({
            "entity_id": entity_id,
            "meeting_id": meeting_id,
            "resolution_id": resolution_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "board approval should succeed: {board_approved_round}"
    );

    let accept_intent_id =
        create_authorized_round_intent(&app, &entity_id, &token, "equity.round.accept", &round_id)
            .await;
    let (status, accepted_round) = post_json(
        &app,
        &format!("/v1/equity/rounds/{round_id}/accept"),
        json!({
            "entity_id": entity_id,
            "intent_id": accept_intent_id,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "accept round: {accepted_round}");

    let wrong_execute_intent_id =
        create_authorized_round_intent(&app, &entity_id, &token, "equity.round.accept", &round_id)
            .await;
    let (status, wrong_execute_intent) = post_json(
        &app,
        "/v1/equity/conversions/execute",
        json!({
            "entity_id": entity_id,
            "round_id": round_id,
            "intent_id": wrong_execute_intent_id,
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "execute with wrong intent type should fail: {wrong_execute_intent}"
    );
}

// ── 10. Cross-domain: full lifecycle ─────────────────────────────────────

#[tokio::test]
async fn test_full_cross_domain_lifecycle() {
    // This test exercises formation + contacts + treasury + execution together
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;
    let e_query = format!("entity_id={entity_id}");

    // Create a contact (employee)
    let (status, _contact) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Eve Employee",
            "email": "eve@testcorp.com",
            "category": "employee",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Create a treasury account
    let (status, _) = post_json(
        &app,
        "/v1/treasury/accounts",
        json!({
            "entity_id": entity_id,
            "account_code": "OperatingExpenses",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Create an intent to hire
    let (status, intent) = post_json(
        &app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": "hire_employee",
            "authority_tier": "tier_1",
            "description": "Hire Eve as engineer",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let intent_id = intent["intent_id"].as_str().unwrap();

    // Walk intent through FSM
    let (s, _) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/evaluate?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, _) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/authorize?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, _) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/execute?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    // Create obligation from intent
    let (status, ob) = post_json(
        &app,
        "/v1/execution/obligations",
        json!({
            "entity_id": entity_id,
            "intent_id": intent_id,
            "obligation_type": "onboarding_paperwork",
            "assignee_type": "internal",
            "description": "Complete I-9 and W-4 forms",
            "due_date": "2026-03-01",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ob_id = ob["obligation_id"].as_str().unwrap();

    // Fulfill obligation
    let (status, _) = post_json(
        &app,
        &format!("/v1/obligations/{ob_id}/fulfill?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify all entities visible via list endpoints
    let (s, contacts) = get_json(&app, &format!("/v1/entities/{entity_id}/contacts"), &token).await;
    assert_eq!(s, StatusCode::OK);
    assert!(!contacts.as_array().unwrap().is_empty());

    let (s, intents) = get_json(&app, &format!("/v1/entities/{entity_id}/intents"), &token).await;
    assert_eq!(s, StatusCode::OK);
    assert!(!intents.as_array().unwrap().is_empty());

    let (s, obligations) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/obligations"),
        &token,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(!obligations.as_array().unwrap().is_empty());
}

// ── Helper: PATCH JSON ───────────────────────────────────────────────

async fn patch_json(app: &Router, path: &str, body: Value, token: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::PATCH)
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

// ── 14. Webhook lifecycle ───────────────────────────────────────────

#[tokio::test]
async fn test_webhook_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    unsafe {
        std::env::set_var("STRIPE_WEBHOOK_SECRET", "stripe-secret-test");
        std::env::set_var(
            "STRIPE_BILLING_WEBHOOK_SECRET",
            "stripe-billing-secret-test",
        );
    }

    // 1. Stripe webhook
    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/webhooks/stripe")
        .header("content-type", "application/json")
        .header("x-webhook-secret", "stripe-secret-test")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "id": "evt_test_123",
                "type": "payment_intent.succeeded",
                "data": { "object": { "amount": 5000 } }
            }))
            .unwrap(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK, "stripe webhook: {body}");
    assert_eq!(body["received"], true);
    assert_eq!(body["event_id"], "evt_test_123");

    // 2. Stripe billing webhook
    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/webhooks/stripe-billing")
        .header("content-type", "application/json")
        .header("x-webhook-secret", "stripe-billing-secret-test")
        .body(Body::from(
            serde_json::to_vec(&json!({
                "id": "evt_billing_456",
                "type": "invoice.paid",
                "data": { "object": { "subscription": "sub_123" } }
            }))
            .unwrap(),
        ))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK, "stripe billing webhook: {body}");
    assert_eq!(body["received"], true);
    assert_eq!(body["event_id"], "evt_billing_456");

    // 3. Webhook with missing optional fields
    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/webhooks/stripe")
        .header("content-type", "application/json")
        .header("x-webhook-secret", "stripe-secret-test")
        .body(Body::from(serde_json::to_vec(&json!({})).unwrap()))
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK, "empty webhook: {body}");
    assert_eq!(body["received"], true);
    assert!(body["event_id"].is_null());
}

// ── 15. Auth & workspace provisioning lifecycle ─────────────────────

#[tokio::test]
async fn test_auth_workspace_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let token = make_token(WorkspaceId::new());

    // 1. Provision a workspace
    let (status, body) = post_json(
        &app,
        "/v1/workspaces/provision",
        json!({
            "name": "Test Workspace",
            "owner_email": "admin@test.com"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "provision workspace: {body}");
    let ws_id_str = body["workspace_id"].as_str().unwrap();
    assert_eq!(body["name"], "Test Workspace");
    assert!(body["api_key"].as_str().unwrap().starts_with("sk_"));
    let api_key = body["api_key"].as_str().unwrap().to_owned();
    let _key_id = body["api_key_id"].as_str().unwrap();

    // 1b. Direct API-key bearer auth works without token exchange.
    let (status, body) = get_json(&app, "/v1/api-keys", &api_key).await;
    assert_eq!(status, StatusCode::OK, "direct api-key auth failed: {body}");
    assert_eq!(body.as_array().unwrap().len(), 1);

    // Create a token for this specific workspace
    let ws_id: WorkspaceId = ws_id_str.parse().unwrap();
    let ws_token = make_token(ws_id);

    // 2. Create another API key in that workspace
    let (status, body) = post_json(
        &app,
        "/v1/api-keys",
        json!({
            "name": "secondary-key",
            "scopes": ["all"]
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create api key: {body}");
    assert!(body["raw_key"].as_str().unwrap().starts_with("sk_"));
    assert_eq!(body["name"], "secondary-key");
    let second_key_id = body["key_id"].as_str().unwrap().to_owned();

    // 3. List API keys
    let (status, body) = get_json(&app, "/v1/api-keys", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "list api keys: {body}");
    let keys = body.as_array().unwrap();
    assert_eq!(keys.len(), 2, "should have 2 keys");

    // 4. Token exchange
    let (status, body) = post_json(
        &app,
        "/v1/auth/token-exchange",
        json!({
            "api_key": api_key,
            "ttl_seconds": 1800
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "token exchange: {body}");
    assert!(body["access_token"].as_str().is_some());
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 1800);

    // 5. Empty workspace name should fail
    let (status, _body) = post_json(
        &app,
        "/v1/workspaces/provision",
        json!({ "name": "" }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "empty name should fail");

    // 6. Verify we can list the workspace via admin
    let (status, body) = get_json(&app, "/v1/admin/workspaces", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "list workspaces: {body}");
    let workspaces = body.as_array().unwrap();
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0]["name"], "Test Workspace");

    // Store second_key_id to suppress warning
    let _ = second_key_id;
}

// ── 16. Agent management lifecycle ──────────────────────────────────

#[tokio::test]
async fn test_agent_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);

    // 1. Provision workspace first (agents live in workspace repos)
    let token = make_token(WorkspaceId::new());
    let (status, ws_body) = post_json(
        &app,
        "/v1/workspaces/provision",
        json!({ "name": "Agent Workspace" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let ws_id_str = ws_body["workspace_id"].as_str().unwrap();

    // Create a token for this specific workspace
    let ws_id: WorkspaceId = ws_id_str.parse().unwrap();
    let ws_token = make_token(ws_id);

    // 2. Create an agent
    let (status, body) = post_json(
        &app,
        "/v1/agents",
        json!({
            "name": "CFO Agent",
            "system_prompt": "You are a helpful CFO assistant.",
            "model": "claude-sonnet-4-6"
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create agent: {body}");
    let agent_id = body["agent_id"].as_str().unwrap();
    assert_eq!(body["name"], "CFO Agent");
    assert_eq!(body["status"], "active");
    assert_eq!(body["model"], "claude-sonnet-4-6");

    // 3. List agents
    let (status, body) = get_json(&app, "/v1/agents", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "list agents: {body}");
    let agents = body.as_array().unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["name"], "CFO Agent");

    // 4. Update agent
    let (status, body) = patch_json(
        &app,
        &format!("/v1/agents/{agent_id}"),
        json!({
            "name": "Updated CFO Agent",
            "webhook_url": "https://hooks.example.com/agent"
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update agent: {body}");
    assert_eq!(body["name"], "Updated CFO Agent");
    assert_eq!(body["webhook_url"], "https://hooks.example.com/agent");

    // 5. Add skill to agent
    let (status, body) = post_json(
        &app,
        &format!("/v1/agents/{agent_id}/skills"),
        json!({
            "name": "financial_analysis",
            "description": "Analyze financial statements"
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "add skill: {body}");
    let skills = body["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "financial_analysis");

    // 6. Send message to agent
    let (status, body) = post_json(
        &app,
        &format!("/v1/agents/{agent_id}/messages"),
        json!({
            "message": "What is the current runway?"
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "send message: {body}");
    assert_eq!(body["status"], "queued");
}

// ── 17. Compliance lifecycle ────────────────────────────────────────

#[tokio::test]
async fn test_compliance_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;

    // 1. File tax document
    let (status, body) = post_json(
        &app,
        "/v1/tax/filings",
        json!({
            "entity_id": entity_id,
            "document_type": "form_1120",
            "tax_year": 2025,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "file tax doc: {body}");
    assert!(body["filing_id"].as_str().is_some());
    assert_eq!(body["document_type"], "form_1120");
    assert_eq!(body["tax_year"], 2025);
    assert_eq!(body["status"], "pending");

    // 2. Create deadline
    let (status, body) = post_json(
        &app,
        "/v1/deadlines",
        json!({
            "entity_id": entity_id,
            "deadline_type": "tax_filing",
            "due_date": "2026-04-15",
            "description": "Annual corporate tax filing due",
            "recurrence": "annual",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create deadline: {body}");
    assert!(body["deadline_id"].as_str().is_some());
    assert_eq!(body["deadline_type"], "tax_filing");
    assert_eq!(body["due_date"], "2026-04-15");
    assert_eq!(body["recurrence"], "annual");

    // 3. Classify contractor
    let (status, body) = post_json(
        &app,
        "/v1/contractors/classify",
        json!({
            "entity_id": entity_id,
            "contractor_name": "Jane Consultant",
            "state": "CA",
            "factors": { "works_for_others": true, "sets_own_hours": true },
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "classify contractor: {body}");
    assert!(body["classification_id"].as_str().is_some());
    assert_eq!(body["contractor_name"], "Jane Consultant");
    assert_eq!(body["state"], "CA");
    // Should have a risk level and classification result
    assert!(body["risk_level"].as_str().is_some());
    assert!(body["classification"].as_str().is_some());
}

// ── 18. Billing lifecycle ───────────────────────────────────────────

#[tokio::test]
async fn test_billing_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let token = make_token(WorkspaceId::new());

    // 1. List plans (no workspace needed)
    let (status, body) = get_json(&app, "/v1/billing/plans", &token).await;
    assert_eq!(status, StatusCode::OK, "list plans: {body}");
    let plans = body.as_array().unwrap();
    assert!(plans.len() >= 3, "should have at least 3 plans");
    assert!(plans.iter().any(|p| p["plan_id"] == "free"));
    assert!(plans.iter().any(|p| p["plan_id"] == "pro"));
    assert!(plans.iter().any(|p| p["plan_id"] == "enterprise"));

    // 2. Provision workspace for billing tests
    let (status, ws_body) = post_json(
        &app,
        "/v1/workspaces/provision",
        json!({ "name": "Billing Test WS" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let ws_id_str = ws_body["workspace_id"].as_str().unwrap();

    // Create a token for this specific workspace
    let ws_id: WorkspaceId = ws_id_str.parse().unwrap();
    let ws_token = make_token(ws_id);

    // 3. Checkout (creates subscription stub)
    let (status, body) = post_json(
        &app,
        "/v1/billing/checkout",
        json!({
            "plan_id": "pro",
            "success_url": "https://example.com/success",
            "cancel_url": "https://example.com/cancel"
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "checkout: {body}");
    assert!(body["checkout_url"].as_str().is_some());
    assert!(body["session_id"].as_str().is_some());
    assert_eq!(body["plan_id"], "pro");

    // 4. Billing status
    let (status, body) = get_json(&app, "/v1/billing/status", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "billing status: {body}");
    assert_eq!(body["plan"], "pro");

    // 5. Create subscription
    let (status, body) = post_json(
        &app,
        "/v1/subscriptions",
        json!({
            "plan": "enterprise"
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create subscription: {body}");
    let sub_id = body["subscription_id"].as_str().unwrap();
    assert_eq!(body["plan"], "enterprise");
    assert_eq!(body["status"], "active");

    // 6. Get subscription
    let (status, body) = get_json(&app, &format!("/v1/subscriptions/{sub_id}"), &ws_token).await;
    assert_eq!(status, StatusCode::OK, "get subscription: {body}");
    assert_eq!(body["subscription_id"], sub_id);
    assert_eq!(body["plan"], "enterprise");

    // 7. Tick subscriptions
    let (status, body) = post_json(&app, "/v1/subscriptions/tick", json!({}), &ws_token).await;
    assert_eq!(status, StatusCode::OK, "tick subscriptions: {body}");
    assert!(body["workspaces_processed"].as_i64().is_some());

    // 8. Portal
    let (status, body) = post_json(
        &app,
        "/v1/billing/portal",
        json!({
            "return_url": "https://example.com/return"
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "portal: {body}");
    assert!(body["portal_url"].as_str().is_some());
}

// ── 19. Admin endpoints ─────────────────────────────────────────────

#[tokio::test]
async fn test_admin_endpoints() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let token = make_token(WorkspaceId::new());

    // 1. System health
    let (status, body) = get_json(&app, "/v1/admin/system-health", &token).await;
    assert_eq!(status, StatusCode::OK, "system health: {body}");
    assert_eq!(body["status"], "healthy");
    assert!(body["version"].as_str().is_some());

    // 2. List workspaces (empty initially)
    let (status, body) = get_json(&app, "/v1/admin/workspaces", &token).await;
    assert_eq!(status, StatusCode::OK, "list workspaces empty: {body}");
    assert_eq!(body.as_array().unwrap().len(), 0);

    // 3. Provision a workspace + create an entity
    let (status, ws_body) = post_json(
        &app,
        "/v1/workspaces/provision",
        json!({ "name": "Admin Test WS" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let ws_id_str = ws_body["workspace_id"].as_str().unwrap();

    // Create a token for this specific workspace
    let ws_id: WorkspaceId = ws_id_str.parse().unwrap();
    let ws_token = make_token(ws_id);

    // Create entity in this workspace
    let (status, entity_body) = post_json(
        &app,
        "/v1/formations",
        json!({
            "entity_type": "llc",
            "legal_name": "Admin Test LLC",
            "jurisdiction": "Wyoming",
            "members": [{
                "name": "Owner",
                "investor_type": "natural_person",
                "email": "owner@test.com",
                "ownership_pct": 100.0,
                "share_count": 1000,
                "role": "member"
            }],
            "authorized_shares": 1000000,
            "par_value": "0.001",
        }),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create entity: {entity_body}");

    // 4. List workspaces (now has 1)
    let (status, body) = get_json(&app, "/v1/admin/workspaces", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "list workspaces: {body}");
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["name"], "Admin Test WS");

    // 5. Workspace status by path
    let (status, body) = get_json(
        &app,
        &format!("/v1/workspaces/{ws_id_str}/status"),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "ws status by path: {body}");

    // 6. Workspace entities by path
    let (status, body) = get_json(
        &app,
        &format!("/v1/workspaces/{ws_id_str}/entities"),
        &ws_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "ws entities by path: {body}");
    let entities = body.as_array().unwrap();
    assert_eq!(entities.len(), 1);

    // 7. Audit events
    let (status, body) = get_json(&app, "/v1/admin/audit-events", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "audit events: {body}");
    // Should have events from workspace provisioning and entity creation
    assert!(!body.as_array().unwrap().is_empty());

    // 8. Config
    let (status, body) = get_json(&app, "/v1/config", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "config: {body}");

    // 9. Demo seed
    let (status, body) = post_json(&app, "/v1/demo/seed", json!({}), &ws_token).await;
    assert_eq!(status, StatusCode::OK, "demo seed: {body}");

    // 10. Digests
    let (status, body) = get_json(&app, "/v1/digests", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "list digests: {body}");

    // 11. JWKS
    let (status, body) = get_json(&app, "/v1/jwks", &ws_token).await;
    assert_eq!(status, StatusCode::OK, "jwks: {body}");
    assert!(body["keys"].as_array().is_some());
}

// ── 20. Branch prune endpoint ───────────────────────────────────────

#[tokio::test]
async fn test_branch_prune() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let _ = ws_id;
    let e_query = format!("entity_id={entity_id}");

    // 1. Create a branch
    let (status, body) = post_json(
        &app,
        &format!("/v1/branches?{e_query}"),
        json!({ "name": "feature/prune-test", "from": "main" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create branch: {body}");

    // 2. Verify it exists
    let (status, body) = get_json(&app, &format!("/v1/branches?{e_query}"), &token).await;
    assert_eq!(status, StatusCode::OK);
    let branches: Vec<&Value> = body
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["name"] == "feature/prune-test")
        .collect();
    assert_eq!(branches.len(), 1);

    // 3. Prune (POST alternative to DELETE)
    let (status, _) = post_json(
        &app,
        &format!("/v1/branches/feature%2Fprune-test/prune?{e_query}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "prune branch");

    // 4. Verify it's gone
    let (status, body) = get_json(&app, &format!("/v1/branches?{e_query}"), &token).await;
    assert_eq!(status, StatusCode::OK);
    let branches: Vec<&Value> = body
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["name"] == "feature/prune-test")
        .collect();
    assert_eq!(branches.len(), 0);
}

// ── Three-way merge ──────────────────────────────────────────────────

#[tokio::test]
async fn test_three_way_merge_via_api() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id, token) = create_entity(&app).await;
    let e_query = format!("entity_id={entity_id}");

    // 1. Create a feature branch.
    let (status, body) = post_json(
        &app,
        &format!("/v1/branches?{e_query}"),
        json!({ "name": "feature/diverge", "from": "main" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create branch: {body}");

    // 2. Make divergent commits using the store layer directly.
    //    This simulates two agents editing different files concurrently.
    {
        let layout = api_rs::store::RepoLayout::new(tmp.path().to_path_buf());
        let ws: uuid::Uuid = ws_id.into_uuid();
        let eid: uuid::Uuid = entity_id.parse().unwrap();
        let store =
            api_rs::store::entity_store::EntityStore::open(&layout, ws.into(), eid.into()).unwrap();

        // Commit a new file on main.
        store
            .commit(
                "main",
                "add config on main",
                vec![api_rs::git::commit::FileWrite::raw(
                    "config.json".to_owned(),
                    serde_json::to_vec_pretty(&json!({"setting": "on_main"})).unwrap(),
                )],
            )
            .unwrap();

        // Commit a different new file on feature branch.
        store
            .commit(
                "feature/diverge",
                "add metadata on feature",
                vec![api_rs::git::commit::FileWrite::raw(
                    "metadata.json".to_owned(),
                    serde_json::to_vec_pretty(&json!({"source": "feature"})).unwrap(),
                )],
            )
            .unwrap();
    }

    // 3. Merge feature branch into main via the API.
    let (status, body) = post_json(
        &app,
        &format!("/v1/branches/feature%2Fdiverge/merge?{e_query}"),
        json!({ "into": "main" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "three-way merge: {body}");
    assert_eq!(body["merged"], true);
    assert_eq!(body["strategy"], "three_way");
    assert!(body["commit"].as_str().is_some(), "commit OID present");
}

// ── OpenAPI ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_openapi_spec() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let token = make_token(WorkspaceId::new());

    let (status, spec) = get_json(&app, "/v1/openapi.json", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(spec["openapi"], "3.1.0");
    assert_eq!(spec["info"]["title"], "The Corporation API");

    let paths = spec["paths"].as_object().unwrap();
    assert!(
        paths.len() >= 100,
        "expected >= 100 paths, got {}",
        paths.len()
    );

    // Count total operations (each path can have multiple methods)
    let total_ops: usize = paths
        .values()
        .map(|v| v.as_object().map_or(0, |m| m.len()))
        .sum();
    assert!(
        total_ops >= 115,
        "expected >= 115 operations, got {total_ops}"
    );
}
