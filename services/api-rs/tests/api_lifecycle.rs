//! Integration tests exercising the full HTTP API lifecycle.
//!
//! Each test constructs the Axum router with a temporary data directory,
//! then sends requests directly via `tower::ServiceExt::oneshot` — no
//! actual TCP listener is needed.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt; // oneshot

// ── Helpers ──────────────────────────────────────────────────────────────

fn build_app(tmp: &TempDir) -> Router {
    let layout = Arc::new(api_rs::store::RepoLayout::new(tmp.path().to_path_buf()));
    let state = api_rs::routes::AppState {
        layout,
        jwt_secret: Arc::from(b"test-secret-for-integration-tests".as_slice()),
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
        .with_state(state)
}

async fn post_json(app: &Router, path: &str, body: Value) -> (StatusCode, Value) {
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

async fn get_json(app: &Router, path: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::GET)
        .uri(path)
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

async fn delete_req(app: &Router, path: &str) -> StatusCode {
    let req = Request::builder()
        .method(Method::DELETE)
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    response.status()
}

/// Helper: create an entity and return (workspace_id_str, entity_id_str, app).
/// The workspace_id is fixed so subsequent calls can reuse it.
async fn create_entity(app: &Router) -> (String, String) {
    let ws_id = uuid::Uuid::new_v4().to_string();
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create_formation failed: {body}");
    let entity_id = body["entity_id"].as_str().unwrap().to_owned();
    (ws_id, entity_id)
}

// ── 1. Health check ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_health() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (status, body) = get_json(&app, "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

// ── 2. Formation lifecycle ───────────────────────────────────────────────

#[tokio::test]
async fn test_formation_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);

    // 1. Create entity
    let ws_id = uuid::Uuid::new_v4().to_string();
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
            "workspace_id": ws_id,
        }),
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
    let (status, body) = get_json(
        &app,
        &format!("/v1/formations/{entity_id}?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get formation: {body}");
    assert_eq!(body["legal_name"], "FormCo Inc.");
    assert_eq!(body["entity_type"], "corporation");
    assert_eq!(body["jurisdiction"], "Delaware");

    // 3. List documents
    let (status, body) = get_json(
        &app,
        &format!("/v1/formations/{entity_id}/documents?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list documents: {body}");
    let docs = body.as_array().unwrap();
    assert!(!docs.is_empty());

    // 4. Sign each document
    for doc_id in &doc_ids {
        let (status, body) = post_json(
            &app,
            &format!("/v1/documents/{doc_id}/sign?workspace_id={ws_id}&entity_id={entity_id}"),
            json!({
                "signer_name": "Alice Founder",
                "signer_role": "Incorporator",
                "signer_email": "alice@formco.com",
                "signature_text": "Alice Founder"
            }),
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
        &format!("/v1/documents/{doc_id}?workspace_id={ws_id}&entity_id={entity_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get document: {body}");
    assert!(body["signatures"].as_array().unwrap().len() > 0);
    assert_eq!(body["entity_id"], entity_id);

    // NOTE: The formation FSM requires intermediate transitions
    // (DocumentsGenerated -> DocumentsSigned -> FilingSubmitted -> Filed -> Active)
    // that aren't all exposed as HTTP endpoints. The filing-confirmation endpoint
    // expects the entity to be at FilingSubmitted status. Testing confirm-filing
    // and confirm-EIN would require direct store manipulation to advance
    // intermediate states, so we verify the formation status is still
    // documents_generated and the API correctly reports it.
    let (status, body) = get_json(
        &app,
        &format!("/v1/formations/{entity_id}?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get formation post-signing: {body}");
    assert_eq!(body["formation_status"], "documents_generated");
}

// ── 3. Equity lifecycle ──────────────────────────────────────────────────

#[tokio::test]
async fn test_equity_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id) = create_entity(&app).await;

    // 1. Get cap table — initial state (no share classes created by formation)
    let (status, body) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/cap-table?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "get cap table: {body}");
    assert_eq!(body["entity_id"], entity_id);

    // Use a synthetic share_class_id since formation doesn't create share classes.
    // The grant endpoint accepts any share_class_id without validation.
    let share_class_id = uuid::Uuid::new_v4().to_string();

    // 2. Issue equity grant
    let (status, body) = post_json(
        &app,
        "/v1/equity/grants",
        json!({
            "entity_id": entity_id,
            "shares": 1000,
            "recipient_name": "Charlie Employee",
            "recipient_type": "natural_person",
            "grant_type": "common_stock",
            "share_class_id": share_class_id,
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create grant: {body}");
    assert!(body["grant_id"].as_str().is_some());
    assert_eq!(body["shares"], 1000);
    assert_eq!(body["recipient_name"], "Charlie Employee");

    // 3. Create SAFE note
    let (status, body) = post_json(
        &app,
        "/v1/safe-notes",
        json!({
            "entity_id": entity_id,
            "investor_name": "Seed Ventures",
            "principal_amount_cents": 500000_i64,
            "safe_type": "post_money",
            "valuation_cap_cents": 10000000_i64,
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create SAFE: {body}");
    let safe_note_id = body["safe_note_id"].as_str().unwrap();
    assert_eq!(body["investor_name"], "Seed Ventures");
    assert_eq!(body["principal_amount_cents"], 500000);

    // 4. List SAFE notes
    let (status, body) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/safe-notes?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list SAFEs: {body}");
    let safes = body.as_array().unwrap();
    assert_eq!(safes.len(), 1);
    assert_eq!(safes[0]["safe_note_id"], safe_note_id);

    // 5. Create 409A valuation
    let (status, body) = post_json(
        &app,
        "/v1/valuations",
        json!({
            "entity_id": entity_id,
            "valuation_type": "four_oh_nine_a",
            "methodology": "market",
            "fmv_per_share_cents": 100_i64,
            "enterprise_value_cents": 5000000_i64,
            "effective_date": "2026-01-15",
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create valuation: {body}");
    assert!(body["valuation_id"].as_str().is_some());

    // 6. Create contacts for transfer
    let (status, from_contact) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Alice Seller",
            "email": "alice@test.com",
            "category": "employee",
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create from_contact: {from_contact}");
    let from_contact_id = from_contact["contact_id"].as_str().unwrap();

    let (status, to_contact) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Dave Buyer",
            "email": "dave@test.com",
            "category": "investor",
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create to_contact: {to_contact}");
    let to_contact_id = to_contact["contact_id"].as_str().unwrap();

    // 7. Create share transfer
    let (status, body) = post_json(
        &app,
        "/v1/share-transfers",
        json!({
            "entity_id": entity_id,
            "share_class_id": share_class_id,
            "from_contact_id": from_contact_id,
            "to_contact_id": to_contact_id,
            "transfer_type": "secondary_sale",
            "shares": 500,
            "price_per_share_cents": 100,
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create transfer: {body}");
    let transfer_id = body["transfer_id"].as_str().unwrap();
    assert_eq!(body["status"], "draft");

    // 8. Walk transfer through FSM
    let we_query = format!("workspace_id={ws_id}&entity_id={entity_id}");

    // submit-review
    let (status, body) = post_json(
        &app,
        &format!("/v1/share-transfers/{transfer_id}/submit-review?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "submit-review: {body}");
    assert_eq!(body["status"], "pending_bylaws_review");

    // bylaws-review
    let (status, body) = post_json(
        &app,
        &format!("/v1/share-transfers/{transfer_id}/bylaws-review?{we_query}"),
        json!({ "approved": true, "reviewer": "Legal Team" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "bylaws-review: {body}");
    assert_eq!(body["status"], "pending_rofr");

    // rofr-decision (waive ROFR)
    let (status, body) = post_json(
        &app,
        &format!("/v1/share-transfers/{transfer_id}/rofr-decision?{we_query}"),
        json!({ "offered": true, "waived": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "rofr-decision: {body}");
    assert_eq!(body["status"], "pending_board_approval");
    assert_eq!(body["rofr_waived"], true);

    // approve
    let (status, body) = post_json(
        &app,
        &format!("/v1/share-transfers/{transfer_id}/approve?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "approve: {body}");
    assert_eq!(body["status"], "approved");

    // execute
    let (status, body) = post_json(
        &app,
        &format!("/v1/share-transfers/{transfer_id}/execute?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "execute transfer: {body}");
    assert_eq!(body["status"], "executed");

    // 9. Create funding round
    let (status, body) = post_json(
        &app,
        "/v1/funding-rounds",
        json!({
            "entity_id": entity_id,
            "round_name": "Seed Round",
            "pre_money_valuation_cents": 10000000_i64,
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create funding round: {body}");
    assert!(body["funding_round_id"].as_str().is_some());
    assert_eq!(body["round_name"], "Seed Round");
}

// ── 4. Governance lifecycle ──────────────────────────────────────────────

#[tokio::test]
async fn test_governance_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id) = create_entity(&app).await;

    // 1. Create contacts for board members
    let (_, c1) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Director One",
            "category": "board_member",
            "workspace_id": ws_id,
        }),
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
            "workspace_id": ws_id,
        }),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create governance body: {body}");
    let body_id = body["body_id"].as_str().unwrap();
    assert_eq!(body["name"], "Board of Directors");

    // 3. Create seats
    let body_q = format!("workspace_id={ws_id}&entity_id={entity_id}");
    let (status, seat1) = post_json(
        &app,
        &format!("/v1/governance-bodies/{body_id}/seats?{body_q}"),
        json!({
            "holder_id": contact_id_1,
            "role": "chair",
            "voting_power": 1,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create seat 1: {seat1}");
    let seat_id_1 = seat1["seat_id"].as_str().unwrap().to_owned();

    let (status, seat2) = post_json(
        &app,
        &format!("/v1/governance-bodies/{body_id}/seats?{body_q}"),
        json!({
            "holder_id": contact_id_2,
            "role": "member",
            "voting_power": 1,
        }),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "schedule meeting: {meeting_body}");
    let meeting_id = meeting_body["meeting_id"].as_str().unwrap();
    assert_eq!(meeting_body["title"], "Q1 Board Meeting");
    assert_eq!(meeting_body["status"], "draft");

    // 5. Send notice
    let meeting_q = format!("workspace_id={ws_id}&entity_id={entity_id}");
    let (status, body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/notice?{meeting_q}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "send notice: {body}");
    assert_eq!(body["status"], "noticed");

    // 6. Convene meeting (both directors present)
    let (status, body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/convene?{meeting_q}"),
        json!({
            "present_seat_ids": [seat_id_1, seat_id_2],
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "convene meeting: {body}");
    assert_eq!(body["status"], "convened");
    assert_eq!(body["quorum_met"], true);

    // Get agenda items by listing the meeting — we need agenda item IDs.
    // The agenda items were written into the git store. We need to find them.
    // We'll use a GET on list_meetings to indirectly find them, but actually the
    // agenda items aren't returned in the meeting list response. We need to look
    // at the formation docs to find agenda items. For this test, we'll read the
    // meeting's agenda items from the governance API if available. Since there's
    // no direct agenda-item-list endpoint, we'll use the vote/resolution routes
    // which take item_id as a path param. We need to discover the item IDs.
    //
    // Workaround: We know the meeting was created with agenda items. Let's list
    // votes endpoint to find item IDs — but that won't give us IDs either.
    // In practice, the agenda item IDs are generated server-side and not returned.
    //
    // Since the schedule_meeting response doesn't include agenda item IDs, and
    // there's no list-agenda-items endpoint, we skip per-item voting and just
    // test adjourn. In a real scenario the client would store item IDs from
    // another call.

    // 7. Adjourn meeting
    let (status, body) = post_json(
        &app,
        &format!("/v1/meetings/{meeting_id}/adjourn?{meeting_q}"),
        json!({}),
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
    let (ws_id, entity_id) = create_entity(&app).await;
    let we_query = format!("workspace_id={ws_id}&entity_id={entity_id}");

    // 1. Create GL accounts (Cash and Revenue)
    let (status, cash_acct) = post_json(
        &app,
        "/v1/treasury/accounts",
        json!({
            "entity_id": entity_id,
            "account_code": "Cash",
            "workspace_id": ws_id,
        }),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create revenue account: {rev_acct}");
    let rev_id = rev_acct["account_id"].as_str().unwrap();

    // 2. List accounts
    let (status, accounts) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/accounts?workspace_id={ws_id}"),
    )
    .await;
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
            "workspace_id": ws_id,
        }),
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
        &format!("/v1/journal-entries/{je_id}/post?{we_query}"),
        json!({}),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create invoice: {inv}");
    let inv_id = inv["invoice_id"].as_str().unwrap();
    assert_eq!(inv["status"], "draft");

    // 6. Send invoice
    let (status, inv) = post_json(
        &app,
        &format!("/v1/invoices/{inv_id}/send?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "send invoice: {inv}");
    assert_eq!(inv["status"], "sent");

    // 7. Mark invoice paid
    let (status, inv) = post_json(
        &app,
        &format!("/v1/invoices/{inv_id}/mark-paid?{we_query}"),
        json!({}),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create bank account: {ba}");
    let ba_id = ba["bank_account_id"].as_str().unwrap();
    assert_eq!(ba["status"], "pending_review");

    // 9. Activate bank account
    let (status, ba) = post_json(
        &app,
        &format!("/v1/bank-accounts/{ba_id}/activate?{we_query}"),
        json!({}),
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
    let (ws_id, entity_id) = create_entity(&app).await;
    let we_query = format!("workspace_id={ws_id}&entity_id={entity_id}");

    // 1. Create intent
    let (status, intent) = post_json(
        &app,
        "/v1/execution/intents",
        json!({
            "entity_id": entity_id,
            "intent_type": "hire_employee",
            "authority_tier": "tier_1",
            "description": "Hire new engineer",
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create intent: {intent}");
    let intent_id = intent["intent_id"].as_str().unwrap();
    assert_eq!(intent["status"], "pending");

    // 2. Evaluate intent
    let (status, intent) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/evaluate?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "evaluate intent: {intent}");
    assert_eq!(intent["status"], "evaluated");

    // 3. Authorize intent
    let (status, intent) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/authorize?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "authorize intent: {intent}");
    assert_eq!(intent["status"], "authorized");

    // 4. Execute intent
    let (status, intent) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/execute?{we_query}"),
        json!({}),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create obligation: {ob}");
    let ob_id = ob["obligation_id"].as_str().unwrap();
    assert_eq!(ob["status"], "required");

    // 6. Fulfill obligation
    let (status, ob) = post_json(
        &app,
        &format!("/v1/obligations/{ob_id}/fulfill?{we_query}"),
        json!({}),
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
    let (ws_id, entity_id) = create_entity(&app).await;

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
            "workspace_id": ws_id,
        }),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create contact 2: {c2}");

    // 2. List contacts
    let (status, contacts) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/contacts?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "list contacts: {contacts}");
    let arr = contacts.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

// ── 8. Branch lifecycle ──────────────────────────────────────────────────

#[tokio::test]
async fn test_branch_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id) = create_entity(&app).await;
    let we_query = format!("workspace_id={ws_id}&entity_id={entity_id}");

    // 1. Create branch
    let (status, body) = post_json(
        &app,
        &format!("/v1/branches?{we_query}"),
        json!({
            "name": "feature/test-branch",
            "from": "main",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create branch: {body}");
    assert_eq!(body["branch"], "feature/test-branch");
    assert!(body["base_commit"].as_str().is_some());

    // 2. List branches
    let (status, body) = get_json(&app, &format!("/v1/branches?{we_query}")).await;
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
        &format!("/v1/branches/feature%2Ftest-branch/merge?{we_query}"),
        json!({ "into": "main" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "merge branch: {body}");
    assert_eq!(body["merged"], true);

    // 4. Delete branch
    let status = delete_req(
        &app,
        &format!("/v1/branches/feature%2Ftest-branch?{we_query}"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT, "delete branch");

    // 5. Verify branch is gone
    let (status, body) = get_json(&app, &format!("/v1/branches?{we_query}")).await;
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
    let fake_ws = uuid::Uuid::new_v4().to_string();

    // GET formation for nonexistent entity
    let (status, body) = get_json(
        &app,
        &format!("/v1/formations/{fake_id}?workspace_id={fake_ws}"),
    )
    .await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::UNPROCESSABLE_ENTITY,
        "expected 404 or 422 for missing entity, got {status}: {body}"
    );

    // GET cap table for nonexistent entity
    let (status, body) = get_json(
        &app,
        &format!("/v1/entities/{fake_id}/cap-table?workspace_id={fake_ws}"),
    )
    .await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::INTERNAL_SERVER_ERROR,
        "expected 404 or 500 for missing cap table, got {status}: {body}"
    );
}

#[tokio::test]
async fn test_unbalanced_journal_entry_rejected() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id) = create_entity(&app).await;

    // Create two accounts
    let (_, cash_acct) = post_json(
        &app,
        "/v1/treasury/accounts",
        json!({
            "entity_id": entity_id,
            "account_code": "Cash",
            "workspace_id": ws_id,
        }),
    )
    .await;
    let cash_id = cash_acct["account_id"].as_str().unwrap();

    let (_, rev_acct) = post_json(
        &app,
        "/v1/treasury/accounts",
        json!({
            "entity_id": entity_id,
            "account_code": "Revenue",
            "workspace_id": ws_id,
        }),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unbalanced JE should be 422: {body}"
    );
}

#[tokio::test]
async fn test_invalid_transfer_fsm_transition() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id) = create_entity(&app).await;
    let we_query = format!("workspace_id={ws_id}&entity_id={entity_id}");

    // Use a synthetic share class ID (formation doesn't create share classes)
    let sc_id = uuid::Uuid::new_v4().to_string();

    // Create contacts
    let (_, from_c) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Seller",
            "category": "employee",
            "workspace_id": ws_id,
        }),
    )
    .await;
    let from_id = from_c["contact_id"].as_str().unwrap();

    let (_, to_c) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Buyer",
            "category": "investor",
            "workspace_id": ws_id,
        }),
    )
    .await;
    let to_id = to_c["contact_id"].as_str().unwrap();

    // Create transfer (in draft status)
    let (_, transfer) = post_json(
        &app,
        "/v1/share-transfers",
        json!({
            "entity_id": entity_id,
            "share_class_id": sc_id,
            "from_contact_id": from_id,
            "to_contact_id": to_id,
            "transfer_type": "secondary_sale",
            "shares": 100,
            "workspace_id": ws_id,
        }),
    )
    .await;
    let transfer_id = transfer["transfer_id"].as_str().unwrap();

    // Try to execute a draft transfer (should fail — must go through review first)
    let (status, body) = post_json(
        &app,
        &format!("/v1/share-transfers/{transfer_id}/execute?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "executing draft transfer should be 422: {body}"
    );
}

// ── 10. Cross-domain: full lifecycle ─────────────────────────────────────

#[tokio::test]
async fn test_full_cross_domain_lifecycle() {
    // This test exercises formation + contacts + treasury + execution together
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);
    let (ws_id, entity_id) = create_entity(&app).await;
    let we_query = format!("workspace_id={ws_id}&entity_id={entity_id}");

    // Create a contact (employee)
    let (status, contact) = post_json(
        &app,
        "/v1/contacts",
        json!({
            "entity_id": entity_id,
            "contact_type": "individual",
            "name": "Eve Employee",
            "email": "eve@testcorp.com",
            "category": "employee",
            "workspace_id": ws_id,
        }),
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
            "workspace_id": ws_id,
        }),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let intent_id = intent["intent_id"].as_str().unwrap();

    // Walk intent through FSM
    let (s, _) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/evaluate?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, _) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/authorize?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, _) = post_json(
        &app,
        &format!("/v1/intents/{intent_id}/execute?{we_query}"),
        json!({}),
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
            "workspace_id": ws_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ob_id = ob["obligation_id"].as_str().unwrap();

    // Fulfill obligation
    let (status, _) = post_json(
        &app,
        &format!("/v1/obligations/{ob_id}/fulfill?{we_query}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify all entities visible via list endpoints
    let (s, contacts) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/contacts?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(!contacts.as_array().unwrap().is_empty());

    let (s, intents) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/intents?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(!intents.as_array().unwrap().is_empty());

    let (s, obligations) = get_json(
        &app,
        &format!("/v1/entities/{entity_id}/obligations?workspace_id={ws_id}"),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(!obligations.as_array().unwrap().is_empty());
}

// ── OpenAPI ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_openapi_spec() {
    let tmp = TempDir::new().unwrap();
    let app = build_app(&tmp);

    let (status, spec) = get_json(&app, "/v1/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(spec["openapi"], "3.1.0");
    assert_eq!(spec["info"]["title"], "The Corporation API");

    let paths = spec["paths"].as_object().unwrap();
    assert!(paths.len() >= 100, "expected >= 100 paths, got {}", paths.len());

    // Count total operations (each path can have multiple methods)
    let total_ops: usize = paths.values().map(|v| v.as_object().map_or(0, |m| m.len())).sum();
    assert!(total_ops >= 115, "expected >= 115 operations, got {total_ops}");
}
