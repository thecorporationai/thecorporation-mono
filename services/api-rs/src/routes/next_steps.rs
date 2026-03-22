use serde::Serialize;
use utoipa::ToSchema;
use axum::{Json, Router, extract::{Path, State}};
use axum::routing::get;
use chrono::Utc;

use super::AppState;
use crate::auth::RequireExecutionRead;
use crate::error::AppError;
use crate::domain::ids::EntityId;
use crate::store::entity_store::EntityStore;
use crate::domain::formation::content::MemberInput;
use crate::domain::formation::types::FormationStatus;
use crate::domain::governance::body::GovernanceBody;
use crate::domain::governance::seat::GovernanceSeat;
use crate::domain::governance::types::{SeatStatus, MeetingStatus};
use crate::domain::governance::meeting::Meeting;
use crate::domain::equity::instrument::Instrument;
use crate::domain::execution::obligation::Obligation;
use crate::domain::execution::types::ObligationStatus;
use crate::domain::treasury::bank_account::BankAccount;

// ── Response types ─────────────────────────────────────────────────

/// A single recommended action.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NextStepItem {
    pub category: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub command: String,
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
    pub top: Option<NextStepItem>,
    pub backlog: Vec<NextStepItem>,
    pub summary: NextStepsSummary,
}

impl NextStepsSummary {
    pub fn from_items(top: &Option<NextStepItem>, backlog: &[NextStepItem]) -> Self {
        let mut s = Self { critical: 0, high: 0, medium: 0, low: 0 };
        for item in top.iter().chain(backlog.iter()) {
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

// ── Recommendation engine ──────────────────────────────────────────

fn cmd(parts: &[&str]) -> String {
    let mut c = String::from("npx corp");
    for p in parts {
        c.push(' ');
        c.push_str(p);
    }
    c
}

/// Compute next-step recommendations for an entity.
/// Runs inside `spawn_blocking` — no async.
pub fn compute_next_steps(store: &EntityStore, entity_id: EntityId) -> Vec<NextStepItem> {
    let eid = entity_id.to_string();
    let mut items: Vec<NextStepItem> = Vec::new();

    // Determine if entity is fully active (used to gate post-formation recommendations).
    let is_active = store.read_entity("main")
        .map(|e| e.formation_status() == FormationStatus::Active)
        .unwrap_or(false);

    // 1. Formation status
    if let Ok(entity) = store.read_entity("main") {
        match entity.formation_status() {
            FormationStatus::Pending => {
                let has_members = store
                    .read_json::<Vec<MemberInput>>("main", "formation/pending_members.json")
                    .map(|m| !m.is_empty())
                    .unwrap_or(false);

                if has_members {
                    items.push(NextStepItem {
                        category: "formation".into(),
                        title: format!("Finalize formation for {}", entity.legal_name()),
                        description: Some("Formation is pending — finalize to generate documents and cap table".into()),
                        command: cmd(&["form", "finalize", &eid]),
                        urgency: "critical".into(),
                    });
                } else {
                    items.push(NextStepItem {
                        category: "formation".into(),
                        title: format!("Add a founder to {}", entity.legal_name()),
                        description: Some("At least one founder is required before formation can be finalized".into()),
                        command: cmd(&["form", "add-founder", &eid, "--name", "\"...\"", "--email", "\"...\"", "--role", "member", "--pct", "100"]),
                        urgency: "critical".into(),
                    });
                }
            }
            FormationStatus::DocumentsGenerated => {
                // Unsigned documents are surfaced by section 2 below.
                // Add a formation-level hint so "next" always has a clear action.
                items.push(NextStepItem {
                    category: "formation".into(),
                    title: format!("Sign formation documents for {}", entity.legal_name()),
                    description: Some("Documents have been generated — sign them to advance formation".into()),
                    command: cmd(&["form", "activate", &eid]),
                    urgency: "critical".into(),
                });
            }
            FormationStatus::DocumentsSigned => {
                items.push(NextStepItem {
                    category: "formation".into(),
                    title: format!("Submit state filing for {}", entity.legal_name()),
                    description: Some("Documents are signed — submit filing to the state".into()),
                    command: cmd(&["form", "activate", &eid]),
                    urgency: "critical".into(),
                });
            }
            FormationStatus::FilingSubmitted => {
                items.push(NextStepItem {
                    category: "formation".into(),
                    title: format!("Confirm state filing for {}", entity.legal_name()),
                    description: Some("Filing has been submitted — confirm once the state accepts it".into()),
                    command: cmd(&["form", "activate", &eid, "--filing-id", "\"...\"" ]),
                    urgency: "critical".into(),
                });
            }
            FormationStatus::Filed => {
                items.push(NextStepItem {
                    category: "formation".into(),
                    title: format!("Apply for EIN for {}", entity.legal_name()),
                    description: Some("State filing is complete — apply for an EIN with the IRS".into()),
                    command: cmd(&["form", "activate", &eid]),
                    urgency: "critical".into(),
                });
            }
            FormationStatus::EinApplied => {
                items.push(NextStepItem {
                    category: "formation".into(),
                    title: format!("Confirm EIN for {}", entity.legal_name()),
                    description: Some("EIN application submitted — confirm once received to activate the entity".into()),
                    command: cmd(&["form", "activate", &eid, "--ein", "\"...\"" ]),
                    urgency: "critical".into(),
                });
            }
            FormationStatus::Active
            | FormationStatus::Rejected
            | FormationStatus::Dissolved => {}
        }
    }

    // 2. Unsigned documents (always shown regardless of formation state)
    if let Ok(doc_ids) = store.list_document_ids("main") {
        for doc_id in doc_ids {
            if let Ok(doc) = store.read_document("main", doc_id) {
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

    // 3. No governance bodies (only after entity is active)
    let body_ids = store.list_ids::<GovernanceBody>("main").unwrap_or_default();
    if body_ids.is_empty() {
        if is_active {
            items.push(NextStepItem {
                category: "governance".into(),
                title: "Create board of directors".into(),
                description: Some("No governance bodies exist yet".into()),
                command: cmd(&["governance", "--entity-id", &eid, "create-body", "--name", "\"Board of Directors\"", "--body-type", "board_of_directors"]),
                urgency: "medium".into(),
            });
        }
    } else {
        // 4. Unfilled seats
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

    // 5. No equity instruments (only after entity is active)
    if let Ok(instrument_ids) = store.list_ids::<Instrument>("main") {
        if instrument_ids.is_empty() && is_active {
            items.push(NextStepItem {
                category: "cap_table".into(),
                title: "Set up cap table — create share classes".into(),
                description: Some("No equity instruments exist yet".into()),
                command: cmd(&["cap-table", "--entity-id", &eid, "create-instrument"]),
                urgency: "medium".into(),
            });
        }
    }

    // 6 & 7. Obligations
    if let Ok(obl_ids) = store.list_ids::<Obligation>("main") {
        let today = Utc::now().date_naive();
        for obl_id in obl_ids {
            if let Ok(obl) = store.read::<Obligation>("main", obl_id) {
                if matches!(obl.status(), ObligationStatus::Required | ObligationStatus::InProgress) {
                    let urgency = match obl.due_date() {
                        Some(due) => {
                            let days = (due - today).num_days();
                            if days <= 0 { "critical" }
                            else if days <= 7 { "high" }
                            else if days <= 30 { "medium" }
                            else { "low" }
                        }
                        None => "low",
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

    // 8. No bank account (only after entity is active)
    if let Ok(acct_ids) = store.list_ids::<BankAccount>("main") {
        if acct_ids.is_empty() && is_active {
            items.push(NextStepItem {
                category: "finance".into(),
                title: "Open a bank account".into(),
                description: None,
                command: cmd(&["finance", "--entity-id", &eid, "open-account"]),
                urgency: "low".into(),
            });
        }
    }

    // 9 & 10. Meetings
    if let Ok(meeting_ids) = store.list_ids::<Meeting>("main") {
        for mid in meeting_ids {
            if let Ok(mtg) = store.read::<Meeting>("main", mid) {
                match mtg.status() {
                    MeetingStatus::Convened => {
                        items.push(NextStepItem {
                            category: "governance".into(),
                            title: "Meeting in session — cast your vote".into(),
                            description: Some(format!("Meeting {} is convened and awaiting votes", mid)),
                            command: cmd(&["governance", "--entity-id", &eid, "vote", "--meeting-id", &mid.to_string()]),
                            urgency: "high".into(),
                        });
                    }
                    MeetingStatus::Noticed => {
                        items.push(NextStepItem {
                            category: "governance".into(),
                            title: "Upcoming meeting — convene when ready".into(),
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

// ── Sort + build response helper ───────────────────────────────────

fn build_response(mut items: Vec<NextStepItem>) -> NextStepsResponse {
    if items.is_empty() {
        return NextStepsResponse {
            top: None,
            backlog: vec![],
            summary: NextStepsSummary { critical: 0, high: 0, medium: 0, low: 0 },
        };
    }
    items.sort_by_key(|i| match i.urgency.as_str() {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        _ => 3,
    });
    let top = items.remove(0);
    let summary = NextStepsSummary::from_items(&Some(top.clone()), &items);
    NextStepsResponse { top: Some(top), backlog: items, summary }
}

// ── Handlers ───────────────────────────────────────────────────────

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
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let workspace_id = auth.workspace_id();
    let response = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        if let Some(ref scope) = entity_scope {
            if !scope.contains(&entity_id) {
                return Err(AppError::Forbidden("entity not in scope".into()));
            }
        }
        let store = EntityStore::open(
            layout, workspace_id, entity_id, valkey, s3,
        ).map_err(|e| AppError::NotFound(format!("entity not found: {e}")))?;
        Ok::<_, AppError>(build_response(compute_next_steps(&store, entity_id)))
    })
    .await?;
    Ok(Json(response))
}

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
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let response = super::shared::with_blocking_store(&state, move |layout, valkey, s3| {
        let entity_ids = layout.list_entity_ids(workspace_id);
        let mut all_items: Vec<NextStepItem> = Vec::new();
        for eid in entity_ids {
            if let Some(ref scope) = entity_scope {
                if !scope.contains(&eid) { continue; }
            }
            if let Ok(store) = EntityStore::open(
                layout, workspace_id, eid, valkey, s3
            ) {
                all_items.extend(compute_next_steps(&store, eid));
            }
        }
        Ok::<_, AppError>(build_response(all_items))
    })
    .await?;
    Ok(Json(response))
}

// ── Router & OpenAPI ───────────────────────────────────────────────

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
