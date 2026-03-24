//! Next-step recommendations: context-aware suggestions for what to do next.
//!
//! Scans entity state (formation status, unsigned documents, governance gaps,
//! pending obligations, missing equity setup, etc.) and returns a prioritised
//! list of actions with CLI commands.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use serde::Serialize;

use corp_auth::RequireFormationRead;
use corp_core::equity::{Instrument, VestingSchedule};
use corp_core::execution::{Obligation, ObligationStatus};
use corp_core::formation::{Document, DocumentStatus, Entity, FormationStatus};
use corp_core::governance::{GovernanceBody, GovernanceSeat, Meeting, MeetingStatus, SeatStatus};
use corp_core::ids::EntityId;
use corp_core::treasury::BankAccount;

use crate::error::AppError;
use crate::state::AppState;

// ── Response types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct NextStepItem {
    pub category: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub command: String,
    pub urgency: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NextStepsSummary {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct NextStepsResponse {
    pub top: Option<NextStepItem>,
    pub backlog: Vec<NextStepItem>,
    pub summary: NextStepsSummary,
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new().route("/entities/{entity_id}/next-steps", get(entity_next_steps))
}

// ── Handler ─────────────────────────────────────────────────────────────────

async fn entity_next_steps(
    RequireFormationRead(principal): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<NextStepsResponse>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;

    let eid = entity_id.to_string();
    let mut items: Vec<NextStepItem> = Vec::new();

    // Is the entity fully active?
    let entity: Entity = store
        .read(entity_id, "main")
        .await
        .map_err(|e| AppError::NotFound(format!("entity: {e}")))?;
    let is_active = entity.formation_status == FormationStatus::Active;

    // ── 1. Formation status ─────────────────────────────────────────────────

    match entity.formation_status {
        FormationStatus::Pending => {
            items.push(step(
                "formation",
                "critical",
                &format!("Advance formation for {}", entity.legal_name),
                Some("Formation is pending — advance to generate documents"),
                &format!("corp form advance {eid}"),
            ));
        }
        FormationStatus::DocumentsGenerated => {
            items.push(step(
                "formation",
                "critical",
                &format!("Sign formation documents for {}", entity.legal_name),
                Some("Documents generated — sign them to continue"),
                &format!("corp form documents {eid}"),
            ));
        }
        FormationStatus::DocumentsSigned => {
            items.push(step(
                "formation",
                "critical",
                &format!("Advance to filing for {}", entity.legal_name),
                Some("Documents signed — advance to submit state filing"),
                &format!("corp form advance {eid}"),
            ));
        }
        FormationStatus::FilingSubmitted => {
            items.push(step(
                "formation",
                "critical",
                &format!("Confirm state filing for {}", entity.legal_name),
                Some("Filing submitted — confirm once the state accepts it"),
                &format!("corp form confirm-filing {eid}"),
            ));
        }
        FormationStatus::Filed => {
            items.push(step(
                "formation",
                "critical",
                &format!("Apply for EIN for {}", entity.legal_name),
                Some("State filing complete — advance to apply for EIN"),
                &format!("corp form advance {eid}"),
            ));
        }
        FormationStatus::EinApplied => {
            items.push(step(
                "formation",
                "critical",
                &format!("Confirm EIN for {}", entity.legal_name),
                Some("EIN application submitted — confirm once received"),
                &format!("corp form confirm-ein {eid} --ein \"XX-XXXXXXX\""),
            ));
        }
        FormationStatus::Active | FormationStatus::Rejected | FormationStatus::Dissolved => {}
    }

    // ── 2. Unsigned documents ───────────────────────────────────────────────

    let docs: Vec<Document> = store.read_all("main").await.unwrap_or_default();
    for doc in &docs {
        if doc.status == DocumentStatus::Draft {
            items.push(step(
                "documents",
                "high",
                &format!("Sign {}", doc.title),
                None,
                &format!("corp form sign {}", doc.document_id),
            ));
        }
    }

    // ── 3. No governance bodies ─────────────────────────────────────────────

    let bodies: Vec<GovernanceBody> = store.read_all("main").await.unwrap_or_default();
    if bodies.is_empty() && is_active {
        items.push(step(
            "governance", "medium",
            "Create board of directors",
            Some("No governance bodies exist yet"),
            "corp governance create-body --name \"Board of Directors\" --body-type board_of_directors",
        ));
    }

    // ── 4. Unfilled seats ───────────────────────────────────────────────────

    if !bodies.is_empty() {
        let seats: Vec<GovernanceSeat> = store.read_all("main").await.unwrap_or_default();
        let unfilled = seats
            .iter()
            .filter(|s| matches!(s.status, SeatStatus::Resigned | SeatStatus::Expired))
            .count();
        if unfilled > 0 {
            items.push(step(
                "governance",
                "medium",
                &format!(
                    "{unfilled} unfilled governance seat{}",
                    if unfilled == 1 { "" } else { "s" }
                ),
                Some("Appoint members to fill resigned or expired seats"),
                "corp governance seats",
            ));
        }
    }

    // ── 5. No equity instruments ────────────────────────────────────────────

    let instruments: Vec<Instrument> = store.read_all("main").await.unwrap_or_default();
    if instruments.is_empty() && is_active {
        items.push(step(
            "cap_table", "medium",
            "Set up cap table — create share classes and instruments",
            Some("No equity instruments exist yet"),
            "corp cap-table init && corp cap-table create-instrument --symbol CS-A --kind common_equity",
        ));
    }

    // ── 6. Pending obligations ──────────────────────────────────────────────

    let obligations: Vec<Obligation> = store.read_all("main").await.unwrap_or_default();
    let today = Utc::now().date_naive();
    for obl in &obligations {
        if matches!(
            obl.status,
            ObligationStatus::Required | ObligationStatus::InProgress
        ) {
            let urgency = match obl.due_date {
                Some(due) => {
                    let days = (due - today).num_days();
                    if days <= 0 {
                        "critical"
                    } else if days <= 7 {
                        "high"
                    } else if days <= 30 {
                        "medium"
                    } else {
                        "low"
                    }
                }
                None => "low",
            };
            items.push(step(
                "compliance",
                urgency,
                &obl.description,
                obl.due_date.map(|d| format!("Due {d}")).as_deref(),
                "corp execution obligations",
            ));
        }
    }

    // ── 7. No bank account ──────────────────────────────────────────────────

    let bank_accounts: Vec<BankAccount> = store.read_all("main").await.unwrap_or_default();
    if bank_accounts.is_empty() && is_active {
        items.push(step(
            "finance",
            "low",
            "Open a bank account",
            None,
            "corp finance open-account",
        ));
    }

    // ── 8. Meetings needing action ──────────────────────────────────────────

    let meetings: Vec<Meeting> = store.read_all("main").await.unwrap_or_default();
    for mtg in &meetings {
        match mtg.status {
            MeetingStatus::Convened => {
                items.push(step(
                    "governance",
                    "high",
                    "Meeting in session — cast your vote",
                    Some(&format!("Meeting {} is convened", mtg.meeting_id)),
                    &format!("corp governance vote {}", mtg.meeting_id),
                ));
            }
            MeetingStatus::Noticed => {
                items.push(step(
                    "governance",
                    "medium",
                    "Upcoming meeting — convene when ready",
                    Some(&format!("Meeting {} has been noticed", mtg.meeting_id)),
                    &format!("corp governance convene {}", mtg.meeting_id),
                ));
            }
            _ => {}
        }
    }

    // ── 9. Unvested schedules with no materialized events ───────────────────

    let schedules: Vec<VestingSchedule> = store.read_all("main").await.unwrap_or_default();
    if !schedules.is_empty() {
        // If there are schedules but no vesting events, suggest materializing
        let events: Vec<corp_core::equity::VestingEvent> =
            store.read_all("main").await.unwrap_or_default();
        if events.is_empty() {
            items.push(step(
                "cap_table",
                "medium",
                "Materialize vesting events for active schedules",
                Some("Vesting schedules exist but no events have been generated"),
                "corp cap-table materialize-vesting --schedule-id @last",
            ));
        }
    }

    // ── Build response ──────────────────────────────────────────────────────

    Ok(Json(build_response(items)))
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn step(
    category: &str,
    urgency: &str,
    title: &str,
    description: Option<&str>,
    command: &str,
) -> NextStepItem {
    NextStepItem {
        category: category.into(),
        title: title.into(),
        description: description.map(Into::into),
        command: command.into(),
        urgency: urgency.into(),
    }
}

fn build_response(mut items: Vec<NextStepItem>) -> NextStepsResponse {
    if items.is_empty() {
        return NextStepsResponse {
            top: None,
            backlog: vec![],
            summary: NextStepsSummary {
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
            },
        };
    }
    items.sort_by_key(|i| match i.urgency.as_str() {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        _ => 3,
    });
    let top = items.remove(0);
    let summary = {
        let mut s = NextStepsSummary {
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
        };
        for item in std::iter::once(&top).chain(items.iter()) {
            match item.urgency.as_str() {
                "critical" => s.critical += 1,
                "high" => s.high += 1,
                "medium" => s.medium += 1,
                _ => s.low += 1,
            }
        }
        s
    };
    NextStepsResponse {
        top: Some(top),
        backlog: items,
        summary,
    }
}
