//! Governance API route handlers.
//!
//! Implements the full governance API surface: bodies, seats, meetings,
//! agenda items, votes, resolutions, the entity governance profile, and two
//! shortcut endpoints for common written-consent flows.
//!
//! ## Route map
//!
//! | Method | Path | Scope |
//! |--------|------|-------|
//! | GET    | `/entities/{entity_id}/governance/bodies` | `GovernanceRead` |
//! | POST   | `/entities/{entity_id}/governance/bodies` | `GovernanceWrite` |
//! | GET    | `/entities/{entity_id}/governance/bodies/{body_id}` | `GovernanceRead` |
//! | POST   | `/entities/{entity_id}/governance/bodies/{body_id}/deactivate` | `GovernanceWrite` |
//! | GET    | `/entities/{entity_id}/governance/seats` | `GovernanceRead` |
//! | POST   | `/entities/{entity_id}/governance/seats` | `GovernanceWrite` |
//! | GET    | `/entities/{entity_id}/governance/seats/{seat_id}` | `GovernanceRead` |
//! | POST   | `/entities/{entity_id}/governance/seats/{seat_id}/resign` | `GovernanceWrite` |
//! | GET    | `/entities/{entity_id}/governance/meetings` | `GovernanceRead` |
//! | POST   | `/entities/{entity_id}/governance/meetings` | `GovernanceWrite` |
//! | GET    | `/entities/{entity_id}/governance/meetings/{meeting_id}` | `GovernanceRead` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/notice` | `GovernanceWrite` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/convene` | `GovernanceWrite` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/adjourn` | `GovernanceWrite` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/cancel` | `GovernanceWrite` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/reopen` | `GovernanceWrite` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/attendance` | `GovernanceWrite` |
//! | GET    | `/entities/{entity_id}/governance/meetings/{meeting_id}/items` | `GovernanceRead` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/items` | `GovernanceWrite` |
//! | GET    | `/entities/{entity_id}/governance/meetings/{meeting_id}/votes` | `GovernanceRead` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/votes` | `GovernanceVote` |
//! | POST   | `/entities/{entity_id}/governance/meetings/{meeting_id}/items/{item_id}/resolve` | `GovernanceWrite` |
//! | GET    | `/entities/{entity_id}/governance/meetings/{meeting_id}/resolutions` | `GovernanceRead` |
//! | GET    | `/entities/{entity_id}/governance/profile` | `GovernanceRead` |
//! | PUT    | `/entities/{entity_id}/governance/profile` | `GovernanceWrite` |
//! | POST   | `/entities/{entity_id}/governance/written-consent` | `GovernanceWrite` |
//! | POST   | `/entities/{entity_id}/governance/quick-approve` | `GovernanceWrite` |

use std::collections::HashSet;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use corp_auth::{RequireGovernanceRead, RequireGovernanceVote, RequireGovernanceWrite};
use corp_core::governance::{
    AgendaItem, GovernanceBody, GovernanceProfile, GovernanceSeat, Meeting, Resolution, Vote,
};
use corp_core::governance::types::{
    AgendaItemType, BodyStatus, BodyType, MeetingStatus, MeetingType, QuorumThreshold,
    ResolutionType, SeatRole, VoteValue, VotingMethod, VotingPower,
};
use corp_core::ids::{
    AgendaItemId, ContactId, EntityId, GovernanceBodyId, GovernanceSeatId, MeetingId,
    ResolutionId, VoteId,
};
use corp_storage::entity_store::EntityStore;

use crate::error::AppError;
use crate::state::AppState;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Git branch used for all governance data.
const BRANCH: &str = "main";

/// Store path for the governance profile singleton.
const PROFILE_PATH: &str = "governance/profile.json";

// ── Nested-path helpers ───────────────────────────────────────────────────────

fn agenda_item_path(meeting_id: MeetingId, item_id: AgendaItemId) -> String {
    format!("governance/meetings/{}/agenda/{}.json", meeting_id, item_id)
}

fn agenda_index_path(meeting_id: MeetingId) -> String {
    format!("governance/meetings/{}/agenda/.index.json", meeting_id)
}

fn vote_path(meeting_id: MeetingId, vote_id: VoteId) -> String {
    format!("governance/meetings/{}/votes/{}.json", meeting_id, vote_id)
}

fn vote_index_path(meeting_id: MeetingId) -> String {
    format!("governance/meetings/{}/votes/.index.json", meeting_id)
}

fn resolution_path(meeting_id: MeetingId, resolution_id: ResolutionId) -> String {
    format!(
        "governance/meetings/{}/resolutions/{}.json",
        meeting_id, resolution_id
    )
}

fn resolution_index_path(meeting_id: MeetingId) -> String {
    format!("governance/meetings/{}/resolutions/.index.json", meeting_id)
}

// ── Index helpers ─────────────────────────────────────────────────────────────

/// Read the ID-string index at `index_path`, or return an empty vec.
async fn read_index(store: &EntityStore, index_path: &str) -> Result<Vec<String>, AppError> {
    match store.read_json::<Vec<String>>(index_path, BRANCH).await {
        Ok(ids) => Ok(ids),
        Err(_) => Ok(Vec::new()),
    }
}

/// Append `id_str` to the index file at `index_path` if not already present.
async fn append_index(
    store: &EntityStore,
    index_path: &str,
    id_str: &str,
    commit_msg: &str,
) -> Result<(), AppError> {
    let mut ids = read_index(store, index_path).await?;
    if !ids.contains(&id_str.to_owned()) {
        ids.push(id_str.to_owned());
        store
            .write_json(index_path, &ids, BRANCH, commit_msg)
            .await
            .map_err(AppError::Storage)?;
    }
    Ok(())
}

// ── Route registration ────────────────────────────────────────────────────────

/// Build and return the governance sub-router.
pub fn routes() -> Router<AppState> {
    Router::new()
        // Bodies
        .route(
            "/entities/{entity_id}/governance/bodies",
            get(list_bodies).post(create_body),
        )
        .route(
            "/entities/{entity_id}/governance/bodies/{body_id}",
            get(get_body),
        )
        .route(
            "/entities/{entity_id}/governance/bodies/{body_id}/deactivate",
            post(deactivate_body),
        )
        // Seats
        .route(
            "/entities/{entity_id}/governance/seats",
            get(list_seats).post(create_seat),
        )
        .route(
            "/entities/{entity_id}/governance/seats/{seat_id}",
            get(get_seat),
        )
        .route(
            "/entities/{entity_id}/governance/seats/{seat_id}/resign",
            post(resign_seat),
        )
        // Meetings
        .route(
            "/entities/{entity_id}/governance/meetings",
            get(list_meetings).post(create_meeting),
        )
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}",
            get(get_meeting),
        )
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/notice",
            post(send_notice),
        )
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/convene",
            post(convene_meeting),
        )
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/adjourn",
            post(adjourn_meeting),
        )
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/cancel",
            post(cancel_meeting),
        )
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/reopen",
            post(reopen_meeting),
        )
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/attendance",
            post(record_attendance),
        )
        // Agenda items
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/items",
            get(list_agenda_items).post(create_agenda_item),
        )
        // Votes
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/votes",
            get(list_votes).post(cast_vote),
        )
        // Resolutions
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/items/{item_id}/resolve",
            post(resolve_item),
        )
        .route(
            "/entities/{entity_id}/governance/meetings/{meeting_id}/resolutions",
            get(list_resolutions),
        )
        // Profile
        .route(
            "/entities/{entity_id}/governance/profile",
            get(get_profile).put(update_profile),
        )
        // Shortcuts
        .route(
            "/entities/{entity_id}/governance/written-consent",
            post(create_written_consent),
        )
        .route(
            "/entities/{entity_id}/governance/quick-approve",
            post(quick_approve),
        )
}

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateBodyRequest {
    pub name: String,
    pub body_type: BodyType,
    pub quorum_rule: QuorumThreshold,
    pub voting_method: VotingMethod,
}

#[derive(Debug, Deserialize)]
pub struct CreateSeatRequest {
    pub body_id: GovernanceBodyId,
    pub holder_id: ContactId,
    pub role: SeatRole,
    pub appointed_date: NaiveDate,
    pub term_expiration: Option<NaiveDate>,
    /// Voting weight assigned to this seat. Must be greater than zero.
    pub voting_power: u32,
}

#[derive(Debug, Deserialize)]
pub struct CreateMeetingRequest {
    pub body_id: GovernanceBodyId,
    pub meeting_type: MeetingType,
    pub title: String,
    pub scheduled_date: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub notice_days: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct RecordAttendanceRequest {
    pub seat_ids: Vec<GovernanceSeatId>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgendaItemRequest {
    pub title: String,
    pub item_type: AgendaItemType,
    pub description: Option<String>,
    pub resolution_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CastVoteRequest {
    pub agenda_item_id: AgendaItemId,
    pub seat_id: GovernanceSeatId,
    pub value: VoteValue,
}

#[derive(Debug, Deserialize)]
pub struct ResolveItemRequest {
    pub resolution_type: ResolutionType,
    pub resolution_text: String,
}

/// Full-replacement upsert for the governance profile.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProfileRequest {
    pub entity_type: String,
    pub legal_name: String,
    pub jurisdiction: String,
    pub effective_date: NaiveDate,
    pub registered_agent_name: Option<String>,
    pub registered_agent_address: Option<String>,
    pub board_size: Option<u32>,
    pub principal_name: Option<String>,
    pub company_address: Option<corp_core::governance::CompanyAddress>,
    pub founders: Vec<corp_core::governance::FounderInfo>,
    pub directors: Vec<corp_core::governance::DirectorInfo>,
    pub officers: Vec<corp_core::governance::OfficerInfo>,
    pub stock_details: Option<corp_core::governance::StockDetails>,
    pub fiscal_year_end: Option<corp_core::governance::FiscalYearEnd>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWrittenConsentRequest {
    pub body_id: GovernanceBodyId,
    pub title: String,
    pub resolution_text: String,
}

#[derive(Debug, Serialize)]
pub struct WrittenConsentResponse {
    pub meeting: Meeting,
    pub agenda_item: AgendaItem,
}

#[derive(Debug, Deserialize)]
pub struct QuickApproveRequest {
    pub body_id: GovernanceBodyId,
    pub title: String,
    pub resolution_text: String,
}

#[derive(Debug, Serialize)]
pub struct QuickApproveResponse {
    pub meeting_id: MeetingId,
    pub agenda_item_id: AgendaItemId,
    pub vote_ids: Vec<VoteId>,
    pub resolution_id: ResolutionId,
}

// ── Body handlers ─────────────────────────────────────────────────────────────

/// `GET /entities/{entity_id}/governance/bodies`
async fn list_bodies(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceBody>>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;
    let bodies = store
        .read_all::<GovernanceBody>(BRANCH)
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(bodies))
}

/// `POST /entities/{entity_id}/governance/bodies`
async fn create_body(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<CreateBodyRequest>,
) -> Result<Json<GovernanceBody>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let body = GovernanceBody::new(
        entity_id,
        req.body_type,
        req.name,
        req.quorum_rule,
        req.voting_method,
    )
    .map_err(|e| AppError::BadRequest(e.to_string()))?;

    store
        .write::<GovernanceBody>(
            &body,
            body.body_id,
            BRANCH,
            &format!("create governance body {}", body.body_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(body))
}

/// `GET /entities/{entity_id}/governance/bodies/{body_id}`
async fn get_body(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((entity_id, body_id)): Path<(EntityId, GovernanceBodyId)>,
) -> Result<Json<GovernanceBody>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;
    let body = store
        .read::<GovernanceBody>(body_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(body))
}

/// `POST /entities/{entity_id}/governance/bodies/{body_id}/deactivate`
async fn deactivate_body(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, body_id)): Path<(EntityId, GovernanceBodyId)>,
) -> Result<Json<GovernanceBody>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let mut body = store
        .read::<GovernanceBody>(body_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    body.deactivate();

    store
        .write::<GovernanceBody>(
            &body,
            body.body_id,
            BRANCH,
            &format!("deactivate governance body {}", body_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(body))
}

// ── Seat handlers ─────────────────────────────────────────────────────────────

/// `GET /entities/{entity_id}/governance/seats`
async fn list_seats(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceSeat>>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;
    let seats = store
        .read_all::<GovernanceSeat>(BRANCH)
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(seats))
}

/// `POST /entities/{entity_id}/governance/seats`
async fn create_seat(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<CreateSeatRequest>,
) -> Result<Json<GovernanceSeat>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let voting_power = VotingPower::new(req.voting_power)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let seat = GovernanceSeat::new(
        req.body_id,
        req.holder_id,
        req.role,
        req.appointed_date,
        req.term_expiration,
        voting_power,
    );

    store
        .write::<GovernanceSeat>(
            &seat,
            seat.seat_id,
            BRANCH,
            &format!("create governance seat {}", seat.seat_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(seat))
}

/// `GET /entities/{entity_id}/governance/seats/{seat_id}`
async fn get_seat(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((entity_id, seat_id)): Path<(EntityId, GovernanceSeatId)>,
) -> Result<Json<GovernanceSeat>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;
    let seat = store
        .read::<GovernanceSeat>(seat_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(seat))
}

/// `POST /entities/{entity_id}/governance/seats/{seat_id}/resign`
async fn resign_seat(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, seat_id)): Path<(EntityId, GovernanceSeatId)>,
) -> Result<Json<GovernanceSeat>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let mut seat = store
        .read::<GovernanceSeat>(seat_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    seat.resign()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    store
        .write::<GovernanceSeat>(
            &seat,
            seat.seat_id,
            BRANCH,
            &format!("resign governance seat {}", seat_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(seat))
}

// ── Meeting handlers ──────────────────────────────────────────────────────────

/// `GET /entities/{entity_id}/governance/meetings`
async fn list_meetings(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Meeting>>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;
    let meetings = store
        .read_all::<Meeting>(BRANCH)
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(meetings))
}

/// `POST /entities/{entity_id}/governance/meetings`
async fn create_meeting(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<CreateMeetingRequest>,
) -> Result<Json<Meeting>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    // Bug 3: Reject if the body has been deactivated.
    let body = store
        .read::<GovernanceBody>(req.body_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;
    if body.status == BodyStatus::Inactive {
        return Err(AppError::BadRequest(format!(
            "body {} is inactive; cannot create new meetings",
            req.body_id
        )));
    }

    let meeting = Meeting::new(
        req.body_id,
        req.meeting_type,
        req.title,
        req.scheduled_date,
        req.location,
        req.notice_days,
    );

    store
        .write::<Meeting>(
            &meeting,
            meeting.meeting_id,
            BRANCH,
            &format!("create meeting {}", meeting.meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(meeting))
}

/// `GET /entities/{entity_id}/governance/meetings/{meeting_id}`
async fn get_meeting(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Meeting>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;
    let meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(meeting))
}

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/notice`
///
/// Transitions the meeting `Draft` → `Noticed`.
async fn send_notice(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Meeting>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let mut meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    meeting
        .send_notice()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    store
        .write::<Meeting>(
            &meeting,
            meeting_id,
            BRANCH,
            &format!("send notice for meeting {}", meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(meeting))
}

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/convene`
///
/// Transitions the meeting `Draft | Noticed` → `Convened`.
async fn convene_meeting(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Meeting>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let mut meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    meeting
        .convene()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    store
        .write::<Meeting>(
            &meeting,
            meeting_id,
            BRANCH,
            &format!("convene meeting {}", meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(meeting))
}

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/adjourn`
///
/// Transitions the meeting `Convened` → `Adjourned`.
async fn adjourn_meeting(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Meeting>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let mut meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    meeting
        .adjourn()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    store
        .write::<Meeting>(
            &meeting,
            meeting_id,
            BRANCH,
            &format!("adjourn meeting {}", meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(meeting))
}

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/cancel`
///
/// Transitions the meeting `Draft | Noticed` → `Cancelled`.
async fn cancel_meeting(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Meeting>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let mut meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    meeting
        .cancel()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    store
        .write::<Meeting>(
            &meeting,
            meeting_id,
            BRANCH,
            &format!("cancel meeting {}", meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(meeting))
}

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/reopen`
///
/// Transitions the meeting `Adjourned` → `Convened`.
async fn reopen_meeting(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Meeting>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let mut meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    meeting
        .reopen()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    store
        .write::<Meeting>(
            &meeting,
            meeting_id,
            BRANCH,
            &format!("reopen meeting {}", meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(meeting))
}

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/attendance`
///
/// Records which seats were present and evaluates quorum. Reads the body to
/// obtain the quorum rule and the voting method (per-capita vs per-unit).
async fn record_attendance(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
    Json(req): Json<RecordAttendanceRequest>,
) -> Result<Json<Meeting>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let mut meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    if meeting.status != MeetingStatus::Convened && meeting.status != MeetingStatus::Noticed {
        return Err(AppError::BadRequest(
            "meeting must be convened or noticed to record attendance".into(),
        ));
    }

    let body = store
        .read::<GovernanceBody>(meeting.body_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    // Collect all eligible (active, non-observer) seats for this body.
    let all_seats = store
        .read_all::<GovernanceSeat>(BRANCH)
        .await
        .map_err(AppError::Storage)?;

    let eligible: Vec<&GovernanceSeat> = all_seats
        .iter()
        .filter(|s| s.body_id == meeting.body_id && s.can_vote())
        .collect();

    let present_set: HashSet<GovernanceSeatId> = req.seat_ids.iter().copied().collect();

    let (present_count, total_eligible) = match body.voting_method {
        VotingMethod::PerCapita => {
            let present = eligible
                .iter()
                .filter(|s| present_set.contains(&s.seat_id))
                .count() as u32;
            let total = eligible.len() as u32;
            (present, total)
        }
        VotingMethod::PerUnit => {
            let present: u32 = eligible
                .iter()
                .filter(|s| present_set.contains(&s.seat_id))
                .map(|s| s.voting_power.value())
                .sum();
            let total: u32 = eligible.iter().map(|s| s.voting_power.value()).sum();
            (present, total)
        }
    };

    meeting.record_attendance(req.seat_ids, present_count, total_eligible, body.quorum_rule);

    store
        .write::<Meeting>(
            &meeting,
            meeting_id,
            BRANCH,
            &format!("record attendance for meeting {}", meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(meeting))
}

// ── Agenda item handlers ──────────────────────────────────────────────────────

/// `GET /entities/{entity_id}/governance/meetings/{meeting_id}/items`
async fn list_agenda_items(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Vec<AgendaItem>>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    // Ensure the meeting exists.
    let _meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    let ids = read_index(&store, &agenda_index_path(meeting_id)).await?;
    let mut items = Vec::with_capacity(ids.len());
    for id_str in &ids {
        let path = format!("governance/meetings/{}/agenda/{}.json", meeting_id, id_str);
        match store.read_json::<AgendaItem>(&path, BRANCH).await {
            Ok(item) => items.push(item),
            Err(e) => {
                tracing::warn!(path = %path, error = %e, "skipping unreadable agenda item");
            }
        }
    }
    Ok(Json(items))
}

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/items`
async fn create_agenda_item(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
    Json(req): Json<CreateAgendaItemRequest>,
) -> Result<Json<AgendaItem>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    // Ensure the meeting exists and is in an acceptable state.
    let meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    if meeting.status == MeetingStatus::Cancelled || meeting.status == MeetingStatus::Adjourned {
        return Err(AppError::BadRequest(
            "cannot add items to a cancelled or adjourned meeting".into(),
        ));
    }

    // Bug 3: Reject if the body has been deactivated.
    let body = store
        .read::<GovernanceBody>(meeting.body_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;
    if body.status == BodyStatus::Inactive {
        return Err(AppError::BadRequest(format!(
            "body {} is inactive; cannot add agenda items",
            meeting.body_id
        )));
    }

    let item = AgendaItem::new(
        meeting_id,
        req.title,
        req.item_type,
        req.description,
        req.resolution_text,
    );

    let path = agenda_item_path(meeting_id, item.item_id);
    store
        .write_json(
            &path,
            &item,
            BRANCH,
            &format!("add agenda item {} to meeting {}", item.item_id, meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    append_index(
        &store,
        &agenda_index_path(meeting_id),
        &item.item_id.to_string(),
        &format!("index agenda item {}", item.item_id),
    )
    .await?;

    Ok(Json(item))
}

// ── Vote handlers ─────────────────────────────────────────────────────────────

/// `GET /entities/{entity_id}/governance/meetings/{meeting_id}/votes`
async fn list_votes(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Vec<Vote>>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    // Ensure the meeting exists.
    let _meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    let ids = read_index(&store, &vote_index_path(meeting_id)).await?;
    let mut votes = Vec::with_capacity(ids.len());
    for id_str in &ids {
        let path = format!(
            "governance/meetings/{}/votes/{}.json",
            meeting_id, id_str
        );
        match store.read_json::<Vote>(&path, BRANCH).await {
            Ok(vote) => votes.push(vote),
            Err(e) => {
                tracing::warn!(path = %path, error = %e, "skipping unreadable vote");
            }
        }
    }
    Ok(Json(votes))
}

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/votes`
///
/// Casts a vote. Requires the meeting to be `Convened` and quorum met (or
/// `WrittenConsent` meeting type). Also validates that the seat is eligible.
async fn cast_vote(
    RequireGovernanceVote(principal): RequireGovernanceVote,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
    Json(req): Json<CastVoteRequest>,
) -> Result<Json<Vote>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    if !meeting.can_vote() {
        return Err(AppError::BadRequest(
            "voting is not permitted: meeting is not convened or quorum has not been met".into(),
        ));
    }

    // Bug 3: Reject if the body has been deactivated.
    let body = store
        .read::<GovernanceBody>(meeting.body_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;
    if body.status == BodyStatus::Inactive {
        return Err(AppError::BadRequest(format!(
            "body {} is inactive; cannot cast votes",
            meeting.body_id
        )));
    }

    let seat = store
        .read::<GovernanceSeat>(req.seat_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    if !seat.can_vote() {
        return Err(AppError::BadRequest(format!(
            "seat {} is not eligible to vote (status: {:?}, role: {:?})",
            req.seat_id, seat.status, seat.role,
        )));
    }

    // Bug 2: Verify the seat belongs to the same body as the meeting.
    if seat.body_id != meeting.body_id {
        return Err(AppError::BadRequest(format!(
            "seat {} belongs to body {}, not meeting body {}",
            req.seat_id, seat.body_id, meeting.body_id
        )));
    }

    // Ensure the agenda item belongs to this meeting.
    let item_path = agenda_item_path(meeting_id, req.agenda_item_id);
    let _item: AgendaItem = store
        .read_json(&item_path, BRANCH)
        .await
        .map_err(|_| {
            AppError::NotFound(format!(
                "agenda item {} not found in meeting {}",
                req.agenda_item_id, meeting_id
            ))
        })?;

    // Bug 1: Check for duplicate vote (same seat on same agenda item).
    let vote_ids = read_index(&store, &vote_index_path(meeting_id)).await.unwrap_or_default();
    for vid_str in &vote_ids {
        let vpath = format!("governance/meetings/{}/votes/{}.json", meeting_id, vid_str);
        if let Ok(existing_vote) = store.read_json::<Vote>(&vpath, BRANCH).await {
            if existing_vote.seat_id == req.seat_id && existing_vote.agenda_item_id == req.agenda_item_id {
                return Err(AppError::BadRequest(format!(
                    "seat {} has already voted on agenda item {}",
                    req.seat_id, req.agenda_item_id
                )));
            }
        }
    }

    let vote = Vote::new(meeting_id, req.agenda_item_id, req.seat_id, req.value);

    let path = vote_path(meeting_id, vote.vote_id);
    store
        .write_json(
            &path,
            &vote,
            BRANCH,
            &format!("cast vote {} on item {}", vote.vote_id, req.agenda_item_id),
        )
        .await
        .map_err(AppError::Storage)?;

    append_index(
        &store,
        &vote_index_path(meeting_id),
        &vote.vote_id.to_string(),
        &format!("index vote {}", vote.vote_id),
    )
    .await?;

    Ok(Json(vote))
}

// ── Resolution handler ────────────────────────────────────────────────────────

/// `POST /entities/{entity_id}/governance/meetings/{meeting_id}/items/{item_id}/resolve`
///
/// Tallies all votes cast on `item_id`, computes whether the resolution
/// passed, persists the `Resolution`, and marks the `AgendaItem` as resolved.
async fn resolve_item(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((entity_id, meeting_id, item_id)): Path<(EntityId, MeetingId, AgendaItemId)>,
    Json(req): Json<ResolveItemRequest>,
) -> Result<Json<Resolution>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    if meeting.status != MeetingStatus::Convened {
        return Err(AppError::BadRequest(
            "meeting must be convened to resolve items".into(),
        ));
    }

    let body = store
        .read::<GovernanceBody>(meeting.body_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    // Tally votes for this specific agenda item.
    let vote_ids = read_index(&store, &vote_index_path(meeting_id)).await?;
    let mut votes_for = 0u32;
    let mut votes_against = 0u32;
    let mut votes_abstain = 0u32;

    for id_str in &vote_ids {
        let path = format!(
            "governance/meetings/{}/votes/{}.json",
            meeting_id, id_str
        );
        if let Ok(vote) = store.read_json::<Vote>(&path, BRANCH).await {
            if vote.agenda_item_id != item_id {
                continue;
            }
            match vote.value {
                VoteValue::For => votes_for += 1,
                VoteValue::Against => votes_against += 1,
                VoteValue::Abstain => votes_abstain += 1,
                VoteValue::Recusal => {} // recusals excluded from all tallies
            }
        }
    }

    let resolution = Resolution::new(
        meeting_id,
        item_id,
        req.resolution_type,
        req.resolution_text,
        votes_for,
        votes_against,
        votes_abstain,
        body.quorum_rule,
    );

    // Persist the resolution.
    let res_path = resolution_path(meeting_id, resolution.resolution_id);
    store
        .write_json(
            &res_path,
            &resolution,
            BRANCH,
            &format!("resolve agenda item {}", item_id),
        )
        .await
        .map_err(AppError::Storage)?;

    // NOTE: the index write and the resolution data write are two separate
    // commits; the current EntityStore API does not provide a multi-path
    // atomic write, so a crash between the two could leave the index stale.
    append_index(
        &store,
        &resolution_index_path(meeting_id),
        &resolution.resolution_id.to_string(),
        &format!("index resolution {}", resolution.resolution_id),
    )
    .await?;

    // Mark the agenda item resolved.
    let item_path = agenda_item_path(meeting_id, item_id);
    let mut item: AgendaItem = store
        .read_json(&item_path, BRANCH)
        .await
        .map_err(|_| {
            AppError::NotFound(format!(
                "agenda item {} not found in meeting {}",
                item_id, meeting_id
            ))
        })?;
    item.resolve();
    store
        .write_json(
            &item_path,
            &item,
            BRANCH,
            &format!("mark agenda item {} resolved", item_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(resolution))
}

/// `GET /entities/{entity_id}/governance/meetings/{meeting_id}/resolutions`
///
/// Returns all resolutions recorded for the given meeting, in the order they
/// were appended to the index.
async fn list_resolutions(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((entity_id, meeting_id)): Path<(EntityId, MeetingId)>,
) -> Result<Json<Vec<Resolution>>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    // Ensure the meeting exists.
    let _meeting = store
        .read::<Meeting>(meeting_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    let ids = read_index(&store, &resolution_index_path(meeting_id)).await?;
    let mut resolutions = Vec::with_capacity(ids.len());
    for id_str in &ids {
        let path = resolution_path(
            meeting_id,
            id_str.parse().map_err(|_| {
                AppError::Internal(format!("invalid resolution id in index: {}", id_str))
            })?,
        );
        match store.read_json::<Resolution>(&path, BRANCH).await {
            Ok(res) => resolutions.push(res),
            Err(e) => {
                tracing::warn!(path = %path, error = %e, "skipping unreadable resolution");
            }
        }
    }
    Ok(Json(resolutions))
}

// ── Profile handlers ──────────────────────────────────────────────────────────

/// `GET /entities/{entity_id}/governance/profile`
async fn get_profile(
    RequireGovernanceRead(principal): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<GovernanceProfile>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;
    let profile: GovernanceProfile = store
        .read_json(PROFILE_PATH, BRANCH)
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => AppError::NotFound(
                    format!("governance profile not found for entity {}", entity_id),
                ),
                other => AppError::Storage(other),
            }
        })?;
    Ok(Json(profile))
}

/// `PUT /entities/{entity_id}/governance/profile`
///
/// Creates or fully replaces the governance profile. When a profile already
/// exists `update()` is called to preserve the version counter; otherwise a
/// fresh profile is initialised at version 1.
async fn update_profile(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<GovernanceProfile>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let profile =
        match store.read_json::<GovernanceProfile>(PROFILE_PATH, BRANCH).await {
            Ok(mut existing) => {
                let r = req.clone();
                existing
                    .update(|p| {
                        p.entity_type = r.entity_type.clone();
                        p.legal_name = r.legal_name.clone();
                        p.jurisdiction = r.jurisdiction.clone();
                        p.effective_date = r.effective_date;
                        p.registered_agent_name = r.registered_agent_name.clone();
                        p.registered_agent_address = r.registered_agent_address.clone();
                        p.board_size = r.board_size;
                        p.principal_name = r.principal_name.clone();
                        p.company_address = r.company_address.clone();
                        p.founders = r.founders.clone();
                        p.directors = r.directors.clone();
                        p.officers = r.officers.clone();
                        p.stock_details = r.stock_details.clone();
                        p.fiscal_year_end = r.fiscal_year_end.clone();
                    })
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                existing
            }
            Err(_) => {
                // No profile yet — create a fresh one.
                GovernanceProfile::new(
                    entity_id,
                    req.entity_type,
                    req.legal_name,
                    req.jurisdiction,
                    req.effective_date,
                    req.registered_agent_name,
                    req.registered_agent_address,
                    req.board_size,
                    req.principal_name,
                    req.company_address,
                    req.founders,
                    req.directors,
                    req.officers,
                    req.stock_details,
                    req.fiscal_year_end,
                )
                .map_err(|e| AppError::BadRequest(e.to_string()))?
            }
        };

    store
        .write_json(
            PROFILE_PATH,
            &profile,
            BRANCH,
            &format!("update governance profile for entity {}", entity_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(profile))
}

// ── Written consent shortcut ──────────────────────────────────────────────────

/// `POST /entities/{entity_id}/governance/written-consent`
///
/// Creates a `WrittenConsent` meeting (which auto-convenes) plus a single
/// `Resolution` agenda item. Returns both objects.
async fn create_written_consent(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<CreateWrittenConsentRequest>,
) -> Result<Json<WrittenConsentResponse>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    let meeting = Meeting::new(
        req.body_id,
        MeetingType::WrittenConsent,
        req.title.clone(),
        None,
        None,
        None,
    );

    store
        .write::<Meeting>(
            &meeting,
            meeting.meeting_id,
            BRANCH,
            &format!("create written consent meeting {}", meeting.meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    let item = AgendaItem::new(
        meeting.meeting_id,
        req.title,
        AgendaItemType::Resolution,
        None,
        Some(req.resolution_text),
    );

    let item_path = agenda_item_path(meeting.meeting_id, item.item_id);
    store
        .write_json(
            &item_path,
            &item,
            BRANCH,
            &format!("add written consent item {}", item.item_id),
        )
        .await
        .map_err(AppError::Storage)?;

    append_index(
        &store,
        &agenda_index_path(meeting.meeting_id),
        &item.item_id.to_string(),
        &format!("index written consent item {}", item.item_id),
    )
    .await?;

    Ok(Json(WrittenConsentResponse {
        meeting,
        agenda_item: item,
    }))
}

// ── Quick-approve shortcut ────────────────────────────────────────────────────

/// `POST /entities/{entity_id}/governance/quick-approve`
///
/// Unanimous written consent in a single call:
///
/// 1. Creates a `WrittenConsent` meeting (auto-convened).
/// 2. Creates a single `Resolution` agenda item.
/// 3. Casts a unanimous `For` vote from every active, voting-eligible seat in
///    the body.
/// 4. Resolves the item as `UnanimousWrittenConsent`.
/// 5. Adjourns the meeting.
///
/// Returns the IDs of all created artifacts.
async fn quick_approve(
    RequireGovernanceWrite(principal): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<QuickApproveRequest>,
) -> Result<Json<QuickApproveResponse>, AppError> {
    let store = state.open_entity_store(principal.workspace_id, entity_id).await?;

    // Verify the body exists and is accessible.
    let body = store
        .read::<GovernanceBody>(req.body_id, BRANCH)
        .await
        .map_err(AppError::Storage)?;

    // 1. Create WrittenConsent meeting — auto-convened by Meeting::new.
    let mut meeting = Meeting::new(
        req.body_id,
        MeetingType::WrittenConsent,
        req.title.clone(),
        None,
        None,
        None,
    );

    store
        .write::<Meeting>(
            &meeting,
            meeting.meeting_id,
            BRANCH,
            &format!("quick-approve: create meeting {}", meeting.meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    // 2. Create the agenda item.
    let item = AgendaItem::new(
        meeting.meeting_id,
        req.title,
        AgendaItemType::Resolution,
        None,
        Some(req.resolution_text.clone()),
    );

    let item_path = agenda_item_path(meeting.meeting_id, item.item_id);
    store
        .write_json(
            &item_path,
            &item,
            BRANCH,
            &format!("quick-approve: add item {}", item.item_id),
        )
        .await
        .map_err(AppError::Storage)?;

    append_index(
        &store,
        &agenda_index_path(meeting.meeting_id),
        &item.item_id.to_string(),
        &format!("quick-approve: index item {}", item.item_id),
    )
    .await?;

    // 3. Collect all active voting seats in this body.
    let all_seats = store
        .read_all::<GovernanceSeat>(BRANCH)
        .await
        .map_err(AppError::Storage)?;

    let voting_seats: Vec<GovernanceSeat> = all_seats
        .into_iter()
        .filter(|s| s.body_id == req.body_id && s.can_vote())
        .collect();

    if voting_seats.is_empty() {
        return Err(AppError::BadRequest(
            "no active voting seats found in body — cannot quick-approve".into(),
        ));
    }

    // 4. Auto-cast a For vote from each seat.
    let mut vote_ids = Vec::with_capacity(voting_seats.len());
    for seat in &voting_seats {
        let vote = Vote::new(
            meeting.meeting_id,
            item.item_id,
            seat.seat_id,
            VoteValue::For,
        );
        let vpath = vote_path(meeting.meeting_id, vote.vote_id);
        store
            .write_json(
                &vpath,
                &vote,
                BRANCH,
                &format!("quick-approve: vote {} from seat {}", vote.vote_id, seat.seat_id),
            )
            .await
            .map_err(AppError::Storage)?;

        append_index(
            &store,
            &vote_index_path(meeting.meeting_id),
            &vote.vote_id.to_string(),
            &format!("quick-approve: index vote {}", vote.vote_id),
        )
        .await?;

        vote_ids.push(vote.vote_id);
    }

    // 5. Resolve the item as UnanimousWrittenConsent.
    let votes_for = voting_seats.len() as u32;
    let resolution = Resolution::new(
        meeting.meeting_id,
        item.item_id,
        ResolutionType::UnanimousWrittenConsent,
        req.resolution_text,
        votes_for,
        0,
        0,
        body.quorum_rule,
    );

    // NOTE: each data write and its corresponding index append are separate
    // commits; the EntityStore API does not expose a multi-path atomic write,
    // so a crash between the two could leave an index stale.
    let res_path = resolution_path(meeting.meeting_id, resolution.resolution_id);
    store
        .write_json(
            &res_path,
            &resolution,
            BRANCH,
            &format!("quick-approve: resolution {}", resolution.resolution_id),
        )
        .await
        .map_err(AppError::Storage)?;

    append_index(
        &store,
        &resolution_index_path(meeting.meeting_id),
        &resolution.resolution_id.to_string(),
        &format!("quick-approve: index resolution {}", resolution.resolution_id),
    )
    .await?;

    // Mark the agenda item resolved.
    let mut resolved_item = item.clone();
    resolved_item.resolve();
    store
        .write_json(
            &item_path,
            &resolved_item,
            BRANCH,
            &format!("quick-approve: mark item {} resolved", resolved_item.item_id),
        )
        .await
        .map_err(AppError::Storage)?;

    // 6. Adjourn the meeting.
    meeting
        .adjourn()
        .map_err(|e| AppError::Internal(format!("quick-approve: adjourn failed: {}", e)))?;

    store
        .write::<Meeting>(
            &meeting,
            meeting.meeting_id,
            BRANCH,
            &format!("quick-approve: adjourn meeting {}", meeting.meeting_id),
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(QuickApproveResponse {
        meeting_id: meeting.meeting_id,
        agenda_item_id: item.item_id,
        vote_ids,
        resolution_id: resolution.resolution_id,
    }))
}
