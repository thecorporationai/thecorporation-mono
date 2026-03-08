//! End-to-end governance meeting lifecycle tests through HTTP routes.

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
        .merge(api_rs::routes::contacts::contacts_routes())
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

async fn create_entity(app: &Router) -> (String, String) {
    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);
    let (status, body) = post_json(
        app,
        "/v1/formations",
        json!({
            "entity_type": "corporation",
            "legal_name": "Meeting Test Corp",
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
    // Get governance bodies
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

    // Get seats
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

#[tokio::test]
async fn board_meeting_full_lifecycle() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let (entity_id, token) = create_entity(&app).await;
    let (body_id, seat_ids, holder_ids) = get_governance_info(&app, &entity_id, &token).await;

    // 1. Schedule meeting with 2 agenda items
    let (status, meeting) = post_json(
        &app,
        "/v1/meetings",
        json!({
            "entity_id": entity_id,
            "body_id": body_id,
            "meeting_type": "board_meeting",
            "title": "Q1 Board Meeting",
            "agenda_item_titles": ["Approve budget", "Elect officers"]
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "schedule meeting failed: {meeting}");
    assert_eq!(meeting["status"], "draft");
    let meeting_id = meeting["meeting_id"].as_str().expect("meeting_id");
    let agenda_item_ids: Vec<String> = meeting["agenda_item_ids"]
        .as_array()
        .expect("agenda_item_ids")
        .iter()
        .map(|v| v.as_str().expect("item id").to_owned())
        .collect();
    assert_eq!(agenda_item_ids.len(), 2);

    // 2. Send notice
    let (status, noticed) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/notice?entity_id={entity_id}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "send notice failed: {noticed}");
    assert_eq!(noticed["status"], "noticed");

    // 3. Convene with all seats present
    let (status, convened) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/convene?entity_id={entity_id}"),
        json!({ "present_seat_ids": seat_ids }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "convene failed: {convened}");
    assert_eq!(convened["status"], "convened");
    assert_eq!(convened["quorum_met"], "met");

    // 4. List agenda items
    let (status, items) = get_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/agenda-items?entity_id={entity_id}"),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list agenda items failed: {items}");
    let items_arr = items.as_array().expect("items array");
    assert_eq!(items_arr.len(), 2);

    // 5. Cast votes on item 1 (all vote For)
    for holder_id in &holder_ids {
        let (status, vote) = post_json(
            &app,
            &format!(
                "/v1/meetings/{meeting_id}/agenda-items/{}/vote?entity_id={entity_id}",
                agenda_item_ids[0]
            ),
            json!({
                "voter_id": holder_id,
                "vote_value": "for"
            }),
            &token,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "cast vote failed: {vote}");
    }

    // 6. Compute resolution for item 1
    let (status, resolution) = post_json(
        &app,
        &format!(
            "/v1/meetings/{meeting_id}/agenda-items/{}/resolution?entity_id={entity_id}",
            agenda_item_ids[0]
        ),
        json!({ "resolution_text": "Budget approved for Q1 2026" }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "compute resolution failed: {resolution}"
    );
    assert_eq!(resolution["passed"], true);

    // 7. Finalize item 1 as Voted
    let (status, finalized) = post_json(
        &app,
        &format!(
            "/v1/meetings/{meeting_id}/agenda-items/{}/finalize?entity_id={entity_id}",
            agenda_item_ids[0]
        ),
        json!({ "entity_id": entity_id, "status": "voted" }),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "finalize item 1 failed: {finalized}"
    );
    assert_eq!(finalized["status"], "voted");

    // 8. Finalize item 2 as Tabled
    let (status, tabled) = post_json(
        &app,
        &format!(
            "/v1/meetings/{meeting_id}/agenda-items/{}/finalize?entity_id={entity_id}",
            agenda_item_ids[1]
        ),
        json!({ "entity_id": entity_id, "status": "tabled" }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "finalize item 2 failed: {tabled}");
    assert_eq!(tabled["status"], "tabled");

    // 9. Adjourn meeting
    let (status, adjourned) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/adjourn?entity_id={entity_id}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "adjourn failed: {adjourned}");
    assert_eq!(adjourned["status"], "adjourned");

    // 10. Verify resolutions
    let (status, resolutions) = get_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/resolutions?entity_id={entity_id}"),
        &token,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "list resolutions failed: {resolutions}"
    );
    let res_arr = resolutions.as_array().expect("resolutions array");
    assert_eq!(res_arr.len(), 1);
    assert_eq!(res_arr[0]["passed"], true);
}

#[tokio::test]
async fn written_consent_lifecycle() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let (entity_id, token) = create_entity(&app).await;
    let (body_id, _seat_ids, _holder_ids) = get_governance_info(&app, &entity_id, &token).await;

    // 1. Create written consent
    let (status, consent) = post_json(
        &app,
        "/v1/meetings/written-consent",
        json!({
            "entity_id": entity_id,
            "body_id": body_id,
            "title": "Approve stock option plan",
            "description": "Unanimous written consent to approve 2026 stock option plan"
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "written consent failed: {consent}");
    let meeting_id = consent["meeting_id"].as_str().expect("meeting_id");
    // Written consent starts in Convened status (no physical meeting required)
    assert_eq!(consent["status"], "convened");

    // 2. Adjourn (written consent skips notice/convene steps)
    let (status, adjourned) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/adjourn?entity_id={entity_id}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "adjourn failed: {adjourned}");
    assert_eq!(adjourned["status"], "adjourned");
}

#[tokio::test]
async fn cancel_meeting_test() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let (entity_id, token) = create_entity(&app).await;
    let (body_id, _seat_ids, _holder_ids) = get_governance_info(&app, &entity_id, &token).await;

    // 1. Schedule meeting
    let (status, meeting) = post_json(
        &app,
        "/v1/meetings",
        json!({
            "entity_id": entity_id,
            "body_id": body_id,
            "meeting_type": "board_meeting",
            "title": "Meeting to cancel",
            "agenda_item_titles": ["Item 1"]
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "schedule failed: {meeting}");
    let meeting_id = meeting["meeting_id"].as_str().expect("meeting_id");
    assert_eq!(meeting["status"], "draft");

    // 2. Cancel meeting
    let (status, cancelled) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/cancel?entity_id={entity_id}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "cancel failed: {cancelled}");
    assert_eq!(cancelled["status"], "cancelled");

    // 3. Try to convene — should fail
    let (status, err) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/convene?entity_id={entity_id}"),
        json!({ "present_seat_ids": [] }),
        &token,
    )
    .await;
    assert_ne!(
        status,
        StatusCode::OK,
        "convene should fail on cancelled meeting: {err}"
    );
}

#[tokio::test]
async fn voting_requires_quorum() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let (entity_id, token) = create_entity(&app).await;
    let (body_id, _seat_ids, holder_ids) = get_governance_info(&app, &entity_id, &token).await;

    // 1. Schedule + notice + convene with 0 present seats (quorum not met)
    let (status, meeting) = post_json(
        &app,
        "/v1/meetings",
        json!({
            "entity_id": entity_id,
            "body_id": body_id,
            "meeting_type": "board_meeting",
            "title": "No quorum meeting",
            "agenda_item_titles": ["Test item"]
        }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "schedule failed: {meeting}");
    let meeting_id = meeting["meeting_id"].as_str().expect("meeting_id");
    let agenda_item_ids: Vec<String> = meeting["agenda_item_ids"]
        .as_array()
        .expect("agenda_item_ids")
        .iter()
        .map(|v| v.as_str().expect("item id").to_owned())
        .collect();

    // Send notice
    let (status, _) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/notice?entity_id={entity_id}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Convene with empty present_seat_ids — quorum not met
    let (status, convened) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/convene?entity_id={entity_id}"),
        json!({ "present_seat_ids": [] }),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "convene failed: {convened}");
    assert_eq!(convened["quorum_met"], "not_met");

    // Try to cast a vote — should fail because quorum not met
    let (status, err) = post_json(
        &app,
        &format!(
            "/v1/meetings/{meeting_id}/agenda-items/{}/vote?entity_id={entity_id}",
            agenda_item_ids[0]
        ),
        json!({
            "voter_id": holder_ids[0],
            "vote_value": "for"
        }),
        &token,
    )
    .await;
    assert_ne!(
        status,
        StatusCode::OK,
        "vote should fail without quorum: {err}"
    );
}
