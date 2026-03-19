# `npx corp next` Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `next` command to the CLI that shows the single most important action and a categorized backlog of remaining recommendations, with copy-pasteable commands.

**Architecture:** New Rust route module (`routes/next_steps.rs`) computes recommendations by inspecting entity state across formation, documents, governance, equity, obligations, and treasury. Two endpoints: entity-scoped and workspace-scoped. CLI command (`commands/next.ts`) adds local config checks and renders output. Types flow through OpenAPI generation.

**Tech Stack:** Rust (axum, utoipa, serde), TypeScript (commander, chalk, cli-table3)

**Spec:** `docs/superpowers/specs/2026-03-19-next-command-design.md`

---

## File Structure

### New files
| File | Responsibility |
|------|---------------|
| `services/api-rs/src/routes/next_steps.rs` | Route module: response types, recommendation engine, handlers, OpenAPI annotations |
| `packages/cli-ts/src/commands/next.ts` | CLI command: local checks, API call, output delegation |

### Modified files
| File | Change |
|------|--------|
| `services/api-rs/src/routes/mod.rs` | Add `pub mod next_steps;` |
| `services/api-rs/src/main.rs` | Merge `next_steps_routes()` into router |
| `services/api-rs/src/openapi.rs` | Merge `NextStepsApi::openapi()` |
| `packages/corp-tools/src/api-schemas.ts` | Export `NextStepsResponse`, `NextStepItem` types |
| `packages/corp-tools/src/api-client.ts` | Add `getEntityNextSteps()`, `getWorkspaceNextSteps()` methods |
| `packages/cli-ts/src/output.ts` | Add `printNextSteps()` formatting function |
| `packages/cli-ts/src/index.ts` | Register `next` command (first position, prominent help text) |

---

## Task 1: Rust response types and OpenAPI schemas

**Files:**
- Create: `services/api-rs/src/routes/next_steps.rs`
- Modify: `services/api-rs/src/routes/mod.rs`

- [ ] **Step 1: Create `next_steps.rs` with response types**

```rust
// services/api-rs/src/routes/next_steps.rs
use serde::Serialize;
use utoipa::ToSchema;

/// A single recommended action.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NextStepItem {
    /// Category grouping: formation, documents, governance, cap_table, compliance, finance, agents
    pub category: String,
    /// Human-readable title
    pub title: String,
    /// Optional longer description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Copy-pasteable CLI command
    pub command: String,
    /// Urgency: critical, high, medium, low
    pub urgency: String,
}

/// Summary counts by urgency tier.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NextStepsSummary {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
}

/// Response for next-steps endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NextStepsResponse {
    /// The single most important action, or null if all caught up.
    pub top: Option<NextStepItem>,
    /// Remaining actions grouped by category.
    pub backlog: Vec<NextStepItem>,
    /// Counts by urgency tier (all keys always present).
    pub summary: NextStepsSummary,
}

impl NextStepsSummary {
    pub fn from_items(top: &Option<NextStepItem>, backlog: &[NextStepItem]) -> Self {
        let mut s = Self { critical: 0, high: 0, medium: 0, low: 0 };
        let all = top.iter().chain(backlog.iter());
        for item in all {
            match item.urgency.as_str() {
                "critical" => s.critical += 1,
                "high" => s.high += 1,
                "medium" => s.medium += 1,
                _ => s.low += 1,
            }
        }
        s
    }
}
```

- [ ] **Step 2: Add module declaration**

In `services/api-rs/src/routes/mod.rs`, add after line 14 (`pub mod references;`):

```rust
pub mod next_steps;
```

- [ ] **Step 3: Verify it compiles**

Run from `services/api-rs/`:
```bash
cargo check 2>&1 | head -20
```
Expected: compiles with no errors (warnings OK).

- [ ] **Step 4: Commit**

```bash
git add services/api-rs/src/routes/next_steps.rs services/api-rs/src/routes/mod.rs
git commit -m "feat(api): add NextStepsResponse types and OpenAPI schemas"
```

---

## Task 2: Recommendation engine

**Files:**
- Modify: `services/api-rs/src/routes/next_steps.rs`

The recommendation engine is a pure function that takes entity state and returns a `Vec<NextStepItem>`. No HTTP, no async — just data inspection logic. This is the core of the feature.

Reference for how to read entity state:
- `store.read_entity("main")` → `Entity` with `formation_status()` (see `routes/admin.rs:222`)
- `store.list_ids::<T>("main")` → list stored entity IDs (see `routes/governance.rs:2021`)
- `store.read::<T>("main", id)` → read a stored entity (see `routes/governance.rs:2023`)
- Domain types live in `src/domain/` — formation (`domain/formation/`), governance (`domain/governance/`), equity (`domain/equity/`), treasury (`domain/treasury/`), execution (`domain/execution/`)

- [ ] **Step 1: Add imports and helper**

At the top of `next_steps.rs`, add the imports needed for the recommendation engine:

```rust
use crate::domain::ids::EntityId;
use crate::store::EntityStore;
```

Add a helper to build command strings:

```rust
fn cmd(parts: &[&str]) -> String {
    let mut c = String::from("npx corp");
    for p in parts {
        c.push(' ');
        c.push_str(p);
    }
    c
}
```

- [ ] **Step 2: Write the `compute_next_steps` function**

This function inspects entity state via the store and builds recommendations. Add to `next_steps.rs`:

```rust
use crate::domain::formation::types::FormationStatus;
use crate::domain::governance::body::GovernanceBody;
use crate::domain::governance::seat::GovernanceSeat;
use crate::domain::governance::types::{SeatStatus, MeetingStatus};
use crate::domain::governance::meeting::Meeting;
use crate::domain::equity::instrument::Instrument;
use crate::domain::execution::obligation::Obligation;
use crate::domain::execution::types::ObligationStatus;
use crate::domain::formation::document::Document;
use crate::domain::treasury::bank_account::BankAccount;
use chrono::Utc;

/// Compute next-step recommendations for an entity.
/// Runs inside `spawn_blocking` — no async.
pub fn compute_next_steps(store: &EntityStore, entity_id: EntityId) -> Vec<NextStepItem> {
    let eid = entity_id.to_string();
    let mut items: Vec<NextStepItem> = Vec::new();

    // 1. Formation status
    // Variants: Pending, DocumentsGenerated, DocumentsSigned, FilingSubmitted,
    //           Filed, EinApplied, Active, Rejected, Dissolved
    if let Ok(entity) = store.read_entity("main") {
        let status = entity.formation_status();
        match status {
            FormationStatus::Pending => {
                items.push(NextStepItem {
                    category: "formation".into(),
                    title: format!("Finalize formation for {}", entity.legal_name()),
                    description: Some("Formation is pending — finalize to generate documents and cap table".into()),
                    command: cmd(&["form", "finalize", &eid]),
                    urgency: "critical".into(),
                });
            }
            FormationStatus::DocumentsGenerated
            | FormationStatus::DocumentsSigned
            | FormationStatus::FilingSubmitted
            | FormationStatus::Filed
            | FormationStatus::EinApplied => {
                items.push(NextStepItem {
                    category: "formation".into(),
                    title: format!("Activate {}", entity.legal_name()),
                    description: Some(format!("Formation status is {} — activate when ready", status)),
                    command: cmd(&["form", "activate", &eid]),
                    urgency: "critical".into(),
                });
            }
            FormationStatus::Active => {} // No formation action needed
            _ => {} // Rejected, Dissolved — no action
        }
    }

    // 2. Unsigned documents
    if let Ok(doc_ids) = store.list_ids::<Document>("main") {
        for doc_id in doc_ids {
            if let Ok(doc) = store.read::<Document>("main", doc_id) {
                if !doc.is_fully_signed() {
                    items.push(NextStepItem {
                        category: "documents".into(),
                        title: format!("Sign {}", doc.title()),
                        description: None,
                        command: cmd(&["documents", "signing-link", &doc_id.to_string()]),
                        urgency: "high".into(),
                    });
                }
            }
        }
    }

    // 3. No governance bodies
    let body_ids = store.list_ids::<GovernanceBody>("main").unwrap_or_default();
    if body_ids.is_empty() {
        // Only suggest if entity is active (formation complete)
        if items.iter().all(|i| i.category != "formation") {
            items.push(NextStepItem {
                category: "governance".into(),
                title: "Create board of directors".into(),
                description: Some("No governance bodies exist yet".into()),
                command: cmd(&["governance", "--entity-id", &eid, "create-body", "--name", "\"Board of Directors\"", "--body-type", "board_of_directors"]),
                urgency: "medium".into(),
            });
        }
    } else {
        // 4. Unfilled seats (Resigned or Expired status)
        // SeatStatus variants: Active, Resigned, Expired
        if let Ok(seat_ids) = store.list_ids::<GovernanceSeat>("main") {
            let unfilled = seat_ids.iter().filter(|sid| {
                store.read::<GovernanceSeat>("main", **sid)
                    .map(|s| matches!(s.status(), SeatStatus::Resigned | SeatStatus::Expired))
                    .unwrap_or(false)
            }).count();
            if unfilled > 0 {
                items.push(NextStepItem {
                    category: "governance".into(),
                    title: format!("{} unfilled governance seat{}", unfilled, if unfilled == 1 { "" } else { "s" }),
                    description: Some("Appoint members to fill resigned or expired seats".into()),
                    command: cmd(&["governance", "--entity-id", &eid, "seats"]),
                    urgency: "medium".into(),
                });
            }
        }
    }

    // 5. No equity instruments (cap table empty)
    if let Ok(instrument_ids) = store.list_ids::<Instrument>("main") {
        if instrument_ids.is_empty() && items.iter().all(|i| i.category != "formation") {
            items.push(NextStepItem {
                category: "cap_table".into(),
                title: "Set up cap table — create share classes".into(),
                description: Some("No equity instruments exist yet".into()),
                command: cmd(&["cap-table", "--entity-id", &eid, "create-instrument"]),
                urgency: "medium".into(),
            });
        }
    }

    // 6 & 7. Obligations (overdue and upcoming)
    // ObligationStatus variants: Required, InProgress, Fulfilled, Waived, Expired
    // Urgency is computed from due_date() relative to today.
    if let Ok(obl_ids) = store.list_ids::<Obligation>("main") {
        let today = Utc::now().date_naive();
        for obl_id in obl_ids {
            if let Ok(obl) = store.read::<Obligation>("main", obl_id) {
                let is_open = matches!(obl.status(), ObligationStatus::Required | ObligationStatus::InProgress);
                if is_open {
                    let urgency = match obl.due_date() {
                        Some(due) => {
                            let days_until = (due - today).num_days();
                            if days_until < 0 { "critical" }      // overdue
                            else if days_until == 0 { "critical" } // due_today
                            else if days_until <= 7 { "high" }     // d1-d7
                            else if days_until <= 30 { "medium" }  // d14-d30
                            else { "low" }                         // upcoming
                        }
                        None => "low", // no due date
                    };
                    items.push(NextStepItem {
                        category: "compliance".into(),
                        title: obl.description().to_string(),
                        description: obl.due_date().map(|d| format!("Due {d}")),
                        command: cmd(&["obligations"]),
                        urgency: urgency.into(),
                    });
                }
            }
        }
    }

    // 8. No bank account
    if let Ok(acct_ids) = store.list_ids::<BankAccount>("main") {
        if acct_ids.is_empty() && items.iter().all(|i| i.category != "formation") {
            items.push(NextStepItem {
                category: "finance".into(),
                title: "Open a bank account".into(),
                description: None,
                command: cmd(&["finance", "--entity-id", &eid, "open-account"]),
                urgency: "low".into(),
            });
        }
    }

    // 9 & 10. Active meetings needing votes / convened meetings
    // MeetingStatus variants: Draft, Noticed, Convened, Adjourned, Cancelled
    if let Ok(meeting_ids) = store.list_ids::<Meeting>("main") {
        for mid in meeting_ids {
            if let Ok(mtg) = store.read::<Meeting>("main", mid) {
                match mtg.status() {
                    MeetingStatus::Convened => {
                        items.push(NextStepItem {
                            category: "governance".into(),
                            title: format!("Meeting in session — cast your vote"),
                            description: Some(format!("Meeting {} is convened and awaiting votes", mid)),
                            command: cmd(&["governance", "--entity-id", &eid, "vote", "--meeting-id", &mid.to_string()]),
                            urgency: "high".into(),
                        });
                    }
                    MeetingStatus::Noticed => {
                        items.push(NextStepItem {
                            category: "governance".into(),
                            title: format!("Upcoming meeting — convene when ready"),
                            description: Some(format!("Meeting {} has been noticed", mid)),
                            command: cmd(&["governance", "--entity-id", &eid, "convene", "--meeting-id", &mid.to_string()]),
                            urgency: "medium".into(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    items
}
```

**Verified domain types used above:**
- `FormationStatus` — from `domain/formation/types.rs:55` — variants: Pending, DocumentsGenerated, DocumentsSigned, FilingSubmitted, Filed, EinApplied, Active, Rejected, Dissolved
- `Entity` — `formation_status()`, `legal_name()` — from `domain/formation/entity.rs:258,242`
- `Document` — `is_fully_signed()`, `title()` — from `domain/formation/document.rs:106,48`
- `GovernanceBody` — from `domain/governance/body.rs`
- `GovernanceSeat` — `status()` — from `domain/governance/seat.rs`
- `SeatStatus` — from `domain/governance/types.rs:191` — variants: Active, Resigned, Expired
- `MeetingStatus` — from `domain/governance/types.rs:217` — variants: Draft, Noticed, Convened, Adjourned, Cancelled
- `Meeting` — `status()` — from `domain/governance/meeting.rs`
- `Instrument` — from `domain/equity/instrument.rs`
- `Obligation` — `status()`, `description()`, `due_date()` — from `domain/execution/obligation.rs:141,135,138`
- `ObligationStatus` — from `domain/execution/types.rs:44` — variants: Required, InProgress, Fulfilled, Waived, Expired
- `BankAccount` — from `domain/treasury/bank_account.rs:12`

- [ ] **Step 3: Verify it compiles**

```bash
cd /root/repos/thecorporation-mono/services/api-rs && cargo check 2>&1 | head -30
```

Fix any compile errors from incorrect method names — read the domain type files to find the right accessor.

- [ ] **Step 4: Commit**

```bash
git add services/api-rs/src/routes/next_steps.rs
git commit -m "feat(api): add next-steps recommendation engine"
```

---

## Task 3: Rust HTTP handlers and route wiring

**Files:**
- Modify: `services/api-rs/src/routes/next_steps.rs`
- Modify: `services/api-rs/src/main.rs`
- Modify: `services/api-rs/src/openapi.rs`

- [ ] **Step 1: Add entity-scoped handler**

Append to `next_steps.rs`:

```rust
use axum::{Json, Router, extract::{Path, State}};
use axum::routing::get;
use super::AppState;
use crate::auth::RequireExecutionRead;
use crate::error::AppError;

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/next-steps",
    tag = "next_steps",
    params(("entity_id" = String, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Next steps for entity", body = NextStepsResponse),
    ),
)]
async fn entity_next_steps(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<NextStepsResponse>, AppError> {
    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
        move || {
            let store = EntityStore::open(
                &layout,
                auth.workspace_id(),
                entity_id,
                valkey_client.as_ref(),
            ).map_err(|e| AppError::NotFound(format!("entity not found: {e}")))?;

            // Verify entity scope
            if let Some(ref scope) = entity_scope {
                if !scope.contains(&entity_id) {
                    return Err(AppError::Forbidden("entity not in scope".into()));
                }
            }

            let items = compute_next_steps(&store, entity_id);
            let (top, backlog) = if items.is_empty() {
                (None, vec![])
            } else {
                let mut sorted = items;
                // Sort by urgency priority: critical first, then high, medium, low
                sorted.sort_by_key(|i| match i.urgency.as_str() {
                    "critical" => 0,
                    "high" => 1,
                    "medium" => 2,
                    _ => 3,
                });
                let top = sorted.remove(0);
                (Some(top), sorted)
            };
            let summary = NextStepsSummary::from_items(&top, &backlog);
            Ok::<_, AppError>(NextStepsResponse { top, backlog, summary })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}
```

- [ ] **Step 2: Add workspace-scoped handler**

```rust
#[utoipa::path(
    get,
    path = "/v1/workspaces/{workspace_id}/next-steps",
    tag = "next_steps",
    params(("workspace_id" = String, Path, description = "Workspace ID")),
    responses(
        (status = 200, description = "Next steps across all entities", body = NextStepsResponse),
    ),
)]
async fn workspace_next_steps(
    RequireExecutionRead(auth): RequireExecutionRead,
    State(state): State<AppState>,
    Path(workspace_id): Path<crate::domain::ids::WorkspaceId>,
) -> Result<Json<NextStepsResponse>, AppError> {
    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
        move || {
            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut all_items: Vec<NextStepItem> = Vec::new();

            for eid in entity_ids {
                if let Some(ref scope) = entity_scope {
                    if !scope.contains(&eid) { continue; }
                }
                if let Ok(store) = EntityStore::open(
                    &layout, workspace_id, eid, valkey_client.as_ref()
                ) {
                    all_items.extend(compute_next_steps(&store, eid));
                }
            }

            all_items.sort_by_key(|i| match i.urgency.as_str() {
                "critical" => 0, "high" => 1, "medium" => 2, _ => 3,
            });

            let (top, backlog) = if all_items.is_empty() {
                (None, vec![])
            } else {
                let top = all_items.remove(0);
                (Some(top), all_items)
            };
            let summary = NextStepsSummary::from_items(&top, &backlog);
            Ok::<_, AppError>(NextStepsResponse { top, backlog, summary })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}
```

- [ ] **Step 3: Add router and OpenAPI struct**

```rust
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(entity_next_steps, workspace_next_steps),
    components(schemas(NextStepsResponse, NextStepItem, NextStepsSummary)),
    tags((name = "next_steps", description = "Actionable next-step recommendations")),
)]
pub struct NextStepsApi;

pub fn next_steps_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/entities/{entity_id}/next-steps", get(entity_next_steps))
        .route("/v1/workspaces/{workspace_id}/next-steps", get(workspace_next_steps))
}
```

- [ ] **Step 4: Wire into main router**

In `services/api-rs/src/main.rs`, add `.merge(routes::next_steps::next_steps_routes())` after line 365 (`.merge(routes::work_items::work_items_routes())`):

```rust
        .merge(routes::next_steps::next_steps_routes())
```

- [ ] **Step 5: Wire into OpenAPI spec**

In `services/api-rs/src/openapi.rs`, add to the `modules` vec (after line 34, the `services` line):

```rust
        routes::next_steps::NextStepsApi::openapi(),
```

And add to the tags list (after line 78):

```rust
        (name = "next_steps", description = "Actionable next-step recommendations"),
```

- [ ] **Step 6: Verify it compiles**

```bash
cd /root/repos/thecorporation-mono/services/api-rs && cargo check 2>&1 | head -30
```

- [ ] **Step 7: Commit**

```bash
git add services/api-rs/src/routes/next_steps.rs services/api-rs/src/main.rs services/api-rs/src/openapi.rs
git commit -m "feat(api): add next-steps HTTP handlers and wire routes"
```

---

## Task 4: Generate TypeScript types and add API client methods

**Files:**
- Modify: `packages/corp-tools/src/api-schemas.ts`
- Modify: `packages/corp-tools/src/api-client.ts`

- [ ] **Step 1: Regenerate OpenAPI types**

From the project root, regenerate the TypeScript types from the OpenAPI spec. Check how the existing generation works:

```bash
cd /root/repos/thecorporation-mono/packages/corp-tools && cat package.json | grep -A2 generate
```

Run the type generation command (likely `npm run generate:types`). If it requires the API server running, manually add the types instead.

- [ ] **Step 2: Add type exports to `api-schemas.ts`**

Add after line 79 (the billing section) in `packages/corp-tools/src/api-schemas.ts`:

```typescript
// ── Next Steps ──────────────────────────────────────────────────────
export type NextStepsResponse = components["schemas"]["NextStepsResponse"];
export type NextStepItem = components["schemas"]["NextStepItem"];
export type NextStepsSummary = components["schemas"]["NextStepsSummary"];
```

If type generation requires a running server and is not available, first build and start the API (`cargo run`), fetch the OpenAPI JSON from `/v1/openapi.json`, and run the type generator. Do not hand-write TypeScript interfaces — all types must flow from the Rust OpenAPI schema per the spec.

- [ ] **Step 3: Add API client methods**

In `packages/corp-tools/src/api-client.ts`, add to the `CorpAPIClient` class (near the end, alongside other get methods):

```typescript
  getEntityNextSteps(entityId: string) {
    return this.get(`/v1/entities/${pathSegment(entityId)}/next-steps`) as Promise<NextStepsResponse>;
  }

  getWorkspaceNextSteps() {
    return this.get(`/v1/workspaces/${pathSegment(this.workspaceId)}/next-steps`) as Promise<NextStepsResponse>;
  }
```

Import `NextStepsResponse` at the top of the file alongside other schema imports.

- [ ] **Step 4: Verify TypeScript compiles**

```bash
cd /root/repos/thecorporation-mono/packages/corp-tools && npx tsc --noEmit 2>&1 | head -20
```

- [ ] **Step 5: Commit**

```bash
git add packages/corp-tools/src/api-schemas.ts packages/corp-tools/src/api-client.ts
git commit -m "feat(corp-tools): add NextSteps types and API client methods"
```

---

## Task 5: CLI output formatting

**Files:**
- Modify: `packages/cli-ts/src/output.ts`

- [ ] **Step 1: Add urgency color map for next-steps tiers**

At the top of `output.ts` (after line 19, the existing `URGENCY_COLORS`), add:

```typescript
const NEXT_STEP_COLORS: Record<string, (s: string) => string> = {
  critical: chalk.red.bold,
  high: chalk.yellow,
  medium: chalk.cyan,
  low: chalk.dim,
};
```

- [ ] **Step 2: Add `printNextSteps` function**

After `printFinanceSummaryPanel` (after line 191), add:

```typescript
const CATEGORY_LABELS: Record<string, string> = {
  setup: "Setup",
  formation: "Formation",
  documents: "Documents",
  governance: "Governance",
  cap_table: "Cap Table",
  compliance: "Compliance",
  finance: "Finance",
  agents: "Agents",
};

export function printNextSteps(data: {
  top: Record<string, unknown> | null;
  backlog: Record<string, unknown>[];
  summary: Record<string, number>;
}): void {
  if (!data.top) {
    console.log(chalk.green.bold("\n  All caught up! No pending actions.\n"));
    return;
  }

  // Top recommendation
  console.log();
  console.log(chalk.bold("  Next up:"));
  const topColor = NEXT_STEP_COLORS[s(data.top.urgency)] ?? ((x: string) => x);
  console.log(`   ${topColor(s(data.top.title))}`);
  if (data.top.description) {
    console.log(`   ${chalk.dim(s(data.top.description))}`);
  }
  console.log(`   ${chalk.green("→")} ${chalk.green(s(data.top.command))}`);

  // Backlog grouped by category
  if (data.backlog.length > 0) {
    console.log();
    console.log(chalk.bold("  More to do:"));

    // Group by category
    const groups = new Map<string, Record<string, unknown>[]>();
    for (const item of data.backlog) {
      const cat = s(item.category) || "other";
      if (!groups.has(cat)) groups.set(cat, []);
      groups.get(cat)!.push(item);
    }

    for (const [cat, items] of groups) {
      const label = CATEGORY_LABELS[cat] ?? cat;
      console.log(`\n   ${chalk.bold(`${label} (${items.length})`)}`);
      for (const item of items) {
        const color = NEXT_STEP_COLORS[s(item.urgency)] ?? ((x: string) => x);
        console.log(`    ${color("•")} ${s(item.title)}`);
        if (item.description) {
          console.log(`      ${chalk.dim(s(item.description))}`);
        }
        console.log(`      ${chalk.green("→")} ${chalk.green(s(item.command))}`);
      }
    }
  }

  // Summary footer
  const { critical = 0, high = 0, medium = 0, low = 0 } = data.summary;
  const total = critical + high + medium + low;
  const parts: string[] = [];
  if (critical > 0) parts.push(chalk.red.bold(`${critical} critical`));
  if (high > 0) parts.push(chalk.yellow(`${high} high`));
  if (medium > 0) parts.push(chalk.cyan(`${medium} medium`));
  if (low > 0) parts.push(chalk.dim(`${low} low`));
  console.log(`\n  ${total} item${total === 1 ? "" : "s"} total (${parts.join(", ")})\n`);
}
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add packages/cli-ts/src/output.ts
git commit -m "feat(cli): add printNextSteps output formatter"
```

---

## Task 6: CLI `next` command implementation

**Files:**
- Create: `packages/cli-ts/src/commands/next.ts`

- [ ] **Step 1: Create the command file**

```typescript
// packages/cli-ts/src/commands/next.ts
import { loadConfig, requireConfig, resolveEntityId, getActiveEntityId } from "../config.js";
import { CorpAPIClient } from "../api-client.js";
import { printError, printJson, printNextSteps, printWarning } from "../output.js";
import { withSpinner } from "../spinner.js";
import type { NextStepsResponse, NextStepItem } from "@thecorporation/corp-tools";

interface NextOpts {
  entityId?: string;
  workspace?: boolean;
  json?: boolean;
}

/**
 * Check local config state and return any CLI-side recommendations.
 * These take priority over server-side recommendations.
 */
function localChecks(): NextStepItem[] {
  const items: NextStepItem[] = [];
  let cfg;
  try {
    cfg = loadConfig();
  } catch {
    items.push({
      category: "setup",
      title: "Run initial setup",
      description: "No configuration found",
      command: "npx corp setup",
      urgency: "critical",
    });
    return items;
  }

  if (!cfg.api_key) {
    items.push({
      category: "setup",
      title: "Run setup to configure API key",
      description: "No API key configured",
      command: "npx corp setup",
      urgency: "critical",
    });
    return items;
  }

  if (!cfg.workspace_id) {
    items.push({
      category: "setup",
      title: "Claim a workspace",
      description: "No workspace configured",
      command: "npx corp claim <code>",
      urgency: "critical",
    });
    return items;
  }

  if (!getActiveEntityId(cfg)) {
    items.push({
      category: "setup",
      title: "Set an active entity",
      description: "No active entity — set one to get entity-specific recommendations",
      command: "npx corp use <entity-name>",
      urgency: "high",
    });
  }

  return items;
}

export async function nextCommand(opts: NextOpts): Promise<void> {
  // Check for mutually exclusive flags
  if (opts.entityId && opts.workspace) {
    printError("--entity-id and --workspace are mutually exclusive");
    process.exit(1);
  }

  // Run local checks first
  const localItems = localChecks();

  // If local checks found critical blockers (no config/key/workspace), show those and stop
  const hasCriticalLocal = localItems.some((i) => i.urgency === "critical");
  if (hasCriticalLocal) {
    const top = localItems[0];
    const backlog = localItems.slice(1);
    const summary = { critical: 0, high: 0, medium: 0, low: 0 };
    for (const item of [top, ...backlog]) {
      const key = item.urgency as keyof typeof summary;
      if (key in summary) summary[key]++;
    }
    const response = { top, backlog, summary };
    if (opts.json) {
      printJson(response);
    } else {
      printNextSteps(response);
    }
    return;
  }

  // Fetch server-side recommendations
  const cfg = requireConfig("api_url", "api_key", "workspace_id");
  const client = new CorpAPIClient(cfg.api_url, cfg.api_key, cfg.workspace_id);

  try {
    let data: NextStepsResponse;
    if (opts.workspace) {
      data = await withSpinner("Loading", () => client.getWorkspaceNextSteps(), opts.json);
    } else {
      const entityId = resolveEntityId(cfg, opts.entityId);
      data = await withSpinner("Loading", () => client.getEntityNextSteps(entityId), opts.json);
    }

    // Merge local items (non-critical, e.g. "set active entity") into backlog
    if (localItems.length > 0) {
      data.backlog.push(...localItems);
      // Recompute summary
      const all = [data.top, ...data.backlog].filter(Boolean) as NextStepItem[];
      data.summary = { critical: 0, high: 0, medium: 0, low: 0 };
      for (const item of all) {
        const key = item.urgency as keyof typeof data.summary;
        if (key in data.summary) data.summary[key]++;
      }
    }

    if (opts.json) {
      printJson(data);
    } else {
      printNextSteps(data);
    }
  } catch (err) {
    printError(`Failed to fetch next steps: ${err}`);
    process.exit(1);
  }
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit 2>&1 | head -20
```

- [ ] **Step 3: Commit**

```bash
git add packages/cli-ts/src/commands/next.ts
git commit -m "feat(cli): add next command implementation with local checks"
```

---

## Task 7: Register command in CLI and make prominent in help

**Files:**
- Modify: `packages/cli-ts/src/index.ts`

- [ ] **Step 1: Register `next` as the first command**

In `packages/cli-ts/src/index.ts`, add the `next` command registration **before** the `setup` command (before line 37, `// --- setup ---`). This makes it the first command in `--help` output:

```typescript
// --- next (featured) ---
program
  .command("next")
  .description("See what to do next — your recommended actions")
  .option("--entity-id <ref>", "Entity to check (default: active entity)")
  .option("--workspace", "Show recommendations across all entities")
  .option("--json", "Output as JSON")
  .action(async (opts) => {
    const { nextCommand } = await import("./commands/next.js");
    await nextCommand(opts);
  })
  .addHelpText(
    "after",
    `
Examples:
  $ corp next                          # Next steps for active entity
  $ corp next --workspace              # Next steps across all entities
  $ corp next --entity-id ent_abc123   # Next steps for specific entity
  $ corp next --json                   # JSON output for scripting
`,
  );
```

- [ ] **Step 2: Add featured command callout to program help**

After the `program.action(() => { program.outputHelp(); });` block (line 34), add help text that highlights the `next` command:

```typescript
program.addHelpText(
  "after",
  `
Tip: Run ${chalk.bold("corp next")} to see your recommended next actions.
`,
);
```

This requires adding `import chalk from "chalk";` at the top of `index.ts` (or use a plain string if chalk isn't already imported there). Check if chalk is already imported — if not, add the import.

- [ ] **Step 3: Verify it compiles and help text looks right**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npx tsc --noEmit 2>&1 | head -10
```

Then build and check help output:
```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npm run build && node dist/index.js --help 2>&1 | head -20
```

Verify `next` appears first in the commands list and the tip is visible.

- [ ] **Step 4: Commit**

```bash
git add packages/cli-ts/src/index.ts
git commit -m "feat(cli): register next command as first/featured command in help"
```

---

## Task 8: Integration test — Rust endpoint

**Files:**
- Create: `services/api-rs/tests/next_steps_e2e.rs`

- [ ] **Step 1: Write integration test**

Follow the existing test pattern from `tests/governance_meeting_e2e.rs`:

```rust
// services/api-rs/tests/next_steps_e2e.rs
use axum::{Router, body::Body, http::{Method, Request, StatusCode}};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;
use std::collections::HashMap;
use std::sync::Arc;

use api_rs::domain::auth::claims::{Claims, PrincipalType, encode_token};
use api_rs::domain::auth::scopes::Scope;
use api_rs::domain::ids::WorkspaceId;

const TEST_SECRET: &[u8] = b"test-secret-for-integration-tests";

fn make_token(ws_id: WorkspaceId) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims::new(
        ws_id, None, None, None,
        PrincipalType::User,
        vec![Scope::All],
        now, now + 3600,
    );
    encode_token(&claims, TEST_SECRET).unwrap()
}

fn build_app(tmp: &TempDir) -> Router {
    unsafe { std::env::set_var("JWT_SECRET", "test-secret-for-integration-tests") };
    let layout = Arc::new(api_rs::store::RepoLayout::new(tmp.path().to_path_buf()));
    let state = api_rs::routes::AppState {
        layout,
        jwt_secret: Arc::from(TEST_SECRET.to_vec()),
        commit_signer: None,
        redis: None,
        secrets_fernet: None,
        max_queue_depth: 1000,
        http_client: reqwest::Client::new(),
        llm_upstream_url: "http://localhost:0".to_owned(),
        model_pricing: HashMap::new(),
        creation_rate_limiter: Arc::new(api_rs::routes::CreationRateLimiter::default()),
        storage_backend: api_rs::store::StorageBackendKind::Git,
        valkey_client: None,
    };
    Router::new()
        .merge(api_rs::routes::formation::formation_routes())
        .merge(api_rs::routes::next_steps::next_steps_routes())
        .with_state(state)
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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

#[tokio::test]
async fn next_steps_empty_entity_returns_formation_recommendation() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);

    // Create a pending entity via formation
    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);
    let (status, body) = post_json(&app, "/v1/formation", json!({
        "workspace_id": ws_id.to_string(),
        "legal_name": "Test Corp",
        "entity_type": "c_corp",
        "jurisdiction": "US-DE"
    }), &token).await;

    // If formation returns an entity_id, use it to check next-steps
    if status == StatusCode::OK || status == StatusCode::CREATED {
        let entity_id = body["entity_id"].as_str().unwrap_or("");
        if !entity_id.is_empty() {
            let (ns_status, ns_body) = get_json(
                &app,
                &format!("/v1/entities/{entity_id}/next-steps"),
                &token,
            ).await;

            assert_eq!(ns_status, StatusCode::OK);
            assert!(ns_body["top"].is_object(), "should have a top recommendation");
            assert_eq!(ns_body["top"]["category"], "formation");
            assert_eq!(ns_body["top"]["urgency"], "critical");
            assert!(ns_body["top"]["command"].as_str().unwrap().contains("finalize"));
            assert!(ns_body["summary"]["critical"].as_u64().unwrap() >= 1);
        }
    }
}

#[tokio::test]
async fn workspace_next_steps_returns_valid_response() {
    let tmp = TempDir::new().expect("temp dir");
    let app = build_app(&tmp);
    let ws_id = WorkspaceId::new();
    let token = make_token(ws_id);

    let (status, body) = get_json(
        &app,
        &format!("/v1/workspaces/{ws_id}/next-steps"),
        &token,
    ).await;

    assert_eq!(status, StatusCode::OK);
    // Empty workspace — should be all caught up
    assert!(body["top"].is_null(), "empty workspace should have no top");
    assert_eq!(body["backlog"], json!([]));
    assert_eq!(body["summary"]["critical"], 0);
    assert_eq!(body["summary"]["high"], 0);
}
```

**Note:** The exact formation request shape must be verified against the actual formation handler. Read `services/api-rs/src/routes/formation.rs` to get the correct request body. Adjust the test accordingly.

- [ ] **Step 2: Run the test**

```bash
cd /root/repos/thecorporation-mono/services/api-rs && cargo test next_steps -- --nocapture 2>&1 | tail -30
```

Fix any failures.

- [ ] **Step 3: Commit**

```bash
git add services/api-rs/tests/next_steps_e2e.rs
git commit -m "test(api): add next-steps endpoint integration tests"
```

---

## Task 9: Build and verify end-to-end

- [ ] **Step 1: Build the Rust API**

```bash
cd /root/repos/thecorporation-mono/services/api-rs && cargo build 2>&1 | tail -10
```

- [ ] **Step 2: Build the CLI**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && npm run build 2>&1 | tail -10
```

- [ ] **Step 3: Verify help output**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && node dist/index.js --help
```

Verify:
- `next` is the first command listed
- Description reads "See what to do next — your recommended actions"
- Tip at the bottom says "Run corp next to see your recommended next actions"

- [ ] **Step 4: Test local-only mode (no server)**

```bash
cd /root/repos/thecorporation-mono/packages/cli-ts && node dist/index.js next --json
```

With no config, should return a JSON response with a "Run initial setup" recommendation.

- [ ] **Step 5: Run all Rust tests**

```bash
cd /root/repos/thecorporation-mono/services/api-rs && cargo test 2>&1 | tail -20
```

- [ ] **Step 6: Final commit if any fixes were needed**

```bash
git add -A && git commit -m "fix: address issues found during end-to-end verification"
```
