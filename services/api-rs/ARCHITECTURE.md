# api-rs Architecture

This document is the authoritative guide for anyone maintaining or extending the API service. It covers where things live, why, and the exact patterns you must follow to add new functionality.

---

## 1. Directory Structure

```
services/api-rs/
├── src/
│   ├── main.rs             — startup, CLI subcommands, router assembly, init_state
│   ├── auth.rs             — Principal extractor, scoped extractors (macro-generated), RequireInternalWorker
│   ├── error.rs            — AppError enum + From<DomainError> for every domain
│   ├── config.rs           — (thin) config helpers
│   ├── validate.rs         — startup data validation subcommand
│   ├── openapi.rs          — utoipa spec aggregation
│   │
│   ├── routes/
│   │   ├── mod.rs          — AppState, CreationRateLimiter, shared query types
│   │   ├── shared.rs       — open_entity_store (the canonical store opener)
│   │   ├── formation/      — entity CRUD, documents, filings, tax
│   │   ├── equity/         — share classes, grants, safe notes, valuations, rounds
│   │   ├── governance/     — bodies, seats, meetings, votes, resolutions, audit
│   │   ├── treasury/       — accounts, journal entries, invoices, payments
│   │   ├── contacts/       — contact management
│   │   ├── execution/      — intents, obligations, receipts, transaction packets
│   │   ├── branches/       — branch create/list/delete/merge
│   │   ├── compliance/     — escalations, evidence links, deadlines
│   │   ├── auth/           — API key issuance, JWT exchange
│   │   ├── agents/         — agent definitions
│   │   ├── agent_executions/ — agent job queue and execution state
│   │   ├── secrets_proxy/  — encrypted secret storage
│   │   ├── secret_proxies/ — proxy configuration records
│   │   ├── llm_proxy/      — LLM request proxy with metering
│   │   ├── references/     — cross-entity reference links
│   │   ├── work_items/     — work item tracking
│   │   ├── services/       — service request fulfillment
│   │   ├── next_steps/     — guided onboarding steps
│   │   ├── admin/          — admin + billing operations
│   │   ├── git_http/       — Git HTTP smart protocol endpoints
│   │   ├── governance_enforcement/ — proof obligation enforcement
│   │   └── validation/     — request validation helpers
│   │
│   ├── store/
│   │   ├── mod.rs          — StorageBackendKind, RepoLayout (path conventions)
│   │   ├── entity_store.rs — EntityStore<'a>: unified Git/KV abstraction
│   │   ├── stored_entity.rs — StoredEntity trait + implementations for all domain types
│   │   └── workspace_store.rs — WorkspaceStore (API keys, workspace metadata)
│   │
│   ├── domain/
│   │   ├── ids.rs          — all typed ID newtypes (UUIDs)
│   │   ├── formation/      — Entity, Document, Filing, TaxProfile, Contract, Deadline…
│   │   ├── equity/         — ShareClass, EquityGrant, SafeNote, Valuation, CapTable…
│   │   ├── governance/     — GovernanceBody, GovernanceSeat, Meeting, Vote, Resolution…
│   │   ├── treasury/       — Account, JournalEntry, Invoice, Payment, BankAccount…
│   │   ├── execution/      — Intent, Obligation, Receipt, TransactionPacket…
│   │   ├── contacts/       — Contact, NotificationPrefs
│   │   ├── agents/         — Agent definitions and error types
│   │   ├── auth/           — Claims, ScopeSet, Scope enum, api_key, ssh_key, SshKeyIndex
│   │   ├── billing/        — billing records
│   │   ├── services/       — ServiceRequest (fulfillment marketplace)
│   │   └── work_items/     — WorkItem
│   │
│   └── git/
│       ├── pack.rs         — pack format parser and builder (zlib, SHA-1, object types)
│       ├── protocol.rs     — GitService enum, pkt-line helpers, path parsing
│       ├── native_transport.rs — info_refs / upload_pack / receive_pack over KV
│       ├── commit.rs       — FileWrite, commit_files (writes to git or KV)
│       ├── repo.rs         — CorpRepo wrapper around libgit2
│       ├── branch.rs       — branch create/list/delete
│       ├── merge.rs        — fast-forward, three-way, squash merge
│       └── signing.rs      — CommitSigner (Ed25519 SSH commit signatures)
│
├── tests/
│   ├── api_lifecycle.rs    — full HTTP lifecycle tests (in-process, no TCP)
│   ├── governance_e2e_correctness.rs
│   ├── governance_meeting_e2e.rs
│   ├── governance_law_props.rs
│   ├── valuation_e2e.rs
│   └── next_steps_e2e.rs
│
└── Cargo.toml
```

**Key layout rules:**
- Every route module owns one domain. Route files are thin — no domain logic.
- `domain/` contains all business logic. It has no knowledge of HTTP or axum.
- `store/` is the I/O boundary. Domain code never touches git2 or redis directly.
- `git/` contains the raw transport and git object code. It is not domain logic.

---

## 2. Request Lifecycle

A request flows through exactly these layers:

```
HTTP request
  → Axum router (match by method + path)
  → Auth extractor  (Principal or RequireXxx scoped extractor)
  → Handler function
      → parse path/query/body params
      → state.enforce_creation_rate_limit(...)  [creation endpoints only]
      → tokio::task::spawn_blocking({
            let layout = state.layout.clone();
            let valkey_client = state.valkey_client.clone();
            move || {
                let store = shared::open_entity_store(
                    &layout, workspace_id, entity_id,
                    auth.entity_ids(), valkey_client.as_ref()
                )?;
                // domain logic using store
                Ok(result)
            }
        }).await??
      → map result to JSON response
  → security_headers middleware (HSTS, X-Frame-Options, etc.)
  → HTTP response
```

**Rules that are always true:**
- Auth extraction happens before the handler body runs. If the extractor fails, the handler never runs.
- All store I/O (git2, redis) runs inside `spawn_blocking`. The store types are not `Send`, so they must stay on the blocking thread pool.
- The `??` pattern on `spawn_blocking` handles both the `JoinError` (task panic) and the inner `Result`.
- Handlers never return `anyhow::Error`. They return `Result<Json<T>, AppError>`.

---

## 3. Storage Architecture

### 3.1 Two Backends, One Interface

The service has two storage backends, selected at startup via `STORAGE_BACKEND`:

| Backend | Env value | Backing store | Use case |
|---|---|---|---|
| Git | `git` (default) | Bare git repos on disk via libgit2 | Local dev, single-node |
| KV | `kv` / `valkey` / `redis` | Redis-protocol server (DragonflyDB, Valkey, Redis) | Production, multi-instance |

`EntityStore<'a>` encapsulates both. Its internal `Backend` enum switches between `CorpRepo` (libgit2) and `CorpStore<redis::Connection>`. All callers use the same methods regardless of which backend is active.

### 3.2 RepoLayout and Path Conventions

`RepoLayout` owns the on-disk path logic. Paths are:

```
{DATA_DIR}/{workspace_id}/{entity_id}.git   — entity repo
{DATA_DIR}/{workspace_id}/_workspace.git   — workspace repo (API keys, settings)
```

In KV mode, these paths are logical keys in the redis namespace, not filesystem paths. `RepoLayout` is still passed in KV mode because it holds the `DATA_DIR` reference, but its filesystem methods are bypassed.

### 3.3 CorpStore and the Two-Phase Commit

When `S3_BUCKET` is set, every KV write follows a two-phase commit:

1. Write the object to S3 (durable, content-addressed).
2. Update the KV ref atomically.

This makes the KV store a rebuildable materialized index. If KV state is lost, it can be reconstructed from S3. The S3 backend is initialized at startup in `init_state` as `Option<Arc<S3Backend>>` on `AppState`.

### 3.4 StoredEntity Trait

Domain types that follow the `{dir}/{id}.json` convention implement `StoredEntity`:

```rust
pub trait StoredEntity: DeserializeOwned + Serialize {
    type Id: fmt::Display + FromStr + Copy;
    fn storage_dir() -> &'static str;
    fn storage_path(id: Self::Id) -> String { ... }  // default: "{dir}/{id}.json"
}
```

All implementations live in `store/stored_entity.rs`. This unlocks the generic methods on `EntityStore`:

```rust
store.read::<ShareClass>("main", class_id)?;
store.read_all::<EquityGrant>("main")?;
store.write::<Invoice>("main", id, &invoice, "Add invoice")?;
```

`Meeting` overrides `storage_path` because it uses `governance/meetings/{id}/meeting.json` instead of the flat `{dir}/{id}.json` default.

### 3.5 Shared KV Connection

In list-all handlers where many entities are opened sequentially, use `EntityStore::list_and_prepare` to get entity IDs and a shared `Rc<RefCell<CorpStore>>`, then pass it to `EntityStore::open_shared`. This avoids one TCP connection per entity.

---

## 4. Domain Model

Each domain maps to one subdirectory under `src/domain/` and one subdirectory under `src/routes/`. Domains own their own error types. The mapping to `AppError` is in `src/error.rs`.

### Formation (`domain/formation/`)

The root of every entity's data. Owns:
- `Entity` — top-level corporate record stored as `corp.json`. Tracks `FormationStatus` (Pending → DocumentsGenerated → DocumentsSigned → FilingSubmitted → Filed → EinApplied → Active → Dissolved) as a strict FSM. Advancing to Active sets `FormationState::Active` and stamps `formation_date`.
- `Document` — formation documents (articles, bylaws, etc.) stored as `formation/{doc_id}.json`.
- `Filing` — state filing record at `formation/filing.json`.
- `TaxProfile` — tax configuration at `tax/profile.json`.
- `Contract` — contracts at `contracts/{id}.json`.
- `TaxFiling` — individual tax filings at `tax/filings/{id}.json`.
- `Deadline` — compliance deadlines at `deadlines/{id}.json`.
- `ContractorClassification` — worker classification at `contractors/{id}.json`.
- `ComplianceEscalation` and `ComplianceEvidenceLink` — compliance records.

### Equity (`domain/equity/`)

Owns the cap table and all equity instruments:
- `ShareClass` — authorized share classes.
- `EquityGrant` — option grants, RSAs, warrants (stored in `cap-table/grants/`).
- `SafeNote` — SAFE instruments.
- `Valuation` — 409A and board-approved valuations.
- `ShareTransfer` — share transfer events.
- `FundingRound` — priced equity rounds.
- `Holder`, `Position`, `Instrument`, `ControlLink`, `LegalEntity` — cap table participants and instruments.
- `EquityRound`, `EquityRuleSet` — round administration and rule enforcement.
- `ConversionExecution`, `TransferWorkflow`, `FundraisingWorkflow` — multi-step workflows.
- `CapTable` — the aggregate cap table snapshot at `cap-table/cap-table.json`.

### Governance (`domain/governance/`)

Owns the governance structure and meeting lifecycle:
- `GovernanceBody` — board, committees.
- `GovernanceSeat` — individual seats within a body.
- `Meeting` — meetings with nested `AgendaItem`, `Vote`, and `Resolution` stored inside `governance/meetings/{meeting_id}/`.
- `GovernanceIncident`, `GovernanceTriggerEvent`, `GovernanceModeChangeEvent` — audit and mode tracking.
- `GovernanceAuditEntry`, `GovernanceAuditCheckpoint`, `GovernanceAuditVerificationReport` — audit trail.
- `doc_ast.rs`, `doc_generator.rs`, `typst_renderer.rs` — governance document AST, markdown generation, and Typst PDF rendering pipeline.
- `policy_ast.rs`, `policy_engine.rs`, `proof_obligations.rs` — governance policy rules and proof obligation checking.

### Execution (`domain/execution/`)

Owns the operational workflow for executing corporate actions:
- `Intent` — a declared corporate action (FSM-driven).
- `Obligation` — a binding obligation derived from an intent.
- `Receipt` — evidence of obligation completion.
- `ApprovalArtifact` — approval records attached to obligations.
- `DocumentRequest` — requests for document production.
- `TransactionPacket` — bundled transaction records for audit.

### Contacts (`domain/contacts/`)

- `Contact` — a person or organization with relationships to entities.
- `NotificationPrefs` — per-contact notification settings.

### Treasury (`domain/treasury/`)

Owns all financial records:
- `Account` — chart of accounts entries.
- `JournalEntry` — double-entry bookkeeping entries.
- `Invoice` — accounts receivable invoices.
- `BankAccount` — connected bank accounts.
- `Payment` — payment records.
- `PayrollRun` — payroll batch runs.
- `Distribution` — equity distributions.
- `Reconciliation` — bank reconciliation records.

### Agents and Work Items

- `agents/` — `Agent` definitions representing AI agents that can be dispatched via the Redis queue.
- `work_items/` — `WorkItem` for tracked tasks.
- `services/` — `ServiceRequest` for the fulfillment marketplace.

---

## 5. Auth Model

### 5.1 Credential Types

Three credential types are accepted on the `Authorization` header:

| Format | Path | How it works |
|---|---|---|
| `Bearer <JWT>` | JWT fast path | Decoded with `JWT_SECRET`. Claims carry `workspace_id`, `scopes`, optional `entity_ids`. |
| `Bearer sk_...` | API key (Bearer prefix) | Scanned against workspace stores. Argon2 hash comparison. |
| `sk_...` | API key (raw) | Same as above, no Bearer prefix required. |
| `Bearer <INTERNAL_WORKER_TOKEN>` | Internal worker | Static token comparison. Never touches the store. |

SSH keys are used only for the Git HTTP protocol. At startup, `SshKeyIndex::build` scans all workspace stores and builds an in-memory fingerprint→workspace map for O(1) lookups.

### 5.2 Principal and ScopeSet

After successful auth, a `Principal` is resolved:

```rust
pub struct Principal {
    workspace_id: WorkspaceId,
    entity_id: Option<EntityId>,       // if token is scoped to a single entity
    contact_id: Option<ContactId>,     // if token represents a contact
    entity_ids: Option<Vec<EntityId>>, // explicit entity allow-list (None = all)
    principal_type: PrincipalType,     // User or ServiceAccount
    scopes: ScopeSet,
}
```

`entity_ids: None` means the token can access all entities in the workspace. `Some([...])` means only those entities are accessible.

### 5.3 Scoped Extractors

Auth is enforced at compile time via scoped extractor types. The `define_scoped_extractor!` macro generates one type per scope:

```rust
// Adding this to a handler signature enforces auth + scope at compile time:
async fn my_handler(
    auth: RequireEquityWrite,     // must have EquityWrite scope
    State(state): State<AppState>,
    ...
) -> Result<Json<MyResponse>, AppError>
```

Available extractors: `RequireFormationCreate`, `RequireFormationRead`, `RequireFormationSign`, `RequireEquityRead`, `RequireEquityWrite`, `RequireEquityTransfer`, `RequireGovernanceRead`, `RequireGovernanceWrite`, `RequireGovernanceVote`, `RequireTreasuryRead`, `RequireTreasuryWrite`, `RequireTreasuryApprove`, `RequireContactsRead`, `RequireContactsWrite`, `RequireExecutionRead`, `RequireExecutionWrite`, `RequireServicesRead`, `RequireServicesWrite`, `RequireGitRead`, `RequireGitWrite`, `RequireBranchCreate`, `RequireBranchMerge`, `RequireBranchDelete`, `RequireAdmin`.

### 5.4 Entity-Scope Authorization

After extracting auth, use `shared::open_entity_store` (never a local variant) to open a store. This function enforces the entity-level allow-list:

```rust
let store = shared::open_entity_store(
    &state.layout,
    auth.workspace_id(),
    entity_id,
    auth.entity_ids(),       // None = allow all; Some([...]) = restrict
    state.valkey_client.as_ref(),
)?;
```

Passing `auth.entity_ids()` ensures that a token scoped to `[entity_A]` cannot read `entity_B` even if it has the right domain scope.

---

## 6. Error Handling Contract

### 6.1 AppError Variants

```
BadRequest(String)             → 400
Unauthorized(String)           → 401
Forbidden(String)              → 403
NotFound(String)               → 404
Conflict(String)               → 409
UnprocessableEntity(String)    → 422
RateLimited { limit, window }  → 429  (+ Retry-After header)
NotImplemented(String)         → 501
ServiceUnavailable(String)     → 503
Internal(String)               → 500  (message is logged, not exposed to client)
```

All responses use the body shape:
```json
{ "error": { "code": "snake_case_code", "detail": "human message" } }
```

`Internal` is the only variant that logs the real message and returns the generic `"internal server error"` string to the client.

### 6.2 Domain Error Conversions

Every domain error type has a `From<DomainError> for AppError` implementation in `src/error.rs`. These are exhaustive `match` expressions — every variant must be explicitly handled. The compiler will catch missing arms.

**When adding a new domain error variant:** add it to the domain's error enum, then add the corresponding arm to its `From` implementation in `error.rs`. Do not use `_ =>` catch-alls; handle every variant explicitly so new variants cause compile errors when they are not handled.

### 6.3 The `?` Operator

Domain errors propagate to `AppError` via `?` because `From` is implemented. In handlers:

```rust
// This works because From<FormationError> for AppError is defined:
let entity = store.read_entity("main")?;
```

`GitStorageError` also converts to `AppError` via `From`. Most not-found storage errors map to `AppError::NotFound`.

---

## 7. Patterns and Conventions

### 7.1 Always Use `shared::open_entity_store`

```rust
// CORRECT — entity-scope auth check is included
use crate::routes::shared;
let store = shared::open_entity_store(
    &state.layout,
    auth.workspace_id(),
    entity_id,
    auth.entity_ids(),
    state.valkey_client.as_ref(),
)?;

// WRONG — bypasses entity-scope authorization
let store = EntityStore::open(&state.layout, workspace_id, entity_id, None)?;
```

The `shared::open_entity_store` function is the single canonical opener. It was introduced to replace per-module `open_store` helpers, some of which omitted the entity-scope check. Never write a local version.

### 7.2 List-Read-Collect Pattern

Reading all records of a type from the `"main"` branch:

```rust
let grants: Vec<EquityGrant> = store.read_all::<EquityGrant>("main")?;
```

`read_all` calls `list_ids` then `read` for each ID. It skips entries that fail to parse only if you map over them manually; the default implementation propagates the first error.

### 7.3 Thin Handlers

Handlers parse inputs and map outputs. All decisions live in domain code.

```rust
async fn create_invoice(
    auth: RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, AppError> {
    // 1. Rate limit (creation endpoints only)
    state.enforce_creation_rate_limit("invoice", auth.workspace_id(), 100, 3600)?;

    // 2. Move what you need into the blocking closure
    let layout = state.layout.clone();
    let valkey = state.valkey_client.clone();
    let workspace_id = auth.workspace_id();
    let entity_ids = auth.entity_ids().map(|ids| ids.to_vec());

    let invoice = tokio::task::spawn_blocking(move || {
        // 3. Open store — always via shared::open_entity_store
        let store = shared::open_entity_store(
            &layout, workspace_id, entity_id,
            entity_ids.as_deref(), valkey.as_ref(),
        )?;

        // 4. Domain logic
        let invoice = Invoice::new(body.amount, body.due_date)?;
        store.write::<Invoice>("main", invoice.id(), &invoice, "Add invoice")?;
        Ok::<_, AppError>(invoice)
    }).await.map_err(|e| AppError::Internal(e.to_string()))??;

    // 5. Map to response
    Ok(Json(InvoiceResponse::from(invoice)))
}
```

### 7.4 `spawn_blocking` for All Store I/O

All `EntityStore` and `WorkspaceStore` operations are synchronous (libgit2 and sync redis). They must run in `spawn_blocking`. Do not call store methods from async context directly.

The double-`?` at the end (`??`) is idiomatic: the outer `?` unwraps `Result<_, JoinError>`, the inner `?` unwraps the handler's `Result<_, AppError>`.

### 7.5 Rate Limiting

Creation endpoints (POST that creates a new persistent record) should call:

```rust
state.enforce_creation_rate_limit("scope_name", auth.workspace_id(), limit, window_seconds)?;
```

`scope_name` should be a short string identifying the resource type (e.g., `"entity"`, `"invoice"`, `"grant"`). The limiter is in-process and keyed by `{workspace_id}:{scope}`. It uses a sliding window with a `VecDeque` of `Instant` values.

### 7.6 Branch Convention

All domain data lives on the `"main"` branch. Pass `"main"` as the branch argument to all store methods unless you are implementing branch-specific functionality (the `routes/branches/` module).

### 7.7 Adding a New StoredEntity

1. Create the domain type in `domain/your_domain/your_type.rs` with `#[derive(Serialize, Deserialize)]`.
2. Create a typed ID in `domain/ids.rs` (UUID newtype with `Display`, `FromStr`, `Copy`).
3. Add `impl StoredEntity for YourType` in `store/stored_entity.rs`, defining `storage_dir`.
4. The generic `store.read::<YourType>`, `store.read_all::<YourType>`, and `store.write::<YourType>` methods become available automatically.

### 7.8 Adding a New Route Module

1. Create `src/routes/your_domain.rs` (or `src/routes/your_domain/mod.rs`).
2. Add `pub mod your_domain;` to `src/routes/mod.rs`.
3. Implement `your_domain_routes() -> Router<AppState>` using `Router::new().route(...)`.
4. Add `.merge(routes::your_domain::your_domain_routes())` to `build_router` in `main.rs`.
5. Use `RequireXxx` scoped extractors on every handler. Never accept a raw `Principal` if a scoped extractor exists for the required operation.

---

## 8. Git Protocol

### 8.1 Overview

The Git HTTP smart protocol is served at `/git/{workspace_id}/{repo}.git/...`. It requires `STORAGE_BACKEND=valkey` — there is no subprocess and no bare repo on disk for this path. The native transport talks directly to the KV backend.

Three endpoints:

| Method | Path | Purpose |
|---|---|---|
| GET | `/git/{ws}/{repo}/info/refs?service=git-upload-pack` | Ref advertisement (fetch) |
| GET | `/git/{ws}/{repo}/info/refs?service=git-receive-pack` | Ref advertisement (push) |
| POST | `/git/{ws}/{repo}/git-upload-pack` | Send objects to client (fetch/clone) |
| POST | `/git/{ws}/{repo}/git-receive-pack` | Receive objects from client (push) |

### 8.2 Auth

Git endpoints use the same Bearer token / API key system with domain-scoped checks:

- `GitRead` scope required for `upload-pack` (fetch/clone).
- `GitWrite` scope required for `receive-pack` (push).
- Workspace ID must match the token's workspace.
- Entity ID must be in the token's entity allow-list (or the list must be absent).

SSH key auth is handled separately via `SshKeyIndex` for non-HTTP git transports.

### 8.3 Pack Format

`src/git/pack.rs` implements the git pack format (version 2):

- **Parse** (`parse_pack`): reads the `PACK` magic, version, object count, then decodes variable-length object headers and zlib-compressed content. Computes SHA-1 for each object.
- **Build** (`build_pack`): writes a PACK stream from a list of `(sha1, GitObjectType, content)` tuples.

**Supported object types:** commit (1), tree (2), blob (3). Tag (4) and delta types (5, 6, 7) return errors. The server does not advertise `ofs-delta`, so clients send full objects only.

**Size limits:**
- Single blob: 10 MB (`MAX_BLOB_SIZE`). Corp repos store structured JSON/text; larger objects indicate a mistake.
- Total pack upload: 2 GB (`MAX_PACK_SIZE`). Enforced by the `DefaultBodyLimit` layer on git routes and by `parse_pack` before processing.

### 8.4 Commit Signing

When `COMMIT_SIGNING_KEY` (an Ed25519 private key in PEM format) is set, all git commits are signed. The `CommitSigner` is stored on `AppState` and passed into the commit pipeline. Commits without signing are still accepted and stored, but unsigned commits do not carry cryptographic provenance.

---

## 9. Testing

### 9.1 Integration Tests (in-process)

`tests/api_lifecycle.rs` and the other `tests/*.rs` files are integration tests. They:

1. Build `AppState` directly with a `TempDir` as `DATA_DIR` and `StorageBackendKind::Git`.
2. Construct the Axum router by calling the same `*_routes()` functions used in production.
3. Issue requests via `tower::ServiceExt::oneshot` — no TCP listener, no network.
4. Assert on HTTP status codes and JSON response bodies.

This pattern gives full coverage of the HTTP layer, auth extraction, domain logic, and storage in a single test without a running server.

### 9.2 Test Helpers

The standard test helpers are:

```rust
async fn post_json(app: &Router, path: &str, body: Value, token: &str) -> (StatusCode, Value)
async fn get_json(app: &Router, path: &str, token: &str) -> (StatusCode, Value)
```

Auth tokens are minted with `make_token(workspace_id)` using `Scope::All` and a constant `TEST_SECRET`. For scope-restricted tests, construct `Claims` manually and call `encode_token`.

### 9.3 DragonflyDB + MinIO Integration Tests

End-to-end tests against the KV backend and S3 durable store require:
- A running DragonflyDB (or Valkey/Redis) instance accessible via `KV_URL`.
- A running MinIO instance with `S3_BUCKET`, `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_ENDPOINT_URL` set.

These are not run by default via `cargo test`. They are run in CI with the appropriate services up.

### 9.4 Property-Based Tests

`tests/governance_law_props.rs` uses `proptest` for property-based tests on governance state machine transitions. Run with `cargo test` as normal.

### 9.5 Startup Validation

`src/validate.rs` implements the `validate` subcommand and the startup validation pass. On server startup (unless `--skip-validation` is passed), every JSON file in the data directory is deserialized against its domain type. Validation errors are printed and the server refuses to start. This catches deserialized-but-structurally-invalid data before the server accepts traffic.

---

## 10. Environment Variables

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `DATA_DIR` | No | `./data/repos` | Root directory for git repos |
| `JWT_SECRET` | Yes (release) | insecure dev value | Shared secret for JWT signing |
| `INTERNAL_WORKER_TOKEN` | Yes | — | Static bearer token for internal workers |
| `STORAGE_BACKEND` | No | `git` | `git` or `kv`/`valkey`/`redis` |
| `KV_URL` | When KV backend | — | Redis-protocol connection URL |
| `REDIS_URL` | No | — | Redis pool for agent execution queue |
| `SECRETS_MASTER_KEY` | Yes (release) | — | Fernet key for secrets at rest |
| `COMMIT_SIGNING_KEY` | No | — | Ed25519 PEM private key for commit signing |
| `S3_BUCKET` | No | — | Enables S3 durable backend |
| `MAX_QUEUE_DEPTH` | No | `1000` | Max agent job queue depth |
| `LLM_UPSTREAM_URL` | No | `https://openrouter.ai/api/v1` | LLM proxy upstream |
| `PORT` | No | `8000` | Listen port |

In release builds (`cfg(not(debug_assertions))`), missing `JWT_SECRET` and `SECRETS_MASTER_KEY` cause a panic at startup.
