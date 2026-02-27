# The Corporation API (Rust/Axum)

Git-backed corporate governance API. Every entity is a bare git repository — documents, cap tables, governance records, and financial data are JSON files committed atomically with full history, branching, and cryptographic signing.

## Quick Start

```bash
# Local development (requires Rust 1.85+)
cd services/api-rs
cargo run

# Run tests
cargo test

# Production (Docker)
docker compose -f ops/docker-compose.prod.yml up backend
```

The server starts on `http://localhost:8000`. Verify with:

```bash
curl http://localhost:8000/health
# {"status":"ok"}
```

## Configuration

All configuration is via environment variables:

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | **Yes** (release) | dev-only fallback | HMAC secret for signing/verifying JWTs. Must be >= 32 bytes in production. |
| `COMMIT_SIGNING_KEY` | No | — | PEM-encoded Ed25519 private key for cryptographic commit signing. When absent, commits are unsigned. |
| `RUST_LOG` | No | — | Controls log verbosity via `tracing-subscriber` `EnvFilter`. Example: `RUST_LOG=api_rs=debug,tower_http=info` |

### Generating Secrets

```bash
# JWT secret (random 64-byte hex string)
openssl rand -hex 64

# Ed25519 commit signing key
ssh-keygen -t ed25519 -f corp-signing-key -N "" -C "corp-engine"
# Set COMMIT_SIGNING_KEY to the contents of corp-signing-key (the private key file)
# Use the public key (corp-signing-key.pub) in .gitconfig for signature verification
```

When `COMMIT_SIGNING_KEY` is set, every git commit produced by the API includes an SSH signature verifiable with `git log --show-signature`. The key fingerprint is logged at startup.

## Architecture

### Git-Based Storage

The API replaces traditional databases (PostgreSQL, Redis) with **local bare git repositories**. This gives every piece of corporate data:

- **Full history** — every change is a git commit with timestamp and author
- **Atomic multi-file commits** — cap table updates, governance votes, and financial entries are committed as single atomic operations
- **Branching** — entities support branches for draft workflows (e.g., draft a board resolution on a feature branch, then merge to main)
- **Three-way merge with JSON conflict resolution** — when branches diverge, the engine performs field-level JSON merging. Source branch wins on same-field conflicts (last-writer-wins)
- **Cryptographic provenance** — optional Ed25519 SSH signatures on every commit, with actor identity trailers

### On-Disk Layout

```
{data_dir}/
  {workspace_id}/
    _workspace.git          # workspace metadata, API keys
    {entity_id}.git         # entity data (corp.json, cap-table/, governance/, etc.)
    {entity_id}.git         # another entity...
  {workspace_id}/
    ...
```

Default `data_dir` is `./data/repos`. In production (Docker), this is mounted as a persistent volume at `/data/repos`.

### Entity Repository Structure

Each entity's bare git repo contains:

```
corp.json                           # Entity record (legal name, type, jurisdiction, status)
formation/
  {document_id}.json                # Formation documents (articles, bylaws, operating agreement)
  filing.json                       # State filing record
cap-table/
  cap-table.json                    # Cap table summary
  classes/{share_class_id}.json     # Share classes
  grants/{grant_id}.json            # Equity grants
  transfers/{transfer_id}.json      # Share transfers
governance/
  bodies/{body_id}.json             # Governance bodies (board, members)
  seats/{seat_id}.json              # Governance seats
  meetings/{meeting_id}/
    meeting.json                    # Meeting record
    agenda/{item_id}.json           # Agenda items
    votes/{vote_id}.json            # Cast votes
    resolutions/{resolution_id}.json # Computed resolutions
treasury/
  accounts/{account_id}.json        # GL accounts
  journal-entries/{entry_id}.json   # Journal entries
  invoices/{invoice_id}.json        # Invoices
  bank-accounts/{bank_id}.json      # Bank accounts
  payments/{payment_id}.json        # Payments
  payroll/{run_id}.json             # Payroll runs
  distributions/{dist_id}.json      # Distributions
  reconciliations/{recon_id}.json   # Reconciliations
contacts/{contact_id}.json          # Contacts
contracts/{contract_id}.json        # Contracts
execution/
  intents/{intent_id}.json          # Execution intents
  obligations/{obligation_id}.json  # Obligations
  receipts/{receipt_id}.json        # Receipts
tax/
  profile.json                      # Tax profile
  filings/{filing_id}.json          # Tax filings
deadlines/{deadline_id}.json        # Compliance deadlines
contractors/{classification_id}.json # Contractor classifications
.corp/
  access-manifest.json              # Stakeholder projection rules
```

### Storage Is Local-Only

Repos are local bare git repositories on the server's filesystem. There is no built-in remote push/pull — the API operates entirely against local repos. Backups are handled at the infrastructure level (volume snapshots, filesystem replication).

Because these are standard git repos, you *can* add a remote and push manually for off-site backup:

```bash
cd /data/repos/{workspace_id}/{entity_id}.git
git remote add origin git@github.com:org/entity-backup.git
git push --mirror origin
```

This is an operational concern, not an API feature.

## API Overview

All endpoints are prefixed with `/v1/`. The API serves an OpenAPI 3.1 spec at:

```
GET /v1/openapi.json
```

### Domain Groups

| Domain | Prefix | Description |
|---|---|---|
| **Formation** | `/v1/formations/`, `/v1/entities/`, `/v1/documents/` | Entity creation, documents, filing confirmation |
| **Equity** | `/v1/equity/`, `/v1/safe-notes/`, `/v1/valuations/`, `/v1/share-transfers/`, `/v1/funding-rounds/` | Cap table, grants, SAFEs, valuations, transfers |
| **Governance** | `/v1/governance-bodies/`, `/v1/meetings/`, `/v1/governance-seats/` | Bodies, seats, meetings, votes, resolutions |
| **Treasury** | `/v1/treasury/`, `/v1/invoices/`, `/v1/payments/`, `/v1/bank-accounts/` | GL accounts, journal entries, invoices, banking, payroll |
| **Contacts** | `/v1/contacts/` | Contact management |
| **Execution** | `/v1/execution/`, `/v1/intents/`, `/v1/obligations/`, `/v1/receipts/` | Intent lifecycle, obligations, receipts |
| **Branches** | `/v1/branches/` | Git branch create, list, merge, delete |
| **Auth** | `/v1/workspaces/`, `/v1/api-keys/`, `/v1/auth/` | Workspace provisioning, API keys, token exchange |
| **Agents** | `/v1/agents/` | Agent registration and messaging |
| **Compliance** | `/v1/tax/`, `/v1/deadlines/`, `/v1/contractors/` | Tax filings, deadlines, contractor classification |
| **Billing** | `/v1/billing/`, `/v1/subscriptions/` | Checkout, portal, plans, subscriptions |
| **Admin** | `/v1/admin/`, `/v1/workspace/`, `/v1/demo/` | System health, workspace status, audit events |

### Authentication

The API supports two authentication methods:

1. **API Keys** — created via `POST /v1/api-keys`. Keys are scoped and workspace-bound. Pass as `Authorization: Bearer {api_key}`.
2. **JWT Tokens** — obtained via `POST /v1/auth/token-exchange` (exchange an API key for a short-lived JWT). Pass as `Authorization: Bearer {jwt}`.

Provisioning a new workspace (`POST /v1/workspaces/provision`) returns both a `workspace_id` and an initial API key.

### Branch Targeting

Most read/write operations default to the `main` branch. To target a different branch, set the `X-Corp-Branch` header:

```
X-Corp-Branch: feature/equity-restructure
```

### Error Responses

All errors follow a consistent envelope:

```json
{
  "error": {
    "code": "not_found",
    "detail": "branch not found: feature/old"
  }
}
```

| HTTP Status | Error Code | Meaning |
|---|---|---|
| 400 | `bad_request` | Invalid input |
| 401 | `unauthorized` | Missing or invalid authentication |
| 403 | `forbidden` | Authenticated but insufficient scopes |
| 404 | `not_found` | Resource or branch not found |
| 409 | `conflict` | Merge conflict or duplicate resource |
| 422 | `validation_error` | Domain validation failure (invalid state transition, etc.) |
| 429 | `rate_limit_exceeded` | Rate limited (includes `Retry-After` header) |
| 500 | `internal_error` | Unexpected server error |

## API Examples

### Provision a Workspace

```bash
curl -X POST http://localhost:8000/v1/workspaces/provision \
  -H "Content-Type: application/json" \
  -d '{"name": "Acme Holdings"}'
```

```json
{
  "workspace_id": "a1b2c3d4-...",
  "name": "Acme Holdings",
  "api_key": "corp_key_...",
  "api_key_id": "e5f6a7b8-..."
}
```

### Create an Entity

```bash
curl -X POST http://localhost:8000/v1/formations \
  -H "Authorization: Bearer corp_key_..." \
  -H "Content-Type: application/json" \
  -d '{
    "entity_type": "llc",
    "legal_name": "Acme Ventures LLC",
    "jurisdiction": "Delaware",
    "members": [
      {"name": "Alice Chen", "email": "alice@acme.vc", "role": "Managing Member", "ownership_pct": "60.00"},
      {"name": "Bob Park", "email": "bob@acme.vc", "role": "Member", "ownership_pct": "40.00"}
    ],
    "workspace_id": "a1b2c3d4-..."
  }'
```

```json
{
  "formation_id": "f9e8d7c6-...",
  "entity_id": "f9e8d7c6-...",
  "formation_status": "documents_pending",
  "document_ids": ["1a2b3c4d-...", "5e6f7a8b-..."],
  "next_action": "sign_documents"
}
```

### Create a Branch

```bash
curl -X POST "http://localhost:8000/v1/branches?workspace_id=...&entity_id=..." \
  -H "Content-Type: application/json" \
  -d '{"name": "draft/board-resolution", "from": "main"}'
```

```json
{
  "branch": "draft/board-resolution",
  "base_commit": "a1b2c3d4e5f6..."
}
```

### Merge a Branch

```bash
curl -X POST "http://localhost:8000/v1/branches/draft%2Fboard-resolution/merge?workspace_id=...&entity_id=..." \
  -H "Content-Type: application/json" \
  -d '{"into": "main"}'
```

```json
{
  "merged": true,
  "strategy": "fast_forward",
  "commit": "b2c3d4e5f6a7..."
}
```

Merge strategies: `fast_forward`, `three_way`, `already_up_to_date`.

## Code Structure

```
src/
  main.rs              # Server bootstrap, router composition
  config.rs            # Environment-based configuration
  error.rs             # AppError → HTTP status code mapping
  openapi.rs           # OpenAPI 3.1 spec generation
  git/
    mod.rs             # Git module (repo, commit, merge, branch, signing, projection)
    repo.rs            # CorpRepo — bare git repository wrapper
    commit.rs          # Atomic multi-file commits with tree overlay
    merge.rs           # Three-way merge with JSON-aware conflict resolution
    branch.rs          # Branch create/list/delete
    signing.rs         # Ed25519 SSH commit signing and actor trailers
    projection.rs      # Stakeholder projection engine (access-manifest-based filtering)
    error.rs           # GitStorageError types
  store/
    mod.rs             # RepoLayout — on-disk path management
    entity_store.rs    # EntityStore — typed read/write for entity repos
    workspace_store.rs # WorkspaceStore — workspace metadata and API keys
  domain/
    ids.rs             # Typed UUID wrappers (EntityId, WorkspaceId, etc.)
    formation/         # Entity types, documents, filing, service logic
    equity/            # Cap table, grants, SAFEs, valuations, transfers
    governance/        # Bodies, seats, meetings, votes, resolutions
    treasury/          # Accounts, journal entries, invoices, banking
    contacts/          # Contact management
    execution/         # Intents, obligations, receipts
    auth/              # JWT claims, API key generation, scopes
    agents/            # Agent registration
    compliance/        # Tax filings, deadlines
    billing/           # Stripe integration stubs
  routes/
    mod.rs             # AppState, shared extractors
    formation.rs       # Formation endpoints
    equity.rs          # Equity endpoints
    governance.rs      # Governance endpoints
    treasury.rs        # Treasury endpoints
    contacts.rs        # Contact endpoints
    execution.rs       # Execution endpoints
    branches.rs        # Branch management endpoints
    auth.rs            # Auth/workspace endpoints
    agents.rs          # Agent endpoints
    compliance.rs      # Compliance endpoints
    billing.rs         # Billing endpoints
    admin.rs           # Admin endpoints
    projection.rs      # Projection endpoints
    webhooks.rs        # Stripe webhook endpoints
```

## Testing

```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_three_way_merge

# Run with output
cargo test -- --nocapture
```

Tests use `tempfile::TempDir` for isolated git repos. Integration tests in `tests/api_lifecycle.rs` exercise the full HTTP API via `tower::ServiceExt::oneshot` — no TCP listener required.

## Deployment

### Docker

```bash
docker build -t corp-api services/api-rs/
docker run -p 8000:8000 \
  -e JWT_SECRET=$(openssl rand -hex 64) \
  -v corp-data:/data/repos \
  corp-api
```

### Docker Compose (Production)

```bash
# From repo root
docker compose -f ops/docker-compose.prod.yml up -d backend
```

The production compose file:
- Mounts a persistent `git_data` volume at `/data/repos`
- Requires `JWT_SECRET` environment variable
- Optionally accepts `COMMIT_SIGNING_KEY`
- Limits memory to 512MB
- Exposes port 8000 on the internal `backend` network (Caddy reverse proxy handles external HTTPS)

### Health Check

```
GET /health → {"status": "ok"}
```

## Observability

Structured logging via `tracing` + `tracing-subscriber`. Control verbosity with `RUST_LOG`:

```bash
RUST_LOG=api_rs=debug,tower_http=trace cargo run
```

Key log events:
- `initialized bare repo` — new entity/workspace repo created
- `committed files` — atomic commit with file count
- `fast-forward merge` / `three-way merge` — merge operations
- `commit signing enabled` — startup confirmation with key fingerprint
- `internal error: ...` — 500-level errors logged at ERROR level
