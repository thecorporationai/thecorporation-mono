//! Carta-scale scenario integration tests for the corporate governance platform.
//!
//! These tests simulate complete, real-world corporate lifecycle workflows
//! modelled after the scenarios defined in
//! `ARCHITECTURE/SCENARIOS/SAAS-STARTUP.md` and `ARCHITECTURE/SCENARIOS/VC-FUND.md`.
//!
//! Every test spins up the full Axum router in-process (no TCP socket) and
//! drives it through the tower `ServiceExt::oneshot` API. Each scenario uses
//! a dedicated tempdir and freshly-minted JWT, so tests are fully isolated
//! and can run in parallel.
//!
//! ## Test matrix
//!
//! | Test | Backend | Notes |
//! |------|---------|-------|
//! | `saas_startup_git` | Git (tempdir) | Full Series A journey |
//! | `saas_startup_kv` | KV (Redis) | Same scenario; `#[ignore]` by default |
//! | `vc_fund_git` | Git (tempdir) | Full fund lifecycle |
//! | `vc_fund_kv` | KV (Redis) | Same scenario; `#[ignore]` by default |

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

use corp_auth::{ApiKeyResolver, AuthError, JwtConfig, Principal};
use corp_core::auth::{Claims, PrincipalType, Scope};
use corp_core::ids::WorkspaceId;
use corp_server::routes::router;
use corp_server::state::{AppState, StorageBackend};

// ── Shared constants ──────────────────────────────────────────────────────────

const TEST_JWT_SECRET: &[u8] = b"scenario-test-secret-do-not-use-in-production";

// ── No-op API key resolver ────────────────────────────────────────────────────

struct NoopApiKeyResolver;

#[async_trait::async_trait]
impl ApiKeyResolver for NoopApiKeyResolver {
    async fn resolve(&self, _raw_key: &str) -> Result<Principal, AuthError> {
        Err(AuthError::InvalidApiKey)
    }
}

// ── Utility: current Unix timestamp ──────────────────────────────────────────

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ── Response body helper ──────────────────────────────────────────────────────

async fn body_json(resp: axum::response::Response) -> Value {
    use axum::body::to_bytes;
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

// ── ScenarioRunner ────────────────────────────────────────────────────────────

/// Stateful test driver for a full governance scenario.
///
/// Holds the in-process router and a JWT token, plus all IDs created during
/// the scenario so subsequent steps can refer back to earlier objects.
#[allow(dead_code)]
struct ScenarioRunner {
    /// Holds the tempdir alive for the duration of the test.
    _dir: TempDir,
    /// Axum router under test.
    app: axum::Router,
    /// Pre-signed JWT with `Scope::All` for this workspace.
    token: String,
    /// Workspace scoped to this runner.
    workspace_id: WorkspaceId,

    // ── Tracked IDs ───────────────────────────────────────────────────────────
    /// entity logical name → entity_id returned by the server.
    entities: HashMap<String, String>,
    /// entity logical name → cap_table_id.
    cap_tables: HashMap<String, String>,
    /// entity logical name → governance body_id (primary body).
    gov_bodies: HashMap<String, String>,
    /// contact logical name → contact_id.
    contacts: HashMap<String, String>,
    /// All equity grant IDs issued during the scenario.
    grants: Vec<String>,
    /// All SAFE note IDs issued during the scenario.
    safes: Vec<String>,
    /// All funding round IDs.
    rounds: Vec<String>,
    /// All bank account IDs.
    bank_accounts: Vec<String>,
    /// All valuation IDs.
    valuations: Vec<String>,
    /// All payroll run IDs.
    payroll_runs: Vec<String>,
    /// Instrument logical name → instrument_id.
    instruments: HashMap<String, String>,
    /// Holder logical name → holder_id.
    holders: HashMap<String, String>,
    /// GL account logical name → account_id.
    accounts: HashMap<String, String>,
}

impl ScenarioRunner {
    /// Construct a runner backed by a git tempdir store.
    fn new_git() -> Self {
        let dir = TempDir::new().expect("create tempdir");
        let data_dir = dir.path().to_str().unwrap().to_owned();

        let jwt_config = Arc::new(JwtConfig::new(TEST_JWT_SECRET));
        let workspace_id = WorkspaceId::new();

        let now = unix_now();
        let claims = Claims {
            sub: "scenario-test-user".to_owned(),
            workspace_id,
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes: vec![Scope::All],
            iat: now,
            exp: now + 7200,
        };
        let token = jwt_config.encode(&claims).expect("encode test JWT");

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
            token,
            workspace_id,
            entities: HashMap::new(),
            cap_tables: HashMap::new(),
            gov_bodies: HashMap::new(),
            contacts: HashMap::new(),
            grants: Vec::new(),
            safes: Vec::new(),
            rounds: Vec::new(),
            bank_accounts: Vec::new(),
            valuations: Vec::new(),
            payroll_runs: Vec::new(),
            instruments: HashMap::new(),
            holders: HashMap::new(),
            accounts: HashMap::new(),
        }
    }

    /// Construct a runner backed by a Redis/KV store.
    ///
    /// Panics if `CORP_REDIS_URL` is not set in the environment.
    #[allow(dead_code)]
    fn new_kv() -> Self {
        let redis_url =
            std::env::var("CORP_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_owned());

        // KV tests share a temp dir for the JWT config path even though
        // entity data lives in Redis.
        let dir = TempDir::new().expect("create tempdir");

        let jwt_config = Arc::new(JwtConfig::new(TEST_JWT_SECRET));
        let workspace_id = WorkspaceId::new();

        let now = unix_now();
        let claims = Claims {
            sub: "scenario-test-user".to_owned(),
            workspace_id,
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes: vec![Scope::All],
            iat: now,
            exp: now + 7200,
        };
        let token = jwt_config.encode(&claims).expect("encode test JWT");

        let state = AppState {
            data_dir: dir.path().to_str().unwrap().to_owned(),
            jwt_config,
            api_key_resolver: Arc::new(NoopApiKeyResolver),
            storage_backend: StorageBackend::Kv {
                redis_url,
                s3_bucket: None,
            },
        };

        let app = router(state);

        Self {
            _dir: dir,
            app,
            token,
            workspace_id,
            entities: HashMap::new(),
            cap_tables: HashMap::new(),
            gov_bodies: HashMap::new(),
            contacts: HashMap::new(),
            grants: Vec::new(),
            safes: Vec::new(),
            rounds: Vec::new(),
            bank_accounts: Vec::new(),
            valuations: Vec::new(),
            payroll_runs: Vec::new(),
            instruments: HashMap::new(),
            holders: HashMap::new(),
            accounts: HashMap::new(),
        }
    }

    // ── Raw HTTP helpers ──────────────────────────────────────────────────────

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

        let req = builder.body(Body::from(bytes)).expect("build request");

        self.app.clone().oneshot(req).await.expect("service error")
    }

    async fn get(&mut self, path: &str) -> (StatusCode, Value) {
        let resp = self.request(Method::GET, path, None).await;
        let status = resp.status();
        let body = body_json(resp).await;
        (status, body)
    }

    async fn post(&mut self, path: &str, body: Value) -> (StatusCode, Value) {
        let resp = self.request(Method::POST, path, Some(body)).await;
        let status = resp.status();
        let body = body_json(resp).await;
        (status, body)
    }

    async fn post_empty(&mut self, path: &str) -> (StatusCode, Value) {
        let resp = self.request(Method::POST, path, None).await;
        let status = resp.status();
        let body = body_json(resp).await;
        (status, body)
    }

    // ── Domain helpers ────────────────────────────────────────────────────────

    /// Create a legal entity and register it under `logical_name`.
    async fn create_entity(
        &mut self,
        logical_name: &str,
        legal_name: &str,
        entity_type: &str,
        jurisdiction: &str,
    ) -> Value {
        let (status, body) = self
            .post(
                "/v1/entities",
                json!({
                    "legal_name": legal_name,
                    "entity_type": entity_type,
                    "jurisdiction": jurisdiction
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_entity({legal_name}) failed: {body}"
        );
        let entity_id = body["entity_id"]
            .as_str()
            .expect("entity_id missing")
            .to_owned();
        self.entities.insert(logical_name.to_owned(), entity_id);
        body
    }

    /// Advance entity formation through one step.
    #[allow(dead_code)]
    async fn advance_formation(&mut self, entity_name: &str) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "advance_formation({entity_name}) failed: {body}"
        );
        body
    }

    /// Confirm the state-filing confirmation number.
    async fn confirm_filing(&mut self, entity_name: &str, confirmation: &str) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/formations/{entity_id}/filing/confirm"),
                json!({ "confirmation_number": confirmation }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "confirm_filing({entity_name}) failed: {body}"
        );
        body
    }

    /// Record the EIN for an entity.
    async fn confirm_ein(&mut self, entity_name: &str, ein: &str) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/formations/{entity_id}/tax/confirm-ein"),
                json!({ "ein": ein }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "confirm_ein({entity_name}) failed: {body}"
        );
        body
    }

    /// Run the full formation lifecycle for an entity: advance from Pending
    /// through to Active, signing documents, confirming filing, and confirming
    /// EIN at the correct states.
    async fn form_entity_to_active(
        &mut self,
        entity_name: &str,
        filing_confirmation: &str,
        ein: &str,
    ) {
        let entity_id = self.entities[entity_name].clone();

        // 1. Advance Pending → DocumentsGenerated.
        let (status, _) = self
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "advance {entity_name} to documents_generated"
        );

        // 2. Sign all generated documents.
        let (status, docs) = self
            .get(&format!("/v1/formations/{entity_id}/documents"))
            .await;
        assert_eq!(status, StatusCode::OK, "list docs for {entity_name}");
        for doc in docs.as_array().expect("docs array") {
            let doc_id = doc["document_id"].as_str().unwrap();
            let (status, _) = self
                .post(
                    &format!("/v1/documents/{doc_id}/sign"),
                    json!({
                        "signer_name": "Test Signer",
                        "signer_role": "Incorporator",
                        "signer_email": "signer@example.com",
                        "signature_text": "/s/ Test Signer",
                        "consent_text": "I consent",
                        "signature_svg": null
                    }),
                )
                .await;
            assert_eq!(
                status,
                StatusCode::OK,
                "sign doc {doc_id} for {entity_name}"
            );
        }

        // 3. Advance DocumentsGenerated → DocumentsSigned.
        let (status, _) = self
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "advance {entity_name} to documents_signed"
        );

        // 4. Advance DocumentsSigned → FilingSubmitted.
        let (status, _) = self
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "advance {entity_name} to filing_submitted"
        );

        // 5. Confirm filing (must be in FilingSubmitted).
        self.confirm_filing(entity_name, filing_confirmation).await;

        // 6. Advance FilingSubmitted → Filed.
        let (status, _) = self
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;
        assert_eq!(status, StatusCode::OK, "advance {entity_name} to filed");

        // 7. Advance Filed → EinApplied.
        let (status, _) = self
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "advance {entity_name} to ein_applied"
        );

        // 8. Confirm EIN (must be in EinApplied).
        self.confirm_ein(entity_name, ein).await;

        // 9. Advance EinApplied → Active.
        let (status, _) = self
            .post_empty(&format!("/v1/formations/{entity_id}/advance"))
            .await;
        assert_eq!(status, StatusCode::OK, "advance {entity_name} to active");
    }

    /// Create a contact and register it under `logical_name`.
    async fn add_contact(
        &mut self,
        entity_name: &str,
        logical_name: &str,
        display_name: &str,
        email: &str,
        contact_type: &str,
        category: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/contacts"),
                json!({
                    "contact_type": contact_type,
                    "name": display_name,
                    "category": category,
                    "email": email,
                    "phone": null,
                    "mailing_address": null,
                    "cap_table_access": null,
                    "notes": null
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "add_contact({display_name}) failed: {body}"
        );
        let contact_id = body["contact_id"]
            .as_str()
            .expect("contact_id missing")
            .to_owned();
        self.contacts.insert(logical_name.to_owned(), contact_id);
        body
    }

    /// Create a cap table for an entity.
    async fn create_cap_table(&mut self, entity_name: &str) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post(&format!("/v1/entities/{entity_id}/cap-table"), json!({}))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_cap_table({entity_name}) failed: {body}"
        );
        let cap_table_id = body["cap_table_id"]
            .as_str()
            .expect("cap_table_id missing")
            .to_owned();
        self.cap_tables.insert(entity_name.to_owned(), cap_table_id);
        body
    }

    /// Create an instrument and register it under `instrument_name`.
    async fn create_instrument(
        &mut self,
        entity_name: &str,
        instrument_name: &str,
        symbol: &str,
        kind: &str,
        par_value: &str,
        authorized_units: i64,
        liquidation_preference: Option<&str>,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let cap_table_id = self.cap_tables[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/instruments"),
                json!({
                    "cap_table_id": cap_table_id,
                    "symbol": symbol,
                    "kind": kind,
                    "authorized_units": authorized_units,
                    "par_value": par_value,
                    "issue_price_cents": null,
                    "liquidation_preference": liquidation_preference,
                    "terms": null
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_instrument({instrument_name}) failed: {body}"
        );
        let instrument_id = body["instrument_id"]
            .as_str()
            .expect("instrument_id missing")
            .to_owned();
        self.instruments
            .insert(instrument_name.to_owned(), instrument_id);
        body
    }

    /// Create an equity holder record and register it under `holder_name`.
    async fn create_holder(
        &mut self,
        entity_name: &str,
        holder_name: &str,
        contact_name: Option<&str>,
        holder_type: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let contact_id = contact_name.map(|n| self.contacts[n].clone());
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/holders"),
                json!({
                    "contact_id": contact_id,
                    "name": holder_name,
                    "holder_type": holder_type
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_holder({holder_name}) failed: {body}"
        );
        let holder_id = body["holder_id"]
            .as_str()
            .expect("holder_id missing")
            .to_owned();
        self.holders.insert(holder_name.to_owned(), holder_id);
        body
    }

    /// Issue an equity grant and track the grant ID.
    #[allow(clippy::too_many_arguments)]
    async fn issue_grant(
        &mut self,
        entity_name: &str,
        class_name: &str,
        recipient_contact_name: &str,
        recipient_display_name: &str,
        grant_type: &str,
        shares: i64,
        price_per_share: Option<i64>,
        vesting_start: Option<&str>,
        vesting_months: Option<u32>,
        cliff_months: Option<u32>,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let cap_table_id = self.cap_tables[entity_name].clone();
        let instrument_id = self.instruments[class_name].clone();
        let recipient_contact_id = self.contacts[recipient_contact_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/grants"),
                json!({
                    "cap_table_id": cap_table_id,
                    "instrument_id": instrument_id,
                    "recipient_contact_id": recipient_contact_id,
                    "recipient_name": recipient_display_name,
                    "grant_type": grant_type,
                    "shares": shares,
                    "price_per_share": price_per_share,
                    "vesting_start": vesting_start,
                    "vesting_months": vesting_months,
                    "cliff_months": cliff_months
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "issue_grant({recipient_display_name}) failed: {body}"
        );
        let grant_id = body["grant_id"]
            .as_str()
            .expect("grant_id missing")
            .to_owned();
        self.grants.push(grant_id);
        body
    }

    /// Issue a SAFE note and track the safe ID.
    async fn issue_safe(
        &mut self,
        entity_name: &str,
        investor_contact_name: &str,
        investor_display_name: &str,
        safe_type: &str,
        investment_amount_cents: i64,
        valuation_cap_cents: Option<i64>,
        discount_percent: Option<u32>,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let cap_table_id = self.cap_tables[entity_name].clone();
        let investor_contact_id = self.contacts[investor_contact_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/safes"),
                json!({
                    "cap_table_id": cap_table_id,
                    "investor_contact_id": investor_contact_id,
                    "investor_name": investor_display_name,
                    "safe_type": safe_type,
                    "investment_amount_cents": investment_amount_cents,
                    "valuation_cap_cents": valuation_cap_cents,
                    "discount_percent": discount_percent
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "issue_safe({investor_display_name}) failed: {body}"
        );
        let safe_id = body["safe_note_id"]
            .as_str()
            .expect("safe_note_id missing")
            .to_owned();
        self.safes.push(safe_id);
        body
    }

    /// Convert a SAFE note to equity.
    ///
    /// Creates a holder for the SAFE investor and converts into the specified
    /// instrument with the given number of shares.
    async fn convert_safe(
        &mut self,
        entity_name: &str,
        safe_index: usize,
        instrument_name: &str,
        conversion_shares: i64,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let safe_id = self.safes[safe_index].clone();
        let instrument_id = self.instruments[instrument_name].clone();

        // Read the SAFE to get investor info, then create a holder.
        let (_, safe) = self
            .get(&format!("/v1/entities/{entity_id}/safes/{safe_id}"))
            .await;
        let investor_name = safe["investor_name"]
            .as_str()
            .unwrap_or("SAFE Investor")
            .to_owned();
        let investor_contact_id = safe["investor_contact_id"].as_str().map(|s| s.to_owned());
        let (_, holder) = self
            .post(
                &format!("/v1/entities/{entity_id}/holders"),
                json!({
                    "contact_id": investor_contact_id,
                    "name": investor_name,
                    "holder_type": "individual"
                }),
            )
            .await;
        let holder_id = holder["holder_id"]
            .as_str()
            .expect("holder_id missing")
            .to_owned();

        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/safes/{safe_id}/convert"),
                json!({
                    "instrument_id": instrument_id,
                    "conversion_shares": conversion_shares,
                    "holder_id": holder_id
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "convert_safe(index={safe_index}) failed: {body}"
        );
        body
    }

    /// Create a valuation and track the valuation ID.
    async fn create_valuation(
        &mut self,
        entity_name: &str,
        valuation_type: &str,
        methodology: &str,
        valuation_amount_cents: i64,
        effective_date: &str,
        prepared_by: Option<&str>,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let cap_table_id = self.cap_tables[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/valuations"),
                json!({
                    "cap_table_id": cap_table_id,
                    "valuation_type": valuation_type,
                    "methodology": methodology,
                    "valuation_amount_cents": valuation_amount_cents,
                    "effective_date": effective_date,
                    "prepared_by": prepared_by
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_valuation({entity_name}) failed: {body}"
        );
        let valuation_id = body["valuation_id"]
            .as_str()
            .expect("valuation_id missing")
            .to_owned();
        self.valuations.push(valuation_id);
        body
    }

    /// Submit a valuation for board approval.
    async fn submit_valuation(&mut self, entity_name: &str, valuation_index: usize) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let valuation_id = self.valuations[valuation_index].clone();
        let (status, body) = self
            .post_empty(&format!(
                "/v1/entities/{entity_id}/valuations/{valuation_id}/submit"
            ))
            .await;
        assert_eq!(status, StatusCode::OK, "submit_valuation failed: {body}");
        body
    }

    /// Board-approve a valuation.
    async fn approve_valuation(
        &mut self,
        entity_name: &str,
        valuation_index: usize,
        approved_by: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let valuation_id = self.valuations[valuation_index].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/valuations/{valuation_id}/approve"),
                json!({ "approved_by": approved_by }),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "approve_valuation failed: {body}");
        body
    }

    /// Create a governance body and register it under `entity_name`.
    async fn create_governance_body(
        &mut self,
        entity_name: &str,
        body_name: &str,
        body_type: &str,
        quorum_rule: &str,
        voting_method: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, resp_body) = self
            .post(
                &format!("/v1/entities/{entity_id}/governance/bodies"),
                json!({
                    "name": body_name,
                    "body_type": body_type,
                    "quorum_rule": quorum_rule,
                    "voting_method": voting_method
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_governance_body({body_name}) failed: {resp_body}"
        );
        let body_id = resp_body["body_id"]
            .as_str()
            .expect("body_id missing")
            .to_owned();
        self.gov_bodies.insert(entity_name.to_owned(), body_id);
        resp_body
    }

    /// Add a governance seat.
    async fn add_governance_seat(
        &mut self,
        entity_name: &str,
        contact_name: &str,
        role: &str,
        appointed_date: &str,
        voting_power: u32,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let body_id = self.gov_bodies[entity_name].clone();
        let holder_id = self.contacts[contact_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/governance/seats"),
                json!({
                    "body_id": body_id,
                    "holder_id": holder_id,
                    "role": role,
                    "appointed_date": appointed_date,
                    "term_expiration": null,
                    "voting_power": voting_power
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "add_governance_seat({contact_name}) failed: {body}"
        );
        body
    }

    /// Create a meeting.
    async fn create_meeting(
        &mut self,
        entity_name: &str,
        title: &str,
        meeting_type: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let body_id = self.gov_bodies[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/governance/meetings"),
                json!({
                    "body_id": body_id,
                    "meeting_type": meeting_type,
                    "title": title,
                    "scheduled_date": null,
                    "location": null,
                    "notice_days": null
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_meeting({title}) failed: {body}"
        );
        body
    }

    /// Add an agenda item to a meeting.
    async fn add_agenda_item(
        &mut self,
        entity_name: &str,
        meeting_id: &str,
        title: &str,
        item_type: &str,
        resolution_text: Option<&str>,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items"),
                json!({
                    "title": title,
                    "item_type": item_type,
                    "description": null,
                    "resolution_text": resolution_text
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "add_agenda_item({title}) failed: {body}"
        );
        body
    }

    /// Resolve an agenda item.
    async fn resolve_agenda_item(
        &mut self,
        entity_name: &str,
        meeting_id: &str,
        item_id: &str,
        resolution_type: &str,
        resolution_text: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post(
                &format!(
                    "/v1/entities/{entity_id}/governance/meetings/{meeting_id}/items/{item_id}/resolve"
                ),
                json!({
                    "resolution_type": resolution_type,
                    "resolution_text": resolution_text
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "resolve_agenda_item({item_id}) failed: {body}"
        );
        body
    }

    /// Use the quick-approve shortcut to create an immediately-resolved board
    /// consent without manually creating meetings and agenda items.
    async fn quick_approve(
        &mut self,
        entity_name: &str,
        title: &str,
        resolution_text: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let body_id = self.gov_bodies[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/governance/quick-approve"),
                json!({
                    "body_id": body_id,
                    "title": title,
                    "resolution_text": resolution_text
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "quick_approve({title}) failed: {body}"
        );
        body
    }

    /// Create a funding round.
    async fn create_round(
        &mut self,
        entity_name: &str,
        round_name: &str,
        target_amount_cents: i64,
        price_per_share_cents: Option<i64>,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let cap_table_id = self.cap_tables[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/rounds"),
                json!({
                    "cap_table_id": cap_table_id,
                    "name": round_name,
                    "target_amount_cents": target_amount_cents,
                    "price_per_share_cents": price_per_share_cents
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_round({round_name}) failed: {body}"
        );
        let round_id = body["round_id"]
            .as_str()
            .expect("round_id missing")
            .to_owned();
        self.rounds.push(round_id);
        body
    }

    /// Advance a funding round one step through its pre-close pipeline.
    ///
    /// `TermSheet` → `Diligence` → `Closing`
    async fn advance_round(&mut self, entity_name: &str, round_index: usize) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let round_id = self.rounds[round_index].clone();
        let (status, body) = self
            .post_empty(&format!(
                "/v1/entities/{entity_id}/rounds/{round_id}/advance"
            ))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "advance_round(index={round_index}) failed: {body}"
        );
        body
    }

    /// Close a funding round.
    ///
    /// The round must already be in `Closing` status; callers are responsible
    /// for advancing through `TermSheet` → `Diligence` → `Closing` first via
    /// [`advance_round`].
    async fn close_round(&mut self, entity_name: &str, round_index: usize) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let round_id = self.rounds[round_index].clone();
        let (status, body) = self
            .post_empty(&format!("/v1/entities/{entity_id}/rounds/{round_id}/close"))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "close_round(index={round_index}) failed: {body}"
        );
        body
    }

    /// Create a GL account.
    async fn create_account(
        &mut self,
        entity_name: &str,
        account_name_key: &str,
        account_code: &str,
        account_name: &str,
        currency: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post(
                &format!("/v1/entities/{entity_id}/accounts"),
                json!({
                    "account_code": account_code,
                    "account_name": account_name,
                    "currency": currency
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_account({account_name}) failed: {body}"
        );
        let account_id = body["account_id"]
            .as_str()
            .expect("account_id missing")
            .to_owned();
        self.accounts
            .insert(account_name_key.to_owned(), account_id);
        body
    }

    /// Create and post a balanced journal entry.
    async fn post_journal_entry(
        &mut self,
        entity_name: &str,
        date: &str,
        description: &str,
        lines: Value,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, entry) = self
            .post(
                &format!("/v1/entities/{entity_id}/journal-entries"),
                json!({
                    "date": date,
                    "description": description,
                    "lines": lines
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_journal_entry({description}) failed: {entry}"
        );

        let entry_id = entry["entry_id"]
            .as_str()
            .expect("entry_id missing")
            .to_owned();

        let (post_status, posted) = self
            .post_empty(&format!(
                "/v1/entities/{entity_id}/journal-entries/{entry_id}/post"
            ))
            .await;
        assert_eq!(
            post_status,
            StatusCode::OK,
            "post_journal_entry({description}) failed: {posted}"
        );
        posted
    }

    /// Create an invoice and send it.
    async fn create_and_send_invoice(
        &mut self,
        entity_name: &str,
        customer_name: &str,
        amount_cents: i64,
        description: &str,
        due_date: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, invoice) = self
            .post(
                &format!("/v1/entities/{entity_id}/invoices"),
                json!({
                    "customer_name": customer_name,
                    "customer_email": null,
                    "amount_cents": amount_cents,
                    "currency": "usd",
                    "description": description,
                    "due_date": due_date
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_invoice({customer_name}) failed: {invoice}"
        );

        let invoice_id = invoice["invoice_id"]
            .as_str()
            .expect("invoice_id missing")
            .to_owned();

        let (send_status, sent) = self
            .post_empty(&format!(
                "/v1/entities/{entity_id}/invoices/{invoice_id}/send"
            ))
            .await;
        assert_eq!(
            send_status,
            StatusCode::OK,
            "send_invoice({customer_name}) failed: {sent}"
        );
        sent
    }

    /// Pay an invoice.
    async fn pay_invoice(&mut self, entity_name: &str, invoice_id: &str) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .post_empty(&format!(
                "/v1/entities/{entity_id}/invoices/{invoice_id}/pay"
            ))
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "pay_invoice({invoice_id}) failed: {body}"
        );
        body
    }

    /// Create a bank account and activate it.
    async fn create_and_activate_bank_account(
        &mut self,
        entity_name: &str,
        institution: &str,
        account_type: &str,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, acct) = self
            .post(
                &format!("/v1/entities/{entity_id}/bank-accounts"),
                json!({
                    "institution": institution,
                    "account_type": account_type,
                    "account_number_last4": "1234",
                    "routing_number_last4": "5678"
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_bank_account({institution}) failed: {acct}"
        );

        let bank_account_id = acct["bank_account_id"]
            .as_str()
            .expect("bank_account_id missing")
            .to_owned();
        self.bank_accounts.push(bank_account_id.clone());

        let (activate_status, activated) = self
            .post_empty(&format!(
                "/v1/entities/{entity_id}/bank-accounts/{bank_account_id}/activate"
            ))
            .await;
        assert_eq!(
            activate_status,
            StatusCode::OK,
            "activate_bank_account({institution}) failed: {activated}"
        );
        activated
    }

    /// Run payroll (create, approve, process).
    async fn run_payroll(
        &mut self,
        entity_name: &str,
        period_start: &str,
        period_end: &str,
        total_gross_cents: i64,
        total_net_cents: i64,
        employee_count: u32,
    ) -> Value {
        let entity_id = self.entities[entity_name].clone();
        let (status, run) = self
            .post(
                &format!("/v1/entities/{entity_id}/payroll-runs"),
                json!({
                    "period_start": period_start,
                    "period_end": period_end,
                    "total_gross_cents": total_gross_cents,
                    "total_net_cents": total_net_cents,
                    "employee_count": employee_count
                }),
            )
            .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "create_payroll_run({entity_name}) failed: {run}"
        );

        let run_id = run["payroll_run_id"]
            .as_str()
            .expect("payroll_run_id missing")
            .to_owned();
        self.payroll_runs.push(run_id.clone());

        // Approve
        let (approve_status, approved) = self
            .post_empty(&format!(
                "/v1/entities/{entity_id}/payroll-runs/{run_id}/approve"
            ))
            .await;
        assert_eq!(
            approve_status,
            StatusCode::OK,
            "approve_payroll_run failed: {approved}"
        );
        assert_eq!(approved["status"], "approved");

        // Process
        let (process_status, processed) = self
            .post_empty(&format!(
                "/v1/entities/{entity_id}/payroll-runs/{run_id}/process"
            ))
            .await;
        assert_eq!(
            process_status,
            StatusCode::OK,
            "process_payroll_run failed: {processed}"
        );
        assert_eq!(processed["status"], "processed");

        processed
    }

    // ── State verification helpers ────────────────────────────────────────────

    async fn assert_grant_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self.get(&format!("/v1/entities/{entity_id}/grants")).await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} grants for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_safe_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self.get(&format!("/v1/entities/{entity_id}/safes")).await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} SAFEs for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_instrument_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .get(&format!("/v1/entities/{entity_id}/instruments"))
            .await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} instruments for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_holder_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self.get(&format!("/v1/entities/{entity_id}/holders")).await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} holders for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_contact_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .get(&format!("/v1/entities/{entity_id}/contacts"))
            .await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} contacts for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_round_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self.get(&format!("/v1/entities/{entity_id}/rounds")).await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} rounds for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_bank_account_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .get(&format!("/v1/entities/{entity_id}/bank-accounts"))
            .await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} bank accounts for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_valuation_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .get(&format!("/v1/entities/{entity_id}/valuations"))
            .await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} valuations for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_payroll_run_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .get(&format!("/v1/entities/{entity_id}/payroll-runs"))
            .await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} payroll runs for {entity_name}, got {count}: {body}"
        );
    }

    async fn assert_governance_body_count(&mut self, entity_name: &str, expected: usize) {
        let entity_id = self.entities[entity_name].clone();
        let (status, body) = self
            .get(&format!("/v1/entities/{entity_id}/governance/bodies"))
            .await;
        assert_eq!(status, StatusCode::OK);
        let count = body.as_array().map(|a| a.len()).unwrap_or(0);
        assert_eq!(
            count, expected,
            "expected {expected} governance bodies for {entity_name}, got {count}: {body}"
        );
    }
}

// ── SaaS Startup scenario ─────────────────────────────────────────────────────

/// Drive the full SaaS startup (Series A journey) scenario.
///
/// Simulates TechStart Inc. from Delaware incorporation through Series A close,
/// option grants, and payroll — the complete Carta-equivalent workflow for a
/// venture-backed C-Corp.
async fn run_saas_startup_scenario(runner: &mut ScenarioRunner) {
    // ── Step 1: Incorporate Delaware C-Corp ──────────────────────────────────

    runner
        .create_entity("corp", "TechStart Inc.", "c_corp", "DE")
        .await;

    // Run the full formation lifecycle: sign docs, confirm filing, confirm EIN, advance to Active.
    runner
        .form_entity_to_active("corp", "DE-2026-TECHSTART-001", "88-1234567")
        .await;

    // ── Step 2: Add founders as contacts ─────────────────────────────────────

    runner
        .add_contact(
            "corp",
            "founder_alice",
            "Alice Chen",
            "alice@techstart.io",
            "individual",
            "founder",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "founder_bob",
            "Bob Patel",
            "bob@techstart.io",
            "individual",
            "founder",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "founder_carol",
            "Carol Wu",
            "carol@techstart.io",
            "individual",
            "founder",
        )
        .await;

    // Additional professional contacts.
    runner
        .add_contact(
            "corp",
            "law_firm",
            "Cooley LLP",
            "corp@cooley.com",
            "organization",
            "law_firm",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "val_firm",
            "Andersen Valuation Group",
            "409a@andersen.com",
            "organization",
            "valuation_firm",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "angel_investor",
            "Meridian Ventures LLC",
            "deals@meridian.vc",
            "organization",
            "investor",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "angel_investor_2",
            "Sandra Lee",
            "sandra@angellist.com",
            "individual",
            "investor",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "lead_investor",
            "Summit Capital Partners",
            "deals@summitcap.com",
            "organization",
            "investor",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "ind_director",
            "Dr. Marcus Webb",
            "mwebb@boardadvisors.com",
            "individual",
            "board_member",
        )
        .await;

    // Employee contacts.
    runner
        .add_contact(
            "corp",
            "employee_1",
            "Jordan Kim",
            "jordan@techstart.io",
            "individual",
            "employee",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "employee_2",
            "Priya Sharma",
            "priya@techstart.io",
            "individual",
            "employee",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "employee_3",
            "Tyler Brooks",
            "tyler@techstart.io",
            "individual",
            "employee",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "employee_4",
            "Nadia Hassan",
            "nadia@techstart.io",
            "individual",
            "employee",
        )
        .await;
    runner
        .add_contact(
            "corp",
            "employee_5",
            "Felix Okafor",
            "felix@techstart.io",
            "individual",
            "employee",
        )
        .await;

    // 14 contacts total (3 founders + law firm + val firm + 2 angels + lead + ind_director + 5 employees).
    runner.assert_contact_count("corp", 14).await;

    // ── Step 3: Create cap table and Common equity instrument ─────────────────

    runner.create_cap_table("corp").await;

    runner
        .create_instrument(
            "corp", "common", "CS", "common_equity", "0.00001", 11_000_000, // 11M authorized (10M founder + 1M option pool)
            None,
        )
        .await;

    runner.assert_instrument_count("corp", 1).await;

    // ── Step 4: Create holders and issue founder shares ───────────────────────

    // Founders need holder records (they are equity holders, not just contacts).
    runner
        .create_holder("corp", "Alice Chen", Some("founder_alice"), "individual")
        .await;
    runner
        .create_holder("corp", "Bob Patel", Some("founder_bob"), "individual")
        .await;
    runner
        .create_holder("corp", "Carol Wu", Some("founder_carol"), "individual")
        .await;

    // Issue founder shares: 4M, 3M, 3M with 4-year vesting, 1-year cliff.
    runner
        .issue_grant(
            "corp",
            "common",
            "founder_alice",
            "Alice Chen",
            "rsa",
            4_000_000,
            Some(1), // $0.00001/share purchase price (par)
            Some("2026-01-01"),
            Some(48),
            Some(12),
        )
        .await;

    runner
        .issue_grant(
            "corp",
            "common",
            "founder_bob",
            "Bob Patel",
            "rsa",
            3_000_000,
            Some(1),
            Some("2026-01-01"),
            Some(48),
            Some(12),
        )
        .await;

    runner
        .issue_grant(
            "corp",
            "common",
            "founder_carol",
            "Carol Wu",
            "rsa",
            3_000_000,
            Some(1),
            Some("2026-01-01"),
            Some(48),
            Some(12),
        )
        .await;

    runner.assert_grant_count("corp", 3).await;
    runner.assert_holder_count("corp", 3).await;

    // ── Step 5: Board of Directors ────────────────────────────────────────────

    runner
        .create_governance_body(
            "corp",
            "Board of Directors",
            "board_of_directors",
            "majority",
            "per_capita",
        )
        .await;

    runner
        .add_governance_seat("corp", "founder_alice", "chair", "2026-01-01", 1)
        .await;
    runner
        .add_governance_seat("corp", "founder_bob", "member", "2026-01-01", 1)
        .await;
    runner
        .add_governance_seat("corp", "founder_carol", "member", "2026-01-01", 1)
        .await;

    runner.assert_governance_body_count("corp", 1).await;

    // ── Step 6: Convene initial board meeting ─────────────────────────────────

    let formation_meeting = runner
        .create_meeting("corp", "Organizational Board Meeting", "board_meeting")
        .await;
    let formation_meeting_id = formation_meeting["meeting_id"]
        .as_str()
        .expect("meeting_id")
        .to_owned();

    // Add agenda items for formation approvals.
    let item_bylaws = runner
        .add_agenda_item(
            "corp",
            &formation_meeting_id,
            "Adopt Bylaws",
            "resolution",
            Some("RESOLVED: The Board hereby adopts the Bylaws of TechStart Inc."),
        )
        .await;
    let item_bylaws_id = item_bylaws["item_id"].as_str().expect("item_id").to_owned();

    let item_officers = runner
        .add_agenda_item(
            "corp",
            &formation_meeting_id,
            "Elect Officers",
            "resolution",
            Some("RESOLVED: Alice Chen is elected CEO; Bob Patel is elected CTO; Carol Wu is elected CFO."),
        )
        .await;
    let item_officers_id = item_officers["item_id"]
        .as_str()
        .expect("item_id")
        .to_owned();

    let item_shares = runner
        .add_agenda_item(
            "corp",
            &formation_meeting_id,
            "Authorize Share Issuance",
            "resolution",
            Some("RESOLVED: The Board authorizes issuance of 10,000,000 shares of Common Stock."),
        )
        .await;
    let item_shares_id = item_shares["item_id"].as_str().expect("item_id").to_owned();

    // Convene the meeting.
    let entity_id = runner.entities["corp"].clone();
    let (convene_status, _) = runner
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{formation_meeting_id}/convene"
        ))
        .await;
    assert_eq!(convene_status, StatusCode::OK);

    // Resolve all agenda items.
    runner
        .resolve_agenda_item(
            "corp",
            &formation_meeting_id,
            &item_bylaws_id,
            "unanimous_written_consent",
            "RESOLVED: The Board hereby adopts the Bylaws of TechStart Inc.",
        )
        .await;

    runner
        .resolve_agenda_item(
            "corp",
            &formation_meeting_id,
            &item_officers_id,
            "unanimous_written_consent",
            "RESOLVED: Alice Chen is elected CEO; Bob Patel is elected CTO; Carol Wu is elected CFO.",
        )
        .await;

    runner
        .resolve_agenda_item(
            "corp",
            &formation_meeting_id,
            &item_shares_id,
            "unanimous_written_consent",
            "RESOLVED: The Board authorizes issuance of 10,000,000 shares of Common Stock.",
        )
        .await;

    // Adjourn.
    let (adjourn_status, adjourned) = runner
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{formation_meeting_id}/adjourn"
        ))
        .await;
    assert_eq!(adjourn_status, StatusCode::OK);
    assert_eq!(
        adjourned["status"], "adjourned",
        "meeting should be adjourned: {adjourned}"
    );

    // ── Step 7: Open bank account ─────────────────────────────────────────────

    runner
        .create_and_activate_bank_account("corp", "Mercury", "checking")
        .await;

    runner.assert_bank_account_count("corp", 1).await;

    // ── Step 8: Create GL accounts ────────────────────────────────────────────

    runner
        .create_account("corp", "cash_acct", "cash", "Cash", "usd")
        .await;
    runner
        .create_account(
            "corp",
            "safe_liability",
            "accounts_payable",
            "SAFE Notes Payable",
            "usd",
        )
        .await;
    runner
        .create_account(
            "corp",
            "equity_acct",
            "founder_capital",
            "Founder Capital",
            "usd",
        )
        .await;
    runner
        .create_account("corp", "revenue_acct", "revenue", "SaaS Revenue", "usd")
        .await;
    runner
        .create_account(
            "corp",
            "opex_acct",
            "operating_expenses",
            "Operating Expenses",
            "usd",
        )
        .await;

    // ── Step 9: Issue first SAFE ($500K, $10M post-money cap) ─────────────────

    runner
        .issue_safe(
            "corp",
            "angel_investor",
            "Meridian Ventures LLC",
            "post_money",
            50_000_000,          // $500,000 in cents
            Some(1_000_000_000), // $10M cap in cents
            None,
        )
        .await;

    // Journal entry: debit cash, credit SAFE liability.
    {
        let cash_id = runner.accounts["cash_acct"].clone();
        let safe_liability_id = runner.accounts["safe_liability"].clone();
        runner
            .post_journal_entry(
                "corp",
                "2026-02-15",
                "SAFE proceeds - Meridian Ventures $500K",
                json!([
                    {
                        "account_id": cash_id,
                        "amount_cents": 50_000_000_i64,
                        "side": "debit",
                        "memo": "SAFE proceeds from Meridian Ventures"
                    },
                    {
                        "account_id": safe_liability_id,
                        "amount_cents": 50_000_000_i64,
                        "side": "credit",
                        "memo": "SAFE note liability"
                    }
                ]),
            )
            .await;
    }

    // ── Step 10: Issue second SAFE ($250K, $12M cap, MFN) ────────────────────

    runner
        .issue_safe(
            "corp",
            "angel_investor_2",
            "Sandra Lee",
            "mfn",
            25_000_000,          // $250,000
            Some(1_200_000_000), // $12M cap
            None,
        )
        .await;

    {
        let cash_id = runner.accounts["cash_acct"].clone();
        let safe_liability_id = runner.accounts["safe_liability"].clone();
        runner
            .post_journal_entry(
                "corp",
                "2026-03-01",
                "SAFE proceeds - Sandra Lee $250K",
                json!([
                    {
                        "account_id": cash_id,
                        "amount_cents": 25_000_000_i64,
                        "side": "debit",
                        "memo": "SAFE proceeds from Sandra Lee"
                    },
                    {
                        "account_id": safe_liability_id,
                        "amount_cents": 25_000_000_i64,
                        "side": "credit",
                        "memo": "SAFE note liability"
                    }
                ]),
            )
            .await;
    }

    runner.assert_safe_count("corp", 2).await;

    // ── Step 11: 409A valuation ($2/share) ────────────────────────────────────

    runner
        .create_valuation(
            "corp",
            "four_oh_nine_a",
            "backsolve",
            2_000_000_00, // $2M enterprise value in cents; $0.20/share effective
            "2026-03-15",
            Some("Andersen Valuation Group"),
        )
        .await;

    runner.submit_valuation("corp", 0).await;
    runner.approve_valuation("corp", 0, "Alice Chen, CEO").await;

    runner.assert_valuation_count("corp", 1).await;

    // Board quick-approve the 409A.
    runner
        .quick_approve(
            "corp",
            "Approve 409A Valuation",
            "RESOLVED: The Board approves the 409A FMV of $0.20/share as determined by Andersen Valuation Group, effective March 15, 2026.",
        )
        .await;

    // ── Step 12: Series A round setup ($5M, $20M pre-money) ──────────────────

    runner
        .create_round(
            "corp",
            "Series A",
            500_000_000_i64, // $5M target
            Some(200),       // $2.00/share price
        )
        .await;

    runner.assert_round_count("corp", 1).await;

    // ── Step 13: Add Series A Preferred instrument ────────────────────────────

    runner
        .create_instrument(
            "corp",
            "series_a_preferred",
            "Series A Preferred",
            "preferred_equity",
            "0.00001",
            5_000_000, // 5M authorized preferred
            Some("1x non-participating liquidation preference"),
        )
        .await;

    runner.assert_instrument_count("corp", 2).await;

    // ── Step 14: Convert SAFEs to equity ─────────────────────────────────────

    runner
        .convert_safe("corp", 0, "series_a_preferred", 500_000)
        .await;
    runner
        .convert_safe("corp", 1, "series_a_preferred", 250_000)
        .await;

    // Verify both SAFEs are converted.
    {
        let entity_id = runner.entities["corp"].clone();
        let (status, safes) = runner.get(&format!("/v1/entities/{entity_id}/safes")).await;
        assert_eq!(status, StatusCode::OK);
        let converted_count = safes
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|s| s["status"] == "converted")
            .count();
        assert_eq!(
            converted_count, 2,
            "both SAFEs should be converted: {safes}"
        );
    }

    // ── Step 15: Issue Series A shares ────────────────────────────────────────

    // Create a holder for the lead investor.
    runner
        .create_holder(
            "corp",
            "Summit Capital Partners",
            Some("lead_investor"),
            "entity",
        )
        .await;

    runner
        .issue_grant(
            "corp",
            "series_a_preferred",
            "lead_investor",
            "Summit Capital Partners",
            "preferred_stock",
            2_500_000, // 2.5M shares at $2 = $5M
            Some(200),
            None,
            None,
            None,
        )
        .await;

    // Journal: wire in Series A proceeds.
    {
        let cash_id = runner.accounts["cash_acct"].clone();
        let equity_id = runner.accounts["equity_acct"].clone();
        runner
            .post_journal_entry(
                "corp",
                "2026-06-15",
                "Series A close - Summit Capital $5M",
                json!([
                    {
                        "account_id": cash_id,
                        "amount_cents": 500_000_000_i64,
                        "side": "debit",
                        "memo": "Series A wire from Summit Capital Partners"
                    },
                    {
                        "account_id": equity_id,
                        "amount_cents": 500_000_000_i64,
                        "side": "credit",
                        "memo": "Series A equity issuance"
                    }
                ]),
            )
            .await;
    }

    // ── Step 16: Close Series A round ─────────────────────────────────────────

    // Advance through TermSheet → Diligence → Closing, then close.
    runner.advance_round("corp", 0).await; // TermSheet → Diligence
    runner.advance_round("corp", 0).await; // Diligence → Closing
    runner.close_round("corp", 0).await; // Closing → Closed

    {
        let entity_id = runner.entities["corp"].clone();
        let (status, rounds) = runner
            .get(&format!("/v1/entities/{entity_id}/rounds"))
            .await;
        assert_eq!(status, StatusCode::OK);
        let closed = rounds
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .any(|r| r["status"] == "closed");
        assert!(closed, "Series A round should be closed: {rounds}");
    }

    // ── Step 17: Add independent director post-Series A ────────────────────────

    runner
        .add_governance_seat("corp", "ind_director", "member", "2026-06-20", 1)
        .await;

    // ── Step 18: Create stock option pool (1M shares) ─────────────────────────

    // We re-use the existing common instrument but track the option grants.
    // Board approval via quick-approve.
    runner
        .quick_approve(
            "corp",
            "Adopt Equity Incentive Plan",
            "RESOLVED: The Board adopts the 2026 Equity Incentive Plan reserving 1,000,000 shares of Common Stock for option grants to employees, directors, and consultants.",
        )
        .await;

    // ── Step 19: Issue options to 5 employees ─────────────────────────────────

    let employee_option_shares = [200_000_i64, 150_000, 150_000, 100_000, 100_000];
    let employee_contacts = [
        "employee_1",
        "employee_2",
        "employee_3",
        "employee_4",
        "employee_5",
    ];
    let employee_names = [
        "Jordan Kim",
        "Priya Sharma",
        "Tyler Brooks",
        "Nadia Hassan",
        "Felix Okafor",
    ];

    for (idx, (&shares, (&contact_name, &display_name))) in employee_option_shares
        .iter()
        .zip(employee_contacts.iter().zip(employee_names.iter()))
        .enumerate()
    {
        runner
            .issue_grant(
                "corp",
                "common",
                contact_name,
                display_name,
                "iso",
                shares,
                Some(20), // $0.20/share (409A FMV)
                Some("2026-07-01"),
                Some(48),
                Some(12),
            )
            .await;
        let _ = idx; // suppress unused warning
    }

    // 3 founder grants + 2 SAFE conversions + 1 Series A grant + 5 employee option grants = 11 total.
    runner.assert_grant_count("corp", 11).await;

    // ── Step 20: Second 409A valuation ($5/share post-Series A) ──────────────

    runner
        .create_valuation(
            "corp",
            "four_oh_nine_a",
            "backsolve",
            50_000_000_00, // $50M enterprise value
            "2026-09-01",
            Some("Andersen Valuation Group"),
        )
        .await;

    runner.submit_valuation("corp", 1).await;
    runner.approve_valuation("corp", 1, "Alice Chen, CEO").await;

    runner.assert_valuation_count("corp", 2).await;

    // Board approval of new 409A.
    runner
        .quick_approve(
            "corp",
            "Approve Updated 409A Valuation",
            "RESOLVED: The Board approves the updated 409A FMV of $5.00/share following the Series A close, effective September 1, 2026.",
        )
        .await;

    // ── Step 21: Convene post-Series A board meeting ──────────────────────────

    let series_a_meeting = runner
        .create_meeting("corp", "Post-Series A Board Meeting", "board_meeting")
        .await;
    let series_a_meeting_id = series_a_meeting["meeting_id"]
        .as_str()
        .expect("meeting_id")
        .to_owned();

    let option_item = runner
        .add_agenda_item(
            "corp",
            &series_a_meeting_id,
            "Ratify Option Grants",
            "resolution",
            Some("RESOLVED: The Board ratifies all option grants issued under the 2026 Equity Incentive Plan."),
        )
        .await;
    let option_item_id = option_item["item_id"].as_str().expect("item_id").to_owned();

    let budget_item = runner
        .add_agenda_item(
            "corp",
            &series_a_meeting_id,
            "Approve Annual Operating Budget",
            "resolution",
            Some("RESOLVED: The Board approves the 2026-2027 operating budget of $4.2M."),
        )
        .await;
    let budget_item_id = budget_item["item_id"].as_str().expect("item_id").to_owned();

    let (convene_status, _) = runner
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{series_a_meeting_id}/convene"
        ))
        .await;
    assert_eq!(convene_status, StatusCode::OK);

    runner
        .resolve_agenda_item(
            "corp",
            &series_a_meeting_id,
            &option_item_id,
            "ordinary",
            "RESOLVED: The Board ratifies all option grants issued under the 2026 Equity Incentive Plan.",
        )
        .await;

    runner
        .resolve_agenda_item(
            "corp",
            &series_a_meeting_id,
            &budget_item_id,
            "ordinary",
            "RESOLVED: The Board approves the 2026-2027 operating budget of $4.2M.",
        )
        .await;

    let (adjourn_status_2, _) = runner
        .post_empty(&format!(
            "/v1/entities/{entity_id}/governance/meetings/{series_a_meeting_id}/adjourn"
        ))
        .await;
    assert_eq!(adjourn_status_2, StatusCode::OK);

    // ── Step 22: Create invoices and payments ─────────────────────────────────

    // Law firm invoice.
    runner
        .create_and_send_invoice(
            "corp",
            "Cooley LLP",
            500_000, // $5,000
            "Series A legal fees",
            "2026-07-31",
        )
        .await;

    // Revenue invoice from first customer.
    let revenue_invoice = runner
        .create_and_send_invoice(
            "corp",
            "Acme Corp",
            50_000, // $500/month
            "SaaS subscription - July 2026",
            "2026-08-01",
        )
        .await;

    let revenue_invoice_id = revenue_invoice["invoice_id"]
        .as_str()
        .expect("invoice_id")
        .to_owned();

    // Customer pays the invoice.
    runner.pay_invoice("corp", &revenue_invoice_id).await;

    // Revenue journal entry.
    {
        let cash_id = runner.accounts["cash_acct"].clone();
        let revenue_id = runner.accounts["revenue_acct"].clone();
        runner
            .post_journal_entry(
                "corp",
                "2026-07-15",
                "SaaS revenue - Acme Corp July",
                json!([
                    {
                        "account_id": cash_id,
                        "amount_cents": 50_000_i64,
                        "side": "debit",
                        "memo": "Customer payment received"
                    },
                    {
                        "account_id": revenue_id,
                        "amount_cents": 50_000_i64,
                        "side": "credit",
                        "memo": "SaaS subscription revenue"
                    }
                ]),
            )
            .await;
    }

    // Operating expense entry.
    {
        let cash_id = runner.accounts["cash_acct"].clone();
        let opex_id = runner.accounts["opex_acct"].clone();
        runner
            .post_journal_entry(
                "corp",
                "2026-07-31",
                "Payroll and operating expenses July 2026",
                json!([
                    {
                        "account_id": opex_id,
                        "amount_cents": 45_000_000_i64,
                        "side": "debit",
                        "memo": "Monthly operating costs including payroll"
                    },
                    {
                        "account_id": cash_id,
                        "amount_cents": 45_000_000_i64,
                        "side": "credit",
                        "memo": "Cash disbursement"
                    }
                ]),
            )
            .await;
    }

    // ── Step 23: Run payroll ───────────────────────────────────────────────────

    runner
        .run_payroll(
            "corp",
            "2026-07-01",
            "2026-07-31",
            40_000_000, // $400K gross
            32_000_000, // $320K net (after withholding)
            5,
        )
        .await;

    runner.assert_payroll_run_count("corp", 1).await;

    // ── Step 24: Final cap table state verification ───────────────────────────

    // Grants: 3 founder + 2 SAFE conversions + 1 Series A + 5 employee = 11.
    runner.assert_grant_count("corp", 11).await;

    // Instruments: Common + Series A Preferred = 2.
    runner.assert_instrument_count("corp", 2).await;

    // Holders: 3 founders + 2 SAFE investor holders + Summit Capital = 6.
    runner.assert_holder_count("corp", 6).await;

    // SAFEs: 2 issued, both converted.
    runner.assert_safe_count("corp", 2).await;

    // Valuations: 2 (initial 409A + post-Series A 409A).
    runner.assert_valuation_count("corp", 2).await;

    // Rounds: 1 (Series A, closed).
    runner.assert_round_count("corp", 1).await;

    // Bank accounts: 1 Mercury checking.
    runner.assert_bank_account_count("corp", 1).await;

    // Governance bodies: 1 Board of Directors.
    runner.assert_governance_body_count("corp", 1).await;

    // Payroll runs: 1.
    runner.assert_payroll_run_count("corp", 1).await;

    // Entity is retrievable and has formation_status set.
    {
        let entity_id = runner.entities["corp"].clone();
        let (status, entity) = runner.get(&format!("/v1/entities/{entity_id}")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            entity["entity_id"].is_string(),
            "entity should have entity_id: {entity}"
        );
        assert_eq!(entity["legal_name"], "TechStart Inc.");
    }
}

// ── VC Fund scenario ──────────────────────────────────────────────────────────

/// Drive the complete VC fund lifecycle scenario.
///
/// Simulates Beacon Capital Management LLC (GP) raising Beacon Seed Fund I, LP
/// from 8 LPs, executing SAFE and equity investments, collecting management
/// fees, and distributing waterfall returns — the complete Carta-equivalent
/// workflow for a venture fund.
async fn run_vc_fund_scenario(runner: &mut ScenarioRunner) {
    // ── Step 1: Form GP LLC (Wyoming) ─────────────────────────────────────────

    runner
        .create_entity("gp_llc", "Beacon Capital Management LLC", "llc", "WY")
        .await;

    // Run the full formation lifecycle for GP LLC.
    runner
        .form_entity_to_active("gp_llc", "WY-2026-BEACON-001", "88-7654321")
        .await;

    // ── Step 2: Form Fund LP entity ───────────────────────────────────────────

    runner
        .create_entity(
            "fund_lp",
            "Beacon Seed Fund I LP",
            "llc", // Platform uses LLC as the closest analog for fund LP vehicles.
            "DE",
        )
        .await;

    // Run the full formation lifecycle for Fund LP.
    runner
        .form_entity_to_active("fund_lp", "DE-2026-BEACON-FUND-001", "88-9876543")
        .await;

    // ── Step 3: Add contacts ──────────────────────────────────────────────────

    // GP principals.
    runner
        .add_contact(
            "gp_llc",
            "principal_a",
            "Victoria Strand",
            "vstrand@beaconcap.com",
            "individual",
            "officer",
        )
        .await;
    runner
        .add_contact(
            "gp_llc",
            "principal_b",
            "Derek Nguyen",
            "dnguyen@beaconcap.com",
            "individual",
            "officer",
        )
        .await;

    // Fund counsel and administrator.
    runner
        .add_contact(
            "fund_lp",
            "fund_counsel",
            "Fenwick & West LLP",
            "funds@fenwick.com",
            "organization",
            "law_firm",
        )
        .await;
    runner
        .add_contact(
            "fund_lp",
            "fund_admin",
            "Andersen Fund Admin LLC",
            "admin@andersenadmin.com",
            "organization",
            "accounting_firm",
        )
        .await;

    // 8 LP contacts (various sizes: $5M, $4M, $3M × 3, $2M × 3).
    let lp_data: &[(&str, &str, &str, &str)] = &[
        (
            "lp_1",
            "Cornerstone Endowment Fund",
            "cef@cornerstone.org",
            "organization",
        ),
        (
            "lp_2",
            "Pacific Rim Ventures",
            "invest@pacificrim.vc",
            "organization",
        ),
        (
            "lp_3",
            "Helena Family Office",
            "invest@helenafamily.com",
            "organization",
        ),
        (
            "lp_4",
            "NextGen Pension Fund",
            "invest@nextgenpension.com",
            "organization",
        ),
        (
            "lp_5",
            "Redwood Foundation",
            "invest@redwoodfdn.org",
            "organization",
        ),
        (
            "lp_6",
            "Astra Capital Group",
            "invest@astracapital.com",
            "organization",
        ),
        (
            "lp_7",
            "Dr. James Osei",
            "josei@medicine.stanford.edu",
            "individual",
        ),
        (
            "lp_8",
            "Maria Santos LLC",
            "msantos@santosfamily.com",
            "organization",
        ),
    ];

    for &(key, name, email, contact_type) in lp_data {
        runner
            .add_contact("fund_lp", key, name, email, contact_type, "investor")
            .await;
    }

    // 10 contacts total on fund_lp (fund_counsel + fund_admin + 8 LPs).
    runner.assert_contact_count("fund_lp", 10).await;

    // ── Step 4: Create cap table for fund ─────────────────────────────────────

    runner.create_cap_table("fund_lp").await;

    // LP interest / membership unit instrument.
    runner
        .create_instrument(
            "fund_lp",
            "lp_interests",
            "LP",
            "membership_unit",
            "1.00",
            25_500, // 25,500 units: 25,000 LP + 500 GP (each unit = $1,000)
            None,
        )
        .await;

    runner.assert_instrument_count("fund_lp", 1).await;

    // ── Step 5: Create holders for all 8 LPs and issue LP interests ──────────

    let lp_commitments: &[(&str, &str, i64, i64)] = &[
        // (lp_key, holder_name, units, price_per_unit_cents)
        ("lp_1", "Cornerstone Endowment Fund", 5_000, 100_000),
        ("lp_2", "Pacific Rim Ventures", 4_000, 100_000),
        ("lp_3", "Helena Family Office", 3_000, 100_000),
        ("lp_4", "NextGen Pension Fund", 3_000, 100_000),
        ("lp_5", "Redwood Foundation", 3_000, 100_000),
        ("lp_6", "Astra Capital Group", 2_000, 100_000),
        ("lp_7", "Dr. James Osei", 2_000, 100_000),
        ("lp_8", "Maria Santos LLC", 3_000, 100_000),
    ];

    for &(lp_key, holder_name, units, price) in lp_commitments {
        runner
            .create_holder("fund_lp", holder_name, Some(lp_key), "entity")
            .await;
        runner
            .issue_grant(
                "fund_lp",
                "lp_interests",
                lp_key,
                holder_name,
                "membership_unit",
                units,
                Some(price),
                None,
                None,
                None,
            )
            .await;
    }

    // Also issue GP commitment (2% = $500K = 500 units).
    runner
        .create_holder("fund_lp", "Beacon Capital Management LLC", None, "entity")
        .await;
    runner
        .add_contact(
            "fund_lp",
            "gp_as_investor",
            "Beacon Capital Management LLC (GP)",
            "gp@beaconcap.com",
            "organization",
            "investor",
        )
        .await;
    runner
        .issue_grant(
            "fund_lp",
            "lp_interests",
            "gp_as_investor",
            "Beacon Capital Management LLC (GP)",
            "membership_unit",
            500,
            Some(100_000),
            None,
            None,
            None,
        )
        .await;

    // 9 grants: 8 LPs + 1 GP commit.
    runner.assert_grant_count("fund_lp", 9).await;
    // 9 holders: 8 LPs + GP.
    runner.assert_holder_count("fund_lp", 9).await;

    // ── Step 6: Create GP advisory committee ──────────────────────────────────

    runner
        .create_governance_body(
            "fund_lp",
            "LP Advisory Committee",
            "llc_member_vote",
            "majority",
            "per_unit",
        )
        .await;

    // Add 3 LP advisory committee members.
    runner
        .add_governance_seat("fund_lp", "lp_1", "chair", "2026-01-20", 5000)
        .await;
    runner
        .add_governance_seat("fund_lp", "lp_2", "member", "2026-01-20", 4000)
        .await;
    runner
        .add_governance_seat("fund_lp", "lp_3", "member", "2026-01-20", 3000)
        .await;

    runner.assert_governance_body_count("fund_lp", 1).await;

    // ── Step 7: Create bank accounts ──────────────────────────────────────────

    // Fund operating account.
    runner
        .create_and_activate_bank_account("fund_lp", "Silicon Valley Bank", "checking")
        .await;
    // Fund investment account (separate from management fee account).
    runner
        .create_and_activate_bank_account("fund_lp", "JP Morgan", "checking")
        .await;
    // GP management fee account.
    runner
        .create_and_activate_bank_account("gp_llc", "Mercury", "checking")
        .await;

    runner.assert_bank_account_count("fund_lp", 2).await;
    runner.assert_bank_account_count("gp_llc", 1).await;

    // ── Step 8: Set up GL accounts ────────────────────────────────────────────

    runner
        .create_account("fund_lp", "fund_cash", "cash", "Fund Cash", "usd")
        .await;
    runner
        .create_account(
            "fund_lp",
            "investments_acct",
            "accounts_receivable",
            "Portfolio Investments",
            "usd",
        )
        .await;
    runner
        .create_account(
            "fund_lp",
            "mgmt_fee_expense",
            "operating_expenses",
            "Management Fee Expense",
            "usd",
        )
        .await;
    runner
        .create_account(
            "fund_lp",
            "lp_capital",
            "founder_capital",
            "Limited Partner Capital",
            "usd",
        )
        .await;

    runner
        .create_account("gp_llc", "gp_cash", "cash", "GP Cash", "usd")
        .await;
    runner
        .create_account(
            "gp_llc",
            "mgmt_fee_revenue",
            "revenue",
            "Management Fee Revenue",
            "usd",
        )
        .await;

    // ── Step 9: First capital call (25% of commitments) ───────────────────────

    // Journal: LP capital calls received ($6.25M = 25% of $25M).
    {
        let fund_cash_id = runner.accounts["fund_cash"].clone();
        let lp_capital_id = runner.accounts["lp_capital"].clone();
        runner
            .post_journal_entry(
                "fund_lp",
                "2026-03-15",
                "Capital call #1 - 25% of committed capital ($6.25M)",
                json!([
                    {
                        "account_id": fund_cash_id,
                        "amount_cents": 625_000_000_i64,
                        "side": "debit",
                        "memo": "First capital call proceeds from 8 LPs + GP"
                    },
                    {
                        "account_id": lp_capital_id,
                        "amount_cents": 625_000_000_i64,
                        "side": "credit",
                        "memo": "LP capital contributions Q1 2026"
                    }
                ]),
            )
            .await;
    }

    // ── Step 10: Management fee invoices ($125K quarterly × 3 quarters) ───────

    for (quarter, due_date) in [
        ("Q1 2026", "2026-04-15"),
        ("Q2 2026", "2026-07-15"),
        ("Q3 2026", "2026-10-15"),
    ] {
        let invoice = runner
            .create_and_send_invoice(
                "fund_lp",
                "Beacon Capital Management LLC",
                12_500_000, // $125,000
                &format!("Management fee {quarter} — 2% of $25M committed capital"),
                due_date,
            )
            .await;

        let invoice_id = invoice["invoice_id"]
            .as_str()
            .expect("invoice_id")
            .to_owned();
        runner.pay_invoice("fund_lp", &invoice_id).await;

        // Management fee journal entry (fund side).
        {
            let mgmt_fee_exp_id = runner.accounts["mgmt_fee_expense"].clone();
            let fund_cash_id = runner.accounts["fund_cash"].clone();
            runner
                .post_journal_entry(
                    "fund_lp",
                    due_date,
                    &format!("Management fee payment to GP {quarter}"),
                    json!([
                        {
                            "account_id": mgmt_fee_exp_id,
                            "amount_cents": 12_500_000_i64,
                            "side": "debit",
                            "memo": "Quarterly management fee expense"
                        },
                        {
                            "account_id": fund_cash_id,
                            "amount_cents": 12_500_000_i64,
                            "side": "credit",
                            "memo": "Fee payment to GP LLC"
                        }
                    ]),
                )
                .await;
        }

        // Management fee journal entry (GP side).
        {
            let gp_cash_id = runner.accounts["gp_cash"].clone();
            let mgmt_fee_rev_id = runner.accounts["mgmt_fee_revenue"].clone();
            runner
                .post_journal_entry(
                    "gp_llc",
                    due_date,
                    &format!("Management fee received from Fund {quarter}"),
                    json!([
                        {
                            "account_id": gp_cash_id,
                            "amount_cents": 12_500_000_i64,
                            "side": "debit",
                            "memo": "Management fee income"
                        },
                        {
                            "account_id": mgmt_fee_rev_id,
                            "amount_cents": 12_500_000_i64,
                            "side": "credit",
                            "memo": "Management fee revenue Q"
                        }
                    ]),
                )
                .await;
        }
    }

    // ── Step 11: SAFE investment in portfolio company #1 ─────────────────────

    // Add portfolio company as a contact.
    runner
        .add_contact(
            "fund_lp",
            "portfolio_co_1",
            "WidgetAI Inc.",
            "cfo@widgetai.com",
            "organization",
            "investor",
        )
        .await;

    // Record SAFE investment.
    runner
        .issue_safe(
            "fund_lp",
            "portfolio_co_1",
            "WidgetAI Inc.",
            "post_money",
            50_000_000,        // $500K
            Some(800_000_000), // $8M cap
            None,
        )
        .await;

    // Investment journal entry.
    {
        let investments_id = runner.accounts["investments_acct"].clone();
        let fund_cash_id = runner.accounts["fund_cash"].clone();
        runner
            .post_journal_entry(
                "fund_lp",
                "2026-04-01",
                "SAFE investment in WidgetAI Inc. $500K",
                json!([
                    {
                        "account_id": investments_id,
                        "amount_cents": 50_000_000_i64,
                        "side": "debit",
                        "memo": "SAFE investment - WidgetAI Inc."
                    },
                    {
                        "account_id": fund_cash_id,
                        "amount_cents": 50_000_000_i64,
                        "side": "credit",
                        "memo": "Wire to WidgetAI Inc."
                    }
                ]),
            )
            .await;
    }

    // ── Step 12: Equity investment in second portfolio company ────────────────

    runner
        .add_contact(
            "fund_lp",
            "portfolio_co_2",
            "DataFlow Systems Inc.",
            "cfo@dataflowsys.com",
            "organization",
            "investor",
        )
        .await;

    runner
        .issue_safe(
            "fund_lp",
            "portfolio_co_2",
            "DataFlow Systems Inc.",
            "post_money",
            40_000_000,        // $400K
            Some(600_000_000), // $6M cap
            None,
        )
        .await;

    {
        let investments_id = runner.accounts["investments_acct"].clone();
        let fund_cash_id = runner.accounts["fund_cash"].clone();
        runner
            .post_journal_entry(
                "fund_lp",
                "2026-05-01",
                "SAFE investment in DataFlow Systems $400K",
                json!([
                    {
                        "account_id": investments_id,
                        "amount_cents": 40_000_000_i64,
                        "side": "debit",
                        "memo": "SAFE investment - DataFlow Systems Inc."
                    },
                    {
                        "account_id": fund_cash_id,
                        "amount_cents": 40_000_000_i64,
                        "side": "credit",
                        "memo": "Wire to DataFlow Systems Inc."
                    }
                ]),
            )
            .await;
    }

    // ── Step 13: Second capital call (15% of commitments) ─────────────────────

    {
        let fund_cash_id = runner.accounts["fund_cash"].clone();
        let lp_capital_id = runner.accounts["lp_capital"].clone();
        runner
            .post_journal_entry(
                "fund_lp",
                "2026-06-01",
                "Capital call #2 - 15% of committed capital ($3.75M)",
                json!([
                    {
                        "account_id": fund_cash_id,
                        "amount_cents": 375_000_000_i64,
                        "side": "debit",
                        "memo": "Second capital call proceeds"
                    },
                    {
                        "account_id": lp_capital_id,
                        "amount_cents": 375_000_000_i64,
                        "side": "credit",
                        "memo": "LP capital contributions CC2"
                    }
                ]),
            )
            .await;
    }

    // ── Step 14: Third investment (equity round participation) ────────────────

    runner
        .add_contact(
            "fund_lp",
            "portfolio_co_3",
            "NovaBio Therapeutics Inc.",
            "cfo@novabio.com",
            "organization",
            "investor",
        )
        .await;

    // Priced equity round - record as a grant on the fund's own cap table.
    runner
        .create_instrument(
            "fund_lp",
            "portfolio_equity",
            "PE",
            "preferred_equity",
            "1.00",
            1_000, // 1,000 units representing fund's ownership stakes
            None,
        )
        .await;

    runner
        .create_holder(
            "fund_lp",
            "NovaBio Therapeutics Inc.",
            Some("portfolio_co_3"),
            "entity",
        )
        .await;

    runner
        .issue_grant(
            "fund_lp",
            "portfolio_equity",
            "portfolio_co_3",
            "NovaBio Therapeutics Inc.",
            "preferred_stock",
            300, // Fund's stake in NovaBio (300 units representing $300K at cost)
            Some(100_000),
            None,
            None,
            None,
        )
        .await;

    {
        let investments_id = runner.accounts["investments_acct"].clone();
        let fund_cash_id = runner.accounts["fund_cash"].clone();
        runner
            .post_journal_entry(
                "fund_lp",
                "2026-06-15",
                "Equity investment in NovaBio Therapeutics $300K",
                json!([
                    {
                        "account_id": investments_id,
                        "amount_cents": 30_000_000_i64,
                        "side": "debit",
                        "memo": "Priced equity round participation - NovaBio"
                    },
                    {
                        "account_id": fund_cash_id,
                        "amount_cents": 30_000_000_i64,
                        "side": "credit",
                        "memo": "Wire to NovaBio Therapeutics"
                    }
                ]),
            )
            .await;
    }

    // ── Step 15: Portfolio valuation mark-to-market ────────────────────────────

    runner
        .create_valuation(
            "fund_lp",
            "fair_market_value",
            "market",
            280_000_000_00, // $280M NAV (up from $25M committed on $1.2M deployed)
            "2026-09-30",
            Some("Andersen Fund Admin LLC"),
        )
        .await;

    runner.submit_valuation("fund_lp", 0).await;
    runner
        .approve_valuation("fund_lp", 0, "Victoria Strand, Managing Partner")
        .await;

    runner.assert_valuation_count("fund_lp", 1).await;

    // ── Step 16: Distribution waterfall simulation ────────────────────────────

    // Simulate exit proceeds from WidgetAI ($1.2M on $500K investment).
    // Waterfall: return of capital ($400K) → preferred return → carry.

    // Convert the WidgetAI SAFE (exit event) into portfolio equity units.
    runner
        .convert_safe("fund_lp", 0, "portfolio_equity", 100)
        .await;

    // Distribution journal entries.
    // 1. Return of capital to LPs: $500K.
    {
        let fund_cash_id = runner.accounts["fund_cash"].clone();
        let lp_capital_id = runner.accounts["lp_capital"].clone();
        runner
            .post_journal_entry(
                "fund_lp",
                "2026-12-01",
                "WidgetAI exit proceeds received - $1.2M",
                json!([
                    {
                        "account_id": fund_cash_id,
                        "amount_cents": 120_000_000_i64,
                        "side": "debit",
                        "memo": "WidgetAI acquisition proceeds"
                    },
                    {
                        "account_id": lp_capital_id,
                        "amount_cents": 120_000_000_i64,
                        "side": "credit",
                        "memo": "Realized gain on WidgetAI exit"
                    }
                ]),
            )
            .await;
    }

    // 2. LP distribution (return of capital $500K + profit share $640K = $1.04M).
    {
        let fund_cash_id = runner.accounts["fund_cash"].clone();
        let lp_capital_id = runner.accounts["lp_capital"].clone();
        runner
            .post_journal_entry(
                "fund_lp",
                "2026-12-15",
                "LP distribution - WidgetAI exit waterfall $1.04M",
                json!([
                    {
                        "account_id": lp_capital_id,
                        "amount_cents": 104_000_000_i64,
                        "side": "debit",
                        "memo": "LP distribution: return of capital + profit share"
                    },
                    {
                        "account_id": fund_cash_id,
                        "amount_cents": 104_000_000_i64,
                        "side": "credit",
                        "memo": "Wire to limited partners pro-rata"
                    }
                ]),
            )
            .await;
    }

    // 3. GP carry distribution (20% of $800K profit = $160K).
    {
        let fund_cash_id = runner.accounts["fund_cash"].clone();
        let mgmt_fee_exp_id = runner.accounts["mgmt_fee_expense"].clone();
        runner
            .post_journal_entry(
                "fund_lp",
                "2026-12-15",
                "GP carried interest distribution $160K",
                json!([
                    {
                        "account_id": mgmt_fee_exp_id,
                        "amount_cents": 16_000_000_i64,
                        "side": "debit",
                        "memo": "GP carry: 20% of $800K profit above preferred return"
                    },
                    {
                        "account_id": fund_cash_id,
                        "amount_cents": 16_000_000_i64,
                        "side": "credit",
                        "memo": "Carry distribution to Beacon Capital Management LLC"
                    }
                ]),
            )
            .await;
    }

    // GP books the carry as revenue.
    {
        let gp_cash_id = runner.accounts["gp_cash"].clone();
        let mgmt_fee_rev_id = runner.accounts["mgmt_fee_revenue"].clone();
        runner
            .post_journal_entry(
                "gp_llc",
                "2026-12-15",
                "Carried interest received - WidgetAI exit",
                json!([
                    {
                        "account_id": gp_cash_id,
                        "amount_cents": 16_000_000_i64,
                        "side": "debit",
                        "memo": "Carry distribution received"
                    },
                    {
                        "account_id": mgmt_fee_rev_id,
                        "amount_cents": 16_000_000_i64,
                        "side": "credit",
                        "memo": "Carried interest income FY2026"
                    }
                ]),
            )
            .await;
    }

    // ── Step 17: LP advisory committee meeting ────────────────────────────────

    // Convene LP advisory committee for fund performance review.
    let advisory_meeting = runner
        .create_meeting("fund_lp", "LP Advisory Committee Q4 2026", "member_meeting")
        .await;
    let advisory_meeting_id = advisory_meeting["meeting_id"]
        .as_str()
        .expect("meeting_id")
        .to_owned();

    let fund_entity_id = runner.entities["fund_lp"].clone();

    let performance_item = runner
        .add_agenda_item(
            "fund_lp",
            &advisory_meeting_id,
            "Fund Performance Review",
            "report",
            None,
        )
        .await;
    let performance_item_id = performance_item["item_id"]
        .as_str()
        .expect("item_id")
        .to_owned();

    let distribution_item = runner
        .add_agenda_item(
            "fund_lp",
            &advisory_meeting_id,
            "Approve WidgetAI Exit Distribution",
            "resolution",
            Some("RESOLVED: The Advisory Committee approves the waterfall distribution of $1.2M in WidgetAI exit proceeds to LPs and GP per the LPA."),
        )
        .await;
    let distribution_item_id = distribution_item["item_id"]
        .as_str()
        .expect("item_id")
        .to_owned();

    let (convene_status, _) = runner
        .post_empty(&format!(
            "/v1/entities/{fund_entity_id}/governance/meetings/{advisory_meeting_id}/convene"
        ))
        .await;
    assert_eq!(convene_status, StatusCode::OK);

    runner
        .resolve_agenda_item(
            "fund_lp",
            &advisory_meeting_id,
            &performance_item_id,
            "ordinary",
            "Fund performance review completed. 3 investments, $1.2M deployed, $1.2M realized from WidgetAI exit.",
        )
        .await;

    runner
        .resolve_agenda_item(
            "fund_lp",
            &advisory_meeting_id,
            &distribution_item_id,
            "ordinary",
            "RESOLVED: The Advisory Committee approves the waterfall distribution of $1.2M in WidgetAI exit proceeds to LPs and GP per the LPA.",
        )
        .await;

    let (adjourn_status, adjourned) = runner
        .post_empty(&format!(
            "/v1/entities/{fund_entity_id}/governance/meetings/{advisory_meeting_id}/adjourn"
        ))
        .await;
    assert_eq!(adjourn_status, StatusCode::OK);
    assert_eq!(adjourned["status"], "adjourned");

    // ── Step 18: Final fund state verification ────────────────────────────────

    // Fund LP: 9 grants (8 LP interests + GP commit) + 1 portfolio equity + 1 SAFE conversion.
    runner.assert_grant_count("fund_lp", 11).await;

    // Instruments: LP interests + portfolio equity.
    runner.assert_instrument_count("fund_lp", 2).await;

    // Holders: 8 LPs + GP entity + NovaBio + WidgetAI (from SAFE conversion) = 11.
    runner.assert_holder_count("fund_lp", 11).await;

    // SAFEs: 2 (WidgetAI converted + DataFlow active).
    runner.assert_safe_count("fund_lp", 2).await;

    // SAFEs: WidgetAI should be converted.
    {
        let entity_id = runner.entities["fund_lp"].clone();
        let (status, safes) = runner.get(&format!("/v1/entities/{entity_id}/safes")).await;
        assert_eq!(status, StatusCode::OK);
        let converted_count = safes
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter(|s| s["status"] == "converted")
            .count();
        assert_eq!(
            converted_count, 1,
            "WidgetAI SAFE should be converted: {safes}"
        );
    }

    // Valuations: 1 (NAV mark-to-market).
    runner.assert_valuation_count("fund_lp", 1).await;

    // Bank accounts: 2 (operating + investment).
    runner.assert_bank_account_count("fund_lp", 2).await;

    // GP LLC: 1 bank account.
    runner.assert_bank_account_count("gp_llc", 1).await;

    // Governance bodies: 1 LP advisory committee.
    runner.assert_governance_body_count("fund_lp", 1).await;

    // Both entities are retrievable.
    for entity_name in ["gp_llc", "fund_lp"] {
        let entity_id = runner.entities[entity_name].clone();
        let (status, entity) = runner.get(&format!("/v1/entities/{entity_id}")).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "entity {entity_name} should be retrievable"
        );
        assert!(
            entity["entity_id"].is_string(),
            "entity should have entity_id: {entity}"
        );
    }
}

// ── Test entry points ─────────────────────────────────────────────────────────

/// Full SaaS startup Series A journey on the git backend.
///
/// Exercises: formation, cap table, founder shares, governance bodies,
/// board meetings, SAFE notes, 409A valuations, Series A close, SAFE
/// conversion, option grants, payroll, treasury accounts, and journal entries.
#[tokio::test]
async fn saas_startup_git() {
    let mut runner = ScenarioRunner::new_git();
    run_saas_startup_scenario(&mut runner).await;
}

/// Full SaaS startup Series A journey on the KV (Redis) backend.
///
/// Identical to `saas_startup_git` but uses a Redis store. Requires a running
/// Redis instance at `CORP_REDIS_URL` (default: `redis://127.0.0.1/`).
#[tokio::test]
#[ignore] // Requires Redis
async fn saas_startup_kv() {
    let mut runner = ScenarioRunner::new_kv();
    run_saas_startup_scenario(&mut runner).await;
}

/// Full VC fund lifecycle on the git backend.
///
/// Exercises: GP LLC + Fund LP formation, LP interests, capital calls,
/// management fees, SAFE and equity investments, portfolio valuation,
/// waterfall distributions, and LP advisory committee governance.
#[tokio::test]
async fn vc_fund_git() {
    let mut runner = ScenarioRunner::new_git();
    run_vc_fund_scenario(&mut runner).await;
}

/// Full VC fund lifecycle on the KV (Redis) backend.
///
/// Identical to `vc_fund_git` but uses a Redis store. Requires a running
/// Redis instance at `CORP_REDIS_URL` (default: `redis://127.0.0.1/`).
#[tokio::test]
#[ignore] // Requires Redis
async fn vc_fund_kv() {
    let mut runner = ScenarioRunner::new_kv();
    run_vc_fund_scenario(&mut runner).await;
}
