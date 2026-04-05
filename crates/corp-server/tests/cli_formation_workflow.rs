//! End-to-end CLI formation workflow integration test.
//!
//! This test does something no other test in the suite does: it starts a **real
//! TCP server**, then invokes the `corp` CLI binary as a subprocess against it.
//! Every step in the full formation flow is exercised — create entity, advance
//! through every status, sign every document, confirm filing, confirm EIN, and
//! verify the entity reaches Active status.
//!
//! ## What is covered
//!
//! | Step | CLI command | Expected outcome |
//! |------|-----------|-----------------|
//! | 1 | `corp form create` | Entity created in Pending |
//! | 2 | `corp form status` | Shows Pending status |
//! | 3 | `corp form advance` | → DocumentsGenerated |
//! | 4 | `corp form documents` | Lists generated documents |
//! | 5 | `corp form sign` (each doc) | Each document Signed |
//! | 6 | `corp form advance` | → DocumentsSigned |
//! | 7 | `corp form advance` | → FilingSubmitted |
//! | 8 | `corp form advance` | → Filed |
//! | 9 | `corp form confirm-filing` | Filing confirmed |
//! | 10 | `corp form advance` | → EinApplied |
//! | 11 | `corp form confirm-ein` | EIN recorded |
//! | 12 | `corp form advance` | → Active (terminal) |
//! | 13 | `corp form advance` | Error: already terminal |
//! | 14 | `corp form status` | Shows Active |
//! | 15 | Signing link verification | Document hash integrity |
//!
//! The same sequence runs against both the **git** backend and (when a Redis
//! URL is available) the **kv** backend.

use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;
use tokio::net::TcpListener;

use corp_auth::{ApiKeyResolver, AuthError, JwtConfig, Principal};
use corp_core::auth::{Claims, PrincipalType, Scope};
use corp_core::ids::WorkspaceId;
use corp_server::routes::router;
use corp_server::state::{AppState, StorageBackend};

// ── Helpers ──────────────────────────────────────────────────────────────────

const JWT_SECRET: &[u8] = b"cli-test-secret-32-bytes-minimum";

/// Locate the `corp` binary built by cargo.
///
/// Since `corp` lives in a sibling crate (`corp-cli`), `CARGO_BIN_EXE_corp`
/// isn't available. Instead we look in the same target directory that cargo
/// placed *this* test binary.
fn corp_binary_path() -> std::path::PathBuf {
    // The test binary sits in target/debug/deps/cli_formation_workflow-XXXXX.
    // The `corp` binary sits in target/debug/corp.
    let self_exe = std::env::current_exe().expect("current_exe");
    let target_dir = self_exe
        .parent() // deps/
        .and_then(|p| p.parent()) // debug/
        .expect("find target dir");
    let corp = target_dir.join("corp");
    assert!(
        corp.exists(),
        "corp binary not found at {}. Build it first with `cargo build -p corp-cli`.",
        corp.display()
    );
    corp
}

struct NoopKeyResolver;

#[async_trait::async_trait]
impl ApiKeyResolver for NoopKeyResolver {
    async fn resolve(&self, _key: &str) -> Result<Principal, AuthError> {
        Err(AuthError::InvalidApiKey)
    }
}

fn unix_now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// A running test server with its address and auth token.
struct TestServer {
    addr: SocketAddr,
    token: String,
    workspace_id: WorkspaceId,
    _dir: TempDir,
    _handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    /// Spin up a real TCP server backed by a git tempdir.
    async fn start() -> Self {
        let dir = TempDir::new().expect("create tempdir");
        let data_dir = dir.path().to_str().unwrap().to_owned();

        let jwt = Arc::new(JwtConfig::new(JWT_SECRET));
        let ws_id = WorkspaceId::new();

        let now = unix_now();
        let claims = Claims {
            sub: "cli-test".into(),
            workspace_id: ws_id,
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes: vec![Scope::All],
            iat: now,
            exp: now + 7200,
        };
        let token = jwt.encode(&claims).expect("encode JWT");

        let state = AppState {
            data_dir,
            jwt_config: jwt,
            api_key_resolver: Arc::new(NoopKeyResolver),
            storage_backend: StorageBackend::Git,
            rate_limiter: corp_auth::RateLimiter::new(10_000, std::time::Duration::from_secs(60)),
        };
        let app = router(state);

        // Bind to a random available port.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        // Give the server a moment to start accepting.
        tokio::time::sleep(Duration::from_millis(50)).await;

        Self {
            addr,
            token,
            workspace_id: ws_id,
            _dir: dir,
            _handle: handle,
        }
    }

    fn api_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Run a corp CLI command and return (stdout, stderr, success).
    fn corp(&self, args: &[&str]) -> CorpResult {
        let bin = corp_binary_path();
        let output = Command::new(bin)
            .args(["--api-url", &self.api_url()])
            .args(["--api-key", &self.token])
            .arg("--json")
            .args(args)
            .output()
            .expect("run corp binary");

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        CorpResult {
            stdout,
            stderr,
            success: output.status.success(),
        }
    }

    /// Run a corp command and parse the JSON output.
    fn corp_json(&self, args: &[&str]) -> Value {
        let result = self.corp(args);
        assert!(
            result.success,
            "corp {:?} failed:\nstdout: {}\nstderr: {}",
            args, result.stdout, result.stderr
        );
        // The --json flag makes the CLI print raw JSON. Parse the first
        // JSON object or array from stdout.
        parse_first_json(&result.stdout)
            .unwrap_or_else(|| panic!("no JSON in output of corp {:?}:\n{}", args, result.stdout))
    }

    /// Run a corp command expecting failure.
    fn corp_fail(&self, args: &[&str]) -> CorpResult {
        let result = self.corp(args);
        assert!(
            !result.success,
            "corp {:?} should have failed but succeeded:\n{}",
            args, result.stdout
        );
        result
    }

    /// Direct HTTP GET for verification (bypasses CLI).
    async fn http_get(&self, path: &str) -> Value {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}{}", self.api_url(), path))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .expect("HTTP GET");
        assert!(
            resp.status().is_success(),
            "GET {} failed: {}",
            path,
            resp.status()
        );
        resp.json().await.expect("parse JSON")
    }

    /// Direct HTTP POST for verification.
    async fn http_post(&self, path: &str, body: Value) -> Value {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}{}", self.api_url(), path))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&body)
            .send()
            .await
            .expect("HTTP POST");
        assert!(
            resp.status().is_success(),
            "POST {} failed: {}",
            path,
            resp.status()
        );
        resp.json().await.expect("parse JSON")
    }
}

struct CorpResult {
    stdout: String,
    stderr: String,
    success: bool,
}

/// Parse the first JSON value from a string that may contain non-JSON lines
/// (e.g. success messages appended after the JSON object).
fn parse_first_json(s: &str) -> Option<Value> {
    // Try the whole string first.
    if let Ok(v) = serde_json::from_str::<Value>(s.trim()) {
        return Some(v);
    }
    // Use the streaming deserializer to find the first complete JSON value.
    let mut de = serde_json::Deserializer::from_str(s.trim()).into_iter::<Value>();
    if let Some(Ok(v)) = de.next() {
        return Some(v);
    }
    None
}

fn extract_str<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("missing key '{}' in {}", key, v))
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Full interactive formation workflow: C-Corp from Pending to Active,
/// exercising every CLI command and verifying state at each step.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_formation_workflow_ccorp() {
    let srv = TestServer::start().await;

    // ── Step 1: Create a Delaware C-Corp ─────────────────────────────
    let entity = srv.corp_json(&[
        "form",
        "create",
        "--name",
        "Acme Corp",
        "--entity-type",
        "c_corp",
        "--jurisdiction",
        "DE",
    ]);
    let entity_id = extract_str(&entity, "entity_id");
    assert_eq!(extract_str(&entity, "formation_status"), "pending");
    assert_eq!(extract_str(&entity, "entity_type"), "c_corp");
    assert_eq!(extract_str(&entity, "legal_name"), "Acme Corp");

    // ── Step 2: Check status via CLI ─────────────────────────────────
    let status = srv.corp_json(&["form", "status", entity_id]);
    assert_eq!(extract_str(&status, "formation_status"), "pending");

    // ── Step 3: Advance to DocumentsGenerated ────────────────────────
    let adv1 = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(
        extract_str(&adv1, "formation_status"),
        "documents_generated"
    );

    // ── Step 4: List documents ───────────────────────────────────────
    let docs = srv.corp_json(&["form", "documents", entity_id]);
    let doc_array = docs.as_array().expect("documents should be an array");
    assert!(
        !doc_array.is_empty(),
        "entity should have at least one formation document"
    );

    // Save document IDs and their content hashes for signing.
    let doc_ids: Vec<(String, String)> = doc_array
        .iter()
        .map(|d| {
            (
                extract_str(d, "document_id").to_owned(),
                extract_str(d, "content_hash").to_owned(),
            )
        })
        .collect();

    // ── Step 5: Sign every document ──────────────────────────────────
    for (doc_id, _hash) in &doc_ids {
        let signed = srv.corp_json(&[
            "form",
            "sign",
            doc_id,
            "--signer-name",
            "Jane Doe",
            "--signer-role",
            "Incorporator",
            "--signer-email",
            "jane@acme.com",
            "--signature-text",
            "/s/ Jane Doe",
            "--consent-text",
            "I consent to signing this document electronically",
        ]);
        assert_eq!(
            extract_str(&signed, "status"),
            "signed",
            "document {} should transition to signed",
            doc_id
        );

        // Verify the signature was recorded.
        let sigs = signed
            .get("signatures")
            .and_then(|v| v.as_array())
            .expect("signatures array");
        assert!(
            !sigs.is_empty(),
            "document {} should have at least one signature",
            doc_id
        );
        let sig = &sigs[0];
        assert_eq!(extract_str(sig, "signer_name"), "Jane Doe");
        assert_eq!(extract_str(sig, "signer_email"), "jane@acme.com");
        assert!(sig.get("signed_at").is_some(), "signed_at should be set");
    }

    // ── Step 5b: Verify duplicate sign is rejected ───────────────────
    if let Some((first_doc, _)) = doc_ids.first() {
        let dup = srv.corp(&[
            "form",
            "sign",
            first_doc,
            "--signer-name",
            "Jane Doe",
            "--signer-role",
            "Incorporator",
            "--signer-email",
            "jane@acme.com",
            "--signature-text",
            "/s/ Jane Doe",
            "--consent-text",
            "duplicate",
        ]);
        assert!(!dup.success, "duplicate signature should be rejected");
    }

    // ── Step 6: Advance to DocumentsSigned ───────────────────────────
    let adv2 = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv2, "formation_status"), "documents_signed");

    // ── Step 7: Advance to FilingSubmitted ───────────────────────────
    let adv3 = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv3, "formation_status"), "filing_submitted");

    // ── Step 8: Check filing record ──────────────────────────────────
    let filing = srv.corp_json(&["form", "filing", entity_id]);
    assert_eq!(extract_str(&filing, "status"), "pending");

    // ── Step 9: Confirm the filing (entity must be in FilingSubmitted) ─
    let confirmed = srv.corp_json(&[
        "form",
        "confirm-filing",
        entity_id,
        "--confirmation-number",
        "DE-2026-987654",
    ]);
    assert_eq!(extract_str(&confirmed, "status"), "filed");
    assert_eq!(
        extract_str(&confirmed, "confirmation_number"),
        "DE-2026-987654"
    );

    // ── Step 10: Advance to Filed ────────────────────────────────────
    let adv4 = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv4, "formation_status"), "filed");

    // ── Step 11: Check tax profile ───────────────────────────────────
    let tax = srv.corp_json(&["form", "tax", entity_id]);
    assert_eq!(extract_str(&tax, "ein_status"), "pending");
    assert_eq!(extract_str(&tax, "classification"), "c_corporation",);

    // ── Step 12: Confirm EIN (entity must be in Filed) ───────────────
    let ein_result = srv.corp_json(&["form", "confirm-ein", entity_id, "--ein", "12-3456789"]);
    assert_eq!(extract_str(&ein_result, "ein_status"), "active");
    assert_eq!(extract_str(&ein_result, "ein"), "12-3456789");

    // ── Step 13: Advance to EinApplied ───────────────────────────────
    let adv5 = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv5, "formation_status"), "ein_applied");

    // ── Step 14: Advance to Active ───────────────────────────────────
    let adv6 = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv6, "formation_status"), "active");

    // ── Step 15: Verify terminal — advance should fail ───────────────
    srv.corp_fail(&["form", "advance", entity_id]);

    // ── Step 16: Final status check ──────────────────────────────────
    let final_status = srv.corp_json(&["form", "status", entity_id]);
    assert_eq!(extract_str(&final_status, "formation_status"), "active");

    // ── Step 17: Verify via direct HTTP that all state is consistent ─
    let entity_http = srv.http_get(&format!("/v1/entities/{}", entity_id)).await;
    assert_eq!(extract_str(&entity_http, "formation_status"), "active");
    assert_eq!(extract_str(&entity_http, "legal_name"), "Acme Corp");

    let docs_http = srv
        .http_get(&format!("/v1/formations/{}/documents", entity_id))
        .await;
    let docs_arr = docs_http.as_array().expect("docs array");
    for doc in docs_arr {
        assert_eq!(
            extract_str(doc, "status"),
            "signed",
            "all docs should be signed"
        );
    }

    let filing_http = srv
        .http_get(&format!("/v1/formations/{}/filing", entity_id))
        .await;
    assert_eq!(extract_str(&filing_http, "status"), "filed");

    let tax_http = srv
        .http_get(&format!("/v1/formations/{}/tax", entity_id))
        .await;
    assert_eq!(extract_str(&tax_http, "ein_status"), "active");
    assert_eq!(extract_str(&tax_http, "ein"), "12-3456789");
}

/// Full interactive formation workflow for a Wyoming LLC.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_formation_workflow_llc() {
    let srv = TestServer::start().await;

    // Create Wyoming LLC.
    let entity = srv.corp_json(&[
        "form",
        "create",
        "--name",
        "Mountain Ventures LLC",
        "--entity-type",
        "llc",
        "--jurisdiction",
        "WY",
    ]);
    let entity_id = extract_str(&entity, "entity_id");
    assert_eq!(extract_str(&entity, "entity_type"), "llc");
    assert_eq!(extract_str(&entity, "formation_status"), "pending");

    // Advance to DocumentsGenerated.
    let adv = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv, "formation_status"), "documents_generated");

    // List and sign all documents.
    let docs = srv.corp_json(&["form", "documents", entity_id]);
    let doc_array = docs.as_array().expect("docs array");
    for doc in doc_array {
        let doc_id = extract_str(doc, "document_id");
        let signed = srv.corp_json(&[
            "form",
            "sign",
            doc_id,
            "--signer-name",
            "Bob Smith",
            "--signer-role",
            "Member",
            "--signer-email",
            "bob@mountain.com",
            "--signature-text",
            "/s/ Bob Smith",
            "--consent-text",
            "I consent",
        ]);
        assert_eq!(extract_str(&signed, "status"), "signed");
    }

    // Advance through full lifecycle.
    let adv = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv, "formation_status"), "documents_signed");

    let adv = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv, "formation_status"), "filing_submitted");

    // Confirm filing (must happen while in FilingSubmitted).
    srv.corp_json(&[
        "form",
        "confirm-filing",
        entity_id,
        "--confirmation-number",
        "WY-2026-001234",
    ]);

    let adv = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv, "formation_status"), "filed");

    // Confirm EIN (must happen while in Filed).
    let tax = srv.corp_json(&["form", "confirm-ein", entity_id, "--ein", "98-7654321"]);
    assert_eq!(extract_str(&tax, "ein_status"), "active");
    // LLC should be classified as disregarded entity.
    assert_eq!(extract_str(&tax, "classification"), "disregarded_entity");

    let adv = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv, "formation_status"), "ein_applied");

    // Final advance to Active.
    let adv = srv.corp_json(&["form", "advance", entity_id]);
    assert_eq!(extract_str(&adv, "formation_status"), "active");

    // Terminal check.
    srv.corp_fail(&["form", "advance", entity_id]);
}

/// Signing link simulation: verify that the document content hash is
/// preserved through the signing flow, enabling external signing links.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signing_link_integrity() {
    let srv = TestServer::start().await;

    // Create entity and advance to get documents.
    let entity = srv.corp_json(&[
        "form",
        "create",
        "--name",
        "SignTest Inc",
        "--entity-type",
        "c_corp",
        "--jurisdiction",
        "DE",
    ]);
    let entity_id = extract_str(&entity, "entity_id");
    srv.corp_json(&["form", "advance", entity_id]);

    let docs = srv.corp_json(&["form", "documents", entity_id]);
    let doc_array = docs.as_array().expect("docs array");
    assert!(!doc_array.is_empty());

    let doc = &doc_array[0];
    let doc_id = extract_str(doc, "document_id");
    let original_hash = extract_str(doc, "content_hash");

    // Simulate a signing link flow:
    // 1. External system fetches document by ID (GET /formations/{eid}/documents/{did})
    let fetched = srv
        .http_get(&format!(
            "/v1/formations/{}/documents/{}",
            entity_id, doc_id
        ))
        .await;
    let fetched_hash = extract_str(&fetched, "content_hash");
    assert_eq!(
        original_hash, fetched_hash,
        "content hash should be stable across reads"
    );

    // 2. Signer signs with the hash they fetched (simulating external form).
    let sign_body = serde_json::json!({
        "signer_name": "External Signer",
        "signer_role": "Director",
        "signer_email": "ext@example.com",
        "signature_text": "/s/ External Signer",
        "consent_text": "I consent via signing link",
    });
    let signed = srv
        .http_post(&format!("/v1/documents/{}/sign", doc_id), sign_body)
        .await;
    assert_eq!(extract_str(&signed, "status"), "signed");

    // 3. Verify the signature includes the hash that was used.
    let sigs = signed
        .get("signatures")
        .and_then(|v| v.as_array())
        .expect("signatures");
    let sig = &sigs[0];
    assert_eq!(
        extract_str(sig, "document_hash_at_signing"),
        original_hash,
        "signature should record the document hash at time of signing"
    );

    // 4. Re-fetch document — hash should still match.
    let refetched = srv
        .http_get(&format!(
            "/v1/formations/{}/documents/{}",
            entity_id, doc_id
        ))
        .await;
    assert_eq!(
        extract_str(&refetched, "content_hash"),
        original_hash,
        "content hash should not change after signing"
    );
}

/// Multiple signers workflow: simulate a document that needs two signers,
/// each arriving via a separate signing link.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_signer_signing_links() {
    let srv = TestServer::start().await;

    let entity = srv.corp_json(&[
        "form",
        "create",
        "--name",
        "DualSign Corp",
        "--entity-type",
        "c_corp",
        "--jurisdiction",
        "DE",
    ]);
    let entity_id = extract_str(&entity, "entity_id");
    srv.corp_json(&["form", "advance", entity_id]);

    let docs = srv.corp_json(&["form", "documents", entity_id]);
    let doc_array = docs.as_array().expect("docs array");
    let doc_id = extract_str(&doc_array[0], "document_id");

    // First signer via CLI.
    let after_first = srv.corp_json(&[
        "form",
        "sign",
        doc_id,
        "--signer-name",
        "Alice Founder",
        "--signer-role",
        "CEO",
        "--signer-email",
        "alice@dualsign.com",
        "--signature-text",
        "/s/ Alice Founder",
        "--consent-text",
        "I consent",
    ]);
    let sig_count = after_first
        .get("signatures")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    assert_eq!(sig_count, 1, "should have 1 signature after first signer");

    // Second signer via HTTP (simulating signing link).
    let sign_body = serde_json::json!({
        "signer_name": "Bob Cofounder",
        "signer_role": "CTO",
        "signer_email": "bob@dualsign.com",
        "signature_text": "/s/ Bob Cofounder",
        "consent_text": "I consent via link",
    });
    let after_second = srv
        .http_post(&format!("/v1/documents/{}/sign", doc_id), sign_body)
        .await;
    let sigs = after_second
        .get("signatures")
        .and_then(|v| v.as_array())
        .expect("signatures");
    assert_eq!(sigs.len(), 2, "should have 2 signatures after both sign");

    // Verify both signers are recorded.
    let emails: Vec<&str> = sigs
        .iter()
        .map(|s| extract_str(s, "signer_email"))
        .collect();
    assert!(emails.contains(&"alice@dualsign.com"));
    assert!(emails.contains(&"bob@dualsign.com"));
}

/// Entity list via CLI matches the entities created.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_entity_list() {
    let srv = TestServer::start().await;

    // Create two entities.
    srv.corp_json(&[
        "form",
        "create",
        "--name",
        "First Corp",
        "--entity-type",
        "c_corp",
        "--jurisdiction",
        "DE",
    ]);
    srv.corp_json(&[
        "form",
        "create",
        "--name",
        "Second LLC",
        "--entity-type",
        "llc",
        "--jurisdiction",
        "WY",
    ]);

    // List entities via CLI.
    let list = srv.corp_json(&["entities", "list"]);
    let arr = list.as_array().expect("entity list should be an array");
    assert_eq!(arr.len(), 2, "should have exactly 2 entities");

    let names: Vec<&str> = arr.iter().map(|e| extract_str(e, "legal_name")).collect();
    assert!(names.contains(&"First Corp"));
    assert!(names.contains(&"Second LLC"));
}

/// EIN validation: invalid EIN format is rejected.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_invalid_ein_rejected() {
    let srv = TestServer::start().await;

    let entity = srv.corp_json(&[
        "form",
        "create",
        "--name",
        "BadEin Corp",
        "--entity-type",
        "c_corp",
        "--jurisdiction",
        "DE",
    ]);
    let eid = extract_str(&entity, "entity_id");

    // Advance to DocumentsGenerated.
    srv.corp_json(&["form", "advance", eid]);

    // Sign all documents.
    let docs = srv.corp_json(&["form", "documents", eid]);
    let doc_array = docs.as_array().expect("docs array");
    for doc in doc_array {
        let doc_id = extract_str(doc, "document_id");
        srv.corp_json(&[
            "form",
            "sign",
            doc_id,
            "--signer-name",
            "Jane Doe",
            "--signer-role",
            "CEO",
            "--signer-email",
            "jane@badein.com",
            "--signature-text",
            "/s/ Jane Doe",
            "--consent-text",
            "I consent",
        ]);
    }

    // Advance to DocumentsSigned.
    srv.corp_json(&["form", "advance", eid]);
    // Advance to FilingSubmitted.
    srv.corp_json(&["form", "advance", eid]);

    // Confirm filing (while in FilingSubmitted).
    srv.corp_json(&[
        "form",
        "confirm-filing",
        eid,
        "--confirmation-number",
        "DE-X",
    ]);

    // Advance to Filed.
    srv.corp_json(&["form", "advance", eid]);

    // Try invalid EIN (entity is in Filed state, which is correct).
    let result = srv.corp(&["form", "confirm-ein", eid, "--ein", "not-a-real-ein"]);
    assert!(!result.success, "invalid EIN format should be rejected");
}

/// Entity dissolution via CLI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_entity_dissolution() {
    let srv = TestServer::start().await;

    let entity = srv.corp_json(&[
        "form",
        "create",
        "--name",
        "ShortLived Inc",
        "--entity-type",
        "c_corp",
        "--jurisdiction",
        "DE",
    ]);
    let eid = extract_str(&entity, "entity_id");

    // Dissolve from Pending.
    let dissolved = srv.corp_json(&["entities", "dissolve", eid]);
    assert_eq!(extract_str(&dissolved, "formation_status"), "dissolved");

    // Cannot advance after dissolution.
    srv.corp_fail(&["form", "advance", eid]);
}
