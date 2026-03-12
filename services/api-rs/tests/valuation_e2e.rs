//! End-to-end valuation lifecycle tests through HTTP routes.

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
    };

    Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(json!({"status": "ok"})) }),
        )
        .merge(api_rs::routes::formation::formation_routes())
        .merge(api_rs::routes::execution::execution_routes())
        .merge(api_rs::routes::governance::governance_routes())
        .merge(api_rs::routes::contacts::contacts_routes())
        .merge(api_rs::routes::equity::equity_routes())
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

async fn sign_all_formation_documents(
    app: &Router,
    entity_id: &str,
    token: &str,
    signer_name: &str,
    signer_email: &str,
) {
    let (status, docs) =
        get_json(app, &format!("/v1/formations/{entity_id}/documents"), token).await;
    assert_eq!(status, StatusCode::OK, "list formation documents failed: {docs}");
    let docs = docs.as_array().expect("documents array");
    for doc in docs {
        let doc_id = doc["document_id"].as_str().expect("document_id");
        let (status, full_doc) = get_json(
            app,
            &format!("/v1/documents/{doc_id}?entity_id={entity_id}"),
            token,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "get document {doc_id} failed: {full_doc}"
        );
        let signer_role = full_doc["content"]["signature_requirements"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|req| req["role"].as_str())
            .unwrap_or("incorporator");

        let (status, body) = post_json(
            app,
            &format!("/v1/documents/{doc_id}/sign?entity_id={entity_id}"),
            json!({
                "signer_name": signer_name,
                "signer_role": signer_role,
                "signer_email": signer_email,
                "signature_text": signer_name,
            }),
            token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "sign document {doc_id} failed: {body}");
    }
}

async fn satisfy_filing_gates(
    app: &Router,
    entity_id: &str,
    token: &str,
    signer_name: &str,
    signer_role: &str,
    signer_email: &str,
) {
    let (status, attestation) = post_json(
        app,
        &format!("/v1/formations/{entity_id}/filing-attestation"),
        json!({
            "signer_name": signer_name,
            "signer_role": signer_role,
            "signer_email": signer_email,
            "consent_text": "I attest the filing information is accurate.",
        }),
        token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "record filing attestation failed: {attestation}"
    );

    let (status, evidence) = post_json(
        app,
        &format!("/v1/formations/{entity_id}/registered-agent-consent-evidence"),
        json!({
            "evidence_uri": "s3://formation/ra-consent.pdf",
            "evidence_type": "registered_agent_consent_pdf",
            "notes": "registered agent engagement executed"
        }),
        token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "record registered-agent consent evidence failed: {evidence}"
    );
}

async fn advance_entity_to_active(app: &Router, entity_id: &str, token: &str) {
    sign_all_formation_documents(app, entity_id, token, "Alice Director", "alice@test.com").await;

    let (status, body) = post_json(
        app,
        &format!("/v1/formations/{entity_id}/mark-documents-signed"),
        json!({}),
        token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "mark documents signed failed: {body}"
    );

    satisfy_filing_gates(
        app,
        entity_id,
        token,
        "Alice Director",
        "director",
        "alice@test.com",
    )
    .await;

    let (status, body) = post_json(
        app,
        &format!("/v1/formations/{entity_id}/submit-filing"),
        json!({}),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "submit filing failed: {body}");

    let (status, body) = post_json(
        app,
        &format!("/v1/formations/{entity_id}/filing-confirmation"),
        json!({
            "external_filing_id": "DE-STATE-ACTIVE",
            "receipt_reference": "RCPT-ACTIVE"
        }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "confirm filing failed: {body}");

    let (status, body) = post_json(
        app,
        &format!("/v1/formations/{entity_id}/apply-ein"),
        json!({}),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "apply ein failed: {body}");

    let (status, body) = post_json(
        app,
        &format!("/v1/formations/{entity_id}/ein-confirmation"),
        json!({
            "ein": "12-3456789"
        }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "confirm ein failed: {body}");
    assert_eq!(body["formation_status"], "active");
}

async fn create_entity(app: &Router) -> (String, String) {
    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);
    let formation_date = (chrono::Utc::now().date_naive() - chrono::Duration::days(180)).to_string();
    let (status, body) = post_json(
        app,
        "/v1/formations",
        json!({
            "entity_type": "corporation",
            "legal_name": "Valuation Test Corp",
            "jurisdiction": "Delaware",
            "formation_date": formation_date,
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
                    "name": "Alice Director",
                    "investor_type": "natural_person",
                    "email": "alice@test.com",
                    "ownership_pct": 50.0,
                    "share_count": 5000,
                    "role": "director",
                    "officer_title": "ceo",
                    "is_incorporator": true,
                    "address": { "street": "2261 Market St", "city": "San Francisco", "state": "CA", "zip": "94114" }
                },
                {
                    "name": "Bob Director",
                    "investor_type": "natural_person",
                    "email": "bob@test.com",
                    "ownership_pct": 50.0,
                    "share_count": 5000,
                    "role": "director",
                    "officer_title": "secretary",
                    "address": { "street": "548 Market St", "city": "San Francisco", "state": "CA", "zip": "94104" }
                }
            ],
            "authorized_shares": 10000000,
            "par_value": "0.0001",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create entity failed: {body}");
    let entity_id = body["entity_id"].as_str().expect("entity_id").to_owned();

    advance_entity_to_active(app, &entity_id, &token).await;

    // Create contacts for board members
    let (status, c1) = post_json(
        app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Alice Director",
            "category": "board_member",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create contact 1 failed: {c1}");
    let contact_id_1 = c1["contact_id"].as_str().expect("contact_id").to_owned();

    let (status, c2) = post_json(
        app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Bob Director",
            "category": "board_member",
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create contact 2 failed: {c2}");
    let contact_id_2 = c2["contact_id"].as_str().expect("contact_id").to_owned();

    // Create governance body (board of directors)
    let (status, gb) = post_json(
        app,
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
    assert_eq!(
        status,
        StatusCode::OK,
        "create governance body failed: {gb}"
    );
    let body_id = gb["body_id"].as_str().expect("body_id").to_owned();

    // Create seats
    let e_query = format!("entity_id={entity_id}");
    let (status, _s1) = post_json(
        app,
        &format!("/v1/governance-bodies/{body_id}/seats?{e_query}"),
        json!({
            "holder_id": contact_id_1,
            "role": "chair",
            "voting_power": 1,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create seat 1 failed: {_s1}");

    let (status, _s2) = post_json(
        app,
        &format!("/v1/governance-bodies/{body_id}/seats?{e_query}"),
        json!({
            "holder_id": contact_id_2,
            "role": "member",
            "voting_power": 1,
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create seat 2 failed: {_s2}");

    (entity_id, token)
}

/// Helper: get governance bodies and seats for an entity
async fn get_governance_info(
    app: &Router,
    entity_id: &str,
    token: &str,
) -> (String, Vec<String>, Vec<String>) {
    let (status, bodies) = get_json(
        app,
        &format!("/v1/entities/{entity_id}/governance-bodies"),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list bodies failed: {bodies}");
    let bodies_arr = bodies.as_array().expect("bodies array");
    assert!(
        !bodies_arr.is_empty(),
        "expected at least one governance body"
    );
    let body_id = bodies_arr[0]["body_id"]
        .as_str()
        .expect("body_id")
        .to_owned();

    let (status, seats) = get_json(
        app,
        &format!("/v1/governance-bodies/{body_id}/seats?entity_id={entity_id}"),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list seats failed: {seats}");
    let seats_arr = seats.as_array().expect("seats array");
    let seat_ids: Vec<String> = seats_arr
        .iter()
        .map(|s| s["seat_id"].as_str().expect("seat_id").to_owned())
        .collect();
    let holder_ids: Vec<String> = seats_arr
        .iter()
        .map(|s| s["holder_id"].as_str().expect("holder_id").to_owned())
        .collect();

    (body_id, seat_ids, holder_ids)
}

/// Run the full board meeting lifecycle on a single agenda item and return the resolution_id.
async fn run_board_approval(
    app: &Router,
    entity_id: &str,
    token: &str,
    meeting_id: &str,
    agenda_item_id: &str,
    seat_ids: &[String],
    holder_ids: &[String],
) -> String {
    // Notice
    let (status, _) = post_json(
        app,
        &format!("/v1/meetings/{meeting_id}/notice?entity_id={entity_id}"),
        json!({}),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "send notice failed");

    // Convene
    let (status, _) = post_json(
        app,
        &format!("/v1/meetings/{meeting_id}/convene?entity_id={entity_id}"),
        json!({ "present_seat_ids": seat_ids }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "convene failed");

    // Vote For on the agenda item
    for holder_id in holder_ids {
        let (status, _) = post_json(
            app,
            &format!(
                "/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/vote?entity_id={entity_id}"
            ),
            json!({ "voter_id": holder_id, "vote_value": "for" }),
            token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "cast vote failed");
    }

    // Compute resolution
    let (status, resolution) = post_json(
        app,
        &format!(
            "/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/resolution?entity_id={entity_id}"
        ),
        json!({ "resolution_text": "Approve 409A valuation" }),
        token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "compute resolution failed: {resolution}"
    );
    assert_eq!(resolution["passed"], true);
    let resolution_id = resolution["resolution_id"]
        .as_str()
        .expect("resolution_id")
        .to_owned();

    // Finalize agenda item
    let (status, _) = post_json(
        app,
        &format!(
            "/v1/meetings/{meeting_id}/agenda-items/{agenda_item_id}/finalize?entity_id={entity_id}"
        ),
        json!({ "entity_id": entity_id, "status": "voted" }),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "finalize item failed");

    // Adjourn
    let (status, _) = post_json(
        app,
        &format!("/v1/meetings/{meeting_id}/adjourn?entity_id={entity_id}"),
        json!({}),
        token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "adjourn failed");

    resolution_id
}

#[tokio::test]
async fn create_409a_with_board_approval() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let (entity_id, token) = create_entity(&app).await;
    let (_body_id, seat_ids, holder_ids) = get_governance_info(&app, &entity_id, &token).await;
    let effective_date = chrono::Utc::now().date_naive() - chrono::Duration::days(30);
    let report_date = effective_date - chrono::Duration::days(5);
    let expected_expiration = effective_date + chrono::Duration::days(365);

    // 1. Create valuation
    let (status, valuation) = post_json(
        &app,
        "/v1/valuations",
        json!({
            "entity_id": entity_id,
            "valuation_type": "four_oh_nine_a",
            "effective_date": effective_date.to_string(),
            "fmv_per_share_cents": 100,
            "enterprise_value_cents": 5000000_00i64,
            "methodology": "market",
            "dlom": "25%",
            "report_date": report_date.to_string()
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create valuation failed: {valuation}"
    );
    assert_eq!(valuation["status"], "draft");
    assert_eq!(valuation["expiration_date"], expected_expiration.to_string());
    let valuation_id = valuation["valuation_id"].as_str().expect("valuation_id");

    // 2. Submit for approval
    let (status, submitted) = post_json(
        &app,
        &format!("/v1/valuations/{valuation_id}/submit-for-approval"),
        json!({ "entity_id": entity_id }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "submit for approval failed: {submitted}"
    );
    assert_eq!(submitted["status"], "pending_approval");
    let meeting_id = submitted["meeting_id"].as_str().expect("meeting_id");
    let agenda_item_id = submitted["agenda_item_id"]
        .as_str()
        .expect("agenda_item_id");

    // 3. Run board approval lifecycle
    let resolution_id = run_board_approval(
        &app,
        &entity_id,
        &token,
        meeting_id,
        agenda_item_id,
        &seat_ids,
        &holder_ids,
    )
    .await;

    // 4. Approve valuation
    let (status, approved) = post_json(
        &app,
        &format!("/v1/valuations/{valuation_id}/approve"),
        json!({ "entity_id": entity_id, "resolution_id": resolution_id }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "approve valuation failed: {approved}"
    );
    assert_eq!(approved["status"], "approved");
    assert_eq!(approved["board_approval_resolution_id"], resolution_id);

    // 5. Get current 409A
    let (status, current) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/current-409a"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get current 409a failed: {current}");
    assert_eq!(current["valuation_id"], valuation_id);
}

#[tokio::test]
async fn submit_adds_to_existing_meeting() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let (entity_id, token) = create_entity(&app).await;
    let (body_id, _seat_ids, _holder_ids) = get_governance_info(&app, &entity_id, &token).await;
    let today = chrono::Utc::now().date_naive();
    let meeting_date = today + chrono::Duration::days(30);
    let effective_date = today - chrono::Duration::days(15);
    let report_date = effective_date - chrono::Duration::days(1);

    // 1. Schedule a board meeting with 1 agenda item
    let (status, meeting) = post_json(
        &app,
        "/v1/meetings",
        json!({
            "entity_id": entity_id,
            "body_id": body_id,
            "meeting_type": "board_meeting",
            "title": "Q1 Board Meeting",
            "scheduled_date": meeting_date.to_string(),
            "agenda_item_titles": ["Approve budget"]
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "schedule meeting failed: {meeting}");
    let existing_meeting_id = meeting["meeting_id"].as_str().expect("meeting_id");

    // 2. Create valuation and submit for approval
    let (status, valuation) = post_json(
        &app,
        "/v1/valuations",
        json!({
            "entity_id": entity_id,
            "valuation_type": "four_oh_nine_a",
            "effective_date": effective_date.to_string(),
            "methodology": "income",
            "fmv_per_share_cents": 150,
            "enterprise_value_cents": 7500000_00i64,
            "dlom": "30%",
            "report_date": report_date.to_string()
        }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create valuation failed: {valuation}"
    );
    let valuation_id = valuation["valuation_id"].as_str().expect("valuation_id");

    let (status, submitted) = post_json(
        &app,
        &format!("/v1/valuations/{valuation_id}/submit-for-approval"),
        json!({ "entity_id": entity_id }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "submit failed: {submitted}");

    // 3. Verify it used the existing meeting
    assert_eq!(
        submitted["meeting_id"].as_str().expect("meeting_id"),
        existing_meeting_id,
        "should reuse existing meeting"
    );

    // 4. List agenda items — should be 2
    let (status, items) = get_json(
        &app,
        &format!("/v1/meetings/{existing_meeting_id}/agenda-items?entity_id={entity_id}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list agenda items failed: {items}");
    let items_arr = items.as_array().expect("items array");
    assert_eq!(items_arr.len(), 2, "expected 2 agenda items");
}

#[tokio::test]
async fn approve_supersedes_previous() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let (entity_id, token) = create_entity(&app).await;
    let (_body_id, seat_ids, holder_ids) = get_governance_info(&app, &entity_id, &token).await;
    let today = chrono::Utc::now().date_naive();
    let first_effective = (today - chrono::Duration::days(90)).to_string();
    let second_effective = (today - chrono::Duration::days(30)).to_string();

    // Helper: create, submit, run board approval, and approve a 409A valuation
    async fn create_and_approve_409a(
        app: &Router,
        entity_id: &str,
        token: &str,
        seat_ids: &[String],
        holder_ids: &[String],
        effective_date: &str,
    ) -> String {
        let (status, valuation) = post_json(
            app,
            "/v1/valuations",
            json!({
                "entity_id": entity_id,
                "valuation_type": "four_oh_nine_a",
                "effective_date": effective_date,
                "methodology": "market",
                "fmv_per_share_cents": 100,
                "enterprise_value_cents": 5000000_00i64,
                "dlom": "25%",
                "report_date": effective_date
            }),
            token,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create valuation failed: {valuation}"
        );
        let valuation_id = valuation["valuation_id"]
            .as_str()
            .expect("valuation_id")
            .to_owned();

        let (status, submitted) = post_json(
            app,
            &format!("/v1/valuations/{valuation_id}/submit-for-approval"),
            json!({ "entity_id": entity_id }),
            token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "submit failed: {submitted}");
        let meeting_id = submitted["meeting_id"]
            .as_str()
            .expect("meeting_id")
            .to_owned();
        let agenda_item_id = submitted["agenda_item_id"]
            .as_str()
            .expect("agenda_item_id")
            .to_owned();

        let resolution_id = run_board_approval(
            app,
            entity_id,
            token,
            &meeting_id,
            &agenda_item_id,
            seat_ids,
            holder_ids,
        )
        .await;

        let (status, approved) = post_json(
            app,
            &format!("/v1/valuations/{valuation_id}/approve"),
            json!({ "entity_id": entity_id, "resolution_id": resolution_id }),
            token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "approve failed: {approved}");
        assert_eq!(approved["status"], "approved");

        valuation_id
    }

    // 1. Create and approve first 409A
    let first_id = create_and_approve_409a(
        &app,
        &entity_id,
        &token,
        &seat_ids,
        &holder_ids,
        &first_effective,
    )
    .await;

    // 2. Create and approve second 409A
    let second_id = create_and_approve_409a(
        &app,
        &entity_id,
        &token,
        &seat_ids,
        &holder_ids,
        &second_effective,
    )
    .await;

    // 3. List valuations — first should be superseded, second approved
    let (status, valuations) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/valuations"),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "list valuations failed: {valuations}"
    );
    let valuations_arr = valuations.as_array().expect("valuations array");
    assert_eq!(valuations_arr.len(), 2);

    let first = valuations_arr
        .iter()
        .find(|v| v["valuation_id"].as_str() == Some(&first_id))
        .expect("first valuation");
    let second = valuations_arr
        .iter()
        .find(|v| v["valuation_id"].as_str() == Some(&second_id))
        .expect("second valuation");
    assert_eq!(first["status"], "superseded");
    assert_eq!(second["status"], "approved");

    // 4. Current 409A should return the second
    let (status, current) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/current-409a"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get current 409a failed: {current}");
    assert_eq!(current["valuation_id"].as_str(), Some(second_id.as_str()));
}
