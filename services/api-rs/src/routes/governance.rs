//! Governance HTTP routes.
//!
//! Endpoints for governance bodies, seats, meetings, votes, and resolutions.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::{RequireGovernanceRead, RequireGovernanceVote, RequireGovernanceWrite};
use crate::domain::governance::{
    agenda_item::AgendaItem,
    body::GovernanceBody,
    error::GovernanceError,
    meeting::Meeting,
    resolution::Resolution,
    seat::GovernanceSeat,
    types::*,
    vote::Vote,
};
use crate::domain::ids::{
    AgendaItemId, ContactId, EntityId, GovernanceBodyId, GovernanceSeatId, MeetingId, ResolutionId,
    VoteId, WorkspaceId,
};
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::store::entity_store::EntityStore;

// ── Query types ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BodyQuery {
    pub entity_id: EntityId,
}

#[derive(Deserialize)]
pub struct MeetingQuery {
    pub entity_id: EntityId,
}

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateGovernanceBodyRequest {
    pub entity_id: EntityId,
    pub body_type: BodyType,
    pub name: String,
    pub quorum_rule: QuorumThreshold,
    pub voting_method: VotingMethod,
}

#[derive(Deserialize)]
pub struct CreateSeatRequest {
    pub holder_id: ContactId,
    pub role: SeatRole,
    #[serde(default)]
    pub appointed_date: Option<NaiveDate>,
    #[serde(default)]
    pub term_expiration: Option<NaiveDate>,
    #[serde(default)]
    pub voting_power: Option<u32>,
}

#[derive(Deserialize)]
pub struct ScheduleMeetingRequest {
    pub entity_id: EntityId,
    pub body_id: GovernanceBodyId,
    pub meeting_type: MeetingType,
    pub title: String,
    #[serde(default)]
    pub scheduled_date: Option<NaiveDate>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub notice_days: Option<u32>,
    #[serde(default)]
    pub agenda_item_titles: Vec<String>,
}

#[derive(Deserialize)]
pub struct ConveneMeetingRequest {
    pub present_seat_ids: Vec<GovernanceSeatId>,
}

#[derive(Deserialize)]
pub struct CastVoteRequest {
    pub voter_id: ContactId,
    pub vote_value: VoteValue,
}

#[derive(Deserialize)]
pub struct ComputeResolutionRequest {
    pub resolution_text: String,
    #[serde(default)]
    pub effective_date: Option<NaiveDate>,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct GovernanceBodyResponse {
    pub body_id: GovernanceBodyId,
    pub entity_id: EntityId,
    pub body_type: BodyType,
    pub name: String,
    pub quorum_rule: QuorumThreshold,
    pub voting_method: VotingMethod,
    pub status: BodyStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct GovernanceSeatResponse {
    pub seat_id: GovernanceSeatId,
    pub body_id: GovernanceBodyId,
    pub holder_id: ContactId,
    pub role: SeatRole,
    pub appointed_date: Option<NaiveDate>,
    pub term_expiration: Option<NaiveDate>,
    pub voting_power: u32,
    pub status: SeatStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct MeetingResponse {
    pub meeting_id: MeetingId,
    pub body_id: GovernanceBodyId,
    pub meeting_type: MeetingType,
    pub title: String,
    pub scheduled_date: Option<NaiveDate>,
    pub location: String,
    pub status: MeetingStatus,
    pub quorum_met: QuorumStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct AgendaItemResponse {
    pub agenda_item_id: AgendaItemId,
    pub meeting_id: MeetingId,
    pub sequence_number: u32,
    pub title: String,
    pub description: Option<String>,
    pub item_type: AgendaItemType,
    pub status: AgendaItemStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct VoteResponse {
    pub vote_id: VoteId,
    pub agenda_item_id: AgendaItemId,
    pub voter_id: ContactId,
    pub vote_value: VoteValue,
    pub voting_power_applied: u32,
    pub signature_hash: String,
    pub cast_at: String,
}

#[derive(Serialize)]
pub struct ResolutionResponse {
    pub resolution_id: ResolutionId,
    pub meeting_id: MeetingId,
    pub agenda_item_id: AgendaItemId,
    pub resolution_type: ResolutionType,
    pub resolution_text: String,
    pub passed: bool,
    pub effective_date: Option<NaiveDate>,
    pub votes_for: u32,
    pub votes_against: u32,
    pub created_at: String,
}

// ── Conversion helpers ───────────────────────────────────────────────

fn body_to_response(b: &GovernanceBody) -> GovernanceBodyResponse {
    GovernanceBodyResponse {
        body_id: b.body_id(),
        entity_id: b.entity_id(),
        body_type: b.body_type(),
        name: b.name().to_owned(),
        quorum_rule: b.quorum_rule(),
        voting_method: b.voting_method(),
        status: b.status(),
        created_at: b.created_at().to_rfc3339(),
    }
}

fn seat_to_response(s: &GovernanceSeat) -> GovernanceSeatResponse {
    GovernanceSeatResponse {
        seat_id: s.seat_id(),
        body_id: s.body_id(),
        holder_id: s.holder_id(),
        role: s.role(),
        appointed_date: s.appointed_date(),
        term_expiration: s.term_expiration(),
        voting_power: s.voting_power().raw(),
        status: s.status(),
        created_at: s.created_at().to_rfc3339(),
    }
}

fn meeting_to_response(m: &Meeting) -> MeetingResponse {
    MeetingResponse {
        meeting_id: m.meeting_id(),
        body_id: m.body_id(),
        meeting_type: m.meeting_type(),
        title: m.title().to_owned(),
        scheduled_date: m.scheduled_date(),
        location: m.location().to_owned(),
        status: m.status(),
        quorum_met: m.quorum_met(),
        created_at: m.created_at().to_rfc3339(),
    }
}

fn agenda_item_to_response(a: &AgendaItem) -> AgendaItemResponse {
    AgendaItemResponse {
        agenda_item_id: a.agenda_item_id(),
        meeting_id: a.meeting_id(),
        sequence_number: a.sequence_number(),
        title: a.title().to_owned(),
        description: a.description().map(|s| s.to_owned()),
        item_type: a.item_type(),
        status: a.status(),
        created_at: a.created_at().to_rfc3339(),
    }
}

fn vote_to_response(v: &Vote) -> VoteResponse {
    VoteResponse {
        vote_id: v.vote_id(),
        agenda_item_id: v.agenda_item_id(),
        voter_id: v.voter_id(),
        vote_value: v.vote_value(),
        voting_power_applied: v.voting_power_applied().raw(),
        signature_hash: v.signature_hash().to_owned(),
        cast_at: v.cast_at().to_rfc3339(),
    }
}

fn resolution_to_response(r: &Resolution) -> ResolutionResponse {
    ResolutionResponse {
        resolution_id: r.resolution_id(),
        meeting_id: r.meeting_id(),
        agenda_item_id: r.agenda_item_id(),
        resolution_type: r.resolution_type(),
        resolution_text: r.resolution_text().to_owned(),
        passed: r.passed(),
        effective_date: r.effective_date(),
        votes_for: r.votes_for(),
        votes_against: r.votes_against(),
        created_at: r.created_at().to_rfc3339(),
    }
}

// ── Helper to open a store ───────────────────────────────────────────

fn open_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<EntityStore<'a>, AppError> {
    EntityStore::open(layout, workspace_id, entity_id).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {} not found", entity_id))
        }
        other => AppError::Internal(other.to_string()),
    })
}

// ── Handlers: Governance bodies ──────────────────────────────────────

async fn create_governance_body(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateGovernanceBodyRequest>,
) -> Result<Json<GovernanceBodyResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let body = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let body_id = GovernanceBodyId::new();
            let body = GovernanceBody::new(
                body_id,
                entity_id,
                req.body_type,
                req.name,
                req.quorum_rule,
                req.voting_method,
            )?;

            let path = format!("governance/bodies/{}.json", body_id);
            store
                .write_json(
                    "main",
                    &path,
                    &body,
                    &format!("Create governance body {body_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(body)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(body_to_response(&body)))
}

async fn list_governance_bodies(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceBodyResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let bodies = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_ids::<GovernanceBody>("main").map_err(|e| {
                AppError::Internal(format!("list governance bodies: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let b = store.read::<GovernanceBody>("main", id).map_err(|e| {
                    AppError::Internal(format!("read governance body {id}: {e}"))
                })?;
                // Filter to bodies belonging to this entity
                if b.entity_id() == entity_id {
                    results.push(body_to_response(&b));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(bodies))
}

// ── Handlers: Governance seats ───────────────────────────────────────

async fn create_seat(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(body_id): Path<GovernanceBodyId>,
    Query(query): Query<BodyQuery>,
    Json(req): Json<CreateSeatRequest>,
) -> Result<Json<GovernanceSeatResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let seat = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Verify the body exists
            store.read::<GovernanceBody>("main", body_id).map_err(|_| {
                AppError::NotFound(format!("governance body {} not found", body_id))
            })?;

            let seat_id = GovernanceSeatId::new();
            let voting_power = req.voting_power.map(VotingPower::new).transpose()
                .map_err(|e| AppError::BadRequest(format!("invalid voting power: {e}")))?;
            let seat = GovernanceSeat::new(
                seat_id,
                body_id,
                req.holder_id,
                req.role,
                req.appointed_date,
                req.term_expiration,
                voting_power,
            )?;

            let path = format!("governance/seats/{}.json", seat_id);
            store
                .write_json(
                    "main",
                    &path,
                    &seat,
                    &format!("Appoint governance seat {seat_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(seat)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(seat_to_response(&seat)))
}

async fn list_seats(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(body_id): Path<GovernanceBodyId>,
    Query(query): Query<BodyQuery>,
) -> Result<Json<Vec<GovernanceSeatResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let seats = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_ids::<GovernanceSeat>("main").map_err(|e| {
                AppError::Internal(format!("list governance seats: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let s = store.read::<GovernanceSeat>("main", id).map_err(|e| {
                    AppError::Internal(format!("read governance seat {id}: {e}"))
                })?;
                if s.body_id() == body_id {
                    results.push(seat_to_response(&s));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(seats))
}

async fn resign_seat(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(seat_id): Path<GovernanceSeatId>,
    Query(query): Query<BodyQuery>,
) -> Result<Json<GovernanceSeatResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let seat = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut seat = store.read::<GovernanceSeat>("main", seat_id).map_err(|_| {
                AppError::NotFound(format!("governance seat {} not found", seat_id))
            })?;

            seat.resign()?;

            let path = format!("governance/seats/{}.json", seat_id);
            store
                .write_json(
                    "main",
                    &path,
                    &seat,
                    &format!("Resign governance seat {seat_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(seat)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(seat_to_response(&seat)))
}

// ── Handlers: Meetings ───────────────────────────────────────────────

async fn schedule_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<ScheduleMeetingRequest>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Verify body exists
            store
                .read::<GovernanceBody>("main", req.body_id)
                .map_err(|_| {
                    AppError::NotFound(format!("governance body {} not found", req.body_id))
                })?;

            let meeting_id = MeetingId::new();
            let meeting = Meeting::new(
                meeting_id,
                req.body_id,
                req.meeting_type,
                req.title,
                req.scheduled_date,
                req.location.unwrap_or_default(),
                req.notice_days.unwrap_or(10),
            );

            // Build all files to commit atomically: meeting + agenda items
            let mut files = vec![FileWrite::json(
                format!("governance/meetings/{}/meeting.json", meeting_id),
                &meeting,
            )
            .map_err(|e| AppError::Internal(format!("serialize meeting: {e}")))?];

            for (i, title) in req.agenda_item_titles.iter().enumerate() {
                let item_id = AgendaItemId::new();
                let item = AgendaItem::new(
                    item_id,
                    meeting_id,
                    u32::try_from(i + 1).map_err(|_| AppError::BadRequest("too many agenda items".to_owned()))?,
                    title.clone(),
                    None,
                    AgendaItemType::Resolution,
                );
                files.push(
                    FileWrite::json(
                        format!(
                            "governance/meetings/{}/agenda/{}.json",
                            meeting_id, item_id
                        ),
                        &item,
                    )
                    .map_err(|e| AppError::Internal(format!("serialize agenda item: {e}")))?,
                );
            }

            store
                .commit(
                    "main",
                    &format!("Schedule meeting {meeting_id}"),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(meeting)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(meeting_to_response(&meeting)))
}

async fn list_meetings(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(body_id): Path<GovernanceBodyId>,
    Query(query): Query<BodyQuery>,
) -> Result<Json<Vec<MeetingResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let meetings = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_ids::<Meeting>("main").map_err(|e| {
                AppError::Internal(format!("list meetings: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let m = store.read::<Meeting>("main", id).map_err(|e| {
                    AppError::Internal(format!("read meeting {id}: {e}"))
                })?;
                if m.body_id() == body_id {
                    results.push(meeting_to_response(&m));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(meetings))
}

async fn send_notice(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut meeting = store.read::<Meeting>("main", meeting_id).map_err(|_| {
                AppError::NotFound(format!("meeting {} not found", meeting_id))
            })?;

            meeting.send_notice()?;

            let path = format!("governance/meetings/{}/meeting.json", meeting_id);
            store
                .write_json(
                    "main",
                    &path,
                    &meeting,
                    &format!("Send notice for meeting {meeting_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(meeting)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(meeting_to_response(&meeting)))
}

async fn convene_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
    Json(req): Json<ConveneMeetingRequest>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut meeting = store.read::<Meeting>("main", meeting_id).map_err(|_| {
                AppError::NotFound(format!("meeting {} not found", meeting_id))
            })?;

            // Read the body to get quorum rule
            let body = store
                .read::<GovernanceBody>("main", meeting.body_id())
                .map_err(|_| {
                    AppError::NotFound(format!(
                        "governance body {} not found",
                        meeting.body_id()
                    ))
                })?;

            // Read all seats for the body, filter to those that can vote
            let seat_ids = store.list_ids::<GovernanceSeat>("main").unwrap_or_default();
            let mut total_eligible: u32 = 0;
            for id in &seat_ids {
                if let Ok(s) = store.read::<GovernanceSeat>("main", *id) {
                    if s.body_id() == meeting.body_id() && s.can_vote() {
                        total_eligible += 1;
                    }
                }
            }

            // Count present seats that can vote
            let present_count = u32::try_from(req.present_seat_ids.len())
                .map_err(|_| AppError::BadRequest("too many seats".to_owned()))?;
            let quorum_met =
                body.quorum_rule().is_met(present_count, total_eligible);

            meeting.convene(req.present_seat_ids, quorum_met)?;

            let path = format!("governance/meetings/{}/meeting.json", meeting_id);
            store
                .write_json(
                    "main",
                    &path,
                    &meeting,
                    &format!("Convene meeting {meeting_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(meeting)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(meeting_to_response(&meeting)))
}

async fn adjourn_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut meeting = store.read::<Meeting>("main", meeting_id).map_err(|_| {
                AppError::NotFound(format!("meeting {} not found", meeting_id))
            })?;

            meeting.adjourn()?;

            let path = format!("governance/meetings/{}/meeting.json", meeting_id);
            store
                .write_json(
                    "main",
                    &path,
                    &meeting,
                    &format!("Adjourn meeting {meeting_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(meeting)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(meeting_to_response(&meeting)))
}

async fn cancel_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut meeting = store.read::<Meeting>("main", meeting_id).map_err(|_| {
                AppError::NotFound(format!("meeting {} not found", meeting_id))
            })?;

            meeting.cancel()?;

            let path = format!("governance/meetings/{}/meeting.json", meeting_id);
            store
                .write_json(
                    "main",
                    &path,
                    &meeting,
                    &format!("Cancel meeting {meeting_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(meeting)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(meeting_to_response(&meeting)))
}

// ── Handlers: Votes ──────────────────────────────────────────────────

async fn cast_vote(
    RequireGovernanceVote(auth): RequireGovernanceVote,
    State(state): State<AppState>,
    Path((meeting_id, item_id)): Path<(MeetingId, AgendaItemId)>,
    Query(query): Query<MeetingQuery>,
    Json(req): Json<CastVoteRequest>,
) -> Result<Json<VoteResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let vote = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Read the meeting and check it can accept votes
            let meeting = store.read::<Meeting>("main", meeting_id).map_err(|_| {
                AppError::NotFound(format!("meeting {} not found", meeting_id))
            })?;

            if !meeting.can_vote() {
                return Err(GovernanceError::VotingSessionNotOpen.into());
            }

            // Verify agenda item exists
            store
                .read_agenda_item("main", meeting_id, item_id)
                .map_err(|_| {
                    AppError::NotFound(format!("agenda item {} not found", item_id))
                })?;

            // Find the seat for the voter in this body
            let seat_ids = store.list_ids::<GovernanceSeat>("main").unwrap_or_default();
            let mut voter_seat: Option<GovernanceSeat> = None;
            for id in &seat_ids {
                if let Ok(s) = store.read::<GovernanceSeat>("main", *id) {
                    if s.body_id() == meeting.body_id() && s.holder_id() == req.voter_id {
                        voter_seat = Some(s);
                        break;
                    }
                }
            }

            let seat = voter_seat.ok_or_else(|| {
                GovernanceError::Validation(format!(
                    "no seat found for voter {} in body {}",
                    req.voter_id,
                    meeting.body_id()
                ))
            })?;

            // Check seat can vote (active + not observer)
            if !seat.can_vote() {
                return Err(GovernanceError::CannotVoteAsObserver.into());
            }

            // Check for duplicate votes
            let existing_vote_ids = store
                .list_vote_ids("main", meeting_id)
                .unwrap_or_default();
            for vid in &existing_vote_ids {
                if let Ok(v) = store.read_vote("main", meeting_id, *vid) {
                    if v.agenda_item_id() == item_id && v.voter_id() == req.voter_id {
                        return Err(GovernanceError::DuplicateVote {
                            voter_id: req.voter_id.to_string(),
                        }
                        .into());
                    }
                }
            }

            // Create the vote
            let vote_id = VoteId::new();
            let vote = Vote::new(
                vote_id,
                meeting_id,
                item_id,
                seat.seat_id(),
                req.voter_id,
                req.vote_value,
                seat.voting_power(),
            )?;

            let path = format!(
                "governance/meetings/{}/votes/{}.json",
                meeting_id, vote_id
            );
            store
                .write_json(
                    "main",
                    &path,
                    &vote,
                    &format!("Cast vote {vote_id} on agenda item {item_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(vote)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(vote_to_response(&vote)))
}

async fn list_votes(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((meeting_id, item_id)): Path<(MeetingId, AgendaItemId)>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<Vec<VoteResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let votes = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_vote_ids("main", meeting_id)
                .unwrap_or_default();

            let mut results = Vec::new();
            for id in ids {
                if let Ok(v) = store.read_vote("main", meeting_id, id) {
                    if v.agenda_item_id() == item_id {
                        results.push(vote_to_response(&v));
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(votes))
}

// ── Handlers: Resolutions ────────────────────────────────────────────

async fn compute_resolution(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((meeting_id, item_id)): Path<(MeetingId, AgendaItemId)>,
    Query(query): Query<MeetingQuery>,
    Json(req): Json<ComputeResolutionRequest>,
) -> Result<Json<ResolutionResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let resolution = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Read the meeting
            let meeting = store.read::<Meeting>("main", meeting_id).map_err(|_| {
                AppError::NotFound(format!("meeting {} not found", meeting_id))
            })?;

            // Read the body to get quorum rule
            let body = store
                .read::<GovernanceBody>("main", meeting.body_id())
                .map_err(|_| {
                    AppError::NotFound(format!(
                        "governance body {} not found",
                        meeting.body_id()
                    ))
                })?;

            // Read all votes for this agenda item
            let vote_ids = store
                .list_vote_ids("main", meeting_id)
                .unwrap_or_default();

            let mut votes_for: u32 = 0;
            let mut votes_against: u32 = 0;
            let mut votes_abstain: u32 = 0;
            let mut recused_count: u32 = 0;

            for vid in &vote_ids {
                if let Ok(v) = store.read_vote("main", meeting_id, *vid) {
                    if v.agenda_item_id() == item_id {
                        let weight = v.voting_power_applied().raw();
                        match v.vote_value() {
                            VoteValue::For => votes_for += weight,
                            VoteValue::Against => votes_against += weight,
                            VoteValue::Abstain => votes_abstain += weight,
                            VoteValue::Recusal => recused_count += weight,
                        }
                    }
                }
            }

            // Determine if the resolution passed using the body's quorum rule.
            // Total eligible = for + against (abstentions and recusals don't count
            // toward the denominator for pass/fail determination).
            let total_eligible = votes_for + votes_against;
            let passed = body.quorum_rule().is_met(votes_for, total_eligible);

            // Determine resolution type from quorum rule
            let resolution_type = match body.quorum_rule() {
                QuorumThreshold::Majority => ResolutionType::Ordinary,
                QuorumThreshold::Supermajority => ResolutionType::Special,
                QuorumThreshold::Unanimous => ResolutionType::UnanimousWrittenConsent,
            };

            let resolution_id = ResolutionId::new();
            let resolution = Resolution::new(
                resolution_id,
                meeting_id,
                item_id,
                resolution_type,
                req.resolution_text,
                passed,
                req.effective_date,
                votes_for,
                votes_against,
                votes_abstain,
                recused_count,
            );

            let path = format!(
                "governance/meetings/{}/resolutions/{}.json",
                meeting_id, resolution_id
            );
            store
                .write_json(
                    "main",
                    &path,
                    &resolution,
                    &format!("Compute resolution {resolution_id} for agenda item {item_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(resolution)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(resolution_to_response(&resolution)))
}

async fn list_resolutions(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<Vec<ResolutionResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let resolutions = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_resolution_ids("main", meeting_id)
                .unwrap_or_default();

            let mut results = Vec::new();
            for id in ids {
                if let Ok(r) = store.read_resolution("main", meeting_id, id) {
                    results.push(resolution_to_response(&r));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(resolutions))
}

// ── Handlers: Scan expired seats ─────────────────────────────────────

#[derive(Serialize)]
pub struct ScanExpiredResponse {
    pub scanned: usize,
    pub expired: usize,
}

async fn scan_expired_seats(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ScanExpiredResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let body_ids = store.list_ids::<GovernanceBody>("main").map_err(|e| {
                AppError::Internal(format!("list bodies: {e}"))
            })?;

            let today = chrono::Utc::now().date_naive();
            let mut scanned = 0usize;
            let mut expired = 0usize;

            for body_id in body_ids {
                let seat_ids = store.list_ids::<GovernanceSeat>("main").map_err(|e| {
                    AppError::Internal(format!("list seats: {e}"))
                })?;

                for seat_id in seat_ids {
                    scanned += 1;
                    if let Ok(seat) = store.read::<GovernanceSeat>("main", seat_id) {
                        if let Some(term_end) = seat.term_expiration() {
                            if term_end < today && seat.status() == crate::domain::governance::types::SeatStatus::Active {
                                // Seat has expired — mark it
                                let mut seat = seat;
                                seat.resign()?;
                                let path = format!("governance/bodies/{}/seats/{}.json", body_id, seat_id);
                                store
                                    .write_json("main", &path, &seat, &format!("Expire seat {seat_id}"))
                                    .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
                                expired += 1;
                            }
                        }
                    }
                }
            }

            Ok::<_, AppError>(ScanExpiredResponse { scanned, expired })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(result))
}

// ── Handlers: Written consent ────────────────────────────────────────

#[derive(Deserialize)]
pub struct WrittenConsentRequest {
    pub body_id: GovernanceBodyId,
    pub entity_id: EntityId,
    pub title: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct WrittenConsentResponse {
    pub meeting_id: MeetingId,
    pub body_id: GovernanceBodyId,
    pub title: String,
    pub status: MeetingStatus,
    pub consent_type: String,
    pub created_at: String,
}

async fn written_consent(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<WrittenConsentRequest>,
) -> Result<Json<WrittenConsentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Verify body exists
            store.read::<GovernanceBody>("main", req.body_id).map_err(|_| {
                AppError::NotFound(format!("governance body {} not found", req.body_id))
            })?;

            let meeting_id = MeetingId::new();
            let meeting = Meeting::new(
                meeting_id,
                req.body_id,
                MeetingType::WrittenConsent,
                req.title,
                None, // No scheduled date for written consent
                String::new(), // No location
                0, // No notice days
            );

            let path = format!("governance/meetings/{}/meeting.json", meeting_id);
            store
                .write_json("main", &path, &meeting, &format!("Written consent {meeting_id}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(meeting)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(WrittenConsentResponse {
        meeting_id: meeting.meeting_id(),
        body_id: meeting.body_id(),
        title: meeting.title().to_owned(),
        status: meeting.status(),
        consent_type: "written_consent".to_owned(),
        created_at: meeting.created_at().to_rfc3339(),
    }))
}

// ── Handlers: List all meetings (global) ────────────────────────────

async fn list_all_meetings(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<MeetingResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let meetings = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_ids::<Meeting>("main").map_err(|e| {
                AppError::Internal(format!("list meetings: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(m) = store.read::<Meeting>("main",id) {
                    results.push(meeting_to_response(&m));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(meetings))
}

// ── Handlers: List all governance bodies (global) ───────────────────

async fn list_all_governance_bodies(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<GovernanceBodyResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let bodies = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_ids::<GovernanceBody>("main").map_err(|e| {
                AppError::Internal(format!("list bodies: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(b) = store.read::<GovernanceBody>("main",id) {
                    results.push(body_to_response(&b));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(bodies))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn governance_routes() -> Router<AppState> {
    Router::new()
        // Governance bodies
        .route("/v1/governance-bodies", post(create_governance_body))
        .route(
            "/v1/entities/{entity_id}/governance-bodies",
            get(list_governance_bodies),
        )
        // Governance seats
        .route(
            "/v1/governance-bodies/{body_id}/seats",
            post(create_seat).get(list_seats),
        )
        .route(
            "/v1/governance-seats/{seat_id}/resign",
            post(resign_seat),
        )
        // Meetings
        .route("/v1/meetings", post(schedule_meeting))
        .route(
            "/v1/governance-bodies/{body_id}/meetings",
            get(list_meetings),
        )
        .route("/v1/meetings/{meeting_id}/notice", post(send_notice))
        .route("/v1/meetings/{meeting_id}/convene", post(convene_meeting))
        .route("/v1/meetings/{meeting_id}/adjourn", post(adjourn_meeting))
        .route("/v1/meetings/{meeting_id}/cancel", post(cancel_meeting))
        // Votes
        .route(
            "/v1/meetings/{meeting_id}/agenda-items/{item_id}/vote",
            post(cast_vote),
        )
        .route(
            "/v1/meetings/{meeting_id}/agenda-items/{item_id}/votes",
            get(list_votes),
        )
        // Resolutions
        .route(
            "/v1/meetings/{meeting_id}/agenda-items/{item_id}/resolution",
            post(compute_resolution),
        )
        .route(
            "/v1/meetings/{meeting_id}/resolutions",
            get(list_resolutions),
        )
        // Seat scanning
        .route(
            "/v1/governance-seats/scan-expired",
            post(scan_expired_seats),
        )
        // Written consent
        .route("/v1/meetings/written-consent", post(written_consent))
        // List all meetings
        .route("/v1/meetings", get(list_all_meetings))
        // List all governance bodies
        .route("/v1/governance-bodies", get(list_all_governance_bodies))
}
