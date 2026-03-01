//! Equity HTTP routes.
//!
//! Canonical cap-table operations for holders, legal entities, control links,
//! instruments, positions, rounds, and conversion previews/execution.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use super::AppState;
use crate::auth::{RequireEquityRead, RequireEquityWrite};
use crate::domain::equity::{
    control_link::{ControlLink, ControlType},
    conversion_execution::ConversionExecution,
    fundraising_workflow::{
        FundraisingWorkflow, WorkflowExecutionStatus as FundraisingExecutionStatus,
    },
    holder::{Holder, HolderType},
    instrument::{Instrument, InstrumentKind, InstrumentStatus},
    legal_entity::{LegalEntity, LegalEntityRole},
    position::{Position, PositionStatus},
    round::{EquityRound, EquityRoundStatus},
    rule_set::{AntiDilutionMethod, EquityRuleSet},
    share_class::ShareClass,
    transfer::ShareTransfer,
    transfer_workflow::{TransferWorkflow, WorkflowExecutionStatus as TransferExecutionStatus},
    types::{GoverningDocType, ShareCount, TransferStatus, TransferType, TransfereeRights},
};
use crate::domain::execution::{
    approval_artifact::ApprovalArtifact,
    document_request::DocumentRequest,
    intent::Intent,
    transaction_packet::{PacketItem, TransactionPacket, TransactionPacketStatus, WorkflowType},
    types::{AuthorityTier, IntentStatus},
};
use crate::domain::governance::policy_engine::evaluate_intent as evaluate_governance_intent;
use crate::domain::governance::{
    body::GovernanceBody, meeting::Meeting, resolution::Resolution, types::BodyType,
};
use crate::domain::ids::{
    ContactId, ControlLinkId, ConversionExecutionId, EntityId, EquityRoundId, EquityRuleSetId,
    FundraisingWorkflowId, HolderId, InstrumentId, IntentId, LegalEntityId, MeetingId, PacketId,
    PacketSignatureId, PositionId, ResolutionId, ShareClassId, TransferId, TransferWorkflowId,
    WorkspaceId,
};
use crate::domain::treasury::types::Cents;
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::store::entity_store::EntityStore;
use crate::store::stored_entity::StoredEntity;

// ── Queries ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CapTableBasis {
    #[default]
    Outstanding,
    AsConverted,
    FullyDiluted,
}

#[derive(Debug, Deserialize)]
pub struct CapTableQuery {
    #[serde(default)]
    pub basis: CapTableBasis,
    #[serde(default)]
    pub issuer_legal_entity_id: Option<LegalEntityId>,
}

#[derive(Debug, Deserialize)]
pub struct ControlMapQuery {
    pub entity_id: EntityId,
    pub root_entity_id: LegalEntityId,
}

#[derive(Debug, Deserialize)]
pub struct DilutionPreviewQuery {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
}

// ── Request types ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateHolderRequest {
    pub entity_id: EntityId,
    pub contact_id: ContactId,
    #[serde(default)]
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub holder_type: HolderType,
    #[serde(default)]
    pub external_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateLegalEntityRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub role: LegalEntityRole,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateControlLinkRequest {
    pub entity_id: EntityId,
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    #[serde(default)]
    pub voting_power_bps: Option<u32>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInstrumentRequest {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub symbol: String,
    pub kind: InstrumentKind,
    #[serde(default)]
    pub authorized_units: Option<i64>,
    #[serde(default)]
    pub issue_price_cents: Option<i64>,
    #[serde(default)]
    pub terms: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdjustPositionRequest {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub quantity_delta: i64,
    #[serde(default)]
    pub principal_delta_cents: i64,
    #[serde(default)]
    pub source_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRoundRequest {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub name: String,
    #[serde(default)]
    pub pre_money_cents: Option<i64>,
    #[serde(default)]
    pub round_price_cents: Option<i64>,
    #[serde(default)]
    pub target_raise_cents: Option<i64>,
    #[serde(default)]
    pub conversion_target_instrument_id: Option<InstrumentId>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyRoundTermsRequest {
    pub entity_id: EntityId,
    pub anti_dilution_method: AntiDilutionMethod,
    #[serde(default)]
    pub conversion_precedence: Vec<InstrumentKind>,
    #[serde(default)]
    pub protective_provisions: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardApproveRoundRequest {
    pub entity_id: EntityId,
    pub meeting_id: MeetingId,
    pub resolution_id: ResolutionId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptRoundRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
    #[serde(default)]
    pub accepted_by_contact_id: Option<ContactId>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewConversionRequest {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    #[serde(default)]
    pub source_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteConversionRequest {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    pub intent_id: IntentId,
    #[serde(default)]
    pub source_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTransferWorkflowRequest {
    pub entity_id: EntityId,
    pub share_class_id: ShareClassId,
    pub from_contact_id: ContactId,
    pub to_contact_id: ContactId,
    pub transfer_type: TransferType,
    pub share_count: i64,
    #[serde(default)]
    pub price_per_share_cents: Option<i64>,
    #[serde(default)]
    pub relationship_to_holder: Option<String>,
    pub governing_doc_type: GoverningDocType,
    pub transferee_rights: TransfereeRights,
    pub prepare_intent_id: IntentId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateWorkflowDocsRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub documents: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitTransferReviewRequest {
    pub entity_id: EntityId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTransferReviewRequest {
    pub entity_id: EntityId,
    pub approved: bool,
    pub notes: String,
    pub reviewer: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTransferRofrRequest {
    pub entity_id: EntityId,
    pub offered: bool,
    pub waived: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTransferBoardApprovalRequest {
    pub entity_id: EntityId,
    pub meeting_id: MeetingId,
    pub resolution_id: ResolutionId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordTransferExecutionRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateFundraisingWorkflowRequest {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub name: String,
    #[serde(default)]
    pub pre_money_cents: Option<i64>,
    #[serde(default)]
    pub round_price_cents: Option<i64>,
    #[serde(default)]
    pub target_raise_cents: Option<i64>,
    #[serde(default)]
    pub conversion_target_instrument_id: Option<InstrumentId>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub prepare_intent_id: IntentId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyFundraisingTermsRequest {
    pub entity_id: EntityId,
    pub anti_dilution_method: AntiDilutionMethod,
    #[serde(default)]
    pub conversion_precedence: Vec<InstrumentKind>,
    #[serde(default)]
    pub protective_provisions: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordFundraisingBoardApprovalRequest {
    pub entity_id: EntityId,
    pub meeting_id: MeetingId,
    pub resolution_id: ResolutionId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordFundraisingAcceptanceRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
    #[serde(default)]
    pub accepted_by_contact_id: Option<ContactId>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordFundraisingCloseRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareWorkflowExecutionRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
    pub approval_artifact_id: crate::domain::ids::ApprovalArtifactId,
    #[serde(default)]
    pub document_request_ids: Vec<crate::domain::ids::DocumentRequestId>,
    #[serde(default)]
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileWorkflowPacketRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub required_signers: Vec<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartWorkflowSignaturesRequest {
    pub entity_id: EntityId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordWorkflowSignatureRequest {
    pub entity_id: EntityId,
    pub signer_identity: String,
    #[serde(default)]
    pub channel: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalizeWorkflowRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub phase: Option<String>,
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct HolderResponse {
    pub holder_id: HolderId,
    pub contact_id: ContactId,
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub holder_type: HolderType,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct LegalEntityResponse {
    pub legal_entity_id: LegalEntityId,
    pub workspace_id: WorkspaceId,
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub role: LegalEntityRole,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ControlLinkResponse {
    pub control_link_id: ControlLinkId,
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    pub voting_power_bps: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct InstrumentResponse {
    pub instrument_id: InstrumentId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub symbol: String,
    pub kind: InstrumentKind,
    pub authorized_units: Option<i64>,
    pub issue_price_cents: Option<i64>,
    pub status: InstrumentStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub position_id: PositionId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub quantity_units: i64,
    pub principal_cents: i64,
    pub status: PositionStatus,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct RoundResponse {
    pub round_id: EquityRoundId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub name: String,
    pub pre_money_cents: Option<i64>,
    pub round_price_cents: Option<i64>,
    pub target_raise_cents: Option<i64>,
    pub conversion_target_instrument_id: Option<InstrumentId>,
    pub rule_set_id: Option<EquityRuleSetId>,
    pub board_approval_meeting_id: Option<MeetingId>,
    pub board_approval_resolution_id: Option<ResolutionId>,
    pub board_approved_at: Option<String>,
    pub accepted_by_contact_id: Option<ContactId>,
    pub accepted_at: Option<String>,
    pub status: EquityRoundStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct RuleSetResponse {
    pub rule_set_id: EquityRuleSetId,
    pub anti_dilution_method: AntiDilutionMethod,
    pub conversion_precedence: Vec<InstrumentKind>,
}

#[derive(Debug, Serialize)]
pub struct CapTableInstrumentSummary {
    pub instrument_id: InstrumentId,
    pub symbol: String,
    pub kind: InstrumentKind,
    pub authorized_units: Option<i64>,
    pub issued_units: i64,
    pub diluted_units: i64,
}

#[derive(Debug, Serialize)]
pub struct CapTableHolderSummary {
    pub holder_id: HolderId,
    pub name: String,
    pub outstanding_units: i64,
    pub as_converted_units: i64,
    pub fully_diluted_units: i64,
    pub outstanding_bps: u32,
    pub as_converted_bps: u32,
    pub fully_diluted_bps: u32,
}

#[derive(Debug, Serialize)]
pub struct CapTableResponse {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub basis: CapTableBasis,
    pub total_units: i64,
    pub instruments: Vec<CapTableInstrumentSummary>,
    pub holders: Vec<CapTableHolderSummary>,
    pub generated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ConversionPreviewLine {
    pub source_position_id: PositionId,
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub principal_cents: i64,
    pub conversion_price_cents: i64,
    pub new_units: i64,
    pub basis: String,
}

#[derive(Debug, Serialize)]
pub struct ConversionPreviewResponse {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    pub target_instrument_id: InstrumentId,
    pub lines: Vec<ConversionPreviewLine>,
    pub anti_dilution_adjustment_units: i64,
    pub total_new_units: i64,
}

#[derive(Debug, Serialize)]
pub struct ConversionExecuteResponse {
    pub conversion_execution_id: ConversionExecutionId,
    pub round_id: EquityRoundId,
    pub converted_positions: usize,
    pub target_positions_touched: usize,
    pub total_new_units: i64,
}

#[derive(Debug, Serialize)]
pub struct ControlMapEdge {
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    pub voting_power_bps: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ControlMapResponse {
    pub root_entity_id: LegalEntityId,
    pub traversed_entities: Vec<LegalEntityId>,
    pub edges: Vec<ControlMapEdge>,
}

#[derive(Debug, Serialize)]
pub struct DilutionPreviewResponse {
    pub round_id: EquityRoundId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub pre_round_outstanding_units: i64,
    pub projected_new_units: i64,
    pub projected_post_outstanding_units: i64,
    pub projected_dilution_bps: u32,
}

#[derive(Debug, Serialize)]
pub struct TransferWorkflowResponse {
    pub transfer_workflow_id: TransferWorkflowId,
    pub transfer_id: TransferId,
    pub prepare_intent_id: IntentId,
    pub execute_intent_id: Option<IntentId>,
    pub transfer_status: TransferStatus,
    pub execution_status: TransferExecutionStatus,
    pub active_packet_id: Option<PacketId>,
    pub last_packet_hash: Option<String>,
    pub board_approval_meeting_id: Option<MeetingId>,
    pub board_approval_resolution_id: Option<ResolutionId>,
    pub generated_documents: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct FundraisingWorkflowResponse {
    pub fundraising_workflow_id: FundraisingWorkflowId,
    pub round_id: EquityRoundId,
    pub prepare_intent_id: IntentId,
    pub accept_intent_id: Option<IntentId>,
    pub close_intent_id: Option<IntentId>,
    pub execution_status: FundraisingExecutionStatus,
    pub active_packet_id: Option<PacketId>,
    pub last_packet_hash: Option<String>,
    pub rule_set_id: Option<EquityRuleSetId>,
    pub round_status: EquityRoundStatus,
    pub board_approval_meeting_id: Option<MeetingId>,
    pub board_approval_resolution_id: Option<ResolutionId>,
    pub board_packet_documents: Vec<String>,
    pub closing_packet_documents: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TransactionPacketResponse {
    pub packet_id: PacketId,
    pub entity_id: EntityId,
    pub intent_id: IntentId,
    pub workflow_type: WorkflowType,
    pub workflow_id: String,
    pub status: TransactionPacketStatus,
    pub manifest_hash: String,
    pub items: Vec<PacketItem>,
    pub required_signers: Vec<String>,
    pub signatures: Vec<PacketSignatureResponse>,
    pub created_at: String,
    pub finalized_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PacketSignatureResponse {
    pub signature_id: PacketSignatureId,
    pub signer_identity: String,
    pub channel: String,
    pub signed_at: String,
}

#[derive(Debug, Serialize)]
pub struct WorkflowStatusResponse {
    pub workflow_type: WorkflowType,
    pub workflow_id: String,
    pub execution_status: String,
    pub active_packet_id: Option<PacketId>,
    pub transfer_workflow: Option<TransferWorkflowResponse>,
    pub fundraising_workflow: Option<FundraisingWorkflowResponse>,
    pub packet: Option<TransactionPacketResponse>,
}

// ── Helpers ──────────────────────────────────────────────────────────

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

fn read_all<T: StoredEntity>(store: &EntityStore<'_>) -> Result<Vec<T>, AppError> {
    let ids = store
        .list_ids::<T>("main")
        .map_err(|e| AppError::Internal(format!("list {}: {e}", T::storage_dir())))?;

    let mut out = Vec::new();
    for id in ids {
        let rec = store
            .read::<T>("main", id)
            .map_err(|e| AppError::Internal(format!("read {} {}: {e}", T::storage_dir(), id)))?;
        out.push(rec);
    }
    Ok(out)
}

fn checked_bps(part: i64, total: i64) -> u32 {
    if part <= 0 || total <= 0 {
        return 0;
    }
    let p = i128::from(part) * 10_000_i128;
    let t = i128::from(total);
    let v = (p / t).clamp(0, i128::from(u32::MAX));
    u32::try_from(v).unwrap_or(0)
}

fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn infer_issuer(
    entity_id: EntityId,
    legal_entities: &[LegalEntity],
    explicit: Option<LegalEntityId>,
) -> Result<LegalEntityId, AppError> {
    if let Some(id) = explicit {
        return Ok(id);
    }

    legal_entities
        .iter()
        .find(|le| {
            le.linked_entity_id() == Some(entity_id) && le.role() == LegalEntityRole::Operating
        })
        .or_else(|| {
            legal_entities
                .iter()
                .find(|le| le.linked_entity_id() == Some(entity_id))
        })
        .map(|le| le.legal_entity_id())
        .ok_or_else(|| {
            AppError::NotFound(
                "no legal entity linked to this entity_id; create one via POST /v1/equity/entities"
                    .to_owned(),
            )
        })
}

fn units_for_basis(kind: InstrumentKind, qty: i64, basis: CapTableBasis) -> i64 {
    match basis {
        CapTableBasis::Outstanding => match kind {
            InstrumentKind::CommonEquity
            | InstrumentKind::PreferredEquity
            | InstrumentKind::MembershipUnit => qty,
            _ => 0,
        },
        CapTableBasis::AsConverted => match kind {
            InstrumentKind::CommonEquity
            | InstrumentKind::PreferredEquity
            | InstrumentKind::MembershipUnit
            | InstrumentKind::Safe
            | InstrumentKind::ConvertibleNote
            | InstrumentKind::Warrant => qty,
            InstrumentKind::OptionGrant => 0,
        },
        CapTableBasis::FullyDiluted => qty,
    }
}

fn compute_cap_table(
    entity_id: EntityId,
    issuer_legal_entity_id: LegalEntityId,
    basis: CapTableBasis,
    holders: &[Holder],
    instruments: &[Instrument],
    positions: &[Position],
) -> CapTableResponse {
    let issuer_instruments: Vec<&Instrument> = instruments
        .iter()
        .filter(|i| i.issuer_legal_entity_id() == issuer_legal_entity_id)
        .collect();

    let issuer_positions: Vec<&Position> = positions
        .iter()
        .filter(|p| p.issuer_legal_entity_id() == issuer_legal_entity_id)
        .collect();

    let mut holder_units_outstanding: HashMap<HolderId, i64> = HashMap::new();
    let mut holder_units_as_converted: HashMap<HolderId, i64> = HashMap::new();
    let mut holder_units_fully_diluted: HashMap<HolderId, i64> = HashMap::new();

    let instrument_map: HashMap<InstrumentId, &Instrument> = issuer_instruments
        .iter()
        .map(|i| (i.instrument_id(), *i))
        .collect();

    for p in &issuer_positions {
        let Some(inst) = instrument_map.get(&p.instrument_id()) else {
            continue;
        };
        let qty = p.quantity_units().max(0);

        *holder_units_outstanding.entry(p.holder_id()).or_insert(0) +=
            units_for_basis(inst.kind(), qty, CapTableBasis::Outstanding);
        *holder_units_as_converted.entry(p.holder_id()).or_insert(0) +=
            units_for_basis(inst.kind(), qty, CapTableBasis::AsConverted);
        *holder_units_fully_diluted.entry(p.holder_id()).or_insert(0) +=
            units_for_basis(inst.kind(), qty, CapTableBasis::FullyDiluted);
    }

    // Include unallocated option reserves in fully diluted denominator only.
    let mut unallocated_option_reserve: i64 = 0;
    for inst in &issuer_instruments {
        if inst.kind() == InstrumentKind::OptionGrant {
            let issued = issuer_positions
                .iter()
                .filter(|p| p.instrument_id() == inst.instrument_id())
                .map(|p| p.quantity_units().max(0))
                .sum::<i64>();
            if let Some(auth) = inst.authorized_units() {
                unallocated_option_reserve += (auth - issued).max(0);
            }
        }
    }

    let total_outstanding = holder_units_outstanding.values().copied().sum::<i64>();
    let total_as_converted = holder_units_as_converted.values().copied().sum::<i64>();
    let total_fully_diluted =
        holder_units_fully_diluted.values().copied().sum::<i64>() + unallocated_option_reserve;

    let total_units = match basis {
        CapTableBasis::Outstanding => total_outstanding,
        CapTableBasis::AsConverted => total_as_converted,
        CapTableBasis::FullyDiluted => total_fully_diluted,
    };

    let holder_name: HashMap<HolderId, String> = holders
        .iter()
        .map(|h| (h.holder_id(), h.name().to_owned()))
        .collect();

    let all_holder_ids: HashSet<HolderId> = holder_units_fully_diluted
        .keys()
        .chain(holder_units_as_converted.keys())
        .chain(holder_units_outstanding.keys())
        .copied()
        .collect();

    let mut holder_rows: Vec<CapTableHolderSummary> = all_holder_ids
        .into_iter()
        .map(|hid| {
            let outstanding_units = *holder_units_outstanding.get(&hid).unwrap_or(&0);
            let as_converted_units = *holder_units_as_converted.get(&hid).unwrap_or(&0);
            let fully_diluted_units = *holder_units_fully_diluted.get(&hid).unwrap_or(&0);

            CapTableHolderSummary {
                holder_id: hid,
                name: holder_name
                    .get(&hid)
                    .cloned()
                    .unwrap_or_else(|| "unknown holder".to_owned()),
                outstanding_units,
                as_converted_units,
                fully_diluted_units,
                outstanding_bps: checked_bps(outstanding_units, total_outstanding),
                as_converted_bps: checked_bps(as_converted_units, total_as_converted),
                fully_diluted_bps: checked_bps(fully_diluted_units, total_fully_diluted),
            }
        })
        .collect();
    holder_rows.sort_by(|a, b| a.name.cmp(&b.name));

    let mut instrument_rows = Vec::new();
    for inst in issuer_instruments {
        let issued_units = issuer_positions
            .iter()
            .filter(|p| p.instrument_id() == inst.instrument_id())
            .map(|p| p.quantity_units().max(0))
            .sum::<i64>();

        let diluted_units = match inst.kind() {
            InstrumentKind::OptionGrant => inst.authorized_units().unwrap_or(issued_units),
            _ => issued_units,
        };

        instrument_rows.push(CapTableInstrumentSummary {
            instrument_id: inst.instrument_id(),
            symbol: inst.symbol().to_owned(),
            kind: inst.kind(),
            authorized_units: inst.authorized_units(),
            issued_units,
            diluted_units,
        });
    }
    instrument_rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    CapTableResponse {
        entity_id,
        issuer_legal_entity_id,
        basis,
        total_units,
        instruments: instrument_rows,
        holders: holder_rows,
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn compute_conversion_preview(
    round: &EquityRound,
    rule_set: &EquityRuleSet,
    instruments: &[Instrument],
    positions: &[Position],
) -> Result<(Vec<ConversionPreviewLine>, i64), AppError> {
    let round_price = round.round_price_cents().ok_or_else(|| {
        AppError::BadRequest("round_price_cents is required before conversion".to_owned())
    })?;
    if round_price <= 0 {
        return Err(AppError::BadRequest(
            "round_price_cents must be positive".to_owned(),
        ));
    }

    let inst_map: HashMap<InstrumentId, &Instrument> =
        instruments.iter().map(|i| (i.instrument_id(), i)).collect();

    let precedence: Vec<InstrumentKind> = if rule_set.conversion_precedence().is_empty() {
        vec![
            InstrumentKind::Safe,
            InstrumentKind::ConvertibleNote,
            InstrumentKind::Warrant,
        ]
    } else {
        rule_set.conversion_precedence().to_vec()
    };

    let precedence_rank: HashMap<InstrumentKind, usize> = precedence
        .iter()
        .enumerate()
        .map(|(idx, k)| (*k, idx))
        .collect();

    let mut lines: Vec<ConversionPreviewLine> = positions
        .iter()
        .filter_map(|p| {
            let inst = inst_map.get(&p.instrument_id())?;
            if inst.issuer_legal_entity_id() != round.issuer_legal_entity_id() {
                return None;
            }
            match inst.kind() {
                InstrumentKind::Safe
                | InstrumentKind::ConvertibleNote
                | InstrumentKind::Warrant => {}
                _ => return None,
            }

            let terms = inst.terms();
            let discount_bps = terms
                .get("discount_bps")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(0)
                .min(10_000);
            let cap_price_cents = terms
                .get("cap_price_cents")
                .and_then(|v| v.as_i64())
                .filter(|v| *v > 0);

            let discounted_price = ((i128::from(round_price)
                * i128::from(10_000_u32 - discount_bps))
                / i128::from(10_000_u32)) as i64;
            let discounted_price = discounted_price.max(1);

            let mut conversion_price = round_price;
            let mut basis = "round_price".to_owned();
            if discount_bps > 0 && discounted_price < conversion_price {
                conversion_price = discounted_price;
                basis = "discount".to_owned();
            }
            if let Some(cap) = cap_price_cents {
                if cap < conversion_price {
                    conversion_price = cap;
                    basis = "cap_price".to_owned();
                }
            }

            let new_units = if p.principal_cents() > 0 {
                p.principal_cents() / conversion_price
            } else {
                p.quantity_units().max(0)
            }
            .max(0);

            Some(ConversionPreviewLine {
                source_position_id: p.position_id(),
                holder_id: p.holder_id(),
                instrument_id: p.instrument_id(),
                principal_cents: p.principal_cents(),
                conversion_price_cents: conversion_price,
                new_units,
                basis,
            })
        })
        .collect();

    lines.sort_by_key(|line| {
        let inst_kind = inst_map
            .get(&line.instrument_id)
            .map(|i| i.kind())
            .unwrap_or(InstrumentKind::Safe);
        *precedence_rank.get(&inst_kind).unwrap_or(&usize::MAX)
    });

    // Anti-dilution: compute additional preferred units under configured method.
    let anti_dilution_adjustment_units = match rule_set.anti_dilution_method() {
        AntiDilutionMethod::None => 0,
        AntiDilutionMethod::FullRatchet => {
            let mut adj = 0i64;
            for p in positions {
                let Some(inst) = inst_map.get(&p.instrument_id()) else {
                    continue;
                };
                if inst.issuer_legal_entity_id() != round.issuer_legal_entity_id()
                    || inst.kind() != InstrumentKind::PreferredEquity
                {
                    continue;
                }
                let Some(old_price) = inst.issue_price_cents() else {
                    continue;
                };
                if old_price <= round_price {
                    continue;
                }
                let qty = i128::from(p.quantity_units().max(0));
                let old = i128::from(old_price);
                let newp = i128::from(round_price);
                let adjusted_qty = (qty * old) / newp;
                let add = (adjusted_qty - qty).max(0);
                adj = adj.saturating_add(i64::try_from(add).unwrap_or(i64::MAX));
            }
            adj
        }
        AntiDilutionMethod::BroadBasedWeightedAverage
        | AntiDilutionMethod::NarrowBasedWeightedAverage => {
            // Simplified WA preview using aggregate shares.
            let mut existing = 0f64;
            let mut existing_broad = 0f64;
            for p in positions {
                let Some(inst) = inst_map.get(&p.instrument_id()) else {
                    continue;
                };
                if inst.issuer_legal_entity_id() != round.issuer_legal_entity_id() {
                    continue;
                }
                match inst.kind() {
                    InstrumentKind::CommonEquity | InstrumentKind::PreferredEquity => {
                        existing += p.quantity_units().max(0) as f64;
                        existing_broad += p.quantity_units().max(0) as f64;
                    }
                    InstrumentKind::OptionGrant => {
                        if let Some(auth) = inst.authorized_units() {
                            existing_broad += auth.max(0) as f64;
                        }
                    }
                    _ => {}
                }
            }
            let a = if rule_set.anti_dilution_method()
                == AntiDilutionMethod::BroadBasedWeightedAverage
            {
                existing_broad
            } else {
                existing
            };
            if a <= 0.0 {
                0
            } else {
                let c = round
                    .target_raise_cents()
                    .and_then(|raise| {
                        if round_price > 0 {
                            Some((raise as f64) / (round_price as f64))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0);

                // B approximates shares purchasable at prior preferred issue prices.
                let mut adjustment = 0f64;
                for p in positions {
                    let Some(inst) = inst_map.get(&p.instrument_id()) else {
                        continue;
                    };
                    if inst.issuer_legal_entity_id() != round.issuer_legal_entity_id()
                        || inst.kind() != InstrumentKind::PreferredEquity
                    {
                        continue;
                    }
                    let Some(cp1) = inst.issue_price_cents() else {
                        continue;
                    };
                    if cp1 <= 0 {
                        continue;
                    }
                    let b = (round.target_raise_cents().unwrap_or(0) as f64) / (cp1 as f64);
                    let cp2 = (cp1 as f64) * ((a + b) / (a + c).max(1.0));
                    if cp2 <= 0.0 || cp2 >= (cp1 as f64) {
                        continue;
                    }
                    let existing_qty = p.quantity_units().max(0) as f64;
                    let add = existing_qty * ((cp1 as f64 / cp2) - 1.0);
                    if add.is_finite() && add > 0.0 {
                        adjustment += add;
                    }
                }
                adjustment.floor() as i64
            }
        }
    };

    Ok((lines, anti_dilution_adjustment_units.max(0)))
}

fn ensure_authorized_intent(
    intent: &Intent,
    entity_id: EntityId,
    expected_intent_type: &str,
    metadata_field: Option<(&str, String)>,
) -> Result<(), AppError> {
    if intent.entity_id() != entity_id {
        return Err(AppError::Forbidden(format!(
            "intent {} belongs to a different entity",
            intent.intent_id()
        )));
    }
    if intent.status() != IntentStatus::Authorized {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} must be authorized",
            intent.intent_id()
        )));
    }
    if intent.intent_type() != expected_intent_type {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} must have type {}",
            intent.intent_id(),
            expected_intent_type
        )));
    }

    let decision = evaluate_governance_intent(intent.intent_type(), intent.metadata());
    if !decision.policy_mapped() {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} type {} is not explicitly mapped in governance policy",
            intent.intent_id(),
            intent.intent_type()
        )));
    }
    if !decision.allowed() {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} blocked by policy: {}",
            intent.intent_id(),
            decision.blockers().join("; ")
        )));
    }
    if decision.tier() != AuthorityTier::Tier3 {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} must evaluate to tier_3 under strict equity policy",
            intent.intent_id()
        )));
    }

    if let Some((field, expected)) = metadata_field {
        let got = intent
            .metadata()
            .get(field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::UnprocessableEntity(format!(
                    "intent {} metadata must include {}",
                    intent.intent_id(),
                    field
                ))
            })?;
        if got != expected {
            return Err(AppError::UnprocessableEntity(format!(
                "intent {} {} mismatch",
                intent.intent_id(),
                field
            )));
        }
    }

    Ok(())
}

fn ensure_authorized_round_intent(
    intent: &Intent,
    entity_id: EntityId,
    round_id: EquityRoundId,
    expected_intent_type: &str,
) -> Result<(), AppError> {
    ensure_authorized_intent(
        intent,
        entity_id,
        expected_intent_type,
        Some(("round_id", round_id.to_string())),
    )
}

fn ensure_authorized_transfer_intent(
    intent: &Intent,
    entity_id: EntityId,
    transfer_id: TransferId,
    expected_intent_type: &str,
) -> Result<(), AppError> {
    ensure_authorized_intent(
        intent,
        entity_id,
        expected_intent_type,
        Some(("transfer_id", transfer_id.to_string())),
    )
}

fn amount_from_metadata_cents(metadata: &serde_json::Value) -> Option<i64> {
    metadata
        .get("amount_cents")
        .and_then(serde_json::Value::as_i64)
        .or_else(|| {
            metadata
                .get("amount")
                .and_then(serde_json::Value::as_i64)
                .map(|d| d.saturating_mul(100))
        })
}

fn required_document_types_for_intent(intent_type: &str) -> &'static [&'static str] {
    match intent_type {
        "equity.transfer.execute" => &["stock_transfer_agreement", "transfer_board_consent"],
        "equity.fundraising.accept" => &[
            "board_consent",
            "equity_issuance_approval",
            "subscription_agreement",
        ],
        "equity.fundraising.close" => &["investor_rights_agreement", "subscription_agreement"],
        _ => &[],
    }
}

fn verify_bound_artifacts_for_intent(
    store: &EntityStore<'_>,
    intent: &Intent,
) -> Result<(), AppError> {
    let approval_id = intent.bound_approval_artifact_id().ok_or_else(|| {
        AppError::UnprocessableEntity(format!(
            "intent {} requires a bound approval artifact",
            intent.intent_id()
        ))
    })?;

    let approval = store
        .read::<ApprovalArtifact>("main", approval_id)
        .map_err(|_| AppError::NotFound(format!("approval artifact {} not found", approval_id)))?;
    if !approval.covers_intent(
        intent.intent_type(),
        amount_from_metadata_cents(intent.metadata()),
        chrono::Utc::now(),
    ) {
        return Err(AppError::UnprocessableEntity(format!(
            "approval artifact {} does not cover intent {}",
            approval_id,
            intent.intent_id()
        )));
    }

    let required_doc_types = required_document_types_for_intent(intent.intent_type());
    if required_doc_types.is_empty() {
        return Ok(());
    }

    if intent.bound_document_request_ids().is_empty() {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} requires bound document requests",
            intent.intent_id()
        )));
    }

    let mut requests = Vec::new();
    for request_id in intent.bound_document_request_ids() {
        let request = store
            .read::<DocumentRequest>("main", *request_id)
            .map_err(|_| {
                AppError::NotFound(format!("document request {} not found", request_id))
            })?;
        requests.push(request);
    }

    for required in required_doc_types {
        let found = requests
            .iter()
            .any(|r| r.document_type() == *required && r.is_satisfied());
        if !found {
            return Err(AppError::UnprocessableEntity(format!(
                "required document {} not satisfied for intent {}",
                required,
                intent.intent_id()
            )));
        }
    }
    Ok(())
}

fn validate_board_resolution_for_round(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    meeting_id: MeetingId,
    resolution_id: ResolutionId,
) -> Result<(), AppError> {
    let meeting = store
        .read::<Meeting>("main", meeting_id)
        .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

    let body = store
        .read::<GovernanceBody>("main", meeting.body_id())
        .map_err(|_| {
            AppError::NotFound(format!("governance body {} not found", meeting.body_id()))
        })?;

    if body.entity_id() != entity_id {
        return Err(AppError::BadRequest(format!(
            "meeting {} does not belong to entity {}",
            meeting_id, entity_id
        )));
    }
    if body.body_type() != BodyType::BoardOfDirectors {
        return Err(AppError::UnprocessableEntity(format!(
            "meeting {} is not associated with a board_of_directors body",
            meeting_id
        )));
    }

    let resolution: Resolution = store
        .read_resolution("main", meeting_id, resolution_id)
        .map_err(|_| AppError::NotFound(format!("resolution {} not found", resolution_id)))?;
    if !resolution.passed() {
        return Err(AppError::UnprocessableEntity(format!(
            "resolution {} did not pass",
            resolution_id
        )));
    }

    Ok(())
}

fn validate_resolution_for_equity_workflow(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    meeting_id: MeetingId,
    resolution_id: ResolutionId,
) -> Result<(), AppError> {
    let meeting = store
        .read::<Meeting>("main", meeting_id)
        .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

    let body = store
        .read::<GovernanceBody>("main", meeting.body_id())
        .map_err(|_| {
            AppError::NotFound(format!("governance body {} not found", meeting.body_id()))
        })?;

    if body.entity_id() != entity_id {
        return Err(AppError::BadRequest(format!(
            "meeting {} does not belong to entity {}",
            meeting_id, entity_id
        )));
    }
    if !matches!(
        body.body_type(),
        BodyType::BoardOfDirectors | BodyType::LlcMemberVote
    ) {
        return Err(AppError::UnprocessableEntity(format!(
            "meeting {} must be associated with board_of_directors or llc_member_vote",
            meeting_id
        )));
    }

    let resolution: Resolution = store
        .read_resolution("main", meeting_id, resolution_id)
        .map_err(|_| AppError::NotFound(format!("resolution {} not found", resolution_id)))?;
    if !resolution.passed() {
        return Err(AppError::UnprocessableEntity(format!(
            "resolution {} did not pass",
            resolution_id
        )));
    }

    Ok(())
}

// ── Converters ───────────────────────────────────────────────────────

fn holder_to_response(h: &Holder) -> HolderResponse {
    HolderResponse {
        holder_id: h.holder_id(),
        contact_id: h.contact_id(),
        linked_entity_id: h.linked_entity_id(),
        name: h.name().to_owned(),
        holder_type: h.holder_type(),
        created_at: h.created_at().to_rfc3339(),
    }
}

fn legal_entity_to_response(le: &LegalEntity) -> LegalEntityResponse {
    LegalEntityResponse {
        legal_entity_id: le.legal_entity_id(),
        workspace_id: le.workspace_id(),
        linked_entity_id: le.linked_entity_id(),
        name: le.name().to_owned(),
        role: le.role(),
        created_at: le.created_at().to_rfc3339(),
    }
}

fn control_link_to_response(l: &ControlLink) -> ControlLinkResponse {
    ControlLinkResponse {
        control_link_id: l.control_link_id(),
        parent_legal_entity_id: l.parent_legal_entity_id(),
        child_legal_entity_id: l.child_legal_entity_id(),
        control_type: l.control_type(),
        voting_power_bps: l.voting_power_bps(),
        created_at: l.created_at().to_rfc3339(),
    }
}

fn instrument_to_response(i: &Instrument) -> InstrumentResponse {
    InstrumentResponse {
        instrument_id: i.instrument_id(),
        issuer_legal_entity_id: i.issuer_legal_entity_id(),
        symbol: i.symbol().to_owned(),
        kind: i.kind(),
        authorized_units: i.authorized_units(),
        issue_price_cents: i.issue_price_cents(),
        status: i.status(),
        created_at: i.created_at().to_rfc3339(),
    }
}

fn position_to_response(p: &Position) -> PositionResponse {
    PositionResponse {
        position_id: p.position_id(),
        issuer_legal_entity_id: p.issuer_legal_entity_id(),
        holder_id: p.holder_id(),
        instrument_id: p.instrument_id(),
        quantity_units: p.quantity_units(),
        principal_cents: p.principal_cents(),
        status: p.status(),
        updated_at: p.updated_at().to_rfc3339(),
    }
}

fn round_to_response(r: &EquityRound) -> RoundResponse {
    RoundResponse {
        round_id: r.equity_round_id(),
        issuer_legal_entity_id: r.issuer_legal_entity_id(),
        name: r.name().to_owned(),
        pre_money_cents: r.pre_money_cents(),
        round_price_cents: r.round_price_cents(),
        target_raise_cents: r.target_raise_cents(),
        conversion_target_instrument_id: r.conversion_target_instrument_id(),
        rule_set_id: r.rule_set_id(),
        board_approval_meeting_id: r.board_approval_meeting_id(),
        board_approval_resolution_id: r.board_approval_resolution_id(),
        board_approved_at: r.board_approved_at().map(|v| v.to_rfc3339()),
        accepted_by_contact_id: r.accepted_by_contact_id(),
        accepted_at: r.accepted_at().map(|v| v.to_rfc3339()),
        status: r.status(),
        created_at: r.created_at().to_rfc3339(),
    }
}

fn rule_set_to_response(r: &EquityRuleSet) -> RuleSetResponse {
    RuleSetResponse {
        rule_set_id: r.rule_set_id(),
        anti_dilution_method: r.anti_dilution_method(),
        conversion_precedence: r.conversion_precedence().to_vec(),
    }
}

fn transfer_workflow_to_response(w: &TransferWorkflow) -> TransferWorkflowResponse {
    TransferWorkflowResponse {
        transfer_workflow_id: w.transfer_workflow_id(),
        transfer_id: w.transfer_id(),
        prepare_intent_id: w.prepare_intent_id(),
        execute_intent_id: w.execute_intent_id(),
        transfer_status: w.transfer_status(),
        execution_status: w.execution_status(),
        active_packet_id: w.active_packet_id(),
        last_packet_hash: w.last_packet_hash().map(ToOwned::to_owned),
        board_approval_meeting_id: w.board_approval_meeting_id(),
        board_approval_resolution_id: w.board_approval_resolution_id(),
        generated_documents: w.generated_documents().to_vec(),
        created_at: w.created_at().to_rfc3339(),
        updated_at: w.updated_at().to_rfc3339(),
    }
}

fn fundraising_workflow_to_response(w: &FundraisingWorkflow) -> FundraisingWorkflowResponse {
    FundraisingWorkflowResponse {
        fundraising_workflow_id: w.fundraising_workflow_id(),
        round_id: w.round_id(),
        prepare_intent_id: w.prepare_intent_id(),
        accept_intent_id: w.accept_intent_id(),
        close_intent_id: w.close_intent_id(),
        execution_status: w.execution_status(),
        active_packet_id: w.active_packet_id(),
        last_packet_hash: w.last_packet_hash().map(ToOwned::to_owned),
        rule_set_id: w.rule_set_id(),
        round_status: w.round_status(),
        board_approval_meeting_id: w.board_approval_meeting_id(),
        board_approval_resolution_id: w.board_approval_resolution_id(),
        board_packet_documents: w.board_packet_documents().to_vec(),
        closing_packet_documents: w.closing_packet_documents().to_vec(),
        created_at: w.created_at().to_rfc3339(),
        updated_at: w.updated_at().to_rfc3339(),
    }
}

fn packet_to_response(packet: &TransactionPacket) -> TransactionPacketResponse {
    TransactionPacketResponse {
        packet_id: packet.packet_id(),
        entity_id: packet.entity_id(),
        intent_id: packet.intent_id(),
        workflow_type: packet.workflow_type(),
        workflow_id: packet.workflow_id().to_owned(),
        status: packet.status(),
        manifest_hash: packet.manifest_hash().to_owned(),
        items: packet.items().to_vec(),
        required_signers: packet.required_signers().to_vec(),
        signatures: packet
            .signatures()
            .iter()
            .map(|s| PacketSignatureResponse {
                signature_id: s.signature_id(),
                signer_identity: s.signer_identity().to_owned(),
                channel: s.channel().to_owned(),
                signed_at: s.signed_at().to_rfc3339(),
            })
            .collect(),
        created_at: packet.created_at().to_rfc3339(),
        finalized_at: packet.finalized_at().map(|v| v.to_rfc3339()),
    }
}

fn default_transfer_docs() -> Vec<String> {
    vec![
        "documents/governance/transactions/stock-transfer-agreement.md".to_owned(),
        "documents/governance/transactions/transfer-board-consent.md".to_owned(),
    ]
}

fn default_board_packet_docs() -> Vec<String> {
    vec![
        "documents/governance/transactions/board-consent.md".to_owned(),
        "documents/governance/transactions/equity-issuance-approval.md".to_owned(),
    ]
}

fn default_closing_packet_docs() -> Vec<String> {
    vec![
        "documents/governance/transactions/subscription-agreement.md".to_owned(),
        "documents/governance/transactions/investor-rights-agreement.md".to_owned(),
    ]
}

fn default_required_signers() -> Vec<String> {
    vec!["officer".to_owned(), "board".to_owned()]
}

fn to_packet_items(documents: &[String]) -> Vec<PacketItem> {
    documents
        .iter()
        .enumerate()
        .map(|(idx, doc)| PacketItem {
            item_id: format!("item-{}", idx + 1),
            title: doc.rsplit('/').next().unwrap_or(doc).to_owned(),
            document_path: doc.clone(),
            required: true,
        })
        .collect()
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn create_holder(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateHolderRequest>,
) -> Result<Json<HolderResponse>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("holder name is required".to_owned()));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let holder = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let holder = Holder::new(
                HolderId::new(),
                req.contact_id,
                req.linked_entity_id,
                req.name,
                req.holder_type,
                req.external_reference,
            );
            let path = format!("cap-table/holders/{}.json", holder.holder_id());
            store
                .write_json(
                    "main",
                    &path,
                    &holder,
                    &format!("Create holder {}", holder.holder_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(holder)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(holder_to_response(&holder)))
}

async fn create_legal_entity(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateLegalEntityRequest>,
) -> Result<Json<LegalEntityResponse>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "legal entity name is required".to_owned(),
        ));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let legal_entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let le = LegalEntity::new(
                LegalEntityId::new(),
                workspace_id,
                req.linked_entity_id,
                req.name,
                req.role,
            );
            let path = format!("cap-table/entities/{}.json", le.legal_entity_id());
            store
                .write_json(
                    "main",
                    &path,
                    &le,
                    &format!("Create legal entity {}", le.legal_entity_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(le)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(legal_entity_to_response(&legal_entity)))
}

async fn create_control_link(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateControlLinkRequest>,
) -> Result<Json<ControlLinkResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let link = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let entities = read_all::<LegalEntity>(&store)?;
            let known: HashSet<LegalEntityId> =
                entities.iter().map(|e| e.legal_entity_id()).collect();
            if !known.contains(&req.parent_legal_entity_id)
                || !known.contains(&req.child_legal_entity_id)
            {
                return Err(AppError::BadRequest(
                    "parent_legal_entity_id and child_legal_entity_id must exist".to_owned(),
                ));
            }

            let link = ControlLink::new(
                ControlLinkId::new(),
                req.parent_legal_entity_id,
                req.child_legal_entity_id,
                req.control_type,
                req.voting_power_bps,
                req.notes,
            );
            let path = format!("cap-table/control-links/{}.json", link.control_link_id());
            store
                .write_json(
                    "main",
                    &path,
                    &link,
                    &format!("Create control link {}", link.control_link_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(link)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(control_link_to_response(&link)))
}

async fn create_instrument(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateInstrumentRequest>,
) -> Result<Json<InstrumentResponse>, AppError> {
    if req.symbol.trim().is_empty() {
        return Err(AppError::BadRequest(
            "instrument symbol is required".to_owned(),
        ));
    }
    if req.authorized_units.is_some_and(|v| v < 0) {
        return Err(AppError::BadRequest(
            "authorized_units cannot be negative".to_owned(),
        ));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let instrument = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let entities = read_all::<LegalEntity>(&store)?;
            if !entities
                .iter()
                .any(|e| e.legal_entity_id() == req.issuer_legal_entity_id)
            {
                return Err(AppError::BadRequest(
                    "issuer_legal_entity_id does not exist".to_owned(),
                ));
            }

            let instrument = Instrument::new(
                InstrumentId::new(),
                req.issuer_legal_entity_id,
                req.symbol,
                req.kind,
                req.authorized_units,
                req.issue_price_cents,
                req.terms,
            );
            let path = format!("cap-table/instruments/{}.json", instrument.instrument_id());
            store
                .write_json(
                    "main",
                    &path,
                    &instrument,
                    &format!("Create instrument {}", instrument.instrument_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(instrument)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(instrument_to_response(&instrument)))
}

async fn adjust_position(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<AdjustPositionRequest>,
) -> Result<Json<PositionResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let position = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let holders = read_all::<Holder>(&store)?;
            if !holders.iter().any(|h| h.holder_id() == req.holder_id) {
                return Err(AppError::BadRequest("holder_id does not exist".to_owned()));
            }

            let instruments = read_all::<Instrument>(&store)?;
            let instrument = instruments
                .iter()
                .find(|i| i.instrument_id() == req.instrument_id)
                .ok_or_else(|| AppError::BadRequest("instrument_id does not exist".to_owned()))?;
            if instrument.issuer_legal_entity_id() != req.issuer_legal_entity_id {
                return Err(AppError::BadRequest(
                    "instrument issuer does not match issuer_legal_entity_id".to_owned(),
                ));
            }

            let all_positions = read_all::<Position>(&store)?;
            let existing = all_positions.into_iter().find(|p| {
                p.issuer_legal_entity_id() == req.issuer_legal_entity_id
                    && p.holder_id() == req.holder_id
                    && p.instrument_id() == req.instrument_id
            });

            let mut position = if let Some(mut p) = existing {
                p.apply_delta(
                    req.quantity_delta,
                    req.principal_delta_cents,
                    req.source_reference.clone(),
                    None,
                    None,
                )?;
                p
            } else {
                if req.quantity_delta < 0 || req.principal_delta_cents < 0 {
                    return Err(AppError::BadRequest(
                        "cannot create a new position with negative deltas".to_owned(),
                    ));
                }
                Position::new(
                    PositionId::new(),
                    req.issuer_legal_entity_id,
                    req.holder_id,
                    req.instrument_id,
                    req.quantity_delta,
                    req.principal_delta_cents,
                    req.source_reference,
                    None,
                    None,
                )?
            };

            // Keep a deterministic hash of current position values for traceability.
            let hash = hash_json(&serde_json::json!({
                "quantity_units": position.quantity_units(),
                "principal_cents": position.principal_cents(),
                "holder_id": position.holder_id(),
                "instrument_id": position.instrument_id(),
            }));
            position.apply_delta(
                0,
                0,
                position.source_reference().map(str::to_owned),
                None,
                Some(hash),
            )?;

            let path = format!("cap-table/positions/{}.json", position.position_id());
            store
                .write_json(
                    "main",
                    &path,
                    &position,
                    &format!("Adjust position {}", position.position_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(position)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(position_to_response(&position)))
}

async fn create_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateRoundRequest>,
) -> Result<Json<RoundResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let entities = read_all::<LegalEntity>(&store)?;
            if !entities
                .iter()
                .any(|e| e.legal_entity_id() == req.issuer_legal_entity_id)
            {
                return Err(AppError::BadRequest(
                    "issuer_legal_entity_id does not exist".to_owned(),
                ));
            }

            if let Some(target) = req.conversion_target_instrument_id {
                let instruments = read_all::<Instrument>(&store)?;
                if !instruments.iter().any(|i| i.instrument_id() == target) {
                    return Err(AppError::BadRequest(
                        "conversion_target_instrument_id does not exist".to_owned(),
                    ));
                }
            }

            let round = EquityRound::new(
                EquityRoundId::new(),
                req.issuer_legal_entity_id,
                req.name,
                req.pre_money_cents,
                req.round_price_cents,
                req.target_raise_cents,
                req.conversion_target_instrument_id,
                req.metadata,
            );
            let path = format!("cap-table/rounds/{}.json", round.equity_round_id());
            store
                .write_json(
                    "main",
                    &path,
                    &round,
                    &format!("Create equity round {}", round.equity_round_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(round)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(round_to_response(&round)))
}

async fn apply_round_terms(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(round_id): Path<EquityRoundId>,
    Json(req): Json<ApplyRoundTermsRequest>,
) -> Result<Json<RuleSetResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let rules = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut round = store
                .read::<EquityRound>("main", round_id)
                .map_err(|_| AppError::NotFound(format!("equity round {} not found", round_id)))?;

            let rules = EquityRuleSet::new(
                EquityRuleSetId::new(),
                req.anti_dilution_method,
                req.conversion_precedence,
                req.protective_provisions,
            );
            round.apply_terms(rules.rule_set_id())?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rules/{}.json", rules.rule_set_id()),
                    &rules,
                )
                .map_err(|e| AppError::Internal(format!("serialize rules: {e}")))?,
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Apply terms to round {}", round.equity_round_id()),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(rules)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(rule_set_to_response(&rules)))
}

async fn board_approve_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(round_id): Path<EquityRoundId>,
    Json(req): Json<BoardApproveRoundRequest>,
) -> Result<Json<RoundResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut round = store
                .read::<EquityRound>("main", round_id)
                .map_err(|_| AppError::NotFound(format!("equity round {} not found", round_id)))?;

            validate_board_resolution_for_round(
                &store,
                entity_id,
                req.meeting_id,
                req.resolution_id,
            )?;
            round.record_board_approval(req.meeting_id, req.resolution_id)?;

            let path = format!("cap-table/rounds/{}.json", round.equity_round_id());
            store
                .write_json(
                    "main",
                    &path,
                    &round,
                    &format!("Board approve round {}", round.equity_round_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(round)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(round_to_response(&round)))
}

async fn accept_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(round_id): Path<EquityRoundId>,
    Json(req): Json<AcceptRoundRequest>,
) -> Result<Json<RoundResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut round = store
                .read::<EquityRound>("main", round_id)
                .map_err(|_| AppError::NotFound(format!("equity round {} not found", round_id)))?;
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;

            ensure_authorized_round_intent(&intent, entity_id, round_id, "equity.round.accept")?;
            round.accept(req.accepted_by_contact_id)?;
            intent.mark_executed()?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Accept round {}", round.equity_round_id()),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(round)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(round_to_response(&round)))
}

async fn create_transfer_workflow(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateTransferWorkflowRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    if req.price_per_share_cents.is_some_and(|v| v < 0) {
        return Err(AppError::BadRequest(
            "price_per_share_cents cannot be negative".to_owned(),
        ));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            store
                .read::<ShareClass>("main", req.share_class_id)
                .map_err(|_| AppError::BadRequest("share_class_id does not exist".to_owned()))?;

            let mut intent = store
                .read::<Intent>("main", req.prepare_intent_id)
                .map_err(|_| {
                    AppError::NotFound(format!("intent {} not found", req.prepare_intent_id))
                })?;
            ensure_authorized_intent(&intent, entity_id, "equity.transfer.prepare", None)?;

            let transfer = ShareTransfer::new(
                TransferId::new(),
                entity_id,
                workspace_id,
                req.share_class_id,
                req.from_contact_id,
                req.to_contact_id,
                req.transfer_type,
                ShareCount::new(req.share_count),
                req.price_per_share_cents.map(Cents::new),
                req.relationship_to_holder,
                req.governing_doc_type,
                req.transferee_rights,
            )?;

            let mut workflow = TransferWorkflow::new(
                TransferWorkflowId::new(),
                entity_id,
                workspace_id,
                transfer.transfer_id(),
                req.prepare_intent_id,
            );
            workflow.sync_from_transfer(&transfer);
            intent.mark_executed()?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/transfers/{}.json", transfer.transfer_id()),
                    &transfer,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer workflow: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!(
                        "Create transfer workflow {}",
                        workflow.transfer_workflow_id()
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn generate_transfer_workflow_docs(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<GenerateWorkflowDocsRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "transfer workflow belongs to a different entity".to_owned(),
                ));
            }
            store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;

            let docs = if req.documents.is_empty() {
                default_transfer_docs()
            } else {
                req.documents
            };
            workflow.add_generated_documents(docs);

            let path = format!(
                "cap-table/transfer-workflows/{}.json",
                workflow.transfer_workflow_id()
            );
            store
                .write_json(
                    "main",
                    &path,
                    &workflow,
                    &format!("Generate docs for transfer workflow {}", workflow_id),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn submit_transfer_workflow_for_review(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<SubmitTransferReviewRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "transfer workflow belongs to a different entity".to_owned(),
                ));
            }
            let mut transfer = store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;
            transfer.submit_for_review()?;
            workflow.sync_from_transfer(&transfer);

            let files = vec![
                FileWrite::json(
                    format!("cap-table/transfers/{}.json", transfer.transfer_id()),
                    &transfer,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer workflow: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Submit transfer workflow {} for review", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn record_transfer_workflow_review(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<RecordTransferReviewRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    if req.notes.trim().is_empty() {
        return Err(AppError::BadRequest("notes are required".to_owned()));
    }
    if req.reviewer.trim().is_empty() {
        return Err(AppError::BadRequest("reviewer is required".to_owned()));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "transfer workflow belongs to a different entity".to_owned(),
                ));
            }
            let mut transfer = store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;
            transfer.record_bylaws_review(req.approved, req.notes, req.reviewer)?;
            workflow.sync_from_transfer(&transfer);

            let files = vec![
                FileWrite::json(
                    format!("cap-table/transfers/{}.json", transfer.transfer_id()),
                    &transfer,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer workflow: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Record transfer workflow {} review", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn record_transfer_workflow_rofr(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<RecordTransferRofrRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "transfer workflow belongs to a different entity".to_owned(),
                ));
            }
            let mut transfer = store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;
            transfer.record_rofr_decision(req.offered, req.waived)?;
            workflow.sync_from_transfer(&transfer);

            let files = vec![
                FileWrite::json(
                    format!("cap-table/transfers/{}.json", transfer.transfer_id()),
                    &transfer,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer workflow: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Record transfer workflow {} ROFR", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn record_transfer_workflow_board_approval(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<RecordTransferBoardApprovalRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "transfer workflow belongs to a different entity".to_owned(),
                ));
            }
            let mut transfer = store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;

            validate_resolution_for_equity_workflow(
                &store,
                entity_id,
                req.meeting_id,
                req.resolution_id,
            )?;
            transfer.approve(Some(req.resolution_id))?;
            workflow.record_board_approval(req.meeting_id, req.resolution_id);
            workflow.sync_from_transfer(&transfer);

            let files = vec![
                FileWrite::json(
                    format!("cap-table/transfers/{}.json", transfer.transfer_id()),
                    &transfer,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer workflow: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Record transfer workflow {} board approval", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn record_transfer_workflow_execution(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<RecordTransferExecutionRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "transfer workflow belongs to a different entity".to_owned(),
                ));
            }

            let mut transfer = store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;
            ensure_authorized_transfer_intent(
                &intent,
                entity_id,
                transfer.transfer_id(),
                "equity.transfer.execute",
            )?;

            transfer.execute()?;
            workflow.set_execute_intent_id(req.intent_id);
            workflow.sync_from_transfer(&transfer);
            intent.mark_executed()?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/transfers/{}.json", transfer.transfer_id()),
                    &transfer,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer workflow: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Record transfer workflow {} execution", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn get_transfer_workflow(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "transfer workflow belongs to a different entity".to_owned(),
                ));
            }
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn create_fundraising_workflow(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateFundraisingWorkflowRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let entities = read_all::<LegalEntity>(&store)?;
            if !entities
                .iter()
                .any(|e| e.legal_entity_id() == req.issuer_legal_entity_id)
            {
                return Err(AppError::BadRequest(
                    "issuer_legal_entity_id does not exist".to_owned(),
                ));
            }

            if let Some(target) = req.conversion_target_instrument_id {
                let instruments = read_all::<Instrument>(&store)?;
                if !instruments.iter().any(|i| i.instrument_id() == target) {
                    return Err(AppError::BadRequest(
                        "conversion_target_instrument_id does not exist".to_owned(),
                    ));
                }
            }

            let mut intent = store
                .read::<Intent>("main", req.prepare_intent_id)
                .map_err(|_| {
                    AppError::NotFound(format!("intent {} not found", req.prepare_intent_id))
                })?;
            ensure_authorized_intent(&intent, entity_id, "equity.fundraising.prepare", None)?;

            let round = EquityRound::new(
                EquityRoundId::new(),
                req.issuer_legal_entity_id,
                req.name,
                req.pre_money_cents,
                req.round_price_cents,
                req.target_raise_cents,
                req.conversion_target_instrument_id,
                req.metadata,
            );
            let mut workflow = FundraisingWorkflow::new(
                FundraisingWorkflowId::new(),
                entity_id,
                workspace_id,
                round.equity_round_id(),
                req.prepare_intent_id,
            );
            workflow.sync_from_round(&round);
            intent.mark_executed()?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/fundraising-workflows/{}.json",
                        workflow.fundraising_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize fundraising workflow: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!(
                        "Create fundraising workflow {}",
                        workflow.fundraising_workflow_id()
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn apply_fundraising_workflow_terms(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<ApplyFundraisingTermsRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "fundraising workflow belongs to a different entity".to_owned(),
                ));
            }
            let mut round = store
                .read::<EquityRound>("main", workflow.round_id())
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", workflow.round_id()))
                })?;

            let rules = EquityRuleSet::new(
                EquityRuleSetId::new(),
                req.anti_dilution_method,
                req.conversion_precedence,
                req.protective_provisions,
            );
            round.apply_terms(rules.rule_set_id())?;
            workflow.sync_from_round(&round);

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rules/{}.json", rules.rule_set_id()),
                    &rules,
                )
                .map_err(|e| AppError::Internal(format!("serialize rules: {e}")))?,
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/fundraising-workflows/{}.json",
                        workflow.fundraising_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize fundraising workflow: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Apply terms for fundraising workflow {}", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn generate_fundraising_board_packet(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<GenerateWorkflowDocsRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "fundraising workflow belongs to a different entity".to_owned(),
                ));
            }

            let docs = if req.documents.is_empty() {
                default_board_packet_docs()
            } else {
                req.documents
            };
            workflow.add_board_packet_documents(docs);

            let path = format!(
                "cap-table/fundraising-workflows/{}.json",
                workflow.fundraising_workflow_id()
            );
            store
                .write_json(
                    "main",
                    &path,
                    &workflow,
                    &format!(
                        "Generate board packet for fundraising workflow {}",
                        workflow_id
                    ),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn record_fundraising_workflow_board_approval(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<RecordFundraisingBoardApprovalRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "fundraising workflow belongs to a different entity".to_owned(),
                ));
            }

            let mut round = store
                .read::<EquityRound>("main", workflow.round_id())
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", workflow.round_id()))
                })?;

            validate_resolution_for_equity_workflow(
                &store,
                entity_id,
                req.meeting_id,
                req.resolution_id,
            )?;
            round.record_board_approval(req.meeting_id, req.resolution_id)?;
            workflow.sync_from_round(&round);

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/fundraising-workflows/{}.json",
                        workflow.fundraising_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize fundraising workflow: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!(
                        "Record board approval for fundraising workflow {}",
                        workflow_id
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn record_fundraising_workflow_acceptance(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<RecordFundraisingAcceptanceRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "fundraising workflow belongs to a different entity".to_owned(),
                ));
            }

            let mut round = store
                .read::<EquityRound>("main", workflow.round_id())
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", workflow.round_id()))
                })?;
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;

            ensure_authorized_round_intent(
                &intent,
                entity_id,
                workflow.round_id(),
                "equity.fundraising.accept",
            )?;
            round.accept(req.accepted_by_contact_id)?;
            workflow.set_accept_intent_id(req.intent_id);
            workflow.sync_from_round(&round);
            intent.mark_executed()?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/fundraising-workflows/{}.json",
                        workflow.fundraising_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize fundraising workflow: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Record acceptance for fundraising workflow {}", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn generate_fundraising_closing_packet(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<GenerateWorkflowDocsRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "fundraising workflow belongs to a different entity".to_owned(),
                ));
            }

            let docs = if req.documents.is_empty() {
                default_closing_packet_docs()
            } else {
                req.documents
            };
            workflow.add_closing_packet_documents(docs);

            let path = format!(
                "cap-table/fundraising-workflows/{}.json",
                workflow.fundraising_workflow_id()
            );
            store
                .write_json(
                    "main",
                    &path,
                    &workflow,
                    &format!(
                        "Generate closing packet for fundraising workflow {}",
                        workflow_id
                    ),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn record_fundraising_workflow_close(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<RecordFundraisingCloseRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "fundraising workflow belongs to a different entity".to_owned(),
                ));
            }

            let mut round = store
                .read::<EquityRound>("main", workflow.round_id())
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", workflow.round_id()))
                })?;
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;
            ensure_authorized_round_intent(
                &intent,
                entity_id,
                workflow.round_id(),
                "equity.fundraising.close",
            )?;

            round.close()?;
            workflow.set_close_intent_id(req.intent_id);
            workflow.sync_from_round(&round);
            intent.mark_executed()?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/fundraising-workflows/{}.json",
                        workflow.fundraising_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize fundraising workflow: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Record close for fundraising workflow {}", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn get_fundraising_workflow(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "fundraising workflow belongs to a different entity".to_owned(),
                ));
            }
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn prepare_transfer_workflow_execution(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<PrepareWorkflowExecutionRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "transfer workflow belongs to a different entity".to_owned(),
                ));
            }
            let transfer = store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;

            ensure_authorized_transfer_intent(
                &intent,
                entity_id,
                transfer.transfer_id(),
                "equity.transfer.execute",
            )?;
            store
                .read::<ApprovalArtifact>("main", req.approval_artifact_id)
                .map_err(|_| {
                    AppError::NotFound(format!(
                        "approval artifact {} not found",
                        req.approval_artifact_id
                    ))
                })?;
            for request_id in &req.document_request_ids {
                store
                    .read::<DocumentRequest>("main", *request_id)
                    .map_err(|_| {
                        AppError::NotFound(format!("document request {} not found", request_id))
                    })?;
            }

            intent.bind_approval_artifact(req.approval_artifact_id);
            for request_id in req.document_request_ids {
                intent.bind_document_request(request_id);
            }

            workflow.set_execute_intent_id(req.intent_id);
            workflow.mark_prereqs_ready();

            let files = vec![
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "EQUITY: prepare transfer workflow {} execution",
                        workflow_id
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfer_workflow_to_response(&response)))
}

async fn prepare_fundraising_workflow_execution(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<PrepareWorkflowExecutionRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    let phase = req.phase.unwrap_or_else(|| "accept".to_owned());
    let is_close = phase.eq_ignore_ascii_case("close");

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            if workflow.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "fundraising workflow belongs to a different entity".to_owned(),
                ));
            }
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;
            let expected = if is_close {
                "equity.fundraising.close"
            } else {
                "equity.fundraising.accept"
            };
            ensure_authorized_round_intent(&intent, entity_id, workflow.round_id(), expected)?;
            store
                .read::<ApprovalArtifact>("main", req.approval_artifact_id)
                .map_err(|_| {
                    AppError::NotFound(format!(
                        "approval artifact {} not found",
                        req.approval_artifact_id
                    ))
                })?;
            for request_id in &req.document_request_ids {
                store
                    .read::<DocumentRequest>("main", *request_id)
                    .map_err(|_| {
                        AppError::NotFound(format!("document request {} not found", request_id))
                    })?;
            }

            intent.bind_approval_artifact(req.approval_artifact_id);
            for request_id in req.document_request_ids {
                intent.bind_document_request(request_id);
            }

            if is_close {
                workflow.set_close_intent_id(req.intent_id);
            } else {
                workflow.set_accept_intent_id(req.intent_id);
            }
            workflow.mark_prereqs_ready();

            let files = vec![
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/fundraising-workflows/{}.json",
                        workflow.fundraising_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize fundraising workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "EQUITY: prepare fundraising workflow {} {}",
                        workflow_id, expected
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(fundraising_workflow_to_response(&response)))
}

async fn compile_transfer_workflow_packet(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<CompileWorkflowPacketRequest>,
) -> Result<Json<TransactionPacketResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let packet = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            let intent_id = workflow.execute_intent_id().ok_or_else(|| {
                AppError::UnprocessableEntity(format!(
                    "transfer workflow {} has no bound execute intent",
                    workflow_id
                ))
            })?;
            let documents = if workflow.generated_documents().is_empty() {
                default_transfer_docs()
            } else {
                workflow.generated_documents().to_vec()
            };
            let packet = TransactionPacket::new(
                PacketId::new(),
                entity_id,
                intent_id,
                WorkflowType::Transfer,
                workflow_id.to_string(),
                to_packet_items(&documents),
                if req.required_signers.is_empty() {
                    default_required_signers()
                } else {
                    req.required_signers
                },
            );
            workflow.mark_packet_compiled(packet.packet_id(), packet.manifest_hash().to_owned());

            let files = vec![
                FileWrite::json(
                    format!("execution/packets/{}.json", packet.packet_id()),
                    &packet,
                )
                .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!("EXECUTION: compile transfer packet {}", packet.packet_id()),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(packet)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(packet_to_response(&packet)))
}

async fn compile_fundraising_workflow_packet(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<CompileWorkflowPacketRequest>,
) -> Result<Json<TransactionPacketResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    let phase = req.phase.unwrap_or_else(|| "accept".to_owned());
    let is_close = phase.eq_ignore_ascii_case("close");

    let packet = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;

            let intent_id = if is_close {
                workflow.close_intent_id()
            } else {
                workflow.accept_intent_id()
            }
            .ok_or_else(|| {
                AppError::UnprocessableEntity(format!(
                    "fundraising workflow {} has no bound {} intent",
                    workflow_id,
                    if is_close { "close" } else { "accept" }
                ))
            })?;

            let documents = if is_close {
                if workflow.closing_packet_documents().is_empty() {
                    default_closing_packet_docs()
                } else {
                    workflow.closing_packet_documents().to_vec()
                }
            } else if workflow.board_packet_documents().is_empty() {
                default_board_packet_docs()
            } else {
                workflow.board_packet_documents().to_vec()
            };

            let packet = TransactionPacket::new(
                PacketId::new(),
                entity_id,
                intent_id,
                WorkflowType::Fundraising,
                workflow_id.to_string(),
                to_packet_items(&documents),
                if req.required_signers.is_empty() {
                    default_required_signers()
                } else {
                    req.required_signers
                },
            );
            workflow.mark_packet_compiled(packet.packet_id(), packet.manifest_hash().to_owned());

            let files = vec![
                FileWrite::json(
                    format!("execution/packets/{}.json", packet.packet_id()),
                    &packet,
                )
                .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/fundraising-workflows/{}.json",
                        workflow.fundraising_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize fundraising workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "EXECUTION: compile fundraising packet {}",
                        packet.packet_id()
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(packet)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(packet_to_response(&packet)))
}

async fn start_transfer_workflow_signatures(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<StartWorkflowSignaturesRequest>,
) -> Result<Json<TransactionPacketResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    let packet = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            let packet_id = workflow.active_packet_id().ok_or_else(|| {
                AppError::UnprocessableEntity("workflow has no compiled packet".to_owned())
            })?;
            let mut packet = store
                .read::<TransactionPacket>("main", packet_id)
                .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
            packet.mark_ready_for_signature();
            workflow.mark_signing_in_progress();
            let files = vec![
                FileWrite::json(
                    format!("execution/packets/{}.json", packet.packet_id()),
                    &packet,
                )
                .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                FileWrite::json(
                    format!("cap-table/transfer-workflows/{}.json", workflow_id),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "EXECUTION: start signatures transfer workflow {}",
                        workflow_id
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(packet)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
    Ok(Json(packet_to_response(&packet)))
}

async fn start_fundraising_workflow_signatures(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<StartWorkflowSignaturesRequest>,
) -> Result<Json<TransactionPacketResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    let packet = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            let packet_id = workflow.active_packet_id().ok_or_else(|| {
                AppError::UnprocessableEntity("workflow has no compiled packet".to_owned())
            })?;
            let mut packet = store
                .read::<TransactionPacket>("main", packet_id)
                .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
            packet.mark_ready_for_signature();
            workflow.mark_signing_in_progress();
            let files = vec![
                FileWrite::json(
                    format!("execution/packets/{}.json", packet.packet_id()),
                    &packet,
                )
                .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                FileWrite::json(
                    format!("cap-table/fundraising-workflows/{}.json", workflow_id),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "EXECUTION: start signatures fundraising workflow {}",
                        workflow_id
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(packet)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
    Ok(Json(packet_to_response(&packet)))
}

async fn record_transfer_workflow_signature(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<RecordWorkflowSignatureRequest>,
) -> Result<Json<TransactionPacketResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    if req.signer_identity.trim().is_empty() {
        return Err(AppError::BadRequest(
            "signer_identity is required".to_owned(),
        ));
    }

    let packet = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            let packet_id = workflow.active_packet_id().ok_or_else(|| {
                AppError::UnprocessableEntity("workflow has no compiled packet".to_owned())
            })?;
            let mut packet = store
                .read::<TransactionPacket>("main", packet_id)
                .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
            packet.record_signature(
                PacketSignatureId::new(),
                req.signer_identity,
                req.channel.unwrap_or_else(|| "internal".to_owned()),
            );
            if packet.status() == TransactionPacketStatus::FullySigned {
                packet.mark_executable();
                workflow.mark_signing_complete();
                workflow.mark_executable();
            }
            let files = vec![
                FileWrite::json(
                    format!("execution/packets/{}.json", packet.packet_id()),
                    &packet,
                )
                .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                FileWrite::json(
                    format!("cap-table/transfer-workflows/{}.json", workflow_id),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "EXECUTION: record transfer workflow {} signature",
                        workflow_id
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(packet)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
    Ok(Json(packet_to_response(&packet)))
}

async fn record_fundraising_workflow_signature(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<RecordWorkflowSignatureRequest>,
) -> Result<Json<TransactionPacketResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    if req.signer_identity.trim().is_empty() {
        return Err(AppError::BadRequest(
            "signer_identity is required".to_owned(),
        ));
    }

    let packet = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            let packet_id = workflow.active_packet_id().ok_or_else(|| {
                AppError::UnprocessableEntity("workflow has no compiled packet".to_owned())
            })?;
            let mut packet = store
                .read::<TransactionPacket>("main", packet_id)
                .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
            packet.record_signature(
                PacketSignatureId::new(),
                req.signer_identity,
                req.channel.unwrap_or_else(|| "internal".to_owned()),
            );
            if packet.status() == TransactionPacketStatus::FullySigned {
                packet.mark_executable();
                workflow.mark_signing_complete();
                workflow.mark_executable();
            }
            let files = vec![
                FileWrite::json(
                    format!("execution/packets/{}.json", packet.packet_id()),
                    &packet,
                )
                .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                FileWrite::json(
                    format!("cap-table/fundraising-workflows/{}.json", workflow_id),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "EXECUTION: record fundraising workflow {} signature",
                        workflow_id
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(packet)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
    Ok(Json(packet_to_response(&packet)))
}

async fn finalize_transfer_workflow(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<TransferWorkflowId>,
    Json(req): Json<FinalizeWorkflowRequest>,
) -> Result<Json<TransferWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<TransferWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
            let packet_id = workflow.active_packet_id().ok_or_else(|| {
                AppError::UnprocessableEntity("workflow has no compiled packet".to_owned())
            })?;
            let mut packet = store
                .read::<TransactionPacket>("main", packet_id)
                .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
            if !matches!(packet.status(), TransactionPacketStatus::Executable) {
                return Err(AppError::UnprocessableEntity(
                    "packet must be executable before finalize".to_owned(),
                ));
            }
            let mut transfer = store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;
            let execute_intent_id = workflow.execute_intent_id().ok_or_else(|| {
                AppError::UnprocessableEntity("workflow has no execute intent bound".to_owned())
            })?;
            let mut intent = store
                .read::<Intent>("main", execute_intent_id)
                .map_err(|_| {
                    AppError::NotFound(format!("intent {} not found", execute_intent_id))
                })?;
            ensure_authorized_transfer_intent(
                &intent,
                entity_id,
                transfer.transfer_id(),
                "equity.transfer.execute",
            )?;
            verify_bound_artifacts_for_intent(&store, &intent)?;

            transfer.execute()?;
            intent.mark_executed()?;
            workflow.mark_executed();
            packet.add_evidence_ref(format!("transfer:{}", transfer.transfer_id()));
            packet.mark_executed();

            let files = vec![
                FileWrite::json(
                    format!("cap-table/transfers/{}.json", transfer.transfer_id()),
                    &transfer,
                )
                .map_err(|e| AppError::Internal(format!("serialize transfer: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
                FileWrite::json(
                    format!("execution/packets/{}.json", packet.packet_id()),
                    &packet,
                )
                .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/transfer-workflows/{}.json",
                        workflow.transfer_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!("EQUITY: finalize transfer workflow {}", workflow_id),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
    Ok(Json(transfer_workflow_to_response(&workflow)))
}

async fn finalize_fundraising_workflow(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(workflow_id): Path<FundraisingWorkflowId>,
    Json(req): Json<FinalizeWorkflowRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    let phase = req.phase.unwrap_or_else(|| "close".to_owned());
    let is_close = phase.eq_ignore_ascii_case("close");

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut workflow = store
                .read::<FundraisingWorkflow>("main", workflow_id)
                .map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
            let packet_id = workflow.active_packet_id().ok_or_else(|| {
                AppError::UnprocessableEntity("workflow has no compiled packet".to_owned())
            })?;
            let mut packet = store
                .read::<TransactionPacket>("main", packet_id)
                .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
            if !matches!(packet.status(), TransactionPacketStatus::Executable) {
                return Err(AppError::UnprocessableEntity(
                    "packet must be executable before finalize".to_owned(),
                ));
            }
            let mut round = store
                .read::<EquityRound>("main", workflow.round_id())
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", workflow.round_id()))
                })?;
            let intent_id = if is_close {
                workflow.close_intent_id()
            } else {
                workflow.accept_intent_id()
            }
            .ok_or_else(|| {
                AppError::UnprocessableEntity(format!(
                    "workflow missing {} intent",
                    if is_close { "close" } else { "accept" }
                ))
            })?;
            let mut intent = store
                .read::<Intent>("main", intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", intent_id)))?;
            let expected = if is_close {
                "equity.fundraising.close"
            } else {
                "equity.fundraising.accept"
            };
            ensure_authorized_round_intent(&intent, entity_id, workflow.round_id(), expected)?;
            verify_bound_artifacts_for_intent(&store, &intent)?;

            if is_close {
                round.close()?;
                workflow.mark_executed();
            } else {
                round.accept(None)?;
                workflow.mark_prereqs_ready();
            }
            intent.mark_executed()?;
            packet.add_evidence_ref(format!("round:{}", round.equity_round_id()));
            packet.mark_executed();

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
                FileWrite::json(
                    format!("execution/packets/{}.json", packet.packet_id()),
                    &packet,
                )
                .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                FileWrite::json(
                    format!(
                        "cap-table/fundraising-workflows/{}.json",
                        workflow.fundraising_workflow_id()
                    ),
                    &workflow,
                )
                .map_err(|e| AppError::Internal(format!("serialize workflow: {e}")))?,
            ];
            store
                .commit(
                    "main",
                    &format!(
                        "EQUITY: finalize fundraising workflow {} ({})",
                        workflow_id, expected
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(workflow)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
    Ok(Json(fundraising_workflow_to_response(&workflow)))
}

async fn get_workflow_status(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path((workflow_type, workflow_id)): Path<(String, String)>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<WorkflowStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            if workflow_type == "transfer" {
                let id = workflow_id
                    .parse::<uuid::Uuid>()
                    .map(TransferWorkflowId::from_uuid)
                    .map_err(|_| AppError::BadRequest("invalid transfer workflow id".to_owned()))?;
                let workflow = store.read::<TransferWorkflow>("main", id).map_err(|_| {
                    AppError::NotFound(format!("transfer workflow {} not found", workflow_id))
                })?;
                let packet = workflow
                    .active_packet_id()
                    .and_then(|packet_id| store.read::<TransactionPacket>("main", packet_id).ok())
                    .map(|p| packet_to_response(&p));
                return Ok::<_, AppError>(WorkflowStatusResponse {
                    workflow_type: WorkflowType::Transfer,
                    workflow_id,
                    execution_status: format!("{:?}", workflow.execution_status()),
                    active_packet_id: workflow.active_packet_id(),
                    transfer_workflow: Some(transfer_workflow_to_response(&workflow)),
                    fundraising_workflow: None,
                    packet,
                });
            }

            if workflow_type == "fundraising" {
                let id = workflow_id
                    .parse::<uuid::Uuid>()
                    .map(FundraisingWorkflowId::from_uuid)
                    .map_err(|_| {
                        AppError::BadRequest("invalid fundraising workflow id".to_owned())
                    })?;
                let workflow = store.read::<FundraisingWorkflow>("main", id).map_err(|_| {
                    AppError::NotFound(format!("fundraising workflow {} not found", workflow_id))
                })?;
                let packet = workflow
                    .active_packet_id()
                    .and_then(|packet_id| store.read::<TransactionPacket>("main", packet_id).ok())
                    .map(|p| packet_to_response(&p));
                return Ok::<_, AppError>(WorkflowStatusResponse {
                    workflow_type: WorkflowType::Fundraising,
                    workflow_id,
                    execution_status: format!("{:?}", workflow.execution_status()),
                    active_packet_id: workflow.active_packet_id(),
                    transfer_workflow: None,
                    fundraising_workflow: Some(fundraising_workflow_to_response(&workflow)),
                    packet,
                });
            }

            Err(AppError::BadRequest(
                "workflow_type must be 'transfer' or 'fundraising'".to_owned(),
            ))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

async fn get_cap_table(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<CapTableQuery>,
) -> Result<Json<CapTableResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let holders = read_all::<Holder>(&store)?;
            let legal_entities = read_all::<LegalEntity>(&store)?;
            let instruments = read_all::<Instrument>(&store)?;
            let positions = read_all::<Position>(&store)?;

            let issuer_legal_entity_id =
                infer_issuer(entity_id, &legal_entities, query.issuer_legal_entity_id)?;

            Ok::<_, AppError>(compute_cap_table(
                entity_id,
                issuer_legal_entity_id,
                query.basis,
                &holders,
                &instruments,
                &positions,
            ))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

async fn preview_conversion(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Json(req): Json<PreviewConversionRequest>,
) -> Result<Json<ConversionPreviewResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let preview = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let round = store
                .read::<EquityRound>("main", req.round_id)
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", req.round_id))
                })?;
            let rule_set_id = round
                .rule_set_id()
                .ok_or_else(|| AppError::BadRequest("round terms are not applied".to_owned()))?;
            let rules = store
                .read::<EquityRuleSet>("main", rule_set_id)
                .map_err(|_| AppError::NotFound(format!("rule set {} not found", rule_set_id)))?;

            let instruments = read_all::<Instrument>(&store)?;
            let positions = read_all::<Position>(&store)?;

            let target_instrument_id =
                round.conversion_target_instrument_id().ok_or_else(|| {
                    AppError::BadRequest("conversion_target_instrument_id is required".to_owned())
                })?;

            let (lines, anti_dilution_adjustment_units) =
                compute_conversion_preview(&round, &rules, &instruments, &positions)?;

            let total_new_units = lines
                .iter()
                .map(|l| l.new_units)
                .sum::<i64>()
                .saturating_add(anti_dilution_adjustment_units);

            Ok::<_, AppError>(ConversionPreviewResponse {
                entity_id,
                round_id: req.round_id,
                target_instrument_id,
                lines,
                anti_dilution_adjustment_units,
                total_new_units,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(preview))
}

async fn execute_conversion(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<ExecuteConversionRequest>,
) -> Result<Json<ConversionExecuteResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut round = store
                .read::<EquityRound>("main", req.round_id)
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", req.round_id))
                })?;
            let rule_set_id = round
                .rule_set_id()
                .ok_or_else(|| AppError::BadRequest("round terms are not applied".to_owned()))?;
            let rules = store
                .read::<EquityRuleSet>("main", rule_set_id)
                .map_err(|_| AppError::NotFound(format!("rule set {} not found", rule_set_id)))?;
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;
            ensure_authorized_round_intent(
                &intent,
                entity_id,
                req.round_id,
                "equity.round.execute_conversion",
            )?;

            let target_instrument_id =
                round.conversion_target_instrument_id().ok_or_else(|| {
                    AppError::BadRequest("conversion_target_instrument_id is required".to_owned())
                })?;

            let instruments = read_all::<Instrument>(&store)?;
            let mut positions = read_all::<Position>(&store)?;
            let (lines, anti_dilution_adjustment_units) =
                compute_conversion_preview(&round, &rules, &instruments, &positions)?;

            let mut modified_paths = Vec::new();
            let mut touched_targets: HashSet<PositionId> = HashSet::new();

            for line in &lines {
                let Some(src_idx) = positions
                    .iter()
                    .position(|p| p.position_id() == line.source_position_id)
                else {
                    continue;
                };

                let src = &mut positions[src_idx];
                let close_hash = hash_json(&serde_json::json!({
                    "round_id": req.round_id,
                    "conversion_price_cents": line.conversion_price_cents,
                    "new_units": line.new_units,
                }));
                src.apply_delta(
                    -src.quantity_units(),
                    -src.principal_cents(),
                    req.source_reference.clone(),
                    None,
                    Some(close_hash),
                )?;
                modified_paths.push(
                    FileWrite::json(
                        format!("cap-table/positions/{}.json", src.position_id()),
                        src,
                    )
                    .map_err(|e| AppError::Internal(format!("serialize source position: {e}")))?,
                );

                // Find or create the target position.
                let existing_target_idx = positions.iter().position(|p| {
                    p.issuer_legal_entity_id() == round.issuer_legal_entity_id()
                        && p.holder_id() == line.holder_id
                        && p.instrument_id() == target_instrument_id
                });

                if let Some(idx) = existing_target_idx {
                    let target = &mut positions[idx];
                    let hash = hash_json(&serde_json::json!({
                        "round_id": req.round_id,
                        "source_position_id": line.source_position_id,
                        "new_units": line.new_units,
                    }));
                    target.apply_delta(
                        line.new_units,
                        0,
                        req.source_reference.clone(),
                        None,
                        Some(hash),
                    )?;
                    touched_targets.insert(target.position_id());
                    modified_paths.push(
                        FileWrite::json(
                            format!("cap-table/positions/{}.json", target.position_id()),
                            target,
                        )
                        .map_err(|e| {
                            AppError::Internal(format!("serialize target position: {e}"))
                        })?,
                    );
                } else {
                    let hash = hash_json(&serde_json::json!({
                        "round_id": req.round_id,
                        "source_position_id": line.source_position_id,
                        "new_units": line.new_units,
                    }));
                    let target = Position::new(
                        PositionId::new(),
                        round.issuer_legal_entity_id(),
                        line.holder_id,
                        target_instrument_id,
                        line.new_units,
                        0,
                        req.source_reference.clone(),
                        None,
                        Some(hash),
                    )?;
                    touched_targets.insert(target.position_id());
                    modified_paths.push(
                        FileWrite::json(
                            format!("cap-table/positions/{}.json", target.position_id()),
                            &target,
                        )
                        .map_err(|e| {
                            AppError::Internal(format!("serialize new target position: {e}"))
                        })?,
                    );
                    positions.push(target);
                }
            }

            // Anti-dilution adjustment units are added to a synthetic holder-neutral position only
            // if positive and target instrument exists.
            if anti_dilution_adjustment_units > 0 {
                let mut anti_holder = read_all::<Holder>(&store)?
                    .into_iter()
                    .find(|h| h.external_reference() == Some("anti_dilution_pool"));
                if anti_holder.is_none() {
                    let generated = Holder::new(
                        HolderId::new(),
                        ContactId::new(),
                        None,
                        "Anti-Dilution Pool".to_owned(),
                        HolderType::Other,
                        Some("anti_dilution_pool".to_owned()),
                    );
                    modified_paths.push(
                        FileWrite::json(
                            format!("cap-table/holders/{}.json", generated.holder_id()),
                            &generated,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize anti holder: {e}")))?,
                    );
                    anti_holder = Some(generated);
                }
                if let Some(holder) = anti_holder {
                    let target = Position::new(
                        PositionId::new(),
                        round.issuer_legal_entity_id(),
                        holder.holder_id(),
                        target_instrument_id,
                        anti_dilution_adjustment_units,
                        0,
                        Some("anti_dilution_adjustment".to_owned()),
                        None,
                        Some(hash_json(&serde_json::json!({
                            "round_id": req.round_id,
                            "anti_dilution_adjustment_units": anti_dilution_adjustment_units,
                        }))),
                    )?;
                    touched_targets.insert(target.position_id());
                    modified_paths.push(
                        FileWrite::json(
                            format!("cap-table/positions/{}.json", target.position_id()),
                            &target,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize anti position: {e}")))?,
                    );
                }
            }

            round.close()?;
            intent.mark_executed()?;
            modified_paths.push(
                FileWrite::json(format!("cap-table/rounds/{}.json", req.round_id), &round)
                    .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
            );
            modified_paths.push(
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            );

            let total_new_units = lines
                .iter()
                .map(|l| l.new_units)
                .sum::<i64>()
                .saturating_add(anti_dilution_adjustment_units);

            let execution = ConversionExecution::new(
                ConversionExecutionId::new(),
                entity_id,
                req.round_id,
                serde_json::json!({
                    "line_count": lines.len(),
                    "anti_dilution_adjustment_units": anti_dilution_adjustment_units,
                    "total_new_units": total_new_units,
                    "rule_set_id": rule_set_id,
                    "target_instrument_id": target_instrument_id,
                }),
                req.source_reference,
            );

            modified_paths.push(
                FileWrite::json(
                    format!(
                        "cap-table/conversions/{}.json",
                        execution.conversion_execution_id()
                    ),
                    &execution,
                )
                .map_err(|e| AppError::Internal(format!("serialize conversion execution: {e}")))?,
            );

            store
                .commit(
                    "main",
                    &format!("Execute conversions for round {}", req.round_id),
                    modified_paths,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(ConversionExecuteResponse {
                conversion_execution_id: execution.conversion_execution_id(),
                round_id: req.round_id,
                converted_positions: lines.len(),
                target_positions_touched: touched_targets.len(),
                total_new_units,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(result))
}

async fn get_control_map(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Query(query): Query<ControlMapQuery>,
) -> Result<Json<ControlMapResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(query.entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, query.entity_id)?;
            let links = read_all::<ControlLink>(&store)?;
            let entities = read_all::<LegalEntity>(&store)?;
            let entity_ids: HashSet<LegalEntityId> =
                entities.iter().map(|e| e.legal_entity_id()).collect();
            if !entity_ids.contains(&query.root_entity_id) {
                return Err(AppError::NotFound(format!(
                    "root_entity_id {} not found",
                    query.root_entity_id
                )));
            }

            let mut visited: HashSet<LegalEntityId> = HashSet::new();
            let mut stack = vec![query.root_entity_id];
            let mut edges = Vec::new();

            while let Some(node) = stack.pop() {
                if !visited.insert(node) {
                    continue;
                }
                for link in links.iter().filter(|l| l.parent_legal_entity_id() == node) {
                    edges.push(ControlMapEdge {
                        parent_legal_entity_id: link.parent_legal_entity_id(),
                        child_legal_entity_id: link.child_legal_entity_id(),
                        control_type: link.control_type(),
                        voting_power_bps: link.voting_power_bps(),
                    });
                    stack.push(link.child_legal_entity_id());
                }
            }

            let mut traversed_entities: Vec<LegalEntityId> = visited.into_iter().collect();
            traversed_entities.sort_by_key(|id| id.to_string());

            Ok::<_, AppError>(ControlMapResponse {
                root_entity_id: query.root_entity_id,
                traversed_entities,
                edges,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

async fn get_dilution_preview(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Query(query): Query<DilutionPreviewQuery>,
) -> Result<Json<DilutionPreviewResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(query.entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, query.entity_id)?;
            let round = store
                .read::<EquityRound>("main", query.round_id)
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", query.round_id))
                })?;

            let instruments = read_all::<Instrument>(&store)?;
            let positions = read_all::<Position>(&store)?;

            let pre_round_outstanding_units = positions
                .iter()
                .filter(|p| p.issuer_legal_entity_id() == round.issuer_legal_entity_id())
                .filter_map(|p| {
                    instruments
                        .iter()
                        .find(|i| i.instrument_id() == p.instrument_id())
                        .map(|i| (i, p))
                })
                .map(|(i, p)| {
                    units_for_basis(
                        i.kind(),
                        p.quantity_units().max(0),
                        CapTableBasis::Outstanding,
                    )
                })
                .sum::<i64>();

            let projected_new_units = round
                .target_raise_cents()
                .and_then(|raise| round.round_price_cents().map(|price| (raise, price)))
                .and_then(
                    |(raise, price)| {
                        if price > 0 { Some(raise / price) } else { None }
                    },
                )
                .unwrap_or(0)
                .max(0);

            let projected_post_outstanding_units =
                pre_round_outstanding_units.saturating_add(projected_new_units);
            let projected_dilution_bps =
                checked_bps(projected_new_units, projected_post_outstanding_units);

            Ok::<_, AppError>(DilutionPreviewResponse {
                round_id: query.round_id,
                issuer_legal_entity_id: round.issuer_legal_entity_id(),
                pre_round_outstanding_units,
                projected_new_units,
                projected_post_outstanding_units,
                projected_dilution_bps,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn equity_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/equity/holders", post(create_holder))
        .route("/v1/equity/entities", post(create_legal_entity))
        .route("/v1/equity/control-links", post(create_control_link))
        .route("/v1/equity/instruments", post(create_instrument))
        .route("/v1/equity/positions/adjust", post(adjust_position))
        .route("/v1/equity/rounds", post(create_round))
        .route(
            "/v1/equity/rounds/{round_id}/apply-terms",
            post(apply_round_terms),
        )
        .route(
            "/v1/equity/rounds/{round_id}/board-approve",
            post(board_approve_round),
        )
        .route("/v1/equity/rounds/{round_id}/accept", post(accept_round))
        .route(
            "/v1/equity/transfer-workflows",
            post(create_transfer_workflow),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}",
            get(get_transfer_workflow),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/generate-docs",
            post(generate_transfer_workflow_docs),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/submit-review",
            post(submit_transfer_workflow_for_review),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/record-review",
            post(record_transfer_workflow_review),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/record-rofr",
            post(record_transfer_workflow_rofr),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/record-board-approval",
            post(record_transfer_workflow_board_approval),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/record-execution",
            post(record_transfer_workflow_execution),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/prepare-execution",
            post(prepare_transfer_workflow_execution),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/compile-packet",
            post(compile_transfer_workflow_packet),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/start-signatures",
            post(start_transfer_workflow_signatures),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/record-signature",
            post(record_transfer_workflow_signature),
        )
        .route(
            "/v1/equity/transfer-workflows/{workflow_id}/finalize",
            post(finalize_transfer_workflow),
        )
        .route(
            "/v1/equity/fundraising-workflows",
            post(create_fundraising_workflow),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}",
            get(get_fundraising_workflow),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/apply-terms",
            post(apply_fundraising_workflow_terms),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/generate-board-packet",
            post(generate_fundraising_board_packet),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/record-board-approval",
            post(record_fundraising_workflow_board_approval),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/record-investor-acceptance",
            post(record_fundraising_workflow_acceptance),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/generate-closing-packet",
            post(generate_fundraising_closing_packet),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/record-close",
            post(record_fundraising_workflow_close),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/prepare-execution",
            post(prepare_fundraising_workflow_execution),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/compile-packet",
            post(compile_fundraising_workflow_packet),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/start-signatures",
            post(start_fundraising_workflow_signatures),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/record-signature",
            post(record_fundraising_workflow_signature),
        )
        .route(
            "/v1/equity/fundraising-workflows/{workflow_id}/finalize",
            post(finalize_fundraising_workflow),
        )
        .route(
            "/v1/equity/workflows/{workflow_type}/{workflow_id}/status",
            get(get_workflow_status),
        )
        .route("/v1/equity/conversions/preview", post(preview_conversion))
        .route("/v1/equity/conversions/execute", post(execute_conversion))
        .route("/v1/entities/{entity_id}/cap-table", get(get_cap_table))
        .route("/v1/equity/control-map", get(get_control_map))
        .route("/v1/equity/dilution/preview", get(get_dilution_preview))
}
