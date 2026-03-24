//! Dual-backend API tests — runs every endpoint group against both the git
//! backend and the KV (Redis) backend.
// The `s3` cfg mirrors corp-server's own allow — it is gated on a feature that
// exists in corp-storage but not in this crate's Cargo.toml.
#![allow(unexpected_cfgs)]
//!
//! ## Structure
//!
//! [`TestCtx`] is a shared test harness that can be configured for either
//! backend.  The `dual_backend_test!` macro generates a `mod` containing a
//! `git` test (always enabled) and a `kv` test (marked `#[ignore]` so it only
//! runs when Redis is reachable via `REDIS_URL`).
//!
//! ## Running KV tests
//!
//! ```text
//! REDIS_URL=redis://127.0.0.1:6379 cargo test -p corp-server -- --include-ignored
//! ```
//!
//! All `git` tests pass without any environment variables.

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

// ── Constants ─────────────────────────────────────────────────────────────────

const TEST_JWT_SECRET: &[u8] = b"dual-backend-test-secret-do-not-use";

// ── No-op key resolver ────────────────────────────────────────────────────────

struct NoopApiKeyResolver;

#[async_trait::async_trait]
impl ApiKeyResolver for NoopApiKeyResolver {
    async fn resolve(&self, _raw_key: &str) -> Result<Principal, AuthError> {
        Err(AuthError::InvalidApiKey)
    }
}

// ── TestCtx ───────────────────────────────────────────────────────────────────

/// Shared test context that works with either the git or kv backend.
struct TestCtx {
    /// Axum router under test (cloned on each request).
    app: axum::Router,
    /// The workspace every request is scoped to.
    workspace_id: WorkspaceId,
    /// Pre-signed JWT with `Scope::All` for this workspace.
    token: String,
    /// Data directory string (needed by workspace-initialisation helpers).
    data_dir: String,
    /// Which storage backend this context was created with.
    storage_backend: StorageBackend,
    /// Keeps the tempdir alive for the duration of the test.
    _tempdir: TempDir,
}

impl TestCtx {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Create a context backed by the git (on-disk) backend.
    fn with_git() -> Self {
        let dir = TempDir::new().expect("create tempdir");
        let data_dir = dir.path().to_str().unwrap().to_owned();

        let jwt_config = Arc::new(JwtConfig::new(TEST_JWT_SECRET));
        let workspace_id = WorkspaceId::new();

        let token = mint_token(&jwt_config, workspace_id);

        let state = AppState {
            data_dir: data_dir.clone(),
            jwt_config,
            api_key_resolver: Arc::new(NoopApiKeyResolver),
            storage_backend: StorageBackend::Git,
        };

        Self {
            app: router(state),
            workspace_id,
            token,
            data_dir,
            _tempdir: dir,
        }
    }

    /// Create a context backed by the KV (Redis) backend.
    ///
    /// Panics if `REDIS_URL` is not set — callers should guard with a
    /// `std::env::var("REDIS_URL")` check and return early before calling this.
    async fn with_kv() -> Self {
        let redis_url = std::env::var("REDIS_URL")
            .expect("REDIS_URL must be set to run kv backend tests");

        // Tempdir is still needed for the data_dir field (not used by kv).
        let dir = TempDir::new().expect("create tempdir");
        let data_dir = dir.path().to_str().unwrap().to_owned();

        let jwt_config = Arc::new(JwtConfig::new(TEST_JWT_SECRET));
        let workspace_id = WorkspaceId::new();

        let token = mint_token(&jwt_config, workspace_id);

        let state = AppState {
            data_dir: data_dir.clone(),
            jwt_config,
            api_key_resolver: Arc::new(NoopApiKeyResolver),
            storage_backend: StorageBackend::Kv {
                redis_url,
                #[cfg(feature = "s3")]
                s3_bucket: None,
            },
        };

        Self {
            app: router(state),
            workspace_id,
            token,
            data_dir,
            _tempdir: dir,
        }
    }

    // ── HTTP helpers ──────────────────────────────────────────────────────────

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

        self.app.clone().oneshot(req).await.expect("service error")
    }

    async fn get(&mut self, path: &str) -> axum::response::Response {
        self.request(Method::GET, path, None).await
    }

    async fn post(&mut self, path: &str, body: Value) -> axum::response::Response {
        self.request(Method::POST, path, Some(body)).await
    }

    async fn post_empty(&mut self, path: &str) -> axum::response::Response {
        self.request(Method::POST, path, None).await
    }

    async fn patch(&mut self, path: &str, body: Value) -> axum::response::Response {
        self.request(Method::PATCH, path, Some(body)).await
    }

    async fn put(&mut self, path: &str, body: Value) -> axum::response::Response {
        self.request(Method::PUT, path, Some(body)).await
    }

    async fn delete(&mut self, path: &str) -> axum::response::Response {
        self.request(Method::DELETE, path, None).await
    }
}

// ── Standalone helpers ────────────────────────────────────────────────────────

/// Mint a JWT with `Scope::All` for the given workspace.
fn mint_token(jwt_config: &JwtConfig, workspace_id: WorkspaceId) -> String {
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
    jwt_config.encode(&claims).expect("encode test JWT")
}

/// Current Unix timestamp in seconds.
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Decode the response body as a `serde_json::Value`.
async fn body_json(resp: axum::response::Response) -> Value {
    use axum::body::to_bytes;
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

/// Initialise the workspace store directly (bypasses the HTTP layer).
///
/// Required before any endpoint that reads from the workspace entity index
/// (e.g. `GET /v1/entities`, `GET /v1/api-keys`).  Entity creation registers
/// the entity in its own store but does not write the workspace-level index.
async fn ensure_workspace(ctx: &TestCtx) {
    use std::path::PathBuf;
    use corp_storage::workspace_store::{Backend as WsBackend, WorkspaceStore};

    let ws_path = PathBuf::from(&ctx.data_dir)
        .join(ctx.workspace_id.to_string())
        .join("workspace");

    std::fs::create_dir_all(&ws_path).expect("create workspace dir");

    let backend = WsBackend::Git {
        repo_path: Arc::new(ws_path),
    };

    let _ = WorkspaceStore::init(backend, ctx.workspace_id).await;
}

// ── Scenario helpers ──────────────────────────────────────────────────────────

/// Create a C-Corp entity; returns the parsed response body.
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
        "create_entity failed with {status}: {body}"
    );
    body
}

/// Create a governance body for `entity_id`; returns the parsed body.
async fn create_board(ctx: &mut TestCtx, entity_id: &str) -> Value {
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
    let status = resp.status();
    let body = body_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create_board failed with {status}: {body}"
    );
    body
}

/// Create a contact for `entity_id`; returns the parsed body.
async fn create_contact(ctx: &mut TestCtx, entity_id: &str) -> Value {
    let resp = ctx
        .post(
            &format!("/v1/entities/{entity_id}/contacts"),
            json!({
                "contact_type": "individual",
                "name": "Alice Founder",
                "category": "founder",
                "email": "alice@example.com",
                "phone": null,
                "mailing_address": null,
                "cap_table_access": null,
                "notes": null
            }),
        )
        .await;
    let status = resp.status();
    let body = body_json(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "create_contact failed with {status}: {body}"
    );
    body
}

/// Advance an entity from Pending to a target formation status, signing all
/// documents along the way. Valid targets: "documents_signed",
/// "filing_submitted", "filed", "ein_applied", "active".
async fn advance_to(ctx: &mut TestCtx, entity_id: &str, target_status: &str) {
    // Advance Pending → DocumentsGenerated.
    let resp = ctx
        .post_empty(&format!("/v1/formations/{entity_id}/advance"))
        .await;
    assert_eq!(resp.status(), StatusCode::OK, "advance to documents_generated");

    // Sign all generated documents.
    let docs_resp = ctx.get(&format!("/v1/formations/{entity_id}/documents")).await;
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

    // Advance through remaining states until we reach target_status.
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
        assert_eq!(resp.status(), StatusCode::OK, "advance to {expected}");
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

// ── dual_backend_test! macro ──────────────────────────────────────────────────

/// Generate a `mod $name { async fn git() {...}  async fn kv() {...} }` pair.
///
/// - `git` always runs (no `#[ignore]`).
/// - `kv` is marked `#[ignore]` and only runs when `REDIS_URL` is set.
///
/// The body must be an async block `async { ... }` that operates on `ctx: TestCtx`.
macro_rules! dual_backend_test {
    ($name:ident, |$ctx:ident| $body:expr) => {
        mod $name {
            use super::*;

            async fn run($ctx: TestCtx) {
                #[allow(unused_mut)]
                let mut $ctx = $ctx;
                $body
            }

            #[tokio::test]
            async fn git() {
                run(TestCtx::with_git()).await;
            }

            #[tokio::test]
            #[ignore = "requires REDIS_URL; run with --include-ignored"]
            async fn kv() {
                if std::env::var("REDIS_URL").is_err() {
                    eprintln!("REDIS_URL not set — skipping kv test");
                    return;
                }
                run(TestCtx::with_kv().await).await;
            }
        }
    };
}

// ── 1. Health ─────────────────────────────────────────────────────────────────

dual_backend_test!(health_check, |ctx| {
    let req = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = ctx.app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "ok");
});

// ── 2. Formation: create entity ────────────────────────────────────────────────

dual_backend_test!(formation_create_entity, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    assert!(entity["entity_id"].is_string(), "missing entity_id: {entity}");
    assert_eq!(entity["legal_name"], "Acme Corp");
    assert_eq!(entity["entity_type"], "c_corp");
    assert_eq!(entity["formation_status"], "pending");
});

// ── 3. Formation: list entities ────────────────────────────────────────────────

dual_backend_test!(formation_list_entities, |ctx| {
    ensure_workspace(&ctx).await;
    let resp = ctx.get("/v1/entities").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_array(), "expected array: {body}");
});

// ── 4. Formation: get entity ───────────────────────────────────────────────────

dual_backend_test!(formation_get_entity, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    let resp = ctx.get(&format!("/v1/entities/{id}")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let fetched = body_json(resp).await;
    assert_eq!(fetched["entity_id"], entity["entity_id"]);
    assert_eq!(fetched["legal_name"], "Acme Corp");
});

// ── 5. Formation: advance status ──────────────────────────────────────────────

dual_backend_test!(formation_advance, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    let resp = ctx.post_empty(&format!("/v1/formations/{id}/advance")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let advanced = body_json(resp).await;
    assert_ne!(
        advanced["formation_status"], "pending",
        "status must advance: {advanced}"
    );
});

// ── 6. Formation: dissolve entity ────────────────────────────────────────────

dual_backend_test!(formation_dissolve, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    let resp = ctx.post_empty(&format!("/v1/entities/{id}/dissolve")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let dissolved = body_json(resp).await;
    assert_eq!(dissolved["formation_status"], "dissolved", "{dissolved}");
});

// ── 7. Formation: list documents ─────────────────────────────────────────────

dual_backend_test!(formation_list_documents, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    // Advance to generate documents.
    ctx.post_empty(&format!("/v1/formations/{id}/advance")).await;

    let resp = ctx.get(&format!("/v1/formations/{id}/documents")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let docs = body_json(resp).await;
    assert!(docs.is_array(), "expected array: {docs}");
});

// ── 8. Formation: sign document ───────────────────────────────────────────────

dual_backend_test!(formation_sign_document, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    ctx.post_empty(&format!("/v1/formations/{id}/advance")).await;

    let docs_resp = ctx.get(&format!("/v1/formations/{id}/documents")).await;
    assert_eq!(docs_resp.status(), StatusCode::OK);
    let docs = body_json(docs_resp).await;
    let arr = docs.as_array().expect("documents should be array");

    if arr.is_empty() {
        // No documents generated — sign test not applicable.
        return;
    }

    let doc_id = arr[0]["document_id"].as_str().unwrap();
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
    assert_eq!(sign_resp.status(), StatusCode::OK);
    let signed = body_json(sign_resp).await;
    assert!(
        !signed["signatures"].as_array().unwrap_or(&vec![]).is_empty(),
        "expected at least one signature: {signed}"
    );
});

// ── 9. Formation: get filing ──────────────────────────────────────────────────

dual_backend_test!(formation_get_filing, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    let resp = ctx.get(&format!("/v1/formations/{id}/filing")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let filing = body_json(resp).await;
    assert!(filing["filing_id"].is_string(), "{filing}");
});

// ── 10. Formation: confirm filing ─────────────────────────────────────────────

dual_backend_test!(formation_confirm_filing, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    // Entity must be in FilingSubmitted state to confirm filing.
    advance_to(&mut ctx, id, "filing_submitted").await;

    let resp = ctx
        .post(
            &format!("/v1/formations/{id}/filing/confirm"),
            json!({ "confirmation_number": "DE-99887766" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let filing = body_json(resp).await;
    assert!(filing["filing_id"].is_string(), "{filing}");
});

// ── 11. Formation: get tax profile ────────────────────────────────────────────

dual_backend_test!(formation_get_tax_profile, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    let resp = ctx.get(&format!("/v1/formations/{id}/tax")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let profile = body_json(resp).await;
    assert!(profile["tax_profile_id"].is_string(), "{profile}");
});

// ── 12. Formation: confirm EIN ────────────────────────────────────────────────

dual_backend_test!(formation_confirm_ein, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();

    // Entity must be in Filed state to confirm EIN.
    advance_to(&mut ctx, id, "filed").await;

    let resp = ctx
        .post(
            &format!("/v1/formations/{id}/tax/confirm-ein"),
            json!({ "ein": "12-3456789" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let profile = body_json(resp).await;
    assert!(profile["tax_profile_id"].is_string(), "{profile}");
});

// ── 13. Formation: full lifecycle ─────────────────────────────────────────────

dual_backend_test!(formation_full_lifecycle, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let id = entity["entity_id"].as_str().unwrap();
    assert_eq!(entity["formation_status"], "pending");

    let mut prev = "pending".to_owned();
    let mut advances = 0u32;

    loop {
        let resp = ctx.post_empty(&format!("/v1/formations/{id}/advance")).await;
        if resp.status() != StatusCode::OK {
            break;
        }
        let updated = body_json(resp).await;
        let next = updated["formation_status"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned();
        assert_ne!(next, prev, "status must change on each advance");
        prev = next.clone();
        advances += 1;
        if next == "active" || advances >= 10 {
            break;
        }
    }

    assert!(advances > 0, "must be able to advance at least once from pending");

    // Entity should still be retrievable.
    let get_resp = ctx.get(&format!("/v1/entities/{id}")).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
});

// ── 14. Equity: cap table CRUD ───────────────────────────────────────────────

dual_backend_test!(equity_cap_table_crud, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // Create cap table.
    let resp = ctx.post(&format!("/v1/entities/{eid}/cap-table"), json!({})).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = body_json(resp).await;
    assert!(ct["cap_table_id"].is_string(), "{ct}");
    assert_eq!(ct["entity_id"], eid);

    // Read it back.
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/cap-table")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    let tables = body_json(list_resp).await;
    let arr = tables.as_array().expect("cap-table list should be array");
    assert!(!arr.is_empty(), "cap table should appear in list");
    assert_eq!(arr[0]["cap_table_id"], ct["cap_table_id"]);
});

// ── 15. Equity: share classes ─────────────────────────────────────────────────

dual_backend_test!(equity_share_classes, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let ct_resp = ctx.post(&format!("/v1/entities/{eid}/cap-table"), json!({})).await;
    let ct_id = body_json(ct_resp).await["cap_table_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/share-classes")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    let before = body_json(list_resp).await;
    assert!(before.as_array().map(|a| a.is_empty()).unwrap_or(false));

    // Create.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/share-classes"),
            json!({
                "cap_table_id": ct_id,
                "class_code": "COMMON",
                "stock_type": "common",
                "par_value": "0.00001",
                "authorized_shares": 10_000_000i64,
                "liquidation_preference": null
            }),
        )
        .await;
    assert_eq!(create_resp.status(), StatusCode::OK);
    let sc = body_json(create_resp).await;
    assert!(sc["share_class_id"].is_string(), "{sc}");
    assert_eq!(sc["class_code"], "COMMON");

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/share-classes")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let after = body_json(list_resp2).await;
    assert_eq!(after.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 16. Equity: grants ────────────────────────────────────────────────────────

dual_backend_test!(equity_grants, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // Create a contact to receive the grant.
    let contact = create_contact(&mut ctx, eid).await;
    let contact_id = contact["contact_id"].as_str().unwrap().to_owned();

    // Cap table + share class.
    let ct_resp = ctx.post(&format!("/v1/entities/{eid}/cap-table"), json!({})).await;
    let ct_id = body_json(ct_resp).await["cap_table_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let sc_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/share-classes"),
            json!({
                "cap_table_id": ct_id,
                "class_code": "COMMON",
                "stock_type": "common",
                "par_value": "0.00001",
                "authorized_shares": 10_000_000i64,
                "liquidation_preference": null
            }),
        )
        .await;
    let sc_id = body_json(sc_resp).await["share_class_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/grants")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create grant.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/grants"),
            json!({
                "cap_table_id": ct_id,
                "share_class_id": sc_id,
                "recipient_contact_id": contact_id,
                "recipient_name": "Alice Founder",
                "grant_type": "rsa",
                "shares": 1_000_000i64,
                "price_per_share": 1i64,
                "vesting_start": "2024-01-01",
                "vesting_months": 48,
                "cliff_months": 12
            }),
        )
        .await;
    let status = create_resp.status();
    let grant = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create grant: {grant}");
    assert!(grant["grant_id"].is_string(), "{grant}");

    let grant_id = grant["grant_id"].as_str().unwrap().to_owned();

    // Get by ID.
    let get_resp = ctx.get(&format!("/v1/entities/{eid}/grants/{grant_id}")).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["grant_id"], grant["grant_id"]);

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/grants")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let grants = body_json(list_resp2).await;
    assert_eq!(grants.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 17. Equity: SAFEs ─────────────────────────────────────────────────────────

dual_backend_test!(equity_safes, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let contact = create_contact(&mut ctx, eid).await;
    let contact_id = contact["contact_id"].as_str().unwrap().to_owned();

    let ct_resp = ctx.post(&format!("/v1/entities/{eid}/cap-table"), json!({})).await;
    let ct_id = body_json(ct_resp).await["cap_table_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // List SAFEs (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/safes")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Issue SAFE.
    let issue_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/safes"),
            json!({
                "cap_table_id": ct_id,
                "investor_contact_id": contact_id,
                "investor_name": "Bob Investor",
                "safe_type": "post_money",
                "investment_amount_cents": 500_000_00i64,
                "valuation_cap_cents": 5_000_000_00i64,
                "discount_percent": 20u32
            }),
        )
        .await;
    let status = issue_resp.status();
    let safe = body_json(issue_resp).await;
    assert_eq!(status, StatusCode::OK, "issue safe: {safe}");
    assert!(safe["safe_note_id"].is_string(), "{safe}");

    let safe_id = safe["safe_note_id"].as_str().unwrap().to_owned();

    // List SAFEs (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/safes")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let safes = body_json(list_resp2).await;
    assert_eq!(safes.as_array().map(|a| a.len()).unwrap_or(0), 1);

    // Convert SAFE.
    let convert_resp = ctx
        .post(&format!("/v1/entities/{eid}/safes/{safe_id}/convert"), json!({}))
        .await;
    let convert_status = convert_resp.status();
    let convert_body = body_json(convert_resp).await;
    assert!(
        convert_status == StatusCode::OK || convert_status == StatusCode::BAD_REQUEST,
        "convert safe returned unexpected {convert_status}: {convert_body}"
    );
});

// ── 18. Equity: valuations ────────────────────────────────────────────────────

dual_backend_test!(equity_valuations, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let ct_resp = ctx.post(&format!("/v1/entities/{eid}/cap-table"), json!({})).await;
    let ct_id = body_json(ct_resp).await["cap_table_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/valuations")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/valuations"),
            json!({
                "cap_table_id": ct_id,
                "valuation_type": "four_oh_nine_a",
                "methodology": "market",
                "valuation_amount_cents": 10_000_000_00i64,
                "effective_date": "2024-06-30",
                "prepared_by": "Acme Valuation LLC"
            }),
        )
        .await;
    let status = create_resp.status();
    let valuation = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create valuation: {valuation}");
    assert!(valuation["valuation_id"].is_string(), "{valuation}");

    let val_id = valuation["valuation_id"].as_str().unwrap().to_owned();

    // Submit.
    let submit_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/valuations/{val_id}/submit"))
        .await;
    let submit_status = submit_resp.status();
    let submit_body = body_json(submit_resp).await;
    assert!(
        submit_status == StatusCode::OK || submit_status == StatusCode::BAD_REQUEST,
        "submit valuation unexpected {submit_status}: {submit_body}"
    );

    // Approve.
    let approve_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/valuations/{val_id}/approve"),
            json!({ "approved_by": "Board of Directors" }),
        )
        .await;
    let approve_status = approve_resp.status();
    let approve_body = body_json(approve_resp).await;
    assert!(
        approve_status == StatusCode::OK || approve_status == StatusCode::BAD_REQUEST,
        "approve valuation unexpected {approve_status}: {approve_body}"
    );
});

// ── 19. Equity: transfers ─────────────────────────────────────────────────────

dual_backend_test!(equity_transfers, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let ct_resp = ctx.post(&format!("/v1/entities/{eid}/cap-table"), json!({})).await;
    let ct_id = body_json(ct_resp).await["cap_table_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let sc_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/share-classes"),
            json!({
                "cap_table_id": ct_id,
                "class_code": "COMMON",
                "stock_type": "common",
                "par_value": "0.00001",
                "authorized_shares": 10_000_000i64,
                "liquidation_preference": null
            }),
        )
        .await;
    let sc_id = body_json(sc_resp).await["share_class_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Create two holders.
    let h1_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/holders"),
            json!({ "contact_id": null, "name": "Holder One", "holder_type": "individual" }),
        )
        .await;
    let h1_id = body_json(h1_resp).await["holder_id"]
        .as_str()
        .expect("h1 holder_id should be a string")
        .to_owned();

    let h2_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/holders"),
            json!({ "contact_id": null, "name": "Holder Two", "holder_type": "individual" }),
        )
        .await;
    let h2_id = body_json(h2_resp).await["holder_id"]
        .as_str()
        .expect("h2 holder_id should be a string")
        .to_owned();

    // List transfers (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/transfers")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create transfer.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/transfers"),
            json!({
                "cap_table_id": ct_id,
                "from_holder_id": h1_id,
                "to_holder_id": h2_id,
                "share_class_id": sc_id,
                "shares": 1000i64,
                "transfer_type": "secondary_sale",
                "price_per_share_cents": 100i64
            }),
        )
        .await;
    let status = create_resp.status();
    let transfer = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create transfer: {transfer}");
    assert!(transfer["transfer_id"].is_string(), "{transfer}");

    let txfr_id = transfer["transfer_id"].as_str().unwrap().to_owned();

    // Approve.
    let approve_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/transfers/{txfr_id}/approve"))
        .await;
    let approve_status = approve_resp.status();
    let approve_body = body_json(approve_resp).await;
    assert!(
        approve_status == StatusCode::OK || approve_status == StatusCode::BAD_REQUEST,
        "approve transfer unexpected {approve_status}: {approve_body}"
    );

    // Execute.
    let execute_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/transfers/{txfr_id}/execute"))
        .await;
    let execute_status = execute_resp.status();
    let execute_body = body_json(execute_resp).await;
    assert!(
        execute_status == StatusCode::OK || execute_status == StatusCode::BAD_REQUEST,
        "execute transfer unexpected {execute_status}: {execute_body}"
    );
});

// ── 20. Equity: rounds ────────────────────────────────────────────────────────

dual_backend_test!(equity_rounds, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let ct_resp = ctx.post(&format!("/v1/entities/{eid}/cap-table"), json!({})).await;
    let ct_id = body_json(ct_resp).await["cap_table_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/rounds")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create round.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/rounds"),
            json!({
                "cap_table_id": ct_id,
                "name": "Seed Round",
                "target_amount_cents": 2_000_000_00i64,
                "price_per_share_cents": 100i64
            }),
        )
        .await;
    let status = create_resp.status();
    let round = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create round: {round}");
    assert!(round["round_id"].is_string(), "{round}");

    let round_id = round["round_id"].as_str().unwrap().to_owned();

    // Close round.
    let close_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/rounds/{round_id}/close"))
        .await;
    let close_status = close_resp.status();
    let close_body = body_json(close_resp).await;
    assert!(
        close_status == StatusCode::OK || close_status == StatusCode::BAD_REQUEST,
        "close round unexpected {close_status}: {close_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/rounds")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let rounds = body_json(list_resp2).await;
    assert_eq!(rounds.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 21. Equity: holders ───────────────────────────────────────────────────────

dual_backend_test!(equity_holders, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/holders")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/holders"),
            json!({
                "contact_id": null,
                "name": "Foundry Capital LLC",
                "holder_type": "entity"
            }),
        )
        .await;
    let status = create_resp.status();
    let holder = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create holder: {holder}");
    assert!(holder["holder_id"].is_string(), "{holder}");

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/holders")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let holders = body_json(list_resp2).await;
    assert_eq!(holders.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 22. Governance: bodies CRUD ───────────────────────────────────────────────

dual_backend_test!(governance_bodies_crud, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/governance/bodies")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let gov_body = create_board(&mut ctx, eid).await;
    assert!(gov_body["body_id"].is_string(), "{gov_body}");
    assert_eq!(gov_body["name"], "Board of Directors");
    let body_id = gov_body["body_id"].as_str().unwrap().to_owned();

    // Get by ID.
    let get_resp = ctx
        .get(&format!("/v1/entities/{eid}/governance/bodies/{body_id}"))
        .await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["body_id"], gov_body["body_id"]);

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/governance/bodies")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let bodies = body_json(list_resp2).await;
    assert_eq!(bodies.as_array().map(|a| a.len()).unwrap_or(0), 1);

    // Deactivate.
    let deact_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{eid}/governance/bodies/{body_id}/deactivate"
        ))
        .await;
    assert_eq!(deact_resp.status(), StatusCode::OK);
    let deactivated = body_json(deact_resp).await;
    assert_eq!(
        deactivated["status"], "inactive",
        "body should be deactivated: {deactivated}"
    );
});

// ── 23. Governance: seats ─────────────────────────────────────────────────────

dual_backend_test!(governance_seats, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let gov_body = create_board(&mut ctx, eid).await;
    let body_id = gov_body["body_id"].as_str().unwrap().to_owned();

    let contact = create_contact(&mut ctx, eid).await;
    let contact_id = contact["contact_id"].as_str().unwrap().to_owned();

    // List seats (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/governance/seats")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create seat.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/seats"),
            json!({
                "body_id": body_id,
                "holder_id": contact_id,
                "role": "member",
                "appointed_date": "2024-01-01",
                "term_expiration": null,
                "voting_power": 1u32
            }),
        )
        .await;
    let status = create_resp.status();
    let seat = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create seat: {seat}");
    assert!(seat["seat_id"].is_string(), "{seat}");

    let seat_id = seat["seat_id"].as_str().unwrap().to_owned();

    // Get seat.
    let get_resp = ctx
        .get(&format!("/v1/entities/{eid}/governance/seats/{seat_id}"))
        .await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/governance/seats")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let seats = body_json(list_resp2).await;
    assert_eq!(seats.as_array().map(|a| a.len()).unwrap_or(0), 1);

    // Resign seat.
    let resign_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{eid}/governance/seats/{seat_id}/resign"
        ))
        .await;
    let resign_status = resign_resp.status();
    let resign_body = body_json(resign_resp).await;
    assert!(
        resign_status == StatusCode::OK || resign_status == StatusCode::BAD_REQUEST,
        "resign seat unexpected {resign_status}: {resign_body}"
    );
});

// ── 24. Governance: meetings full lifecycle ───────────────────────────────────

dual_backend_test!(governance_meetings_lifecycle, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let gov_body = create_board(&mut ctx, eid).await;
    let body_id = gov_body["body_id"].as_str().unwrap().to_owned();

    // List meetings (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/governance/meetings")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create meeting.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/meetings"),
            json!({
                "body_id": body_id,
                "meeting_type": "board_meeting",
                "title": "Q2 Board Meeting",
                "scheduled_date": null,
                "location": "HQ",
                "notice_days": null
            }),
        )
        .await;
    assert_eq!(create_resp.status(), StatusCode::OK);
    let meeting = body_json(create_resp).await;
    assert!(meeting["meeting_id"].is_string(), "{meeting}");
    assert_eq!(meeting["status"], "draft");
    let meeting_id = meeting["meeting_id"].as_str().unwrap().to_owned();

    // Get meeting.
    let get_resp = ctx
        .get(&format!("/v1/entities/{eid}/governance/meetings/{meeting_id}"))
        .await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["meeting_id"], meeting["meeting_id"]);

    // Send notice (Draft → Noticed).
    let notice_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{eid}/governance/meetings/{meeting_id}/notice"
        ))
        .await;
    assert_eq!(notice_resp.status(), StatusCode::OK);
    let noticed = body_json(notice_resp).await;
    assert_eq!(noticed["status"], "noticed", "{noticed}");

    // Convene (Noticed → Convened).
    let convene_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{eid}/governance/meetings/{meeting_id}/convene"
        ))
        .await;
    assert_eq!(convene_resp.status(), StatusCode::OK);
    let convened = body_json(convene_resp).await;
    assert_eq!(convened["status"], "convened", "{convened}");

    // Adjourn (Convened → Adjourned).
    let adjourn_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{eid}/governance/meetings/{meeting_id}/adjourn"
        ))
        .await;
    assert_eq!(adjourn_resp.status(), StatusCode::OK);
    let adjourned = body_json(adjourn_resp).await;
    assert_eq!(adjourned["status"], "adjourned", "{adjourned}");

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/governance/meetings")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let meetings = body_json(list_resp2).await;
    assert_eq!(meetings.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 25. Governance: agenda items ─────────────────────────────────────────────

dual_backend_test!(governance_agenda_items, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let gov_body = create_board(&mut ctx, eid).await;
    let body_id = gov_body["body_id"].as_str().unwrap().to_owned();

    let meeting_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/meetings"),
            json!({
                "body_id": body_id,
                "meeting_type": "board_meeting",
                "title": "Item Test Meeting",
                "scheduled_date": null,
                "location": null,
                "notice_days": null
            }),
        )
        .await;
    let meeting_id = body_json(meeting_resp).await["meeting_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // List items (empty).
    let list_resp = ctx
        .get(&format!(
            "/v1/entities/{eid}/governance/meetings/{meeting_id}/items"
        ))
        .await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Add item.
    let item_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/meetings/{meeting_id}/items"),
            json!({
                "title": "Approve Q2 Budget",
                "item_type": "resolution",
                "description": "Approve budget",
                "resolution_text": "RESOLVED: Budget is approved."
            }),
        )
        .await;
    assert_eq!(item_resp.status(), StatusCode::OK);
    let item = body_json(item_resp).await;
    assert!(item["item_id"].is_string(), "{item}");

    let item_id = item["item_id"].as_str().unwrap().to_owned();

    // List (non-empty).
    let list_resp2 = ctx
        .get(&format!(
            "/v1/entities/{eid}/governance/meetings/{meeting_id}/items"
        ))
        .await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let items = body_json(list_resp2).await;
    assert_eq!(items.as_array().map(|a| a.len()).unwrap_or(0), 1);

    // Convene before resolving.
    ctx.post_empty(&format!(
        "/v1/entities/{eid}/governance/meetings/{meeting_id}/convene"
    ))
    .await;

    // Resolve item.
    let resolve_resp = ctx
        .post(
            &format!(
                "/v1/entities/{eid}/governance/meetings/{meeting_id}/items/{item_id}/resolve"
            ),
            json!({
                "resolution_type": "ordinary",
                "resolution_text": "RESOLVED: Budget is approved."
            }),
        )
        .await;
    assert_eq!(resolve_resp.status(), StatusCode::OK);
});

// ── 26. Governance: votes ─────────────────────────────────────────────────────

dual_backend_test!(governance_votes, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let gov_body = create_board(&mut ctx, eid).await;
    let body_id = gov_body["body_id"].as_str().unwrap().to_owned();

    let contact = create_contact(&mut ctx, eid).await;
    let contact_id = contact["contact_id"].as_str().unwrap().to_owned();

    // Create a seat for the voter.
    let seat_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/seats"),
            json!({
                "body_id": body_id,
                "holder_id": contact_id,
                "role": "member",
                "appointed_date": "2024-01-01",
                "term_expiration": null,
                "voting_power": 1u32
            }),
        )
        .await;
    let seat_body = body_json(seat_resp).await;
    let seat_id = seat_body["seat_id"]
        .as_str()
        .expect("seat_id should be a string")
        .to_owned();

    let meeting_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/meetings"),
            json!({
                "body_id": body_id,
                "meeting_type": "board_meeting",
                "title": "Vote Test Meeting",
                "scheduled_date": null,
                "location": null,
                "notice_days": null
            }),
        )
        .await;
    let meeting_id = body_json(meeting_resp).await["meeting_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let item_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/meetings/{meeting_id}/items"),
            json!({
                "title": "Vote on budget",
                "item_type": "resolution",
                "description": null,
                "resolution_text": "RESOLVED: Approved."
            }),
        )
        .await;
    let item_id = body_json(item_resp).await["item_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Convene.
    ctx.post_empty(&format!(
        "/v1/entities/{eid}/governance/meetings/{meeting_id}/convene"
    ))
    .await;

    // Record attendance (seat_id present) to establish quorum so votes can be cast.
    ctx.post(
        &format!("/v1/entities/{eid}/governance/meetings/{meeting_id}/attendance"),
        json!({ "seat_ids": [seat_id] }),
    )
    .await;

    // List votes (empty).
    let list_resp = ctx
        .get(&format!(
            "/v1/entities/{eid}/governance/meetings/{meeting_id}/votes"
        ))
        .await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Cast a vote.
    let vote_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/meetings/{meeting_id}/votes"),
            json!({
                "agenda_item_id": item_id,
                "seat_id": seat_id,
                "value": "for"
            }),
        )
        .await;
    let vote_status = vote_resp.status();
    let vote_body = body_json(vote_resp).await;
    assert_eq!(vote_status, StatusCode::OK, "cast vote: {vote_body}");
    assert!(vote_body["vote_id"].is_string(), "{vote_body}");

    // List votes (non-empty).
    let list_resp2 = ctx
        .get(&format!(
            "/v1/entities/{eid}/governance/meetings/{meeting_id}/votes"
        ))
        .await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let votes = body_json(list_resp2).await;
    assert_eq!(votes.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 27. Governance: profile ───────────────────────────────────────────────────

dual_backend_test!(governance_profile, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // GET profile (may return 404 or empty on fresh entity).
    let get_resp = ctx.get(&format!("/v1/entities/{eid}/governance/profile")).await;
    let get_status = get_resp.status();
    assert!(
        get_status == StatusCode::OK || get_status == StatusCode::NOT_FOUND,
        "unexpected GET profile status {get_status}"
    );

    // PUT profile.
    let put_resp = ctx
        .put(
            &format!("/v1/entities/{eid}/governance/profile"),
            json!({
                "entity_type": "c_corp",
                "legal_name": "Acme Corp",
                "jurisdiction": "DE",
                "effective_date": "2024-01-01",
                "registered_agent_name": "CT Corporation",
                "registered_agent_address": "1209 Orange St, Wilmington, DE",
                "board_size": 3,
                "principal_name": "Jane Doe",
                "company_address": null,
                "founders": [],
                "directors": [],
                "officers": [],
                "stock_details": null,
                "fiscal_year_end": null
            }),
        )
        .await;
    assert_eq!(put_resp.status(), StatusCode::OK);
    let profile = body_json(put_resp).await;
    assert_eq!(profile["legal_name"], "Acme Corp");

    // GET after PUT.
    let get_resp2 = ctx.get(&format!("/v1/entities/{eid}/governance/profile")).await;
    assert_eq!(get_resp2.status(), StatusCode::OK);
    let fetched = body_json(get_resp2).await;
    assert_eq!(fetched["legal_name"], "Acme Corp");
});

// ── 28. Governance: written consent ──────────────────────────────────────────

dual_backend_test!(governance_written_consent, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let gov_body = create_board(&mut ctx, eid).await;
    let body_id = gov_body["body_id"].as_str().unwrap().to_owned();

    let resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/written-consent"),
            json!({
                "body_id": body_id,
                "title": "Written Consent: Budget Approval",
                "resolution_text": "RESOLVED: Budget approved by written consent."
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result = body_json(resp).await;
    assert!(result["meeting"].is_object(), "{result}");
    assert!(result["agenda_item"].is_object(), "{result}");
});

// ── 29. Governance: quick approve ────────────────────────────────────────────

dual_backend_test!(governance_quick_approve, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    let gov_body = create_board(&mut ctx, eid).await;
    let body_id = gov_body["body_id"].as_str().unwrap().to_owned();

    // quick-approve requires at least one active voting seat in the body.
    let contact = create_contact(&mut ctx, eid).await;
    let contact_id = contact["contact_id"].as_str().unwrap().to_owned();

    ctx.post(
        &format!("/v1/entities/{eid}/governance/seats"),
        json!({
            "body_id": body_id,
            "holder_id": contact_id,
            "role": "member",
            "appointed_date": "2024-01-01",
            "term_expiration": null,
            "voting_power": 1u32
        }),
    )
    .await;

    let resp = ctx
        .post(
            &format!("/v1/entities/{eid}/governance/quick-approve"),
            json!({
                "body_id": body_id,
                "title": "Quick Approve: CEO Appointment",
                "resolution_text": "RESOLVED: Jane Doe is appointed CEO."
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let result = body_json(resp).await;
    assert!(result["meeting_id"].is_string(), "{result}");
    assert!(result["agenda_item_id"].is_string(), "{result}");
    assert!(result["resolution_id"].is_string(), "{result}");
});

// ── 30. Treasury: accounts ────────────────────────────────────────────────────

dual_backend_test!(treasury_accounts, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/accounts")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/accounts"),
            json!({
                "account_code": "cash",
                "account_name": "Cash and Cash Equivalents",
                "currency": "usd"
            }),
        )
        .await;
    let status = create_resp.status();
    let account = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create account: {account}");
    assert!(account["account_id"].is_string(), "{account}");

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/accounts")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let accounts = body_json(list_resp2).await;
    assert_eq!(accounts.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 31. Treasury: journal entries ────────────────────────────────────────────

dual_backend_test!(treasury_journal_entries, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // Create two accounts (cash + equity).
    let cash_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/accounts"),
            json!({ "account_code": "cash", "account_name": "Cash", "currency": "usd" }),
        )
        .await;
    let cash_id = body_json(cash_resp).await["account_id"]
        .as_str()
        .expect("cash account_id should be a string")
        .to_owned();

    let equity_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/accounts"),
            json!({ "account_code": "founder_capital", "account_name": "Founder Capital", "currency": "usd" }),
        )
        .await;
    let equity_id = body_json(equity_resp).await["account_id"]
        .as_str()
        .expect("equity account_id should be a string")
        .to_owned();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/journal-entries")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create journal entry (balanced: debit cash, credit equity).
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/journal-entries"),
            json!({
                "date": "2024-03-15",
                "description": "Initial funding",
                "lines": [
                    {
                        "account_id": cash_id,
                        "amount_cents": 100_000_00i64,
                        "side": "debit",
                        "memo": "Cash received"
                    },
                    {
                        "account_id": equity_id,
                        "amount_cents": 100_000_00i64,
                        "side": "credit",
                        "memo": "Common stock issued"
                    }
                ]
            }),
        )
        .await;
    let status = create_resp.status();
    let entry = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create journal entry: {entry}");
    assert!(entry["entry_id"].is_string(), "{entry}");

    let entry_id = entry["entry_id"].as_str().unwrap().to_owned();

    // Post entry.
    let post_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/journal-entries/{entry_id}/post"))
        .await;
    let post_status = post_resp.status();
    let post_body = body_json(post_resp).await;
    assert_eq!(post_status, StatusCode::OK, "post journal entry: {post_body}");

    // Void posted entry.
    let void_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/journal-entries/{entry_id}/void"))
        .await;
    let void_status = void_resp.status();
    let void_body = body_json(void_resp).await;
    assert!(
        void_status == StatusCode::OK || void_status == StatusCode::BAD_REQUEST,
        "void journal entry unexpected {void_status}: {void_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/journal-entries")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let entries = body_json(list_resp2).await;
    assert_eq!(entries.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 32. Treasury: invoices ────────────────────────────────────────────────────

dual_backend_test!(treasury_invoices, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/invoices")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create invoice.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/invoices"),
            json!({
                "customer_name": "Widgets Inc",
                "customer_email": "billing@widgets.example",
                "amount_cents": 5_000_00i64,
                "currency": "usd",
                "description": "Legal services for Q1",
                "due_date": "2024-04-30"
            }),
        )
        .await;
    let status = create_resp.status();
    let invoice = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create invoice: {invoice}");
    assert!(invoice["invoice_id"].is_string(), "{invoice}");

    let invoice_id = invoice["invoice_id"].as_str().unwrap().to_owned();

    // Send invoice.
    let send_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/invoices/{invoice_id}/send"))
        .await;
    let send_status = send_resp.status();
    let send_body = body_json(send_resp).await;
    assert!(
        send_status == StatusCode::OK || send_status == StatusCode::BAD_REQUEST,
        "send invoice unexpected {send_status}: {send_body}"
    );

    // Pay invoice.
    let pay_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/invoices/{invoice_id}/pay"))
        .await;
    let pay_status = pay_resp.status();
    let pay_body = body_json(pay_resp).await;
    assert!(
        pay_status == StatusCode::OK || pay_status == StatusCode::BAD_REQUEST,
        "pay invoice unexpected {pay_status}: {pay_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/invoices")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let invoices = body_json(list_resp2).await;
    assert_eq!(invoices.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 33. Treasury: payments ────────────────────────────────────────────────────

dual_backend_test!(treasury_payments, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/payments")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create payment.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/payments"),
            json!({
                "recipient_name": "Acme Suppliers LLC",
                "amount_cents": 1_500_00i64,
                "method": "ach",
                "reference": "ACH-00001",
                "paid_at": "2024-03-20T12:00:00Z"
            }),
        )
        .await;
    let status = create_resp.status();
    let payment = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create payment: {payment}");
    assert!(payment["payment_id"].is_string(), "{payment}");

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/payments")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let payments = body_json(list_resp2).await;
    assert_eq!(payments.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 34. Treasury: bank accounts ──────────────────────────────────────────────

dual_backend_test!(treasury_bank_accounts, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/bank-accounts")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/bank-accounts"),
            json!({
                "institution": "First National Bank",
                "account_type": "checking",
                "account_number_last4": "4321",
                "routing_number_last4": "9876"
            }),
        )
        .await;
    let status = create_resp.status();
    let bank_acct = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create bank account: {bank_acct}");
    assert!(bank_acct["bank_account_id"].is_string(), "{bank_acct}");

    let bank_id = bank_acct["bank_account_id"].as_str().unwrap().to_owned();

    // Activate.
    let activate_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/bank-accounts/{bank_id}/activate"))
        .await;
    let activate_status = activate_resp.status();
    let activate_body = body_json(activate_resp).await;
    assert!(
        activate_status == StatusCode::OK || activate_status == StatusCode::BAD_REQUEST,
        "activate bank account unexpected {activate_status}: {activate_body}"
    );

    // Close.
    let close_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/bank-accounts/{bank_id}/close"))
        .await;
    let close_status = close_resp.status();
    let close_body = body_json(close_resp).await;
    assert!(
        close_status == StatusCode::OK || close_status == StatusCode::BAD_REQUEST,
        "close bank account unexpected {close_status}: {close_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/bank-accounts")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let banks = body_json(list_resp2).await;
    assert_eq!(banks.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 35. Treasury: payroll ─────────────────────────────────────────────────────

dual_backend_test!(treasury_payroll, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/payroll-runs")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create payroll run.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/payroll-runs"),
            json!({
                "period_start": "2024-03-01",
                "period_end": "2024-03-31",
                "total_gross_cents": 20_000_00i64,
                "total_net_cents": 15_000_00i64,
                "employee_count": 2u32
            }),
        )
        .await;
    let status = create_resp.status();
    let run = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create payroll run: {run}");
    assert!(run["payroll_run_id"].is_string(), "{run}");

    let run_id = run["payroll_run_id"].as_str().unwrap().to_owned();

    // Approve.
    let approve_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/payroll-runs/{run_id}/approve"))
        .await;
    let approve_status = approve_resp.status();
    let approve_body = body_json(approve_resp).await;
    assert!(
        approve_status == StatusCode::OK || approve_status == StatusCode::BAD_REQUEST,
        "approve payroll unexpected {approve_status}: {approve_body}"
    );

    // Process.
    let process_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/payroll-runs/{run_id}/process"))
        .await;
    let process_status = process_resp.status();
    let process_body = body_json(process_resp).await;
    assert!(
        process_status == StatusCode::OK || process_status == StatusCode::BAD_REQUEST,
        "process payroll unexpected {process_status}: {process_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/payroll-runs")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let runs = body_json(list_resp2).await;
    assert_eq!(runs.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 36. Treasury: reconciliation ─────────────────────────────────────────────

dual_backend_test!(treasury_reconciliation, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // Create an account first.
    let acct_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/accounts"),
            json!({ "account_code": "cash", "account_name": "Cash", "currency": "usd" }),
        )
        .await;
    let acct_id = body_json(acct_resp).await["account_id"]
        .as_str()
        .unwrap()
        .to_owned();

    // List reconciliations (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/reconciliations")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/reconciliations"),
            json!({
                "account_id": acct_id,
                "period_end": "2024-03-31",
                "statement_balance_cents": 50_000_00i64,
                "book_balance_cents": 50_000_00i64
            }),
        )
        .await;
    let status = create_resp.status();
    let rec = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create reconciliation: {rec}");
    assert!(rec["reconciliation_id"].is_string(), "{rec}");

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/reconciliations")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let recs = body_json(list_resp2).await;
    assert_eq!(recs.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 37. Execution: intents ────────────────────────────────────────────────────

dual_backend_test!(execution_intents, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/intents")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create intent.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/intents"),
            json!({
                "intent_type": "hire_employee",
                "authority_tier": "tier1",
                "description": "Hire software engineer",
                "metadata": {}
            }),
        )
        .await;
    let status = create_resp.status();
    let intent = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create intent: {intent}");
    assert!(intent["intent_id"].is_string(), "{intent}");

    let intent_id = intent["intent_id"].as_str().unwrap().to_owned();

    // Get intent.
    let get_resp = ctx.get(&format!("/v1/entities/{eid}/intents/{intent_id}")).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    // Evaluate.
    let eval_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/intents/{intent_id}/evaluate"))
        .await;
    let eval_status = eval_resp.status();
    let eval_body = body_json(eval_resp).await;
    assert!(
        eval_status == StatusCode::OK || eval_status == StatusCode::BAD_REQUEST,
        "evaluate intent unexpected {eval_status}: {eval_body}"
    );

    // Authorize.
    let auth_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/intents/{intent_id}/authorize"))
        .await;
    let auth_status = auth_resp.status();
    let auth_body = body_json(auth_resp).await;
    assert!(
        auth_status == StatusCode::OK || auth_status == StatusCode::BAD_REQUEST,
        "authorize intent unexpected {auth_status}: {auth_body}"
    );

    // Execute.
    let exec_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/intents/{intent_id}/execute"))
        .await;
    let exec_status = exec_resp.status();
    let exec_body = body_json(exec_resp).await;
    assert!(
        exec_status == StatusCode::OK || exec_status == StatusCode::BAD_REQUEST,
        "execute intent unexpected {exec_status}: {exec_body}"
    );

    // Cancel a freshly-created intent.
    let create2_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/intents"),
            json!({
                "intent_type": "vendor_payment",
                "authority_tier": "tier1",
                "description": "Pay vendor",
                "metadata": {}
            }),
        )
        .await;
    let intent2_id = body_json(create2_resp).await["intent_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let cancel_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/intents/{intent2_id}/cancel"))
        .await;
    let cancel_status = cancel_resp.status();
    let cancel_body = body_json(cancel_resp).await;
    assert!(
        cancel_status == StatusCode::OK || cancel_status == StatusCode::BAD_REQUEST,
        "cancel intent unexpected {cancel_status}: {cancel_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/intents")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let intents = body_json(list_resp2).await;
    assert!(
        intents.as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "at least one intent should appear in list"
    );
});

// ── 38. Execution: obligations ────────────────────────────────────────────────

dual_backend_test!(execution_obligations, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/obligations")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create obligation.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/obligations"),
            json!({
                "obligation_type": "annual_report_filing",
                "assignee_type": "internal",
                "assignee_id": null,
                "description": "File annual report with Delaware",
                "due_date": "2025-03-01",
                "intent_id": null
            }),
        )
        .await;
    let status = create_resp.status();
    let obligation = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create obligation: {obligation}");
    assert!(obligation["obligation_id"].is_string(), "{obligation}");

    let ob_id = obligation["obligation_id"].as_str().unwrap().to_owned();

    // Get.
    let get_resp = ctx.get(&format!("/v1/entities/{eid}/obligations/{ob_id}")).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    // Start.
    let start_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/obligations/{ob_id}/start"))
        .await;
    let start_status = start_resp.status();
    let start_body = body_json(start_resp).await;
    assert!(
        start_status == StatusCode::OK || start_status == StatusCode::BAD_REQUEST,
        "start obligation unexpected {start_status}: {start_body}"
    );

    // Fulfill.
    let fulfill_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/obligations/{ob_id}/fulfill"))
        .await;
    let fulfill_status = fulfill_resp.status();
    let fulfill_body = body_json(fulfill_resp).await;
    assert!(
        fulfill_status == StatusCode::OK || fulfill_status == StatusCode::BAD_REQUEST,
        "fulfill obligation unexpected {fulfill_status}: {fulfill_body}"
    );

    // Waive a freshly-created obligation.
    let create2_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/obligations"),
            json!({
                "obligation_type": "tax_payment",
                "assignee_type": "internal",
                "assignee_id": null,
                "description": "Quarterly estimated tax",
                "due_date": null,
                "intent_id": null
            }),
        )
        .await;
    let ob2_id = body_json(create2_resp).await["obligation_id"]
        .as_str()
        .unwrap()
        .to_owned();

    let waive_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/obligations/{ob2_id}/waive"))
        .await;
    let waive_status = waive_resp.status();
    let waive_body = body_json(waive_resp).await;
    assert!(
        waive_status == StatusCode::OK || waive_status == StatusCode::BAD_REQUEST,
        "waive obligation unexpected {waive_status}: {waive_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/obligations")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let obs = body_json(list_resp2).await;
    assert!(obs.as_array().map(|a| a.len()).unwrap_or(0) >= 1);
});

// ── 39. Execution: receipts ───────────────────────────────────────────────────

dual_backend_test!(execution_receipts, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List receipts (receipts are created internally; just verify the endpoint works).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/receipts")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    let body = body_json(list_resp).await;
    assert!(body.is_array(), "expected array: {body}");
});

// ── 40. Contacts: full CRUD ───────────────────────────────────────────────────

dual_backend_test!(contacts_crud, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/contacts")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let contact = create_contact(&mut ctx, eid).await;
    assert!(contact["contact_id"].is_string(), "{contact}");
    assert_eq!(contact["name"], "Alice Founder");
    let contact_id = contact["contact_id"].as_str().unwrap().to_owned();

    // Get.
    let get_resp = ctx
        .get(&format!("/v1/entities/{eid}/contacts/{contact_id}"))
        .await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["contact_id"], contact["contact_id"]);

    // Update (PATCH).
    let update_resp = ctx
        .patch(
            &format!("/v1/entities/{eid}/contacts/{contact_id}"),
            json!({
                "name": "Alice Smith",
                "email": "alice.smith@example.com",
                "phone": null,
                "mailing_address": null,
                "category": null,
                "cap_table_access": null,
                "notes": "Updated"
            }),
        )
        .await;
    assert_eq!(update_resp.status(), StatusCode::OK);
    let updated = body_json(update_resp).await;
    assert_eq!(updated["name"], "Alice Smith");

    // List (non-empty, verify persistence).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/contacts")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let contacts = body_json(list_resp2).await;
    assert_eq!(contacts.as_array().map(|a| a.len()).unwrap_or(0), 1);

    // Deactivate.
    let deact_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{eid}/contacts/{contact_id}/deactivate"
        ))
        .await;
    assert_eq!(deact_resp.status(), StatusCode::OK);
    let deactivated = body_json(deact_resp).await;
    assert_eq!(
        deactivated["status"], "inactive",
        "contact should be deactivated: {deactivated}"
    );
});

// ── 41. Agents: full CRUD ─────────────────────────────────────────────────────

dual_backend_test!(agents_crud, |ctx| {
    // List (empty — agents store auto-creates).
    let list_resp = ctx.get("/v1/agents").await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let create_resp = ctx
        .post(
            "/v1/agents",
            json!({
                "name": "Formation Agent",
                "system_prompt": "You help with entity formations.",
                "model": "claude-sonnet-4-5",
                "entity_id": null
            }),
        )
        .await;
    let status = create_resp.status();
    let agent = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create agent: {agent}");
    assert!(agent["agent_id"].is_string(), "{agent}");

    let agent_id = agent["agent_id"].as_str().unwrap().to_owned();

    // Get.
    let get_resp = ctx.get(&format!("/v1/agents/{agent_id}")).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["agent_id"], agent["agent_id"]);

    // Update (PATCH).
    let update_resp = ctx
        .patch(
            &format!("/v1/agents/{agent_id}"),
            json!({
                "name": "Formation Agent v2",
                "system_prompt": null,
                "model": null
            }),
        )
        .await;
    assert_eq!(update_resp.status(), StatusCode::OK);
    let updated = body_json(update_resp).await;
    assert_eq!(updated["name"], "Formation Agent v2");

    // Add skill.
    let skill_resp = ctx
        .post(
            &format!("/v1/agents/{agent_id}/skills"),
            json!({
                "name": "entity_formation",
                "description": "Handles entity formation workflows",
                "instructions": "Follow the formation checklist."
            }),
        )
        .await;
    let skill_status = skill_resp.status();
    let skill_body = body_json(skill_resp).await;
    assert_eq!(skill_status, StatusCode::OK, "add skill: {skill_body}");

    // Pause.
    let pause_resp = ctx.post_empty(&format!("/v1/agents/{agent_id}/pause")).await;
    let pause_status = pause_resp.status();
    let pause_body = body_json(pause_resp).await;
    assert!(
        pause_status == StatusCode::OK || pause_status == StatusCode::BAD_REQUEST,
        "pause agent unexpected {pause_status}: {pause_body}"
    );

    // Resume.
    let resume_resp = ctx.post_empty(&format!("/v1/agents/{agent_id}/resume")).await;
    let resume_status = resume_resp.status();
    let resume_body = body_json(resume_resp).await;
    assert!(
        resume_status == StatusCode::OK || resume_status == StatusCode::BAD_REQUEST,
        "resume agent unexpected {resume_status}: {resume_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get("/v1/agents").await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let agents = body_json(list_resp2).await;
    assert_eq!(agents.as_array().map(|a| a.len()).unwrap_or(0), 1);

    // Delete.
    let delete_resp = ctx.delete(&format!("/v1/agents/{agent_id}")).await;
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    // List (empty again).
    let list_resp3 = ctx.get("/v1/agents").await;
    assert_eq!(list_resp3.status(), StatusCode::OK);
    assert!(
        body_json(list_resp3).await.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "list should be empty after delete"
    );
});

// ── 42. Work items: full lifecycle ────────────────────────────────────────────

dual_backend_test!(work_items_lifecycle, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/work-items")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/work-items"),
            json!({
                "title": "File Q1 taxes",
                "description": "Prepare and file quarterly tax return",
                "category": "compliance",
                "deadline": "2024-04-15",
                "asap": false
            }),
        )
        .await;
    let status = create_resp.status();
    let item = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create work item: {item}");
    assert!(item["work_item_id"].is_string(), "{item}");

    let item_id = item["work_item_id"].as_str().unwrap().to_owned();

    // Get.
    let get_resp = ctx.get(&format!("/v1/entities/{eid}/work-items/{item_id}")).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    // Claim.
    let claim_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/work-items/{item_id}/claim"),
            json!({
                "claimed_by": "agent-001",
                "claim_ttl_seconds": 3600u64
            }),
        )
        .await;
    let claim_status = claim_resp.status();
    let claim_body = body_json(claim_resp).await;
    assert!(
        claim_status == StatusCode::OK || claim_status == StatusCode::BAD_REQUEST,
        "claim work item unexpected {claim_status}: {claim_body}"
    );

    // Release.
    let release_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/work-items/{item_id}/release"))
        .await;
    let release_status = release_resp.status();
    let release_body = body_json(release_resp).await;
    assert!(
        release_status == StatusCode::OK || release_status == StatusCode::BAD_REQUEST,
        "release work item unexpected {release_status}: {release_body}"
    );

    // Re-claim before completing.
    let claim2_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/work-items/{item_id}/claim"),
            json!({ "claimed_by": "agent-002", "claim_ttl_seconds": null }),
        )
        .await;
    let _ = body_json(claim2_resp).await; // drain

    // Complete.
    let complete_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/work-items/{item_id}/complete"),
            json!({
                "completed_by": "agent-002",
                "result": "Tax return submitted"
            }),
        )
        .await;
    let complete_status = complete_resp.status();
    let complete_body = body_json(complete_resp).await;
    assert!(
        complete_status == StatusCode::OK || complete_status == StatusCode::BAD_REQUEST,
        "complete work item unexpected {complete_status}: {complete_body}"
    );

    // Create a second item to cancel.
    let create2_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/work-items"),
            json!({
                "title": "Renew registered agent",
                "description": "Annual registered agent renewal",
                "category": "compliance",
                "deadline": null,
                "asap": false
            }),
        )
        .await;
    let create2_body = body_json(create2_resp).await;
    let item2_id = create2_body["work_item_id"]
        .as_str()
        .expect("work_item_id should be a string")
        .to_owned();

    let cancel_resp = ctx
        .post_empty(&format!("/v1/entities/{eid}/work-items/{item2_id}/cancel"))
        .await;
    let cancel_status = cancel_resp.status();
    let cancel_body = body_json(cancel_resp).await;
    assert!(
        cancel_status == StatusCode::OK || cancel_status == StatusCode::BAD_REQUEST,
        "cancel work item unexpected {cancel_status}: {cancel_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/work-items")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let items = body_json(list_resp2).await;
    assert!(items.as_array().map(|a| a.len()).unwrap_or(0) >= 1);
});

// ── 43. Services: full lifecycle ──────────────────────────────────────────────

dual_backend_test!(services_lifecycle, |ctx| {
    let entity = create_ccorp(&mut ctx).await;
    let eid = entity["entity_id"].as_str().unwrap();

    // List (empty).
    let list_resp = ctx.get(&format!("/v1/entities/{eid}/service-requests")).await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create service request.
    let create_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/service-requests"),
            json!({
                "service_slug": "registered-agent-de",
                "amount_cents": 50_00i64
            }),
        )
        .await;
    let status = create_resp.status();
    let svc = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create service request: {svc}");
    assert!(svc["request_id"].is_string(), "{svc}");

    let request_id = svc["request_id"].as_str().unwrap().to_owned();

    // Get.
    let get_resp = ctx
        .get(&format!("/v1/entities/{eid}/service-requests/{request_id}"))
        .await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let fetched = body_json(get_resp).await;
    assert_eq!(fetched["request_id"], svc["request_id"]);

    // Checkout.
    let checkout_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{eid}/service-requests/{request_id}/checkout"
        ))
        .await;
    let checkout_status = checkout_resp.status();
    let checkout_body = body_json(checkout_resp).await;
    assert!(
        checkout_status == StatusCode::OK || checkout_status == StatusCode::BAD_REQUEST,
        "checkout unexpected {checkout_status}: {checkout_body}"
    );

    // Pay.
    let pay_resp = ctx
        .post_empty(&format!(
            "/v1/entities/{eid}/service-requests/{request_id}/pay"
        ))
        .await;
    let pay_status = pay_resp.status();
    let pay_body = body_json(pay_resp).await;
    assert!(
        pay_status == StatusCode::OK || pay_status == StatusCode::BAD_REQUEST,
        "pay unexpected {pay_status}: {pay_body}"
    );

    // Fulfill.
    let fulfill_resp = ctx
        .post(
            &format!("/v1/entities/{eid}/service-requests/{request_id}/fulfill"),
            json!({ "fulfillment_note": "Registered agent filed." }),
        )
        .await;
    let fulfill_status = fulfill_resp.status();
    let fulfill_body = body_json(fulfill_resp).await;
    assert!(
        fulfill_status == StatusCode::OK || fulfill_status == StatusCode::BAD_REQUEST,
        "fulfill unexpected {fulfill_status}: {fulfill_body}"
    );

    // List (non-empty).
    let list_resp2 = ctx.get(&format!("/v1/entities/{eid}/service-requests")).await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let requests = body_json(list_resp2).await;
    assert_eq!(requests.as_array().map(|a| a.len()).unwrap_or(0), 1);
});

// ── 44. Admin: list workspaces ────────────────────────────────────────────────

dual_backend_test!(admin_workspaces, |ctx| {
    // Create an entity so the workspace directory exists on disk.
    create_ccorp(&mut ctx).await;

    let resp = ctx.get("/v1/workspaces").await;
    assert_eq!(resp.status(), StatusCode::OK);
    let workspaces = body_json(resp).await;
    assert!(workspaces.is_array(), "expected array: {workspaces}");

    let wid_str = ctx.workspace_id.to_string();
    let ids: Vec<&str> = workspaces
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|w| w["workspace_id"].as_str())
        .collect();
    assert!(
        ids.contains(&wid_str.as_str()),
        "workspace {} not found in list: {workspaces}",
        ctx.workspace_id
    );
});

// ── 45. Admin: list workspace entities ───────────────────────────────────────

dual_backend_test!(admin_list_workspace_entities, |ctx| {
    ensure_workspace(&ctx).await;

    let wid = ctx.workspace_id;
    let resp = ctx.get(&format!("/v1/workspaces/{wid}/entities")).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_array(), "expected array: {body}");
});

// ── 46. Admin: API keys CRUD ──────────────────────────────────────────────────

dual_backend_test!(admin_api_keys, |ctx| {
    ensure_workspace(&ctx).await;

    // List (empty).
    let list_resp = ctx.get("/v1/api-keys").await;
    assert_eq!(list_resp.status(), StatusCode::OK);
    assert!(
        body_json(list_resp).await.as_array().map(|a| a.is_empty()).unwrap_or(false)
    );

    // Create API key.
    let create_resp = ctx
        .post(
            "/v1/api-keys",
            json!({
                "name": "CI Deploy Key",
                "scopes": ["formation:read", "formation:write"],
                "entity_id": null
            }),
        )
        .await;
    let status = create_resp.status();
    let key = body_json(create_resp).await;
    assert_eq!(status, StatusCode::OK, "create api key: {key}");
    assert!(key["key_id"].is_string(), "{key}");
    assert!(key["raw_key"].is_string(), "{key}");

    let key_id = key["key_id"].as_str().unwrap().to_owned();

    // List (non-empty).
    let list_resp2 = ctx.get("/v1/api-keys").await;
    assert_eq!(list_resp2.status(), StatusCode::OK);
    let keys = body_json(list_resp2).await;
    assert_eq!(keys.as_array().map(|a| a.len()).unwrap_or(0), 1);

    // Revoke.
    let revoke_resp = ctx
        .post_empty(&format!("/v1/api-keys/{key_id}/revoke"))
        .await;
    assert_eq!(revoke_resp.status(), StatusCode::OK);
    let revoked = body_json(revoke_resp).await;
    assert_eq!(
        revoked["deleted"], true,
        "key should be marked deleted: {revoked}"
    );

    // List (empty again — soft-deleted excluded).
    let list_resp3 = ctx.get("/v1/api-keys").await;
    assert_eq!(list_resp3.status(), StatusCode::OK);
    assert!(
        body_json(list_resp3).await.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "list should be empty after revoke"
    );
});

// ── 47. Cross-backend consistency ─────────────────────────────────────────────

/// Git baseline: verify structural shape of a formation sequence.
#[tokio::test]
async fn cross_backend_formation_git_is_baseline() {
    let mut ctx = TestCtx::with_git();

    let entity = create_ccorp(&mut ctx).await;
    assert!(entity["entity_id"].is_string());
    assert_eq!(entity["entity_type"], "c_corp");
    assert_eq!(entity["formation_status"], "pending");
    assert!(entity["legal_name"].is_string());

    let filing_resp = ctx
        .get(&format!(
            "/v1/formations/{}/filing",
            entity["entity_id"].as_str().unwrap()
        ))
        .await;
    assert_eq!(filing_resp.status(), StatusCode::OK);
    let filing = body_json(filing_resp).await;
    assert!(filing["filing_id"].is_string());
    assert_eq!(filing["entity_id"], entity["entity_id"]);
}

/// KV vs git: verify that both backends produce structurally identical responses
/// for the same formation sequence.
#[tokio::test]
#[ignore = "requires REDIS_URL; run with --include-ignored"]
async fn cross_backend_formation_kv_matches_git() {
    if std::env::var("REDIS_URL").is_err() {
        eprintln!("REDIS_URL not set — skipping kv cross-backend test");
        return;
    }

    let mut git_ctx = TestCtx::with_git();
    let mut kv_ctx = TestCtx::with_kv().await;

    // Run same sequence on both backends.
    let git_entity = create_ccorp(&mut git_ctx).await;
    let kv_entity = create_ccorp(&mut kv_ctx).await;

    // Structural shape must match (values like entity_id will differ, being random UUIDs).
    for key in &["entity_type", "formation_status", "legal_name"] {
        assert_eq!(
            git_entity[key], kv_entity[key],
            "field '{}' differs between git and kv backends",
            key
        );
    }
    assert!(git_entity["entity_id"].is_string());
    assert!(kv_entity["entity_id"].is_string());

    // Advance both and compare resulting status.
    let git_adv = body_json(
        git_ctx
            .post_empty(&format!(
                "/v1/formations/{}/advance",
                git_entity["entity_id"].as_str().unwrap()
            ))
            .await,
    )
    .await;
    let kv_adv = body_json(
        kv_ctx
            .post_empty(&format!(
                "/v1/formations/{}/advance",
                kv_entity["entity_id"].as_str().unwrap()
            ))
            .await,
    )
    .await;

    assert_eq!(
        git_adv["formation_status"], kv_adv["formation_status"],
        "formation_status after advance differs: git={} kv={}",
        git_adv["formation_status"], kv_adv["formation_status"]
    );
}
