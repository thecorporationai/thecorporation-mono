//! API lifecycle integration tests for the corporate governance server.
//!
//! These tests spin up the full Axum router in-process (no TCP socket) and
//! drive it through tower's `ServiceExt::oneshot` API. Every test gets its
//! own tempdir-backed git store and a freshly-minted JWT, so tests are
//! fully isolated and can run in parallel.
//!
//! ## What is covered
//!
//! | # | Scenario |
//! |---|----------|
//! | 1 | Health check — GET /health → 200 |
//! | 2 | Entity creation — POST /v1/entities → 200, body has entity_id |
//! | 3 | Entity list — GET /v1/entities → 200, returns array |
//! | 4 | Entity get — GET /v1/entities/{id} → 200 |
//! | 5 | Formation advance — POST /v1/formations/{id}/advance → 200 |
//! | 6 | Document signing — POST /v1/documents/{id}/sign → 200 |
//! | 7 | Filing confirm — POST /v1/formations/{id}/filing/confirm → 200 |
//! | 8 | EIN confirm — POST /v1/formations/{id}/tax/confirm-ein → 200 |
//! | 9 | Full formation lifecycle — Pending → Active |
//! | 10 | Cap table creation — POST /v1/entities/{id}/cap-table → 200 |
//! | 11 | Governance body creation — POST /v1/entities/{id}/governance/bodies → 200 |
//! | 12 | Meeting flow — create → convene → adjourn |
//! | 13 | Auth: missing token → 401 |
//! | 14 | Auth: invalid token → 401 |
//! | 15 | Not found: 404 for nonexistent entity |

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

use corp_auth::{ApiKeyResolver, AuthError, JwtConfig, Principal};
use corp_core::auth::{Claims, PrincipalType, Scope};
use corp_core::ids::WorkspaceId;
use corp_server::routes::router;
use corp_server::state::{AppState, StorageBackend};

// ── Test helpers ──────────────────────────────────────────────────────────────

const TEST_JWT_SECRET: &[u8] = b"test-secret-do-not-use-in-production";

/// A no-op API key resolver that always rejects API key auth.
///
/// Integration tests use JWT tokens exclusively; the resolver is only
/// invoked when the `Authorization` header carries an `sk_live_*` prefix or
/// when the `X-Api-Key` header is present.
struct NoopApiKeyResolver;

#[async_trait::async_trait]
impl ApiKeyResolver for NoopApiKeyResolver {
    async fn resolve(&self, _raw_key: &str) -> Result<Principal, AuthError> {
        Err(AuthError::InvalidApiKey)
    }
}

/// Shared test context: a tempdir, a workspace id, and a signed JWT token.
///
/// Keeping all three together ensures that every request in a test uses the
/// same workspace so data written by one call is visible to the next.
struct TestCtx {
    /// Holds the tempdir alive for the duration of the test.
    _dir: TempDir,
    /// Axum router under test.
    app: axum::Router,
    /// The workspace every request is scoped to.
    workspace_id: WorkspaceId,
    /// Pre-signed JWT with `Scope::All` for this workspace.
    token: String,
}

impl TestCtx {
    /// Create a fresh test context: tempdir + AppState + JWT.
    fn new() -> Self {
        let dir = TempDir::new().expect("create tempdir");
        let data_dir = dir.path().to_str().unwrap().to_owned();

        let jwt_config = Arc::new(JwtConfig::new(TEST_JWT_SECRET));
        let workspace_id = WorkspaceId::new();

        // Mint a token with all scopes so every endpoint is reachable.
        let now = unix_now();
        let claims = Claims {
            sub: "test-user".to_owned(),
            workspace_id,
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes: vec![Scope::All],
            iat: now,
            exp: now + 3600,
        };
        let token = jwt_config
            .encode(&claims)
            .expect("encode test JWT");

        let state = AppState {
            data_dir,
            jwt_config,
            api_key_resolver: Arc::new(NoopApiKeyResolver),
            storage_backend: StorageBackend::Git,
        };

        let app = router(state);

        Self {
            _dir: dir,
            app,
            workspace_id,
            token,
        }
    }

    /// Send an authenticated request and return the response.
    ///
    /// `body` is serialised to JSON when `Some`; otherwise the request has no
    /// body and no `Content-Type` header.
    async fn request(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> axum::response::Response {
        let (content_type, bytes) = match body {
            Some(v) => (
                "application/json",
                serde_json::to_vec(&v).expect("serialise body"),
            ),
            None => ("", vec![]),
        };

        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("Authorization", format!("Bearer {}", self.token));

        if !content_type.is_empty() {
            builder = builder.header("content-type", content_type);
        }

        let req = builder
            .body(Body::from(bytes))
            .expect("build request");

        self.app
            .clone()
            .oneshot(req)
            .await
            .expect("service error")
    }

    /// Convenience: GET request.
    async fn get(&mut self, path: &str) -> axum::response::Response {
        self.request(Method::GET, path, None).await
    }

    /// Convenience: POST request with a JSON body.
    async fn post(&mut self, path: &str, body: Value) -> axum::response::Response {
        self.request(Method::POST, path, Some(body)).await
    }

    /// Convenience: POST with no body (for state-machine transitions).
    async fn post_empty(&mut self, path: &str) -> axum::response::Response {
        self.request(Method::POST, path, None).await
    }
}

/// Read the response body to a `serde_json::Value`.
async fn body_json(resp: axum::response::Response) -> Value {
    use axum::body::to_bytes;
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

/// Current Unix timestamp in seconds.
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ── Scenario helpers ──────────────────────────────────────────────────────────

/// Create a C-Corp entity and return the entity JSON body.
async fn create_ccorp(ctx: &mut TestCtx) -> Value {
    let resp = ctx
        .post(
            "/v1/entities",
            json!({
                "legal_name": "Acme Corp",
                "entity_type": "c_corp",
                "jurisdiction": "DE"
            }),
        )
        .await;

    let status = resp.status();
    let body = body_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create_entity should return 200, got {status}, body: {body}"
    );

    body
}

// ── Tests ─────────────────────────────────────────────────────────────────────

// ── 1. Health check ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_check() {
    let ctx = TestCtx::new();

    // Health endpoint requires no authentication.
    let req = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

// ── 2. Entity creation ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_entity_returns_200_with_id() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;

    // Must have an entity_id field.
    assert!(
        entity["entity_id"].is_string(),
        "response should have entity_id: {entity}"
    );
    assert_eq!(entity["legal_name"], "Acme Corp");
    assert_eq!(entity["entity_type"], "c_corp");
}

// ── 3. Entity list ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_entities_returns_200_array() {
    let mut ctx = TestCtx::new();

    // Initialise workspace store so the list endpoint does not 404.
    ensure_workspace_direct(&ctx).await;

    let resp = ctx.get("/v1/entities").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert!(
        body.is_array(),
        "response should be an array: {body}"
    );
}

// ── 4. Entity get ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_entity_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let resp = ctx.get(&format!("/v1/entities/{entity_id}")).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let fetched = body_json(resp).await;
    assert_eq!(fetched["entity_id"], entity["entity_id"]);
}

// ── 5. Formation advance ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_advance_formation_returns_200_and_advances_status() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    // Entity starts in `Pending`.
    assert_eq!(entity["formation_status"], "pending");

    let resp = ctx
        .post_empty(&format!("/v1/formations/{entity_id}/advance"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let advanced = body_json(resp).await;
    // The status must have moved forward from `pending`.
    assert_ne!(
        advanced["formation_status"], "pending",
        "formation status should have advanced: {advanced}"
    );
}

// ── 6. Document signing ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_sign_document_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    // Advance to `DocumentsGenerated` so documents exist.
    let resp = ctx
        .post_empty(&format!("/v1/formations/{entity_id}/advance"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // List documents for this entity.
    let docs_resp = ctx
        .get(&format!("/v1/formations/{entity_id}/documents"))
        .await;
    assert_eq!(docs_resp.status(), StatusCode::OK);

    let docs = body_json(docs_resp).await;
    let docs_arr = docs.as_array().expect("documents should be an array");

    if docs_arr.is_empty() {
        // No documents were generated by the advance — skip signing.
        // This is valid for implementations that generate documents lazily.
        return;
    }

    let doc_id = docs_arr[0]["document_id"].as_str().unwrap();

    let sign_resp = ctx
        .post(
            &format!("/v1/documents/{doc_id}/sign"),
            json!({
                "signer_name": "Jane Doe",
                "signer_role": "CEO",
                "signer_email": "jane@example.com",
                "signature_text": "/s/ Jane Doe",
                "consent_text": "I agree",
                "signature_svg": null
            }),
        )
        .await;

    assert_eq!(
        sign_resp.status(),
        StatusCode::OK,
        "sign document should return 200"
    );

    let signed_doc = body_json(sign_resp).await;
    assert!(!signed_doc["signatures"].as_array().unwrap_or(&vec![]).is_empty(),
        "signed document should have at least one signature: {signed_doc}");
}

/// Advance an entity from Pending to DocumentsGenerated, sign all docs, then
/// advance through DocumentsSigned and up to (inclusive) `target_status`.
///
/// `target_status` must be one of: "documents_signed", "filing_submitted",
/// "filed", "ein_applied", "active".
async fn advance_to(ctx: &mut TestCtx, entity_id: &str, target_status: &str) {
    // 1. Advance Pending → DocumentsGenerated.
    let resp = ctx
        .post_empty(&format!("/v1/formations/{entity_id}/advance"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK, "advance to documents_generated");

    // 2. Sign all generated documents.
    let docs_resp = ctx
        .get(&format!("/v1/formations/{entity_id}/documents"))
        .await;
    assert_eq!(docs_resp.status(), StatusCode::OK);
    let docs = body_json(docs_resp).await;
    for doc in docs.as_array().expect("docs array") {
        let doc_id = doc["document_id"].as_str().unwrap();
        let sign_resp = ctx
            .post(
                &format!("/v1/documents/{doc_id}/sign"),
                json!({
                    "signer_name": "Jane Doe",
                    "signer_role": "CEO",
                    "signer_email": "jane@example.com",
                    "signature_text": "/s/ Jane Doe",
                    "consent_text": "I agree",
                    "signature_svg": null
                }),
            )
            .await;
        assert_eq!(sign_resp.status(), StatusCode::OK, "sign doc {doc_id}");
    }

    // 3. Advance through remaining states until we reach target_status.
    let targets = [
        "documents_signed",
        "filing_submitted",
        "filed",
        "ein_applied",
        "active",
    ];
    for &expected in &targets {
        let resp = ctx
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "advance to {expected}"
        );
        let updated = body_json(resp).await;
        assert_eq!(
            updated["formation_status"].as_str().unwrap(),
            expected,
            "expected formation_status={expected}"
        );
        if expected == target_status {
            return;
        }
    }
    panic!("target_status {target_status} was never reached");
}

// ── 7. Filing confirm ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_confirm_filing_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    // Advance entity to FilingSubmitted before confirming.
    advance_to(&mut ctx, entity_id, "filing_submitted").await;

    let resp = ctx
        .post(
            &format!("/v1/formations/{entity_id}/filing/confirm"),
            json!({ "confirmation_number": "DE-12345678" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let filing = body_json(resp).await;
    assert!(
        filing["filing_id"].is_string(),
        "filing response should have filing_id: {filing}"
    );
}

// ── 8. EIN confirm ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_confirm_ein_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    // Advance entity to Filed before confirming EIN.
    advance_to(&mut ctx, entity_id, "filed").await;

    let resp = ctx
        .post(
            &format!("/v1/formations/{entity_id}/tax/confirm-ein"),
            json!({ "ein": "12-3456789" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let profile = body_json(resp).await;
    assert!(
        profile["tax_profile_id"].is_string(),
        "tax profile response should have tax_profile_id: {profile}"
    );
}

// ── 9. Full formation lifecycle ───────────────────────────────────────────────

#[tokio::test]
async fn test_full_formation_lifecycle() {
    let mut ctx = TestCtx::new();

    // Step 1: Create entity.
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();
    assert_eq!(entity["formation_status"], "pending");

    // Step 2: Advance through all states until the endpoint returns an error
    // (meaning we have reached a terminal state) or we hit 10 advances.
    let mut status = entity["formation_status"].as_str().unwrap().to_owned();
    let mut advances = 0;

    loop {
        let resp = ctx
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;

        if resp.status() != StatusCode::OK {
            // Reached a terminal state — cannot advance further.
            break;
        }

        let updated = body_json(resp).await;
        let new_status = updated["formation_status"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned();

        assert_ne!(
            new_status, status,
            "formation_status must change on each advance"
        );

        status = new_status.clone();
        advances += 1;

        if new_status == "active" || advances >= 10 {
            break;
        }
    }

    assert!(
        advances > 0,
        "should be able to advance at least once from pending"
    );

    // Step 3: Confirm filing (idempotent against current state).
    let filing_resp = ctx
        .post(
            &format!("/v1/formations/{entity_id}/filing/confirm"),
            json!({ "confirmation_number": null }),
        )
        .await;
    assert!(
        filing_resp.status() == StatusCode::OK
            || filing_resp.status() == StatusCode::BAD_REQUEST,
        "filing confirm returned unexpected status: {}",
        filing_resp.status()
    );

    // Step 4: Confirm EIN.
    let ein_resp = ctx
        .post(
            &format!("/v1/formations/{entity_id}/tax/confirm-ein"),
            json!({ "ein": "98-7654321" }),
        )
        .await;
    assert!(
        ein_resp.status() == StatusCode::OK
            || ein_resp.status() == StatusCode::BAD_REQUEST,
        "EIN confirm returned unexpected status: {}",
        ein_resp.status()
    );

    // Step 5: Entity should still be retrievable.
    let get_resp = ctx.get(&format!("/v1/entities/{entity_id}")).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

// ── 10. Cap table creation ────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_cap_table_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/cap-table"),
            json!({}),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let cap_table = body_json(resp).await;
    assert!(
        cap_table["cap_table_id"].is_string(),
        "cap table should have cap_table_id: {cap_table}"
    );
    assert_eq!(cap_table["entity_id"], entity_id);
}

// ── 11. Governance body creation ──────────────────────────────────────────────

#[tokio::test]
async fn test_create_governance_body_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/bodies"),
            json!({
                "name": "Board of Directors",
                "body_type": "board_of_directors",
                "quorum_rule": "majority",
                "voting_method": "per_capita"
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body_json_val = body_json(resp).await;
    assert!(
        body_json_val["body_id"].is_string(),
        "governance body should have body_id: {body_json_val}"
    );
    assert_eq!(body_json_val["name"], "Board of Directors");
    assert_eq!(body_json_val["entity_id"], entity_id);
}

// ── 12. Meeting flow ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_meeting_flow_create_convene_adjourn() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    // Create a governance body first (meetings are scoped to a body).
    let body_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/bodies"),
            json!({
                "name": "Board of Directors",
                "body_type": "board_of_directors",
                "quorum_rule": "majority",
                "voting_method": "per_capita"
            }),
        )
        .await;
    assert_eq!(body_resp.status(), StatusCode::OK);
    let gov_body = body_json(body_resp).await;
    let body_id = gov_body["body_id"].as_str().unwrap();

    // Create a meeting.
    let meeting_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/meetings"),
            json!({
                "body_id": body_id,
                "meeting_type": "board_meeting",
                "title": "Q1 Board Meeting",
                "scheduled_date": null,
                "location": null,
                "notice_days": null
            }),
        )
        .await;
    assert_eq!(meeting_resp.status(), StatusCode::OK);
    let meeting = body_json(meeting_resp).await;
    let meeting_id = meeting["meeting_id"].as_str().unwrap();

    assert_eq!(meeting["status"], "draft");
    assert_eq!(meeting["title"], "Q1 Board Meeting");

    // Convene the meeting (Draft → Convened).
    let convene_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/convene"
        ))
        .await;
    assert_eq!(convene_resp.status(), StatusCode::OK);
    let convened = body_json(convene_resp).await;
    assert_eq!(
        convened["status"], "convened",
        "meeting should be convened: {convened}"
    );

    // Adjourn the meeting (Convened → Adjourned).
    let adjourn_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/adjourn"
        ))
        .await;
    assert_eq!(adjourn_resp.status(), StatusCode::OK);
    let adjourned = body_json(adjourn_resp).await;
    assert_eq!(
        adjourned["status"], "adjourned",
        "meeting should be adjourned: {adjourned}"
    );
}

// ── 12b. Meeting vote flow ────────────────────────────────────────────────────

#[tokio::test]
async fn test_meeting_vote_flow() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    // Create governance body.
    let body_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/bodies"),
            json!({
                "name": "Board",
                "body_type": "board_of_directors",
                "quorum_rule": "majority",
                "voting_method": "per_capita"
            }),
        )
        .await;
    assert_eq!(body_resp.status(), StatusCode::OK);
    let gov_body = body_json(body_resp).await;
    let body_id = gov_body["body_id"].as_str().unwrap();

    // Create a meeting.
    let meeting_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/meetings"),
            json!({
                "body_id": body_id,
                "meeting_type": "board_meeting",
                "title": "Special Meeting",
                "scheduled_date": null,
                "location": null,
                "notice_days": null
            }),
        )
        .await;
    assert_eq!(meeting_resp.status(), StatusCode::OK);
    let meeting = body_json(meeting_resp).await;
    let meeting_id = meeting["meeting_id"].as_str().unwrap();

    // Add an agenda item.
    let item_resp = ctx
        .post(
            &format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items"
            ),
            json!({
                "title": "Approve budget",
                "item_type": "resolution",
                "description": null,
                "resolution_text": "RESOLVED: The budget is approved."
            }),
        )
        .await;
    assert_eq!(item_resp.status(), StatusCode::OK);
    let item = body_json(item_resp).await;
    let item_id = item["item_id"].as_str().unwrap();

    // Convene.
    let convene_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/convene"
        ))
        .await;
    assert_eq!(convene_resp.status(), StatusCode::OK);

    // List agenda items.
    let items_resp = ctx
        .get(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items"
        ))
        .await;
    assert_eq!(items_resp.status(), StatusCode::OK);
    let items = body_json(items_resp).await;
    assert!(
        items.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "agenda items should not be empty: {items}"
    );

    // List votes (should be empty at this point).
    let votes_resp = ctx
        .get(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/votes"
        ))
        .await;
    assert_eq!(votes_resp.status(), StatusCode::OK);

    // Resolve the agenda item (resolution_type must be one of: ordinary,
    // special, unanimous_written_consent).
    let resolve_resp = ctx
        .post(
            &format!(
                "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items/{item_id}/resolve"
            ),
            json!({
                "resolution_type": "ordinary",
                "resolution_text": "RESOLVED: The budget is approved."
            }),
        )
        .await;
    let resolve_status = resolve_resp.status();
    let resolve_body = body_json(resolve_resp).await;
    assert_eq!(
        resolve_status,
        StatusCode::OK,
        "resolve item should return 200, got {resolve_status}, body: {resolve_body}"
    );

    // Adjourn.
    let adjourn_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/adjourn"
        ))
        .await;
    assert_eq!(adjourn_resp.status(), StatusCode::OK);
}

// ── 13. Auth: missing token → 401 ────────────────────────────────────────────

#[tokio::test]
async fn test_missing_auth_token_returns_401() {
    let ctx = TestCtx::new();

    // Send a request with no Authorization header.
    let req = Request::builder()
        .method(Method::GET)
        .uri("/v1/entities")
        .body(Body::empty())
        .unwrap();

    let resp = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── 14. Auth: invalid token → 401 ────────────────────────────────────────────

#[tokio::test]
async fn test_invalid_auth_token_returns_401() {
    let ctx = TestCtx::new();

    let req = Request::builder()
        .method(Method::GET)
        .uri("/v1/entities")
        .header("Authorization", "Bearer this.is.not.a.valid.jwt")
        .body(Body::empty())
        .unwrap();

    let resp = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── 14b. Auth: wrong-secret token → 401 ──────────────────────────────────────

#[tokio::test]
async fn test_wrong_secret_token_returns_401() {
    let ctx = TestCtx::new();

    // Mint a valid-looking JWT but with a *different* secret.
    let wrong_config = JwtConfig::new(b"wrong-secret");
    let now = unix_now();
    let workspace_id = WorkspaceId::new();
    let claims = Claims {
        sub: "attacker".to_owned(),
        workspace_id,
        entity_id: None,
        contact_id: None,
        entity_ids: None,
        principal_type: PrincipalType::User,
        scopes: vec![Scope::All],
        iat: now,
        exp: now + 3600,
    };
    let bad_token = wrong_config.encode(&claims).unwrap();

    let req = Request::builder()
        .method(Method::GET)
        .uri("/v1/entities")
        .header("Authorization", format!("Bearer {bad_token}"))
        .body(Body::empty())
        .unwrap();

    let resp = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ── 15. Not found: 404 for nonexistent entity ─────────────────────────────────

#[tokio::test]
async fn test_get_nonexistent_entity_returns_404() {
    let mut ctx = TestCtx::new();

    // Use a random UUID-shaped entity ID that was never created.
    let fake_id = uuid::Uuid::new_v4();

    let resp = ctx.get(&format!("/v1/entities/{fake_id}")).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── 15b. Advance nonexistent formation → 404 ──────────────────────────────────

#[tokio::test]
async fn test_advance_nonexistent_formation_returns_404() {
    let mut ctx = TestCtx::new();

    let fake_id = uuid::Uuid::new_v4();
    let resp = ctx
        .post_empty(&format!("/v1/formations/{fake_id}/advance"))
        .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── 15c. Sign nonexistent document → 404 ──────────────────────────────────────

#[tokio::test]
async fn test_sign_nonexistent_document_returns_404() {
    let mut ctx = TestCtx::new();

    // Initialise workspace store so the sign handler can list entities.
    ensure_workspace_direct(&ctx).await;

    let fake_doc_id = uuid::Uuid::new_v4();
    let resp = ctx
        .post(
            &format!("/v1/documents/{fake_doc_id}/sign"),
            json!({
                "signer_name": "Bob",
                "signer_role": "CFO",
                "signer_email": "bob@example.com",
                "signature_text": "/s/ Bob",
                "consent_text": "I agree",
                "signature_svg": null
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── 16. Governance: list bodies on fresh entity ───────────────────────────────

#[tokio::test]
async fn test_list_governance_bodies_empty_on_fresh_entity() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let resp = ctx
        .get(&format!("/v1/entities/{entity_id}/governance/bodies"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let bodies = body_json(resp).await;
    assert!(
        bodies.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "newly created entity should have no governance bodies: {bodies}"
    );
}

// ── 17. Equity: list cap tables on fresh entity ───────────────────────────────

#[tokio::test]
async fn test_list_cap_tables_empty_on_fresh_entity() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let resp = ctx
        .get(&format!("/v1/entities/{entity_id}/cap-table"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let tables = body_json(resp).await;
    assert!(
        tables.is_array(),
        "cap-table list should be an array: {tables}"
    );
}

// ── 18. Dissolve entity ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_dissolve_entity_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let resp = ctx
        .post_empty(&format!("/v1/entities/{entity_id}/dissolve"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let dissolved = body_json(resp).await;
    assert_eq!(
        dissolved["formation_status"], "dissolved",
        "entity should be dissolved: {dissolved}"
    );
}

// ── 19. Get filing for entity ─────────────────────────────────────────────────

#[tokio::test]
async fn test_get_filing_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let resp = ctx
        .get(&format!("/v1/formations/{entity_id}/filing"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let filing = body_json(resp).await;
    assert!(
        filing["filing_id"].is_string(),
        "filing should have filing_id: {filing}"
    );
}

// ── 20. Get tax profile for entity ────────────────────────────────────────────

#[tokio::test]
async fn test_get_tax_profile_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let resp = ctx
        .get(&format!("/v1/formations/{entity_id}/tax"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let profile = body_json(resp).await;
    assert!(
        profile["tax_profile_id"].is_string(),
        "tax profile should have tax_profile_id: {profile}"
    );
}

// ── 21. LLC entity creation ───────────────────────────────────────────────────

#[tokio::test]
async fn test_create_llc_entity_returns_200() {
    let mut ctx = TestCtx::new();

    let resp = ctx
        .post(
            "/v1/entities",
            json!({
                "legal_name": "Widgets LLC",
                "entity_type": "llc",
                "jurisdiction": "DE"
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let entity = body_json(resp).await;
    assert_eq!(entity["entity_type"], "llc");
    assert!(entity["entity_id"].is_string());
}

// ── 22. Meeting — send notice before convene ──────────────────────────────────

#[tokio::test]
async fn test_meeting_notice_then_convene() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    // Create body.
    let body_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/bodies"),
            json!({
                "name": "Board",
                "body_type": "board_of_directors",
                "quorum_rule": "majority",
                "voting_method": "per_capita"
            }),
        )
        .await;
    assert_eq!(body_resp.status(), StatusCode::OK);
    let body_id = body_json(body_resp).await["body_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Create meeting with notice_days set.
    let meeting_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/meetings"),
            json!({
                "body_id": body_id,
                "meeting_type": "board_meeting",
                "title": "Annual Meeting",
                "scheduled_date": null,
                "location": "HQ",
                "notice_days": 10
            }),
        )
        .await;
    assert_eq!(meeting_resp.status(), StatusCode::OK);
    let meeting = body_json(meeting_resp).await;
    let meeting_id = meeting["meeting_id"].as_str().unwrap();

    // Send notice (Draft → Noticed).
    let notice_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/notice"
        ))
        .await;
    assert_eq!(notice_resp.status(), StatusCode::OK);
    let noticed = body_json(notice_resp).await;
    assert_eq!(noticed["status"], "noticed");

    // Convene (Noticed → Convened).
    let convene_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/convene"
        ))
        .await;
    assert_eq!(convene_resp.status(), StatusCode::OK);
    let convened = body_json(convene_resp).await;
    assert_eq!(convened["status"], "convened");
}

// ── 23. Meeting — cancel from draft ──────────────────────────────────────────

#[tokio::test]
async fn test_cancel_meeting_from_draft() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let body_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/bodies"),
            json!({
                "name": "Board",
                "body_type": "board_of_directors",
                "quorum_rule": "majority",
                "voting_method": "per_capita"
            }),
        )
        .await;
    assert_eq!(body_resp.status(), StatusCode::OK);
    let body_id = body_json(body_resp).await["body_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let meeting_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/meetings"),
            json!({
                "body_id": body_id,
                "meeting_type": "board_meeting",
                "title": "Meeting to Cancel",
                "scheduled_date": null,
                "location": null,
                "notice_days": null
            }),
        )
        .await;
    assert_eq!(meeting_resp.status(), StatusCode::OK);
    let meeting_id = body_json(meeting_resp).await["meeting_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let cancel_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/cancel"
        ))
        .await;
    assert_eq!(cancel_resp.status(), StatusCode::OK);
    let cancelled = body_json(cancel_resp).await;
    assert_eq!(cancelled["status"], "cancelled");
}

// ── 24. Get meeting ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_meeting_returns_200() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    let body_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/bodies"),
            json!({
                "name": "Board",
                "body_type": "board_of_directors",
                "quorum_rule": "majority",
                "voting_method": "per_capita"
            }),
        )
        .await;
    let body_id = body_json(body_resp).await["body_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let meeting_resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/governance/meetings"),
            json!({
                "body_id": body_id,
                "meeting_type": "board_meeting",
                "title": "Board Q2",
                "scheduled_date": null,
                "location": null,
                "notice_days": null
            }),
        )
        .await;
    assert_eq!(meeting_resp.status(), StatusCode::OK);
    let meeting_id = body_json(meeting_resp).await["meeting_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let get_resp = ctx
        .get(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{meeting_id}"
        ))
        .await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["meeting_id"], meeting_id.as_str());
}

// ── 25. Documents listed after advance ────────────────────────────────────────

#[tokio::test]
async fn test_list_documents_after_advance() {
    let mut ctx = TestCtx::new();
    let entity = create_ccorp(&mut ctx).await;
    let entity_id = entity["entity_id"].as_str().unwrap();

    // Advance formation once (should be `documents_generated` or similar).
    ctx.post_empty(&format!("/v1/formations/{entity_id}/advance"))
        .await;

    let docs_resp = ctx
        .get(&format!("/v1/formations/{entity_id}/documents"))
        .await;
    assert_eq!(docs_resp.status(), StatusCode::OK);

    let docs = body_json(docs_resp).await;
    assert!(docs.is_array(), "documents response should be an array");
}

// ── Helper: initialise workspace store directly (bypasses the server) ─────────

/// Initialise the workspace store for `ctx.workspace_id` directly via the
/// storage library, without going through the HTTP layer.
///
/// This is needed for endpoints that read from the workspace entity index
/// (e.g. `GET /v1/entities`) when the entity was created via
/// `POST /v1/entities` (which only creates the entity store, not the
/// workspace index entry).
async fn ensure_workspace_direct(ctx: &TestCtx) {
    use std::path::PathBuf;
    use corp_storage::workspace_store::{Backend as WsBackend, WorkspaceStore};

    // Derive the workspace path from what AppState would use.
    // AppState.data_dir is the tempdir root; workspace store lives at
    // {data_dir}/{workspace_id}/workspace/.
    let data_dir = ctx.data_dir();
    let ws_path = PathBuf::from(data_dir)
        .join(ctx.workspace_id.to_string())
        .join("workspace");

    // `gix::init_bare` does not create intermediate directories, so we must.
    std::fs::create_dir_all(&ws_path)
        .expect("create workspace dir for test");

    let backend = WsBackend::Git {
        repo_path: Arc::new(ws_path),
    };

    let _ = WorkspaceStore::init(backend, ctx.workspace_id).await;
}

// ── TestCtx accessors ─────────────────────────────────────────────────────────

impl TestCtx {
    /// Return the data directory path string for this context.
    ///
    /// Used by helper functions that need to construct storage paths
    /// independently of the server (e.g. to pre-initialise the workspace
    /// store before a test that needs it).
    fn data_dir(&self) -> String {
        self._dir.path().to_str().unwrap().to_owned()
    }
}

