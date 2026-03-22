//! Governance HTTP routes.
//!
//! Endpoints for governance bodies, seats, meetings, votes, and resolutions.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::AppState;
use super::validation::{require_non_empty_trimmed, validate_not_too_far_future};
use crate::auth::{
    RequireGovernanceRead, RequireGovernanceVote, RequireGovernanceWrite, RequireInternalWorker,
};
use crate::domain::contacts::{
    contact::Contact,
    types::{ContactCategory, ContactType},
};
use crate::domain::equity::{
    fundraising_workflow::{
        FundraisingWorkflow, WorkflowExecutionStatus as FundraisingWorkflowExecutionStatus,
    },
    holder::Holder,
    instrument::{Instrument, InstrumentKind},
    position::Position,
    round::{EquityRound, EquityRoundStatus},
    safe_note::SafeNote,
    transfer_workflow::{
        TransferWorkflow, WorkflowExecutionStatus as TransferWorkflowExecutionStatus,
    },
    types::{SafeStatus, ValuationStatus},
    valuation::Valuation,
};
use crate::domain::formation::types::{EntityType, FormationStatus};
use crate::domain::governance::{
    agenda_item::AgendaItem,
    audit::{
        GovernanceAuditCheckpoint, GovernanceAuditEntry, GovernanceAuditEventType,
        GovernanceAuditVerificationReport,
    },
    body::GovernanceBody,
    delegation_schedule::{CURRENT_SCHEDULE_PATH, DelegationSchedule, ScheduleAmendment},
    doc_generator::{
        GOVERNANCE_DOC_BUNDLES_CURRENT_PATH, GOVERNANCE_DOC_BUNDLES_HISTORY_DIR,
        GovernanceDocBundleCurrent, GovernanceDocBundleManifest, GovernanceDocBundleSummary,
        GovernanceDocEntityType, bundle_documents_prefix, bundle_history_path,
        bundle_manifest_path, render_bundle_from_profile,
    },
    error::GovernanceError,
    incident::{GovernanceIncident, IncidentSeverity, IncidentStatus},
    meeting::Meeting,
    mode::{GovernanceMode, GovernanceModeState},
    mode_history::GovernanceModeChangeEvent,
    policy_engine::{
        PolicyDecision, PolicyEvaluationContext, canonicalize_intent_type, evaluate_full,
    },
    profile::{
        CompanyAddress, DirectorInfo, DocumentOptions, FiscalYearEnd, FounderInfo,
        GOVERNANCE_PROFILE_PATH, GovernanceProfile, OfficerInfo, StockDetails,
    },
    resolution::Resolution,
    seat::GovernanceSeat,
    trigger::{GovernanceTriggerEvent, GovernanceTriggerSource, GovernanceTriggerType},
    types::*,
    vote::Vote,
};
use crate::domain::ids::{
    AgendaItemId, ComplianceEscalationId, ContactId, DocumentId, EntityId,
    GovernanceAuditCheckpointId, GovernanceAuditVerificationId, GovernanceBodyId,
    GovernanceDocBundleId, GovernanceModeEventId, GovernanceSeatId, GovernanceTriggerId,
    IncidentId, IntentId, MeetingId, ResolutionId, ScheduleAmendmentId, VoteId, WorkspaceId,
};
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::git::error::GitStorageError;
use crate::routes::governance_enforcement::{
    BuildAuditEntryInput, LockdownTriggerInput, SetModeWithHistoryInput, apply_lockdown_trigger,
    audit_entry_path, build_audit_entry, list_audit_entries_sorted, read_mode_or_default,
    set_mode_with_history,
};
use crate::store::entity_store::EntityStore;
use crate::store::stored_entity::StoredEntity;

// ── Query types ──────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct BodyQuery {
    pub entity_id: EntityId,
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct MeetingQuery {
    pub entity_id: EntityId,
}

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateGovernanceBodyRequest {
    pub entity_id: EntityId,
    pub body_type: BodyType,
    pub name: String,
    pub quorum_rule: QuorumThreshold,
    pub voting_method: VotingMethod,
}

#[derive(Deserialize, utoipa::ToSchema)]
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

#[derive(Deserialize, utoipa::ToSchema)]
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConveneMeetingRequest {
    pub present_seat_ids: Vec<GovernanceSeatId>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CastVoteRequest {
    pub voter_id: ContactId,
    pub vote_value: VoteValue,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ComputeResolutionRequest {
    pub resolution_text: String,
    #[serde(default)]
    pub effective_date: Option<NaiveDate>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetGovernanceModeRequest {
    pub entity_id: EntityId,
    pub mode: GovernanceMode,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub incident_ids: Vec<IncidentId>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateIncidentRequest {
    pub entity_id: EntityId,
    pub severity: IncidentSeverity,
    pub title: String,
    pub description: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct FinalizeAgendaItemRequest {
    pub entity_id: EntityId,
    pub status: AgendaItemStatus,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AttachResolutionDocumentRequest {
    pub entity_id: EntityId,
    pub document_id: DocumentId,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AmendDelegationScheduleRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub tier1_max_amount_cents: Option<i64>,
    #[serde(default)]
    pub allowed_tier1_intent_types: Option<Vec<String>>,
    #[serde(default)]
    pub next_mandatory_review_at: Option<NaiveDate>,
    #[serde(default)]
    pub meeting_id: Option<MeetingId>,
    #[serde(default)]
    pub adopted_resolution_id: Option<ResolutionId>,
    #[serde(default)]
    pub rationale: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ReauthorizeDelegationScheduleRequest {
    pub entity_id: EntityId,
    pub meeting_id: MeetingId,
    pub adopted_resolution_id: ResolutionId,
    #[serde(default)]
    pub rationale: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateGovernanceProfileRequest {
    pub legal_name: String,
    pub jurisdiction: String,
    pub effective_date: NaiveDate,
    pub adopted_by: String,
    pub last_reviewed: NaiveDate,
    pub next_mandatory_review: NaiveDate,
    #[serde(default)]
    pub registered_agent_name: Option<String>,
    #[serde(default)]
    pub registered_agent_address: Option<String>,
    #[serde(default)]
    pub board_size: Option<u32>,
    #[serde(default)]
    pub incorporator_name: Option<String>,
    #[serde(default)]
    pub incorporator_address: Option<String>,
    #[serde(default)]
    pub principal_name: Option<String>,
    #[serde(default)]
    pub principal_title: Option<String>,
    #[serde(default)]
    pub incomplete_profile: Option<bool>,
    #[serde(default)]
    pub company_address: Option<CompanyAddress>,
    #[serde(default)]
    pub founders: Option<Vec<FounderInfo>>,
    #[serde(default)]
    pub directors: Option<Vec<DirectorInfo>>,
    #[serde(default)]
    pub officers: Option<Vec<OfficerInfo>>,
    #[serde(default)]
    pub stock_details: Option<StockDetails>,
    #[serde(default)]
    pub fiscal_year_end: Option<FiscalYearEnd>,
    #[serde(default)]
    pub document_options: Option<DocumentOptions>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GenerateGovernanceDocBundleRequest {
    #[serde(default)]
    pub template_version: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct InternalLockdownTriggerRequest {
    pub idempotency_key: String,
    pub trigger_type: GovernanceTriggerType,
    pub severity: IncidentSeverity,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub linked_intent_id: Option<IntentId>,
    #[serde(default)]
    pub linked_escalation_id: Option<ComplianceEscalationId>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateGovernanceAuditEventRequest {
    pub entity_id: EntityId,
    pub event_type: GovernanceAuditEventType,
    pub action: String,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub details: serde_json::Value,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub linked_intent_id: Option<IntentId>,
    #[serde(default)]
    pub linked_incident_id: Option<IncidentId>,
    #[serde(default)]
    pub linked_trigger_id: Option<GovernanceTriggerId>,
    #[serde(default)]
    pub linked_mode_event_id: Option<GovernanceModeEventId>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct WriteGovernanceAuditCheckpointRequest {
    pub entity_id: EntityId,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct VerifyGovernanceAuditChainRequest {
    pub entity_id: EntityId,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct EvaluateGovernanceRequest {
    pub entity_id: EntityId,
    pub intent_type: String,
    #[serde(default = "default_evaluate_metadata")]
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
}

fn default_evaluate_metadata() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
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

#[derive(Serialize, utoipa::ToSchema)]
pub struct GovernanceSeatResponse {
    pub seat_id: GovernanceSeatId,
    pub body_id: GovernanceBodyId,
    pub holder_id: ContactId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder_name: Option<String>,
    pub role: SeatRole,
    pub appointed_date: Option<NaiveDate>,
    pub term_expiration: Option<NaiveDate>,
    pub voting_power: u32,
    pub status: SeatStatus,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MeetingResponse {
    pub meeting_id: MeetingId,
    pub body_id: GovernanceBodyId,
    pub meeting_type: MeetingType,
    pub title: String,
    pub scheduled_date: Option<NaiveDate>,
    pub location: String,
    pub status: MeetingStatus,
    pub quorum_met: QuorumStatus,
    pub agenda_item_ids: Vec<AgendaItemId>,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
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

#[derive(Serialize, utoipa::ToSchema)]
pub struct VoteResponse {
    pub vote_id: VoteId,
    pub agenda_item_id: AgendaItemId,
    pub voter_id: ContactId,
    pub vote_value: VoteValue,
    pub voting_power_applied: u32,
    pub signature_hash: String,
    pub cast_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ResolutionResponse {
    pub resolution_id: ResolutionId,
    pub meeting_id: MeetingId,
    pub agenda_item_id: AgendaItemId,
    pub resolution_type: ResolutionType,
    pub resolution_text: String,
    pub passed: bool,
    pub effective_date: Option<NaiveDate>,
    pub document_id: Option<DocumentId>,
    pub votes_for: u32,
    pub votes_against: u32,
    pub votes_abstain: u32,
    pub recused_count: u32,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GovernanceModeResponse {
    pub entity_id: EntityId,
    pub mode: GovernanceMode,
    pub reason: Option<String>,
    pub updated_at: String,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct IncidentResponse {
    pub incident_id: IncidentId,
    pub entity_id: EntityId,
    pub severity: IncidentSeverity,
    pub title: String,
    pub description: String,
    pub status: IncidentStatus,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GenerateGovernanceDocBundleResponse {
    pub manifest: GovernanceDocBundleManifest,
    pub current: GovernanceDocBundleCurrent,
    pub summary: GovernanceDocBundleSummary,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct InternalLockdownTriggerResponse {
    pub trigger_id: GovernanceTriggerId,
    pub incident_id: IncidentId,
    pub mode: GovernanceMode,
    pub mode_event_id: GovernanceModeEventId,
    pub idempotent_replay: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DelegationScheduleChangeResponse {
    pub schedule: DelegationSchedule,
    pub amendment: ScheduleAmendment,
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
        holder_name: None,
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
        agenda_item_ids: Vec::new(),
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
        document_id: r.document_id(),
        votes_for: r.votes_for(),
        votes_against: r.votes_against(),
        votes_abstain: r.votes_abstain(),
        recused_count: r.recused_count(),
        created_at: r.created_at().to_rfc3339(),
    }
}

fn mode_to_response(mode: &GovernanceModeState) -> GovernanceModeResponse {
    GovernanceModeResponse {
        entity_id: mode.entity_id(),
        mode: mode.mode(),
        reason: mode.reason().map(ToOwned::to_owned),
        updated_at: mode.updated_at().to_rfc3339(),
        created_at: mode.created_at().to_rfc3339(),
    }
}

fn incident_to_response(incident: &GovernanceIncident) -> IncidentResponse {
    IncidentResponse {
        incident_id: incident.incident_id(),
        entity_id: incident.entity_id(),
        severity: incident.severity(),
        title: incident.title().to_owned(),
        description: incident.description().to_owned(),
        status: incident.status(),
        created_at: incident.created_at().to_rfc3339(),
        resolved_at: incident.resolved_at().map(|t| t.to_rfc3339()),
    }
}

// ── Helper to open a store ───────────────────────────────────────────

fn open_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    allowed_entity_ids: Option<&[EntityId]>,
    entity_id: EntityId,
    valkey_client: Option<&redis::Client>,
) -> Result<EntityStore<'a>, AppError> {
    if let Some(ids) = allowed_entity_ids
        && !ids.contains(&entity_id)
    {
        return Err(AppError::Forbidden(format!(
            "principal is not authorized for entity {}",
            entity_id
        )));
    }
    EntityStore::open(layout, workspace_id, entity_id, valkey_client).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {} not found", entity_id))
        }
        other => AppError::Internal(other.to_string()),
    })
}

fn read_all<T: StoredEntity>(store: &EntityStore<'_>) -> Result<Vec<T>, AppError> {
    let ids = store
        .list_ids::<T>("main")
        .map_err(|e| AppError::Internal(format!("list {}: {e}", T::storage_dir())))?;

    let mut records = Vec::new();
    for id in ids {
        let record = store
            .read::<T>("main", id)
            .map_err(|e| AppError::Internal(format!("read {} {}: {e}", T::storage_dir(), id)))?;
        records.push(record);
    }
    Ok(records)
}

fn validate_meeting_type_for_body(
    body_type: BodyType,
    meeting_type: MeetingType,
) -> Result<(), AppError> {
    let is_allowed = match body_type {
        BodyType::BoardOfDirectors => {
            matches!(
                meeting_type,
                MeetingType::BoardMeeting | MeetingType::WrittenConsent
            )
        }
        BodyType::LlcMemberVote => {
            matches!(
                meeting_type,
                MeetingType::MemberMeeting | MeetingType::WrittenConsent
            )
        }
    };
    if is_allowed {
        return Ok(());
    }
    Err(AppError::BadRequest(format!(
        "meeting type {meeting_type:?} is not valid for governance body type {body_type:?}"
    )))
}

fn validate_body_type_for_entity(
    entity_type: EntityType,
    body_type: BodyType,
) -> Result<(), AppError> {
    let is_allowed = matches!(
        (entity_type, body_type),
        (EntityType::CCorp, BodyType::BoardOfDirectors)
            | (EntityType::Llc, BodyType::LlcMemberVote)
    );
    if is_allowed {
        return Ok(());
    }
    Err(AppError::BadRequest(format!(
        "governance body type {body_type:?} is not valid for entity type {entity_type}"
    )))
}

fn holder_has_active_membership_units(
    store: &EntityStore<'_>,
    holder_id: ContactId,
) -> Result<bool, AppError> {
    let holder_record_id = store
        .list_ids::<Holder>("main")
        .map_err(|e| AppError::Internal(format!("list holders: {e}")))?
        .into_iter()
        .find_map(|id| {
            store
                .read::<Holder>("main", id)
                .ok()
                .and_then(|holder| (holder.contact_id() == holder_id).then_some(holder.holder_id()))
        });
    let Some(holder_record_id) = holder_record_id else {
        return Ok(false);
    };

    let membership_instrument_ids: std::collections::HashSet<_> = store
        .list_ids::<Instrument>("main")
        .map_err(|e| AppError::Internal(format!("list instruments: {e}")))?
        .into_iter()
        .filter_map(|id| store.read::<Instrument>("main", id).ok())
        .filter(|instrument| instrument.kind() == InstrumentKind::MembershipUnit)
        .map(|instrument| instrument.instrument_id())
        .collect();
    if membership_instrument_ids.is_empty() {
        return Ok(false);
    }

    let has_units = store
        .list_ids::<Position>("main")
        .map_err(|e| AppError::Internal(format!("list positions: {e}")))?
        .into_iter()
        .filter_map(|id| store.read::<Position>("main", id).ok())
        .any(|position| {
            position.holder_id() == holder_record_id
                && position.quantity_units() > 0
                && membership_instrument_ids.contains(&position.instrument_id())
        });
    Ok(has_units)
}

fn validate_holder_for_body(
    store: &EntityStore<'_>,
    body: &GovernanceBody,
    holder: &Contact,
) -> Result<(), AppError> {
    match body.body_type() {
        BodyType::BoardOfDirectors => {
            if holder.contact_type() != ContactType::Individual {
                return Err(AppError::BadRequest(
                    "board seats require an individual contact".to_owned(),
                ));
            }
            if !matches!(
                holder.category(),
                ContactCategory::BoardMember
                    | ContactCategory::Founder
                    | ContactCategory::Officer
                    | ContactCategory::Investor
            ) {
                return Err(AppError::BadRequest(
                    "board seats require a board_member, founder, officer, or investor contact"
                        .to_owned(),
                ));
            }
        }
        BodyType::LlcMemberVote => {
            let is_member_contact = matches!(
                holder.category(),
                ContactCategory::Founder | ContactCategory::Member
            );
            if !is_member_contact
                && !holder_has_active_membership_units(store, holder.contact_id())?
            {
                return Err(AppError::BadRequest(
                    "llc member-vote seats require a member contact".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn has_active_seat(
    store: &EntityStore<'_>,
    body_id: GovernanceBodyId,
    holder_id: ContactId,
) -> Result<bool, AppError> {
    let seat_ids = store
        .list_ids::<GovernanceSeat>("main")
        .map_err(|e| AppError::Internal(format!("list governance seats: {e}")))?;
    Ok(seat_ids
        .into_iter()
        .filter_map(|id| store.read::<GovernanceSeat>("main", id).ok())
        .any(|seat| {
            seat.body_id() == body_id
                && seat.holder_id() == holder_id
                && seat.status() == SeatStatus::Active
        }))
}

fn eligible_voting_power_for_body(
    store: &EntityStore<'_>,
    body_id: GovernanceBodyId,
) -> Result<u32, AppError> {
    Ok(read_all::<GovernanceSeat>(store)?
        .into_iter()
        .filter(|seat| seat.body_id() == body_id && seat.can_vote())
        .map(|seat| seat.voting_power().raw())
        .sum())
}

fn ensure_meeting_can_be_cancelled(
    store: &EntityStore<'_>,
    meeting: &Meeting,
) -> Result<(), AppError> {
    for round in read_all::<EquityRound>(store)? {
        if round.board_approval_meeting_id() == Some(meeting.meeting_id())
            && !matches!(
                round.status(),
                EquityRoundStatus::Closed | EquityRoundStatus::Cancelled
            )
        {
            return Err(AppError::Conflict(format!(
                "meeting {} is linked to active equity round {}",
                meeting.meeting_id(),
                round.equity_round_id()
            )));
        }
    }

    for workflow in read_all::<TransferWorkflow>(store)? {
        if workflow.board_approval_meeting_id() == Some(meeting.meeting_id())
            && !matches!(
                workflow.execution_status(),
                TransferWorkflowExecutionStatus::Executed
                    | TransferWorkflowExecutionStatus::Failed
                    | TransferWorkflowExecutionStatus::Cancelled
            )
        {
            return Err(AppError::Conflict(format!(
                "meeting {} is linked to active transfer workflow {}",
                meeting.meeting_id(),
                workflow.transfer_workflow_id()
            )));
        }
    }

    for workflow in read_all::<FundraisingWorkflow>(store)? {
        if workflow.board_approval_meeting_id() == Some(meeting.meeting_id())
            && !matches!(
                workflow.execution_status(),
                FundraisingWorkflowExecutionStatus::Executed
                    | FundraisingWorkflowExecutionStatus::Failed
                    | FundraisingWorkflowExecutionStatus::Cancelled
            )
        {
            return Err(AppError::Conflict(format!(
                "meeting {} is linked to active fundraising workflow {}",
                meeting.meeting_id(),
                workflow.fundraising_workflow_id()
            )));
        }
    }

    for valuation in read_all::<Valuation>(store)? {
        if valuation.board_approval_meeting_id() == Some(meeting.meeting_id())
            && valuation.status() == ValuationStatus::PendingApproval
        {
            return Err(AppError::Conflict(format!(
                "meeting {} is linked to pending valuation {}",
                meeting.meeting_id(),
                valuation.valuation_id()
            )));
        }
    }

    for safe_note in read_all::<SafeNote>(store)? {
        if safe_note.board_approval_meeting_id() == Some(meeting.meeting_id())
            && safe_note.status() == SafeStatus::Issued
        {
            return Err(AppError::Conflict(format!(
                "meeting {} is linked to issued SAFE note {}",
                meeting.meeting_id(),
                safe_note.safe_note_id()
            )));
        }
    }

    let has_legacy_409a_agenda = store
        .list_agenda_item_ids("main", meeting.meeting_id())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item_id| {
            store
                .read_agenda_item("main", meeting.meeting_id(), item_id)
                .ok()
        })
        .any(|item| item.title().starts_with("Approve 409A Valuation"));
    if has_legacy_409a_agenda
        && read_all::<Valuation>(store)?.into_iter().any(|valuation| {
            valuation.status() == ValuationStatus::PendingApproval
                && valuation.board_approval_meeting_id().is_none()
        })
    {
        return Err(AppError::Conflict(format!(
            "meeting {} may be linked to a pending valuation approval; resolve or recreate that workflow before cancelling",
            meeting.meeting_id()
        )));
    }

    Ok(())
}

fn read_schedule_or_default(store: &EntityStore<'_>, entity_id: EntityId) -> DelegationSchedule {
    store
        .read_json::<DelegationSchedule>("main", CURRENT_SCHEDULE_PATH)
        .unwrap_or_else(|_| DelegationSchedule::default_for_entity(entity_id))
}

fn governance_doc_entity_type_for(entity_type: EntityType) -> GovernanceDocEntityType {
    match entity_type {
        EntityType::CCorp => GovernanceDocEntityType::Corporation,
        EntityType::Llc => GovernanceDocEntityType::Llc,
    }
}

fn ensure_entity_ready_for_governance(
    store: &EntityStore<'_>,
    action: &str,
) -> Result<(), AppError> {
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    if entity.formation_status() != FormationStatus::Active {
        return Err(AppError::BadRequest(format!(
            "{action} requires an active entity, current status is {}",
            entity.formation_status()
        )));
    }
    Ok(())
}

fn read_profile_or_default(
    store: &EntityStore<'_>,
    entity_id: EntityId,
) -> Result<GovernanceProfile, AppError> {
    match store.read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH) {
        Ok(profile) => {
            if profile.entity_id() != entity_id {
                return Err(AppError::UnprocessableEntity(format!(
                    "governance profile entity_id {} does not match requested entity {}",
                    profile.entity_id(),
                    entity_id
                )));
            }
            Ok(profile)
        }
        Err(GitStorageError::NotFound(_)) => {
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            Ok(GovernanceProfile::default_for_entity(&entity))
        }
        Err(e) => Err(AppError::Internal(format!("read governance profile: {e}"))),
    }
}

fn read_doc_bundle_current(
    store: &EntityStore<'_>,
) -> Result<GovernanceDocBundleCurrent, AppError> {
    store
        .read_json::<GovernanceDocBundleCurrent>("main", GOVERNANCE_DOC_BUNDLES_CURRENT_PATH)
        .map_err(|e| match e {
            GitStorageError::NotFound(_) => {
                AppError::NotFound("no governance doc bundle has been generated yet".to_owned())
            }
            other => AppError::Internal(format!("read governance doc bundle current: {other}")),
        })
}

fn list_doc_bundle_summaries(
    store: &EntityStore<'_>,
) -> Result<Vec<GovernanceDocBundleSummary>, AppError> {
    let ids = store
        .list_ids_in_dir::<GovernanceDocBundleId>("main", GOVERNANCE_DOC_BUNDLES_HISTORY_DIR)
        .map_err(|e| AppError::Internal(format!("list governance doc bundles: {e}")))?;
    let mut summaries = Vec::new();
    for bundle_id in ids {
        let path = bundle_history_path(bundle_id);
        let summary = store
            .read_json::<GovernanceDocBundleSummary>("main", &path)
            .map_err(|e| AppError::Internal(format!("read governance doc bundle summary: {e}")))?;
        summaries.push(summary);
    }
    summaries.sort_by(|a, b| b.generated_at.cmp(&a.generated_at));
    Ok(summaries)
}

fn audit_checkpoint_path(checkpoint_id: GovernanceAuditCheckpointId) -> String {
    format!("governance/audit/checkpoints/{checkpoint_id}.json")
}

fn audit_verification_path(verification_id: GovernanceAuditVerificationId) -> String {
    format!("governance/audit/verifications/{verification_id}.json")
}

fn normalize_tier1_intents(intents: Vec<String>) -> Vec<String> {
    let mut normalized = intents
        .into_iter()
        .map(|s| canonicalize_intent_type(&s))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn validate_schedule_resolution(
    store: &EntityStore<'_>,
    meeting_id: MeetingId,
    resolution_id: ResolutionId,
) -> Result<(), AppError> {
    let meeting = store
        .read::<Meeting>("main", meeting_id)
        .map_err(|_| AppError::NotFound(format!("meeting {meeting_id} not found")))?;
    let body = store
        .read::<GovernanceBody>("main", meeting.body_id())
        .map_err(|_| {
            AppError::NotFound(format!("governance body {} not found", meeting.body_id()))
        })?;
    if !matches!(
        body.body_type(),
        BodyType::BoardOfDirectors | BodyType::LlcMemberVote
    ) {
        return Err(AppError::UnprocessableEntity(
            "delegation schedule changes require board/member authority".to_owned(),
        ));
    }
    let resolution = store
        .read_resolution("main", meeting_id, resolution_id)
        .map_err(|_| AppError::NotFound(format!("resolution {resolution_id} not found")))?;
    if !resolution.passed() {
        return Err(AppError::UnprocessableEntity(format!(
            "resolution {resolution_id} did not pass"
        )));
    }
    Ok(())
}

// ── Handlers: Governance profile + doc bundles ──────────────────────

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/profile",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Governance profile", body = GovernanceProfile),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn get_governance_profile(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<GovernanceProfile>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let profile = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let profile = read_profile_or_default(&store, entity_id)?;
            profile.validate().map_err(AppError::UnprocessableEntity)?;
            Ok::<_, AppError>(profile)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(profile))
}

#[utoipa::path(
    put,
    path = "/v1/entities/{entity_id}/governance/profile",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = UpdateGovernanceProfileRequest,
    responses(
        (status = 200, description = "Updated governance profile", body = GovernanceProfile),
        (status = 422, description = "Validation error"),
    ),
)]
async fn update_governance_profile(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<UpdateGovernanceProfileRequest>,
) -> Result<Json<GovernanceProfile>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let profile = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let mut profile = read_profile_or_default(&store, entity_id)?;
            profile.update(
                req.legal_name,
                req.jurisdiction,
                req.effective_date,
                req.adopted_by,
                req.last_reviewed,
                req.next_mandatory_review,
                req.registered_agent_name,
                req.registered_agent_address,
                req.board_size,
                req.incorporator_name,
                req.incorporator_address,
                req.principal_name,
                req.principal_title,
                req.incomplete_profile,
            );
            if let Some(company_address) = req.company_address {
                profile.set_company_address(company_address);
            }
            if let Some(founders) = req.founders {
                profile.set_founders(founders);
            }
            if let Some(directors) = req.directors {
                profile.set_directors(directors);
            }
            if let Some(officers) = req.officers {
                profile.set_officers(officers);
            }
            if let Some(stock_details) = req.stock_details {
                profile.set_stock_details(stock_details);
            }
            if let Some(fiscal_year_end) = req.fiscal_year_end {
                profile.set_fiscal_year_end(fiscal_year_end);
            }
            if let Some(document_options) = req.document_options {
                profile.set_document_options(document_options);
            }
            profile.validate().map_err(AppError::UnprocessableEntity)?;
            store
                .write_json(
                    "main",
                    GOVERNANCE_PROFILE_PATH,
                    &profile,
                    &format!("GOVERNANCE: update profile v{}", profile.version()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(profile)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(profile))
}

#[utoipa::path(
    post,
    path = "/v1/entities/{entity_id}/governance/doc-bundles/generate",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = GenerateGovernanceDocBundleRequest,
    responses(
        (status = 200, description = "Generated governance doc bundle", body = GenerateGovernanceDocBundleResponse),
        (status = 422, description = "Validation error"),
    ),
)]
async fn generate_governance_doc_bundle(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<GenerateGovernanceDocBundleRequest>,
) -> Result<Json<GenerateGovernanceDocBundleResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let profile = read_profile_or_default(&store, entity_id)?;
            profile.validate().map_err(AppError::UnprocessableEntity)?;

            let entity_type = governance_doc_entity_type_for(profile.entity_type());
            let template_version = req.template_version.unwrap_or_else(|| "v1".to_owned());
            let rendered =
                render_bundle_from_profile(entity_type, entity_id, &profile, &template_version)
                    .map_err(|e| {
                        AppError::Internal(format!("render governance doc bundle: {e:#}"))
                    })?;
            if !rendered.manifest.warnings.is_empty() {
                return Err(AppError::UnprocessableEntity(format!(
                    "governance profile is incomplete for production document generation: {}",
                    rendered.manifest.warnings.join("; ")
                )));
            }

            let bundle_id = rendered.manifest.bundle_id;
            let manifest_path = bundle_manifest_path(bundle_id);
            let history_path = bundle_history_path(bundle_id);
            let docs_prefix = bundle_documents_prefix(bundle_id);

            let mut files = Vec::with_capacity(rendered.documents.len() + 3);
            for doc in &rendered.documents {
                files.push(FileWrite::raw(
                    format!("{docs_prefix}/{}", doc.path),
                    doc.content.clone(),
                ));
            }
            files.push(
                FileWrite::json(manifest_path, &rendered.manifest)
                    .map_err(|e| AppError::Internal(format!("serialize manifest: {e}")))?,
            );
            files.push(
                FileWrite::json(GOVERNANCE_DOC_BUNDLES_CURRENT_PATH, &rendered.current)
                    .map_err(|e| AppError::Internal(format!("serialize current pointer: {e}")))?,
            );
            files.push(
                FileWrite::json(history_path, &rendered.summary)
                    .map_err(|e| AppError::Internal(format!("serialize summary: {e}")))?,
            );

            store
                .commit(
                    "main",
                    &format!("GOVERNANCE: generate doc bundle {bundle_id}"),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(GenerateGovernanceDocBundleResponse {
                manifest: rendered.manifest,
                current: rendered.current,
                summary: rendered.summary,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/doc-bundles/current",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Current governance doc bundle", body = GovernanceDocBundleCurrent),
        (status = 404, description = "No bundle generated yet"),
    ),
)]
async fn get_current_governance_doc_bundle(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<GovernanceDocBundleCurrent>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let current = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let current = read_doc_bundle_current(&store)?;
            if current.entity_id != entity_id {
                return Err(AppError::NotFound(format!(
                    "current governance doc bundle does not belong to entity {entity_id}"
                )));
            }
            Ok::<_, AppError>(current)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(current))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/doc-bundles",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance doc bundle summaries", body = Vec<GovernanceDocBundleSummary>),
    ),
)]
async fn list_governance_doc_bundles(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceDocBundleSummary>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let summaries = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let summaries = list_doc_bundle_summaries(&store)?;
            Ok::<_, AppError>(summaries)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(summaries))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/doc-bundles/{bundle_id}",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        ("bundle_id" = GovernanceDocBundleId, Path, description = "Bundle ID"),
    ),
    responses(
        (status = 200, description = "Governance doc bundle manifest", body = GovernanceDocBundleManifest),
        (status = 404, description = "Bundle not found"),
    ),
)]
async fn get_governance_doc_bundle(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((entity_id, bundle_id)): Path<(EntityId, GovernanceDocBundleId)>,
) -> Result<Json<GovernanceDocBundleManifest>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let manifest = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let path = bundle_manifest_path(bundle_id);
            let manifest = store
                .read_json::<GovernanceDocBundleManifest>("main", &path)
                .map_err(|e| match e {
                    GitStorageError::NotFound(_) => {
                        AppError::NotFound(format!("governance doc bundle {bundle_id} not found"))
                    }
                    other => AppError::Internal(format!("read governance doc bundle: {other}")),
                })?;
            if manifest.entity_id != entity_id {
                return Err(AppError::NotFound(format!(
                    "governance doc bundle {bundle_id} not found for entity {entity_id}"
                )));
            }
            Ok::<_, AppError>(manifest)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(manifest))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/triggers",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance trigger events", body = Vec<GovernanceTriggerEvent>),
    ),
)]
async fn list_governance_triggers(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceTriggerEvent>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let triggers = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<GovernanceTriggerEvent>("main")
                .map_err(|e| AppError::Internal(format!("list governance triggers: {e}")))?;
            let mut out = Vec::new();
            for id in ids {
                let trigger = store
                    .read::<GovernanceTriggerEvent>("main", id)
                    .map_err(|e| {
                        AppError::Internal(format!("read governance trigger {id}: {e}"))
                    })?;
                if trigger.entity_id() == entity_id {
                    out.push(trigger);
                }
            }
            out.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
            Ok::<_, AppError>(out)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(triggers))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/mode-history",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance mode change events", body = Vec<GovernanceModeChangeEvent>),
    ),
)]
async fn list_governance_mode_history(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceModeChangeEvent>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let events = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<GovernanceModeChangeEvent>("main")
                .map_err(|e| AppError::Internal(format!("list governance mode history: {e}")))?;
            let mut out = Vec::new();
            for id in ids {
                let event = store
                    .read::<GovernanceModeChangeEvent>("main", id)
                    .map_err(|e| {
                        AppError::Internal(format!("read governance mode event {id}: {e}"))
                    })?;
                if event.entity_id() == entity_id {
                    out.push(event);
                }
            }
            out.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
            Ok::<_, AppError>(out)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(events))
}

#[utoipa::path(
    post,
    path = "/v1/internal/workspaces/{workspace_id}/entities/{entity_id}/governance/triggers/lockdown",
    tag = "governance",
    params(
        ("workspace_id" = WorkspaceId, Path, description = "Workspace ID"),
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = InternalLockdownTriggerRequest,
    responses(
        (status = 200, description = "Lockdown trigger result", body = InternalLockdownTriggerResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn ingest_lockdown_trigger(
    _worker: RequireInternalWorker,
    State(state): State<AppState>,
    Path((workspace_id, entity_id)): Path<(WorkspaceId, EntityId)>,
    Json(req): Json<InternalLockdownTriggerRequest>,
) -> Result<Json<InternalLockdownTriggerResponse>, AppError> {
    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, None, entity_id, valkey_client.as_ref())?;
            let result = apply_lockdown_trigger(
                &store,
                entity_id,
                LockdownTriggerInput {
                    source: GovernanceTriggerSource::ExternalIngestion,
                    trigger_type: req.trigger_type,
                    severity: req.severity,
                    title: req.title,
                    description: req.description,
                    evidence_refs: req.evidence_refs,
                    linked_intent_id: req.linked_intent_id,
                    linked_escalation_id: req.linked_escalation_id,
                    idempotency_key: Some(req.idempotency_key),
                    existing_incident_id: None,
                    updated_by: None,
                },
            )?;
            Ok::<_, AppError>(InternalLockdownTriggerResponse {
                trigger_id: result.trigger.trigger_id(),
                incident_id: result.incident.incident_id(),
                mode: result.mode.mode(),
                mode_event_id: result.mode_event.mode_event_id(),
                idempotent_replay: result.idempotent_replay,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/audit/entries",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance audit entries", body = Vec<GovernanceAuditEntry>),
    ),
)]
async fn list_governance_audit_entries(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceAuditEntry>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let entries = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let mut entries = list_audit_entries_sorted(&store, entity_id)?;
            entries.reverse();
            Ok::<_, AppError>(entries)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(entries))
}

#[utoipa::path(
    post,
    path = "/v1/governance/audit/events",
    tag = "governance",
    request_body = CreateGovernanceAuditEventRequest,
    responses(
        (status = 200, description = "Created governance audit entry", body = GovernanceAuditEntry),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_governance_audit_event(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateGovernanceAuditEventRequest>,
) -> Result<Json<GovernanceAuditEntry>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    if req.action.trim().is_empty() {
        return Err(AppError::BadRequest(
            "governance audit event action must not be empty".to_owned(),
        ));
    }

    let entry = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let entry = build_audit_entry(
                &store,
                entity_id,
                BuildAuditEntryInput {
                    event_type: req.event_type,
                    action: req.action,
                    details: req.details,
                    evidence_refs: req.evidence_refs,
                    linked_intent_id: req.linked_intent_id,
                    linked_incident_id: req.linked_incident_id,
                    linked_trigger_id: req.linked_trigger_id,
                    linked_mode_event_id: req.linked_mode_event_id,
                },
            )?;
            store
                .write_json(
                    "main",
                    &audit_entry_path(entry.audit_entry_id()),
                    &entry,
                    &format!("GOVERNANCE: append audit entry {}", entry.audit_entry_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(entry)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(entry))
}

#[utoipa::path(
    post,
    path = "/v1/governance/audit/checkpoints",
    tag = "governance",
    request_body = WriteGovernanceAuditCheckpointRequest,
    responses(
        (status = 200, description = "Written governance audit checkpoint", body = GovernanceAuditCheckpoint),
        (status = 422, description = "No audit entries to checkpoint"),
    ),
)]
async fn write_governance_audit_checkpoint(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<WriteGovernanceAuditCheckpointRequest>,
) -> Result<Json<GovernanceAuditCheckpoint>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let checkpoint = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let entries = list_audit_entries_sorted(&store, entity_id)?;
            let latest = entries.last().ok_or_else(|| {
                AppError::UnprocessableEntity(
                    "cannot checkpoint governance audit chain without entries".to_owned(),
                )
            })?;
            let total_entries = u64::try_from(entries.len())
                .map_err(|_| AppError::Internal("audit entry count overflow".to_owned()))?;
            let checkpoint = GovernanceAuditCheckpoint::new(
                GovernanceAuditCheckpointId::new(),
                entity_id,
                latest.audit_entry_id(),
                latest.entry_hash().to_owned(),
                total_entries,
            );
            let checkpoint_audit_entry = build_audit_entry(
                &store,
                entity_id,
                BuildAuditEntryInput {
                    event_type: GovernanceAuditEventType::CheckpointWritten,
                    action: "governance audit checkpoint written".to_owned(),
                    details: serde_json::json!({
                        "checkpoint_id": checkpoint.checkpoint_id(),
                        "latest_entry_id": checkpoint.latest_entry_id(),
                        "latest_entry_hash": checkpoint.latest_entry_hash(),
                        "total_entries": checkpoint.total_entries(),
                    }),
                    evidence_refs: Vec::new(),
                    linked_intent_id: None,
                    linked_incident_id: None,
                    linked_trigger_id: None,
                    linked_mode_event_id: None,
                },
            )?;

            store
                .commit(
                    "main",
                    &format!(
                        "GOVERNANCE: write audit checkpoint {}",
                        checkpoint.checkpoint_id()
                    ),
                    vec![
                        FileWrite::json(
                            audit_checkpoint_path(checkpoint.checkpoint_id()),
                            &checkpoint,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize checkpoint: {e}")))?,
                        FileWrite::json(
                            audit_entry_path(checkpoint_audit_entry.audit_entry_id()),
                            &checkpoint_audit_entry,
                        )
                        .map_err(|e| {
                            AppError::Internal(format!("serialize governance audit entry: {e}"))
                        })?,
                    ],
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(checkpoint)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(checkpoint))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/audit/checkpoints",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance audit checkpoints", body = Vec<GovernanceAuditCheckpoint>),
    ),
)]
async fn list_governance_audit_checkpoints(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceAuditCheckpoint>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let checkpoints = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<GovernanceAuditCheckpoint>("main")
                .map_err(|e| {
                    AppError::Internal(format!("list governance audit checkpoints: {e}"))
                })?;
            let mut checkpoints = Vec::new();
            for id in ids {
                let checkpoint = store
                    .read::<GovernanceAuditCheckpoint>("main", id)
                    .map_err(|e| {
                        AppError::Internal(format!("read governance audit checkpoint {id}: {e}"))
                    })?;
                if checkpoint.entity_id() == entity_id {
                    checkpoints.push(checkpoint);
                }
            }
            checkpoints.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
            Ok::<_, AppError>(checkpoints)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(checkpoints))
}

#[utoipa::path(
    post,
    path = "/v1/governance/audit/verify",
    tag = "governance",
    request_body = VerifyGovernanceAuditChainRequest,
    responses(
        (status = 200, description = "Governance audit chain verification report", body = GovernanceAuditVerificationReport),
    ),
)]
async fn verify_governance_audit_chain(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<VerifyGovernanceAuditChainRequest>,
) -> Result<Json<GovernanceAuditVerificationReport>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let report = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let entries = list_audit_entries_sorted(&store, entity_id)?;
            let total_entries = u64::try_from(entries.len())
                .map_err(|_| AppError::Internal("audit entry count overflow".to_owned()))?;
            let latest_entry_hash = entries.last().map(|entry| entry.entry_hash().to_owned());

            let mut anomalies = Vec::new();
            let mut expected_previous_hash: Option<String> = None;
            for (index, entry) in entries.iter().enumerate() {
                if entry.previous_entry_hash().map(ToOwned::to_owned) != expected_previous_hash {
                    anomalies.push(format!(
                        "entry {} previous hash mismatch at index {}",
                        entry.audit_entry_id(),
                        index
                    ));
                }
                if !entry.verify_integrity() {
                    anomalies.push(format!(
                        "entry {} hash verification failed",
                        entry.audit_entry_id()
                    ));
                }
                expected_previous_hash = Some(entry.entry_hash().to_owned());
            }

            let ok = anomalies.is_empty();
            let mut triggered_lockdown = false;
            let mut trigger_id = None;
            let mut incident_id = None;

            if !ok {
                let idempotency_suffix = latest_entry_hash
                    .clone()
                    .unwrap_or_else(|| "none".to_owned());
                let trigger_result = apply_lockdown_trigger(
                    &store,
                    entity_id,
                    LockdownTriggerInput {
                        source: GovernanceTriggerSource::ExecutionGate,
                        trigger_type: GovernanceTriggerType::AuditChainVerificationFailed,
                        severity: IncidentSeverity::Critical,
                        title: "Governance audit chain verification failed".to_owned(),
                        description: format!(
                            "detected {} anomaly/anomalies in governance audit chain",
                            anomalies.len()
                        ),
                        evidence_refs: vec![format!("governance-audit-chain:{idempotency_suffix}")],
                        linked_intent_id: None,
                        linked_escalation_id: None,
                        idempotency_key: Some(format!(
                            "audit-chain-verification:{idempotency_suffix}"
                        )),
                        existing_incident_id: None,
                        updated_by: None,
                    },
                )?;
                triggered_lockdown = true;
                trigger_id = Some(trigger_result.trigger.trigger_id());
                incident_id = Some(trigger_result.incident.incident_id());
            }

            let report = GovernanceAuditVerificationReport::new(
                GovernanceAuditVerificationId::new(),
                entity_id,
                ok,
                total_entries,
                anomalies.clone(),
                latest_entry_hash.clone(),
                triggered_lockdown,
                trigger_id,
                incident_id,
            );
            let verification_audit_entry = build_audit_entry(
                &store,
                entity_id,
                BuildAuditEntryInput {
                    event_type: if ok {
                        GovernanceAuditEventType::ChainVerified
                    } else {
                        GovernanceAuditEventType::ChainVerificationFailed
                    },
                    action: if ok {
                        "governance audit chain verification passed".to_owned()
                    } else {
                        "governance audit chain verification failed".to_owned()
                    },
                    details: serde_json::json!({
                        "verification_id": report.verification_id(),
                        "ok": report.ok(),
                        "total_entries": report.total_entries(),
                        "anomaly_count": report.anomalies().len(),
                    }),
                    evidence_refs: Vec::new(),
                    linked_intent_id: None,
                    linked_incident_id: report.incident_id(),
                    linked_trigger_id: report.trigger_id(),
                    linked_mode_event_id: None,
                },
            )?;

            store
                .commit(
                    "main",
                    &format!(
                        "GOVERNANCE: verify audit chain {}",
                        report.verification_id()
                    ),
                    vec![
                        FileWrite::json(audit_verification_path(report.verification_id()), &report)
                            .map_err(|e| {
                                AppError::Internal(format!("serialize verification: {e}"))
                            })?,
                        FileWrite::json(
                            audit_entry_path(verification_audit_entry.audit_entry_id()),
                            &verification_audit_entry,
                        )
                        .map_err(|e| {
                            AppError::Internal(format!("serialize governance audit entry: {e}"))
                        })?,
                    ],
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(report)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(report))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/audit/verifications",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance audit verification reports", body = Vec<GovernanceAuditVerificationReport>),
    ),
)]
async fn list_governance_audit_verifications(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceAuditVerificationReport>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let reports = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<GovernanceAuditVerificationReport>("main")
                .map_err(|e| {
                    AppError::Internal(format!("list governance audit verifications: {e}"))
                })?;
            let mut reports = Vec::new();
            for id in ids {
                let report = store
                    .read::<GovernanceAuditVerificationReport>("main", id)
                    .map_err(|e| {
                        AppError::Internal(format!("read governance audit verification {id}: {e}"))
                    })?;
                if report.entity_id() == entity_id {
                    reports.push(report);
                }
            }
            reports.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
            Ok::<_, AppError>(reports)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(reports))
}

// ── Handlers: Governance bodies ──────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/governance-bodies",
    tag = "governance",
    request_body = CreateGovernanceBodyRequest,
    responses(
        (status = 200, description = "Created governance body", body = GovernanceBodyResponse),
    ),
)]
async fn create_governance_body(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateGovernanceBodyRequest>,
) -> Result<Json<GovernanceBodyResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("governance.body.create", workspace_id, 120, 60)?;

    let body = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "governance body creation")?;
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            validate_body_type_for_entity(entity.entity_type(), req.body_type)?;
            let body_ids = store
                .list_ids::<GovernanceBody>("main")
                .map_err(|e| AppError::Internal(format!("list governance bodies: {e}")))?;
            let normalized_name = req.name.trim().to_ascii_lowercase();
            for existing_id in body_ids {
                let existing = store
                    .read::<GovernanceBody>("main", existing_id)
                    .map_err(|e| {
                        AppError::Internal(format!("read governance body {existing_id}: {e}"))
                    })?;
                if existing.entity_id() == entity_id
                    && existing.body_type() == req.body_type
                    && existing.status() == BodyStatus::Active
                    && existing.name().trim().to_ascii_lowercase() == normalized_name
                {
                    return Err(AppError::Conflict(format!(
                        "active governance body already exists for {} (body_id: {}). Use: corp governance seats {}",
                        req.name, existing_id, existing_id
                    )));
                }
            }

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(body_to_response(&body)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance-bodies",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance bodies for entity", body = Vec<GovernanceBodyResponse>),
    ),
)]
async fn list_governance_bodies(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<GovernanceBodyResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let bodies = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<GovernanceBody>("main")
                .map_err(|e| AppError::Internal(format!("list governance bodies: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let b = store
                    .read::<GovernanceBody>("main", id)
                    .map_err(|e| AppError::Internal(format!("read governance body {id}: {e}")))?;
                // Filter to bodies belonging to this entity
                if b.entity_id() == entity_id {
                    results.push(body_to_response(&b));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(bodies))
}

// ── Handlers: Governance mode + incidents ───────────────────────────

#[utoipa::path(
    get,
    path = "/v1/governance/mode",
    tag = "governance",
    params(super::EntityIdQuery),
    responses(
        (status = 200, description = "Current governance mode", body = GovernanceModeResponse),
    ),
)]
async fn get_governance_mode(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<GovernanceModeResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let mode = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            Ok::<_, AppError>(read_mode_or_default(&store, entity_id))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(mode_to_response(&mode)))
}

#[utoipa::path(
    post,
    path = "/v1/governance/mode",
    tag = "governance",
    request_body = SetGovernanceModeRequest,
    responses(
        (status = 200, description = "Updated governance mode", body = GovernanceModeResponse),
        (status = 422, description = "Validation error"),
    ),
)]
async fn set_governance_mode(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<SetGovernanceModeRequest>,
) -> Result<Json<GovernanceModeResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    let updated_by = auth.contact_id();

    let mode = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let current = read_mode_or_default(&store, entity_id);

            if matches!(current.mode(), GovernanceMode::IncidentLockdown)
                && matches!(req.mode, GovernanceMode::Normal)
            {
                let has_reason = req
                    .reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty());
                if !has_reason {
                    return Err(AppError::UnprocessableEntity(
                        "unlocking from incident_lockdown requires a non-empty reason".to_owned(),
                    ));
                }
                if req.incident_ids.is_empty() {
                    return Err(AppError::UnprocessableEntity(
                        "unlocking from incident_lockdown requires incident_ids".to_owned(),
                    ));
                }
            }

            let result = set_mode_with_history(
                &store,
                entity_id,
                SetModeWithHistoryInput {
                    target_mode: req.mode,
                    reason: req.reason,
                    incident_ids: req.incident_ids,
                    evidence_refs: req.evidence_refs,
                    trigger_id: None,
                    updated_by,
                    commit_message: format!("GOVERNANCE: set mode {:?}", req.mode),
                },
            )?;
            Ok::<_, AppError>(result.mode)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(mode_to_response(&mode)))
}

#[utoipa::path(
    post,
    path = "/v1/governance/incidents",
    tag = "governance",
    request_body = CreateIncidentRequest,
    responses(
        (status = 200, description = "Created governance incident", body = IncidentResponse),
    ),
)]
async fn create_incident(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateIncidentRequest>,
) -> Result<Json<IncidentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let incident = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let incident = GovernanceIncident::new(
                IncidentId::new(),
                entity_id,
                req.severity,
                req.title,
                req.description,
            );
            let path = format!("governance/incidents/{}.json", incident.incident_id());
            store
                .write_json(
                    "main",
                    &path,
                    &incident,
                    &format!("GOVERNANCE: create incident {}", incident.incident_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(incident)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(incident_to_response(&incident)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance/incidents",
    tag = "governance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance incidents", body = Vec<IncidentResponse>),
    ),
)]
async fn list_incidents(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<IncidentResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let incidents = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<GovernanceIncident>("main")
                .map_err(|e| AppError::Internal(format!("list incidents: {e}")))?;
            let mut out = Vec::new();
            for id in ids {
                let incident = store
                    .read::<GovernanceIncident>("main", id)
                    .map_err(|e| AppError::Internal(format!("read incident {id}: {e}")))?;
                out.push(incident_to_response(&incident));
            }
            out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok::<_, AppError>(out)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(incidents))
}

#[utoipa::path(
    post,
    path = "/v1/governance/incidents/{incident_id}/resolve",
    tag = "governance",
    params(
        ("incident_id" = IncidentId, Path, description = "Incident ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Resolved incident", body = IncidentResponse),
        (status = 404, description = "Incident not found"),
    ),
)]
async fn resolve_incident(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(incident_id): Path<IncidentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<IncidentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let incident = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let mut incident = store
                .read::<GovernanceIncident>("main", incident_id)
                .map_err(|_| AppError::NotFound(format!("incident {incident_id} not found")))?;
            incident.resolve();
            let path = format!("governance/incidents/{}.json", incident_id);
            store
                .write_json(
                    "main",
                    &path,
                    &incident,
                    &format!("GOVERNANCE: resolve incident {incident_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(incident)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(incident_to_response(&incident)))
}

// ── Handlers: Delegation schedule ───────────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/governance/delegation-schedule",
    tag = "governance",
    params(super::EntityIdQuery),
    responses(
        (status = 200, description = "Current delegation schedule", body = DelegationSchedule),
    ),
)]
async fn get_delegation_schedule(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<DelegationSchedule>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let schedule = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            Ok::<_, AppError>(read_schedule_or_default(&store, entity_id))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(schedule))
}

#[utoipa::path(
    post,
    path = "/v1/governance/delegation-schedule/amend",
    tag = "governance",
    request_body = AmendDelegationScheduleRequest,
    responses(
        (status = 200, description = "Amended delegation schedule", body = DelegationScheduleChangeResponse),
        (status = 400, description = "Invalid request"),
        (status = 422, description = "Authority expansion requires resolution"),
    ),
)]
async fn amend_delegation_schedule(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<AmendDelegationScheduleRequest>,
) -> Result<Json<DelegationScheduleChangeResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let current = read_schedule_or_default(&store, entity_id);
            let mut amended = current.clone();

            if let Some(amount) = req.tier1_max_amount_cents {
                amended.set_tier1_max_amount_cents(amount);
            }
            if let Some(intents) = req.allowed_tier1_intent_types {
                amended.set_allowed_tier1_intent_types(normalize_tier1_intents(intents));
            }
            if let Some(next_review) = req.next_mandatory_review_at {
                amended.set_next_mandatory_review_at(next_review);
            }

            if req.meeting_id.is_some() ^ req.adopted_resolution_id.is_some() {
                return Err(AppError::BadRequest(
                    "meeting_id and adopted_resolution_id must be provided together".to_owned(),
                ));
            }

            let prev_allows_all = current.allowed_tier1_intent_types().is_empty();
            let next_allows_all = amended.allowed_tier1_intent_types().is_empty();
            let cap_expansion =
                amended.tier1_max_amount_cents() > current.tier1_max_amount_cents();
            let lane_expansion = if prev_allows_all {
                false
            } else if next_allows_all {
                true
            } else {
                !current
                    .added_tier1_intents(amended.allowed_tier1_intent_types())
                    .is_empty()
            };
            let authority_expansion = cap_expansion || lane_expansion;

            let linked_resolution = match (req.meeting_id, req.adopted_resolution_id) {
                (Some(meeting_id), Some(resolution_id)) => {
                    validate_schedule_resolution(&store, meeting_id, resolution_id)?;
                    Some(resolution_id)
                }
                (None, None) => None,
                _ => unreachable!("handled by xor guard"),
            };
            if authority_expansion && linked_resolution.is_none() {
                return Err(AppError::UnprocessableEntity(
                    "authority-expanding delegation changes require a passed board/member resolution"
                        .to_owned(),
                ));
            }

            if let Some(resolution_id) = linked_resolution {
                amended.set_adopted_resolution_id(Some(resolution_id));
            }

            let added_intents = if prev_allows_all {
                Vec::new()
            } else if next_allows_all {
                Vec::new()
            } else {
                current.added_tier1_intents(amended.allowed_tier1_intent_types())
            };
            let removed_intents = if prev_allows_all {
                Vec::new()
            } else if next_allows_all {
                current.allowed_tier1_intent_types().to_vec()
            } else {
                current.removed_tier1_intents(amended.allowed_tier1_intent_types())
            };

            amended.bump_version();
            let amendment = ScheduleAmendment::new(
                ScheduleAmendmentId::new(),
                entity_id,
                current.version(),
                amended.version(),
                current.tier1_max_amount_cents(),
                amended.tier1_max_amount_cents(),
                added_intents,
                removed_intents,
                authority_expansion,
                linked_resolution,
                req.rationale,
            );

            let amendment_path = format!(
                "governance/delegation-schedule/amendments/{}.json",
                amendment.schedule_amendment_id()
            );
            let files = vec![
                FileWrite::json(CURRENT_SCHEDULE_PATH, &amended)
                    .map_err(|e| AppError::Internal(format!("serialize schedule: {e}")))?,
                FileWrite::json(amendment_path, &amendment)
                    .map_err(|e| AppError::Internal(format!("serialize amendment: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "GOVERNANCE: amend delegation schedule v{}->v{}",
                        current.version(),
                        amended.version()
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(DelegationScheduleChangeResponse {
                schedule: amended,
                amendment,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1/governance/delegation-schedule/reauthorize",
    tag = "governance",
    request_body = ReauthorizeDelegationScheduleRequest,
    responses(
        (status = 200, description = "Reauthorized delegation schedule", body = DelegationScheduleChangeResponse),
        (status = 404, description = "Meeting or resolution not found"),
        (status = 422, description = "Resolution did not pass"),
    ),
)]
async fn reauthorize_delegation_schedule(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<ReauthorizeDelegationScheduleRequest>,
) -> Result<Json<DelegationScheduleChangeResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            validate_schedule_resolution(&store, req.meeting_id, req.adopted_resolution_id)?;

            let current = read_schedule_or_default(&store, entity_id);
            let mut schedule = current.clone();
            schedule.bump_version();
            schedule.reauthorize(req.adopted_resolution_id);

            let amendment = ScheduleAmendment::new(
                ScheduleAmendmentId::new(),
                entity_id,
                current.version(),
                schedule.version(),
                current.tier1_max_amount_cents(),
                schedule.tier1_max_amount_cents(),
                Vec::new(),
                Vec::new(),
                false,
                Some(req.adopted_resolution_id),
                req.rationale,
            );

            let amendment_path = format!(
                "governance/delegation-schedule/amendments/{}.json",
                amendment.schedule_amendment_id()
            );
            let files = vec![
                FileWrite::json(CURRENT_SCHEDULE_PATH, &schedule)
                    .map_err(|e| AppError::Internal(format!("serialize schedule: {e}")))?,
                FileWrite::json(amendment_path, &amendment)
                    .map_err(|e| AppError::Internal(format!("serialize amendment: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "GOVERNANCE: reauthorize delegation schedule v{}->v{}",
                        current.version(),
                        schedule.version()
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(DelegationScheduleChangeResponse {
                schedule,
                amendment,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1/governance/delegation-schedule/history",
    tag = "governance",
    params(super::EntityIdQuery),
    responses(
        (status = 200, description = "List of delegation schedule amendments", body = Vec<ScheduleAmendment>),
    ),
)]
async fn list_delegation_schedule_history(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<ScheduleAmendment>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let history = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<ScheduleAmendment>("main")
                .map_err(|e| AppError::Internal(format!("list schedule amendments: {e}")))?;
            let mut amendments = Vec::new();
            for id in ids {
                let amendment = store
                    .read::<ScheduleAmendment>("main", id)
                    .map_err(|e| AppError::Internal(format!("read amendment {id}: {e}")))?;
                amendments.push(amendment);
            }
            amendments.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
            Ok::<_, AppError>(amendments)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(history))
}

// ── Handlers: Policy evaluation (dry-run) ───────────────────────────

#[utoipa::path(
    post,
    path = "/v1/governance/evaluate",
    tag = "governance",
    request_body = EvaluateGovernanceRequest,
    responses(
        (status = 200, description = "Policy evaluation decision", body = PolicyDecision),
    ),
)]
async fn evaluate_governance(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Json(req): Json<EvaluateGovernanceRequest>,
) -> Result<Json<PolicyDecision>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let decision = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let mode = read_mode_or_default(&store, entity_id);
            let schedule = read_schedule_or_default(&store, entity_id);
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;

            let ctx = PolicyEvaluationContext {
                intent_type: &req.intent_type,
                metadata: &req.metadata,
                mode: mode.mode(),
                schedule: &schedule,
                now: Utc::now(),
                entity_is_active: matches!(entity.formation_status(), FormationStatus::Active),
                service_agreement_executed: entity.service_agreement_executed(),
            };

            Ok::<_, AppError>(evaluate_full(&ctx))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(decision))
}

// ── Handlers: Governance seats ───────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/governance-bodies/{body_id}/seats",
    tag = "governance",
    params(
        ("body_id" = GovernanceBodyId, Path, description = "Governance body ID"),
        BodyQuery,
    ),
    request_body = CreateSeatRequest,
    responses(
        (status = 200, description = "Created governance seat", body = GovernanceSeatResponse),
        (status = 404, description = "Governance body not found"),
    ),
)]
async fn create_seat(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(body_id): Path<GovernanceBodyId>,
    Query(query): Query<BodyQuery>,
    Json(req): Json<CreateSeatRequest>,
) -> Result<Json<GovernanceSeatResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;
    state.enforce_creation_rate_limit("governance.seat.create", workspace_id, 120, 60)?;

    let seat = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "seat appointment")?;

            let body = store.read::<GovernanceBody>("main", body_id).map_err(|_| {
                AppError::NotFound(format!("governance body {} not found", body_id))
            })?;
            if body.entity_id() != entity_id {
                return Err(AppError::BadRequest(
                    "governance body does not belong to entity".to_owned(),
                ));
            }
            if body.status() != BodyStatus::Active {
                return Err(AppError::BadRequest(
                    "governance body is inactive".to_owned(),
                ));
            }
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            validate_body_type_for_entity(entity.entity_type(), body.body_type())?;
            let holder = store
                .read::<Contact>("main", req.holder_id)
                .map_err(|_| AppError::NotFound(format!("contact {} not found", req.holder_id)))?;
            if holder.entity_id() != entity_id {
                return Err(AppError::BadRequest(
                    "seat holder must belong to the same entity".to_owned(),
                ));
            }
            validate_holder_for_body(&store, &body, &holder)?;
            if has_active_seat(&store, body_id, req.holder_id)? {
                return Err(AppError::Conflict(format!(
                    "contact {} already has an active seat on body {}",
                    req.holder_id, body_id
                )));
            }

            let seat_id = GovernanceSeatId::new();
            let voting_power = req
                .voting_power
                .map(VotingPower::new)
                .transpose()
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(seat_to_response(&seat)))
}

#[utoipa::path(
    get,
    path = "/v1/governance-bodies/{body_id}/seats",
    tag = "governance",
    params(
        ("body_id" = GovernanceBodyId, Path, description = "Governance body ID"),
        BodyQuery,
    ),
    responses(
        (status = 200, description = "List of governance seats for body", body = Vec<GovernanceSeatResponse>),
    ),
)]
async fn list_seats(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(body_id): Path<GovernanceBodyId>,
    Query(query): Query<BodyQuery>,
) -> Result<Json<Vec<GovernanceSeatResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let seats = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let body = store.read::<GovernanceBody>("main", body_id).map_err(|_| {
                AppError::NotFound(format!("governance body {} not found", body_id))
            })?;
            if body.entity_id() != entity_id {
                return Err(AppError::BadRequest(
                    "governance body does not belong to entity".to_owned(),
                ));
            }
            let ids = store
                .list_ids::<GovernanceSeat>("main")
                .map_err(|e| AppError::Internal(format!("list governance seats: {e}")))?;

            // Build contact name lookup for holder_name
            let contact_ids = store.list_ids::<Contact>("main").unwrap_or_default();
            let contact_names: std::collections::HashMap<ContactId, String> = contact_ids
                .into_iter()
                .filter_map(|cid| {
                    store.read::<Contact>("main", cid).ok().map(|c| (c.contact_id(), c.name().to_owned()))
                })
                .collect();

            let mut results = Vec::new();
            for id in ids {
                let s = store
                    .read::<GovernanceSeat>("main", id)
                    .map_err(|e| AppError::Internal(format!("read governance seat {id}: {e}")))?;
                if s.body_id() == body_id {
                    let mut resp = seat_to_response(&s);
                    resp.holder_name = contact_names.get(&s.holder_id()).cloned();
                    results.push(resp);
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(seats))
}

#[utoipa::path(
    post,
    path = "/v1/governance-seats/{seat_id}/resign",
    tag = "governance",
    params(
        ("seat_id" = GovernanceSeatId, Path, description = "Governance seat ID"),
        BodyQuery,
    ),
    responses(
        (status = 200, description = "Resigned governance seat", body = GovernanceSeatResponse),
        (status = 404, description = "Seat not found"),
    ),
)]
async fn resign_seat(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(seat_id): Path<GovernanceSeatId>,
    Query(query): Query<BodyQuery>,
) -> Result<Json<GovernanceSeatResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let seat = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(seat_to_response(&seat)))
}

// ── Handlers: Meetings ───────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/meetings",
    tag = "governance",
    request_body = ScheduleMeetingRequest,
    responses(
        (status = 200, description = "Scheduled meeting", body = MeetingResponse),
        (status = 404, description = "Governance body not found"),
    ),
)]
async fn schedule_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<ScheduleMeetingRequest>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    let title = require_non_empty_trimmed(&req.title, "title")?;
    if req.meeting_type != MeetingType::WrittenConsent && req.scheduled_date.is_none() {
        return Err(AppError::BadRequest(
            "scheduled_date is required for scheduled meetings".to_owned(),
        ));
    }
    if let Some(scheduled_date) = req.scheduled_date {
        if scheduled_date < Utc::now().date_naive() {
            return Err(AppError::BadRequest(
                "scheduled_date cannot be in the past".to_owned(),
            ));
        }
        validate_not_too_far_future("scheduled_date", scheduled_date, 730)?;
    }

    let (meeting, agenda_item_ids) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let title = title.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "meeting scheduling")?;

            // Verify body exists
            let body = store
                .read::<GovernanceBody>("main", req.body_id)
                .map_err(|_| {
                    AppError::NotFound(format!("governance body {} not found", req.body_id))
                })?;
            validate_meeting_type_for_body(body.body_type(), req.meeting_type)?;

            let meeting_id = MeetingId::new();
            let meeting = Meeting::new(
                meeting_id,
                req.body_id,
                req.meeting_type,
                title,
                req.scheduled_date,
                req.location.unwrap_or_default(),
                req.notice_days.unwrap_or(10),
            );

            // Build all files to commit atomically: meeting + agenda items
            let mut files = vec![
                FileWrite::json(
                    format!("governance/meetings/{}/meeting.json", meeting_id),
                    &meeting,
                )
                .map_err(|e| AppError::Internal(format!("serialize meeting: {e}")))?,
            ];

            let mut agenda_item_ids = Vec::new();
            for (i, title) in req.agenda_item_titles.iter().enumerate() {
                let item_id = AgendaItemId::new();
                let item = AgendaItem::new(
                    item_id,
                    meeting_id,
                    u32::try_from(i + 1)
                        .map_err(|_| AppError::BadRequest("too many agenda items".to_owned()))?,
                    title.clone(),
                    None,
                    AgendaItemType::Resolution,
                );
                files.push(
                    FileWrite::json(
                        format!("governance/meetings/{}/agenda/{}.json", meeting_id, item_id),
                        &item,
                    )
                    .map_err(|e| AppError::Internal(format!("serialize agenda item: {e}")))?,
                );
                agenda_item_ids.push(item_id);
            }

            store
                .commit("main", &format!("Schedule meeting {meeting_id}"), files)
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>((meeting, agenda_item_ids))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(MeetingResponse {
        agenda_item_ids,
        ..meeting_to_response(&meeting)
    }))
}

#[utoipa::path(
    get,
    path = "/v1/governance-bodies/{body_id}/meetings",
    tag = "governance",
    params(
        ("body_id" = GovernanceBodyId, Path, description = "Governance body ID"),
        BodyQuery,
    ),
    responses(
        (status = 200, description = "List of meetings for body", body = Vec<MeetingResponse>),
    ),
)]
async fn list_meetings(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(body_id): Path<GovernanceBodyId>,
    Query(query): Query<BodyQuery>,
) -> Result<Json<Vec<MeetingResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let meetings = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let body = store.read::<GovernanceBody>("main", body_id).map_err(|_| {
                AppError::NotFound(format!("governance body {} not found", body_id))
            })?;
            if body.entity_id() != entity_id {
                return Err(AppError::BadRequest(
                    "governance body does not belong to entity".to_owned(),
                ));
            }
            let ids = store
                .list_ids::<Meeting>("main")
                .map_err(|e| AppError::Internal(format!("list meetings: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let m = store
                    .read::<Meeting>("main", id)
                    .map_err(|e| AppError::Internal(format!("read meeting {id}: {e}")))?;
                if m.body_id() == body_id {
                    results.push(meeting_to_response(&m));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(meetings))
}

#[utoipa::path(
    get,
    path = "/v1/meetings/{meeting_id}/agenda-items",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        MeetingQuery,
    ),
    responses(
        (status = 200, description = "List of agenda items for meeting", body = Vec<AgendaItemResponse>),
        (status = 404, description = "Meeting not found"),
    ),
)]
async fn list_agenda_items(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<Vec<AgendaItemResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let agenda_items = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            // Ensure the meeting exists and belongs to this entity's store.
            store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

            let ids = store
                .list_agenda_item_ids("main", meeting_id)
                .map_err(|e| AppError::Internal(format!("list agenda items: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(item) = store.read_agenda_item("main", meeting_id, id) {
                    results.push(agenda_item_to_response(&item));
                }
            }
            results.sort_by_key(|i| i.sequence_number);
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(agenda_items))
}

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/notice",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        MeetingQuery,
    ),
    responses(
        (status = 200, description = "Meeting with notice sent", body = MeetingResponse),
        (status = 404, description = "Meeting not found"),
    ),
)]
async fn send_notice(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "meeting notice")?;
            let mut meeting = store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(meeting_to_response(&meeting)))
}

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/convene",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        MeetingQuery,
    ),
    request_body = ConveneMeetingRequest,
    responses(
        (status = 200, description = "Convened meeting", body = MeetingResponse),
        (status = 404, description = "Meeting not found"),
    ),
)]
async fn convene_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
    Json(req): Json<ConveneMeetingRequest>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "meeting convening")?;
            let mut meeting = store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

            // Read the body to get quorum rule
            let body = store
                .read::<GovernanceBody>("main", meeting.body_id())
                .map_err(|_| {
                    AppError::NotFound(format!("governance body {} not found", meeting.body_id()))
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
            let quorum_met = body.quorum_rule().is_met(present_count, total_eligible);

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(meeting_to_response(&meeting)))
}

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/adjourn",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        MeetingQuery,
    ),
    responses(
        (status = 200, description = "Adjourned meeting", body = MeetingResponse),
        (status = 404, description = "Meeting not found"),
    ),
)]
async fn adjourn_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "meeting adjournment")?;
            let mut meeting = store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

            // Recompute quorum before adjourning.
            // For written consents (which skip convene), quorum was never set.
            // For regular meetings, re-derive from current vote counts.
            let body = store
                .read::<GovernanceBody>("main", meeting.body_id())
                .map_err(|_| {
                    AppError::NotFound(format!("governance body {} not found", meeting.body_id()))
                })?;
            let seat_ids = store.list_ids::<GovernanceSeat>("main").unwrap_or_default();
            let mut total_eligible: u32 = 0;
            for id in &seat_ids {
                if let Ok(s) = store.read::<GovernanceSeat>("main", *id) {
                    if s.body_id() == meeting.body_id() && s.can_vote() {
                        total_eligible += 1;
                    }
                }
            }

            if total_eligible == 0 {
                // No eligible voters — quorum cannot be met
                meeting.set_quorum_status(QuorumStatus::NotMet);
            } else if meeting.quorum_met() == QuorumStatus::Unknown {
                // Written consent or unset: compute from present seats
                let present_count = u32::try_from(meeting.present_seat_ids().len()).unwrap_or(0);
                let quorum_met = body.quorum_rule().is_met(present_count, total_eligible);
                meeting.set_quorum_status(if quorum_met {
                    QuorumStatus::Met
                } else {
                    QuorumStatus::NotMet
                });
            }

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(meeting_to_response(&meeting)))
}

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/reopen",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        MeetingQuery,
    ),
    responses(
        (status = 200, description = "Re-opened meeting", body = MeetingResponse),
        (status = 404, description = "Meeting not found"),
    ),
)]
async fn reopen_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "meeting reopening")?;
            let mut meeting = store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

            meeting.reopen()?;

            let path = format!("governance/meetings/{}/meeting.json", meeting_id);
            store
                .write_json(
                    "main",
                    &path,
                    &meeting,
                    &format!("Re-open meeting {meeting_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(meeting)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(meeting_to_response(&meeting)))
}

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/cancel",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        MeetingQuery,
    ),
    responses(
        (status = 200, description = "Cancelled meeting", body = MeetingResponse),
        (status = 404, description = "Meeting not found"),
    ),
)]
async fn cancel_meeting(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<MeetingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "meeting cancellation")?;
            let mut meeting = store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

            ensure_meeting_can_be_cancelled(&store, &meeting)?;
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(meeting_to_response(&meeting)))
}

// ── Handlers: Votes ──────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/agenda-items/{item_id}/vote",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        ("item_id" = AgendaItemId, Path, description = "Agenda item ID"),
        MeetingQuery,
    ),
    request_body = CastVoteRequest,
    responses(
        (status = 200, description = "Cast vote", body = VoteResponse),
        (status = 404, description = "Meeting or agenda item not found"),
        (status = 409, description = "Duplicate vote"),
    ),
)]
async fn cast_vote(
    RequireGovernanceVote(auth): RequireGovernanceVote,
    State(state): State<AppState>,
    Path((meeting_id, item_id)): Path<(MeetingId, AgendaItemId)>,
    Query(query): Query<MeetingQuery>,
    Json(req): Json<CastVoteRequest>,
) -> Result<Json<VoteResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let vote = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "vote casting")?;

            // Read the meeting and check it can accept votes
            let meeting = store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;
            if !meeting.can_vote() {
                return Err(GovernanceError::VotingSessionNotOpen.into());
            }

            // Verify agenda item exists
            let item = store
                .read_agenda_item("main", meeting_id, item_id)
                .map_err(|_| AppError::NotFound(format!("agenda item {} not found", item_id)))?;
            if matches!(
                item.status(),
                AgendaItemStatus::Voted | AgendaItemStatus::Tabled | AgendaItemStatus::Withdrawn
            ) {
                return Err(AppError::Conflict(format!(
                    "agenda item {item_id} already finalized"
                )));
            }
            let has_resolution = store
                .list_resolution_ids("main", meeting_id)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|rid| store.read_resolution("main", meeting_id, rid).ok())
                .any(|resolution| resolution.agenda_item_id() == item_id);
            if has_resolution {
                return Err(AppError::Conflict(format!(
                    "agenda item {item_id} already has a resolution"
                )));
            }

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
            let existing_vote_ids = store.list_vote_ids("main", meeting_id).unwrap_or_default();
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

            let path = format!("governance/meetings/{}/votes/{}.json", meeting_id, vote_id);
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(vote_to_response(&vote)))
}

#[utoipa::path(
    get,
    path = "/v1/meetings/{meeting_id}/agenda-items/{item_id}/votes",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        ("item_id" = AgendaItemId, Path, description = "Agenda item ID"),
        MeetingQuery,
    ),
    responses(
        (status = 200, description = "List of votes for agenda item", body = Vec<VoteResponse>),
    ),
)]
async fn list_votes(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((meeting_id, item_id)): Path<(MeetingId, AgendaItemId)>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<Vec<VoteResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let votes = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store.list_vote_ids("main", meeting_id).unwrap_or_default();

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(votes))
}

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/agenda-items/{item_id}/finalize",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        ("item_id" = AgendaItemId, Path, description = "Agenda item ID"),
    ),
    request_body = FinalizeAgendaItemRequest,
    responses(
        (status = 200, description = "Finalized agenda item", body = AgendaItemResponse),
        (status = 404, description = "Meeting or agenda item not found"),
        (status = 409, description = "Agenda item already finalized"),
        (status = 422, description = "Cannot finalize without resolution"),
    ),
)]
async fn finalize_agenda_item(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((meeting_id, item_id)): Path<(MeetingId, AgendaItemId)>,
    Json(req): Json<FinalizeAgendaItemRequest>,
) -> Result<Json<AgendaItemResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let agenda_item = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "agenda finalization")?;
            store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {meeting_id} not found")))?;
            let mut item = store
                .read_agenda_item("main", meeting_id, item_id)
                .map_err(|_| AppError::NotFound(format!("agenda item {item_id} not found")))?;

            if matches!(
                item.status(),
                AgendaItemStatus::Voted | AgendaItemStatus::Tabled | AgendaItemStatus::Withdrawn
            ) {
                return Err(AppError::Conflict(format!(
                    "agenda item {item_id} already finalized"
                )));
            }

            match req.status {
                AgendaItemStatus::Pending => {
                    return Err(AppError::BadRequest(
                        "cannot finalize agenda item to pending".to_owned(),
                    ));
                }
                AgendaItemStatus::Discussed => item.mark_discussed(),
                AgendaItemStatus::Voted => {
                    let has_resolution = store
                        .list_resolution_ids("main", meeting_id)
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|rid| store.read_resolution("main", meeting_id, rid).ok())
                        .any(|r| r.agenda_item_id() == item_id);
                    if !has_resolution {
                        return Err(AppError::UnprocessableEntity(format!(
                            "cannot finalize agenda item {item_id} as voted before a resolution exists"
                        )));
                    }
                    item.mark_voted();
                }
                AgendaItemStatus::Tabled => item.table(),
                AgendaItemStatus::Withdrawn => item.withdraw(),
            }

            let path = format!("governance/meetings/{meeting_id}/agenda/{item_id}.json");
            store
                .write_json(
                    "main",
                    &path,
                    &item,
                    &format!("GOVERNANCE: finalize agenda item {item_id} as {:?}", req.status),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(item)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(agenda_item_to_response(&agenda_item)))
}

// ── Handlers: Resolutions ────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/agenda-items/{item_id}/resolution",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        ("item_id" = AgendaItemId, Path, description = "Agenda item ID"),
        MeetingQuery,
    ),
    request_body = ComputeResolutionRequest,
    responses(
        (status = 200, description = "Computed resolution", body = ResolutionResponse),
        (status = 404, description = "Meeting or agenda item not found"),
        (status = 409, description = "Resolution already exists for agenda item"),
    ),
)]
async fn compute_resolution(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path((meeting_id, item_id)): Path<(MeetingId, AgendaItemId)>,
    Query(query): Query<MeetingQuery>,
    Json(req): Json<ComputeResolutionRequest>,
) -> Result<Json<ResolutionResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;
    if let Some(effective_date) = req.effective_date {
        if effective_date < Utc::now().date_naive() {
            return Err(AppError::BadRequest(
                "effective_date cannot be in the past".to_owned(),
            ));
        }
        validate_not_too_far_future("effective_date", effective_date, 730)?;
    }

    let resolution = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "resolution computation")?;

            // Read the meeting
            let meeting = store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

            // Read the body to get quorum rule
            let body = store
                .read::<GovernanceBody>("main", meeting.body_id())
                .map_err(|_| {
                    AppError::NotFound(format!("governance body {} not found", meeting.body_id()))
                })?;

            // Read all votes for this agenda item
            let vote_ids = store.list_vote_ids("main", meeting_id).unwrap_or_default();

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

            // Determine if the resolution passed using all eligible voting power
            // on the body, less any recorded recusals.
            let total_eligible = eligible_voting_power_for_body(&store, meeting.body_id())?
                .saturating_sub(recused_count);
            if total_eligible == 0 {
                return Err(AppError::BadRequest(
                    "cannot compute a resolution with zero eligible voting power".to_owned(),
                ));
            }
            let passed = body.quorum_rule().is_met(votes_for, total_eligible);

            // Determine resolution type from quorum rule
            let resolution_type = match body.quorum_rule() {
                QuorumThreshold::Majority => ResolutionType::Ordinary,
                QuorumThreshold::Supermajority => ResolutionType::Special,
                QuorumThreshold::Unanimous => ResolutionType::UnanimousWrittenConsent,
            };

            // Guard against duplicate resolution computation for the same agenda item.
            let existing_resolution_ids = store
                .list_resolution_ids("main", meeting_id)
                .unwrap_or_default();
            for existing_id in existing_resolution_ids {
                if let Ok(existing) = store.read_resolution("main", meeting_id, existing_id)
                    && existing.agenda_item_id() == item_id
                {
                    return Err(AppError::Conflict(format!(
                        "resolution already exists for agenda item {item_id}"
                    )));
                }
            }

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
                    &format!(
                        "GOVERNANCE: compute resolution {resolution_id} for agenda item {item_id}"
                    ),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(resolution)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(resolution_to_response(&resolution)))
}

#[utoipa::path(
    get,
    path = "/v1/meetings/{meeting_id}/resolutions",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        MeetingQuery,
    ),
    responses(
        (status = 200, description = "List of resolutions for meeting", body = Vec<ResolutionResponse>),
    ),
)]
async fn list_resolutions(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Path(meeting_id): Path<MeetingId>,
    Query(query): Query<MeetingQuery>,
) -> Result<Json<Vec<ResolutionResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let resolutions = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let meeting = store
                .read::<Meeting>("main", meeting_id)
                .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;
            let body = store
                .read::<GovernanceBody>("main", meeting.body_id())
                .map_err(|_| {
                    AppError::NotFound(format!("governance body {} not found", meeting.body_id()))
                })?;
            if body.entity_id() != entity_id {
                return Err(AppError::BadRequest(
                    "meeting does not belong to entity".to_owned(),
                ));
            }
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(resolutions))
}

#[utoipa::path(
    post,
    path = "/v1/meetings/{meeting_id}/resolutions/{resolution_id}/attach-document",
    tag = "governance",
    params(
        ("meeting_id" = MeetingId, Path, description = "Meeting ID"),
        ("resolution_id" = ResolutionId, Path, description = "Resolution ID"),
    ),
    request_body = AttachResolutionDocumentRequest,
    responses(
        (status = 200, description = "Resolution with attached document", body = ResolutionResponse),
        (status = 404, description = "Resolution or document not found"),
        (status = 409, description = "Resolution already has a document attached"),
    ),
)]
async fn attach_resolution_document(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Path((meeting_id, resolution_id)): Path<(MeetingId, ResolutionId)>,
    Json(req): Json<AttachResolutionDocumentRequest>,
) -> Result<Json<ResolutionResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let resolution = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            // Validate attached document exists in formation records.
            store.read_document("main", req.document_id).map_err(|_| {
                AppError::NotFound(format!("document {} not found", req.document_id))
            })?;

            let mut resolution = store
                .read_resolution("main", meeting_id, resolution_id)
                .map_err(|_| AppError::NotFound(format!("resolution {resolution_id} not found")))?;

            if let Some(existing) = resolution.document_id() {
                if existing == req.document_id {
                    return Ok::<_, AppError>(resolution);
                }
                return Err(AppError::Conflict(format!(
                    "resolution {resolution_id} already has document {existing} attached"
                )));
            }

            resolution.set_document_id(req.document_id);
            let path = format!("governance/meetings/{meeting_id}/resolutions/{resolution_id}.json");
            store
                .write_json(
                    "main",
                    &path,
                    &resolution,
                    &format!(
                        "GOVERNANCE: attach document {} to resolution {resolution_id}",
                        req.document_id
                    ),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(resolution)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(resolution_to_response(&resolution)))
}

// ── Handlers: Scan expired seats ─────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct ScanExpiredResponse {
    pub scanned: usize,
    pub expired: usize,
}

#[utoipa::path(
    post,
    path = "/v1/governance-seats/scan-expired",
    tag = "governance",
    params(super::EntityIdQuery),
    responses(
        (status = 200, description = "Scan expired seats result", body = ScanExpiredResponse),
    ),
)]
async fn scan_expired_seats(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ScanExpiredResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let body_ids = store
                .list_ids::<GovernanceBody>("main")
                .map_err(|e| AppError::Internal(format!("list bodies: {e}")))?;

            let today = chrono::Utc::now().date_naive();
            let mut scanned = 0usize;
            let mut expired = 0usize;

            for body_id in body_ids {
                let seat_ids = store
                    .list_ids::<GovernanceSeat>("main")
                    .map_err(|e| AppError::Internal(format!("list seats: {e}")))?;

                for seat_id in seat_ids {
                    scanned += 1;
                    if let Ok(seat) = store.read::<GovernanceSeat>("main", seat_id) {
                        if let Some(term_end) = seat.term_expiration() {
                            if term_end < today
                                && seat.status()
                                    == crate::domain::governance::types::SeatStatus::Active
                            {
                                // Seat has expired — mark it
                                let mut seat = seat;
                                seat.resign()?;
                                let path =
                                    format!("governance/bodies/{}/seats/{}.json", body_id, seat_id);
                                store
                                    .write_json(
                                        "main",
                                        &path,
                                        &seat,
                                        &format!("Expire seat {seat_id}"),
                                    )
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(result))
}

// ── Handlers: Written consent ────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct WrittenConsentRequest {
    pub body_id: GovernanceBodyId,
    pub entity_id: EntityId,
    pub title: String,
    pub description: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WrittenConsentResponse {
    pub meeting_id: MeetingId,
    pub body_id: GovernanceBodyId,
    pub title: String,
    pub status: MeetingStatus,
    pub consent_type: String,
    pub created_at: String,
}

#[utoipa::path(
    post,
    path = "/v1/meetings/written-consent",
    tag = "governance",
    request_body = WrittenConsentRequest,
    responses(
        (status = 200, description = "Written consent meeting created", body = WrittenConsentResponse),
        (status = 404, description = "Governance body not found"),
    ),
)]
async fn written_consent(
    RequireGovernanceWrite(auth): RequireGovernanceWrite,
    State(state): State<AppState>,
    Json(req): Json<WrittenConsentRequest>,
) -> Result<Json<WrittenConsentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("governance.written_consent.create", workspace_id, 60, 60)?;
    let title = require_non_empty_trimmed(&req.title, "title")?;

    let meeting = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let title = title.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_governance(&store, "written consent creation")?;

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
                MeetingType::WrittenConsent,
                title,
                None,          // No scheduled date for written consent
                String::new(), // No location
                0,             // No notice days
            );

            // Create an agenda item from the description so there is
            // something to vote on in the written-consent flow.
            let item_id = AgendaItemId::new();
            let item = AgendaItem::new(
                item_id,
                meeting_id,
                1, // first (and only) agenda item
                req.description.clone(),
                Some(req.description),
                AgendaItemType::Resolution,
            );

            let files = vec![
                FileWrite::json(
                    format!("governance/meetings/{}/meeting.json", meeting_id),
                    &meeting,
                )
                .map_err(|e| AppError::Internal(format!("serialize meeting: {e}")))?,
                FileWrite::json(
                    format!("governance/meetings/{}/agenda/{}.json", meeting_id, item_id),
                    &item,
                )
                .map_err(|e| AppError::Internal(format!("serialize agenda item: {e}")))?,
            ];

            store
                .commit("main", &format!("Written consent {meeting_id}"), files)
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(meeting)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

#[utoipa::path(
    get,
    path = "/v1/meetings",
    tag = "governance",
    params(super::EntityIdQuery),
    responses(
        (status = 200, description = "List of all meetings", body = Vec<MeetingResponse>),
    ),
)]
async fn list_all_meetings(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<MeetingResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let meetings = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<Meeting>("main")
                .map_err(|e| AppError::Internal(format!("list meetings: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(m) = store.read::<Meeting>("main", id) {
                    results.push(meeting_to_response(&m));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(meetings))
}

// ── Handlers: List all governance bodies (global) ───────────────────

#[utoipa::path(
    get,
    path = "/v1/governance-bodies",
    tag = "governance",
    params(super::EntityIdQuery),
    responses(
        (status = 200, description = "List of all governance bodies", body = Vec<GovernanceBodyResponse>),
    ),
)]
async fn list_all_governance_bodies(
    RequireGovernanceRead(auth): RequireGovernanceRead,
    State(state): State<AppState>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<GovernanceBodyResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let bodies = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<GovernanceBody>("main")
                .map_err(|e| AppError::Internal(format!("list bodies: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(b) = store.read::<GovernanceBody>("main", id) {
                    results.push(body_to_response(&b));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(bodies))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn governance_routes() -> Router<AppState> {
    Router::new()
        // Governance profile + document bundle generation
        .route(
            "/v1/entities/{entity_id}/governance/profile",
            get(get_governance_profile).put(update_governance_profile),
        )
        .route(
            "/v1/entities/{entity_id}/governance/doc-bundles/generate",
            post(generate_governance_doc_bundle),
        )
        .route(
            "/v1/entities/{entity_id}/governance/doc-bundles/current",
            get(get_current_governance_doc_bundle),
        )
        .route(
            "/v1/entities/{entity_id}/governance/doc-bundles",
            get(list_governance_doc_bundles),
        )
        .route(
            "/v1/entities/{entity_id}/governance/doc-bundles/{bundle_id}",
            get(get_governance_doc_bundle),
        )
        .route(
            "/v1/entities/{entity_id}/governance/triggers",
            get(list_governance_triggers),
        )
        .route(
            "/v1/entities/{entity_id}/governance/mode-history",
            get(list_governance_mode_history),
        )
        .route(
            "/v1/entities/{entity_id}/governance/audit/entries",
            get(list_governance_audit_entries),
        )
        .route(
            "/v1/governance/audit/events",
            post(create_governance_audit_event),
        )
        .route(
            "/v1/governance/audit/checkpoints",
            post(write_governance_audit_checkpoint),
        )
        .route(
            "/v1/entities/{entity_id}/governance/audit/checkpoints",
            get(list_governance_audit_checkpoints),
        )
        .route("/v1/governance/audit/verify", post(verify_governance_audit_chain))
        .route(
            "/v1/entities/{entity_id}/governance/audit/verifications",
            get(list_governance_audit_verifications),
        )
        .route(
            "/v1/internal/workspaces/{workspace_id}/entities/{entity_id}/governance/triggers/lockdown",
            post(ingest_lockdown_trigger),
        )
        // Governance mode + incidents
        .route(
            "/v1/governance/mode",
            get(get_governance_mode).post(set_governance_mode),
        )
        .route("/v1/governance/incidents", post(create_incident))
        .route(
            "/v1/entities/{entity_id}/governance/incidents",
            get(list_incidents),
        )
        .route(
            "/v1/governance/incidents/{incident_id}/resolve",
            post(resolve_incident),
        )
        .route(
            "/v1/governance/delegation-schedule",
            get(get_delegation_schedule),
        )
        .route(
            "/v1/governance/delegation-schedule/amend",
            post(amend_delegation_schedule),
        )
        .route(
            "/v1/governance/delegation-schedule/reauthorize",
            post(reauthorize_delegation_schedule),
        )
        .route(
            "/v1/governance/delegation-schedule/history",
            get(list_delegation_schedule_history),
        )
        // Policy evaluation (dry-run)
        .route("/v1/governance/evaluate", post(evaluate_governance))
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
        .route("/v1/governance-seats/{seat_id}/resign", post(resign_seat))
        // Meetings
        .route("/v1/meetings", post(schedule_meeting))
        .route(
            "/v1/meetings/{meeting_id}/agenda-items",
            get(list_agenda_items),
        )
        .route(
            "/v1/governance-bodies/{body_id}/meetings",
            get(list_meetings),
        )
        .route("/v1/meetings/{meeting_id}/notice", post(send_notice))
        .route("/v1/meetings/{meeting_id}/convene", post(convene_meeting))
        .route("/v1/meetings/{meeting_id}/adjourn", post(adjourn_meeting))
        .route("/v1/meetings/{meeting_id}/reopen", post(reopen_meeting))
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
        .route(
            "/v1/meetings/{meeting_id}/agenda-items/{item_id}/finalize",
            post(finalize_agenda_item),
        )
        // Resolutions
        .route(
            "/v1/meetings/{meeting_id}/agenda-items/{item_id}/resolution",
            post(compute_resolution),
        )
        .route(
            "/v1/meetings/{meeting_id}/resolutions/{resolution_id}/attach-document",
            post(attach_resolution_document),
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

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        get_governance_profile,
        update_governance_profile,
        generate_governance_doc_bundle,
        get_current_governance_doc_bundle,
        list_governance_doc_bundles,
        get_governance_doc_bundle,
        list_governance_triggers,
        list_governance_mode_history,
        ingest_lockdown_trigger,
        list_governance_audit_entries,
        create_governance_audit_event,
        write_governance_audit_checkpoint,
        list_governance_audit_checkpoints,
        verify_governance_audit_chain,
        list_governance_audit_verifications,
        create_governance_body,
        list_governance_bodies,
        get_governance_mode,
        set_governance_mode,
        create_incident,
        list_incidents,
        resolve_incident,
        get_delegation_schedule,
        amend_delegation_schedule,
        reauthorize_delegation_schedule,
        list_delegation_schedule_history,
        evaluate_governance,
        create_seat,
        list_seats,
        resign_seat,
        schedule_meeting,
        list_meetings,
        list_agenda_items,
        send_notice,
        convene_meeting,
        adjourn_meeting,
        reopen_meeting,
        cancel_meeting,
        cast_vote,
        list_votes,
        finalize_agenda_item,
        compute_resolution,
        list_resolutions,
        attach_resolution_document,
        scan_expired_seats,
        written_consent,
        list_all_meetings,
        list_all_governance_bodies,
    ),
    components(schemas(
        BodyQuery,
        MeetingQuery,
        CreateGovernanceBodyRequest,
        CreateSeatRequest,
        ScheduleMeetingRequest,
        ConveneMeetingRequest,
        CastVoteRequest,
        ComputeResolutionRequest,
        SetGovernanceModeRequest,
        CreateIncidentRequest,
        FinalizeAgendaItemRequest,
        AttachResolutionDocumentRequest,
        AmendDelegationScheduleRequest,
        ReauthorizeDelegationScheduleRequest,
        UpdateGovernanceProfileRequest,
        GenerateGovernanceDocBundleRequest,
        InternalLockdownTriggerRequest,
        CreateGovernanceAuditEventRequest,
        WriteGovernanceAuditCheckpointRequest,
        VerifyGovernanceAuditChainRequest,
        EvaluateGovernanceRequest,
        WrittenConsentRequest,
        GovernanceBodyResponse,
        GovernanceSeatResponse,
        MeetingResponse,
        AgendaItemResponse,
        VoteResponse,
        ResolutionResponse,
        GovernanceModeResponse,
        IncidentResponse,
        GenerateGovernanceDocBundleResponse,
        InternalLockdownTriggerResponse,
        DelegationScheduleChangeResponse,
        ScanExpiredResponse,
        WrittenConsentResponse,
        GovernanceProfile,
        GovernanceDocBundleCurrent,
        GovernanceDocBundleSummary,
        GovernanceDocBundleManifest,
        GovernanceTriggerEvent,
        GovernanceModeChangeEvent,
        GovernanceAuditEntry,
        GovernanceAuditCheckpoint,
        GovernanceAuditVerificationReport,
        GovernanceIncident,
        DelegationSchedule,
        ScheduleAmendment,
        PolicyDecision,
    )),
    tags((name = "governance", description = "Governance bodies, meetings, voting, and resolutions")),
)]
pub struct GovernanceApi;
