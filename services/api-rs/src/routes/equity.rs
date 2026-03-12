//! Equity HTTP routes.
//!
//! Canonical cap-table operations for holders, legal entities, control links,
//! instruments, positions, rounds, and conversion previews/execution.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use super::AppState;
use super::validation::{
    reject_dangerous_param_keys, require_non_empty_trimmed_max, require_safe_single_line_max,
    validate_not_too_far_past,
};
use crate::auth::{RequireEquityRead, RequireEquityWrite};
use crate::domain::contacts::contact::Contact;
use crate::domain::contacts::types::{ContactCategory, ContactType};
use crate::domain::equity::{
    control_link::{ControlLink, ControlType},
    conversion_execution::ConversionExecution,
    fundraising_workflow::{
        FundraisingWorkflow, WorkflowExecutionStatus as FundraisingExecutionStatus,
    },
    grant::EquityGrant,
    holder::{Holder, HolderType},
    instrument::{Instrument, InstrumentKind, InstrumentStatus},
    legal_entity::{LegalEntity, LegalEntityRole},
    position::{Position, PositionStatus},
    round::{EquityRound, EquityRoundStatus},
    rule_set::{AntiDilutionMethod, EquityRuleSet},
    safe_note::SafeNote,
    share_class::ShareClass,
    transfer::ShareTransfer,
    transfer_workflow::{TransferWorkflow, WorkflowExecutionStatus as TransferExecutionStatus},
    types::{
        GoverningDocType, GrantType, SafeStatus, SafeType, ShareCount, TransferStatus,
        TransferType, TransfereeRights, ValuationMethodology, ValuationStatus, ValuationType,
    },
    valuation::Valuation,
};
use crate::domain::execution::{
    approval_artifact::ApprovalArtifact,
    document_request::DocumentRequest,
    intent::Intent,
    transaction_packet::{PacketItem, TransactionPacket, TransactionPacketStatus, WorkflowType},
    types::{AuthorityTier, IntentStatus},
};
use crate::domain::formation::{
    document::Document,
    entity::Entity,
    types::{DocumentType, EntityType, FormationStatus},
};
use crate::domain::governance::policy_engine::evaluate_intent as evaluate_governance_intent;
use crate::domain::governance::{
    agenda_item::AgendaItem,
    body::GovernanceBody,
    doc_ast, doc_generator,
    meeting::Meeting,
    profile::{GOVERNANCE_PROFILE_PATH, GovernanceProfile},
    resolution::Resolution,
    types::{AgendaItemType, BodyStatus, BodyType, MeetingStatus, MeetingType},
};
use crate::domain::ids::{
    AgendaItemId, ContactId, ControlLinkId, ConversionExecutionId, DocumentId, EntityId,
    EquityRoundId, EquityRuleSetId, FundraisingWorkflowId, HolderId, InstrumentId, IntentId,
    LegalEntityId, MeetingId, PacketId, PacketSignatureId, PositionId, ResolutionId, SafeNoteId,
    ShareClassId, TransferId, TransferWorkflowId, ValuationId, WorkspaceId,
};
use crate::domain::treasury::types::Cents;
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::store::entity_store::EntityStore;
use crate::store::stored_entity::StoredEntity;
use chrono::NaiveDate;

// ── Queries ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapTableBasis {
    #[default]
    Outstanding,
    AsConverted,
    FullyDiluted,
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct CapTableQuery {
    #[serde(default)]
    pub basis: CapTableBasis,
    #[serde(default)]
    pub issuer_legal_entity_id: Option<LegalEntityId>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ControlMapQuery {
    pub entity_id: EntityId,
    pub root_entity_id: LegalEntityId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct DilutionPreviewQuery {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
}

// ── Request types ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateLegalEntityRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub role: LegalEntityRole,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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
    #[schema(value_type = Object)]
    pub terms: serde_json::Value,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyRoundTermsRequest {
    pub entity_id: EntityId,
    pub anti_dilution_method: AntiDilutionMethod,
    #[serde(default)]
    pub conversion_precedence: Vec<InstrumentKind>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub protective_provisions: serde_json::Value,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BoardApproveRoundRequest {
    pub entity_id: EntityId,
    pub meeting_id: MeetingId,
    pub resolution_id: ResolutionId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptRoundRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
    #[serde(default)]
    pub accepted_by_contact_id: Option<ContactId>,
}

// ── Staged equity round types ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PendingSecurity {
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub quantity: i64,
    #[serde(default)]
    pub principal_cents: i64,
    pub recipient_name: String,
    #[serde(default)]
    pub grant_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PendingSecuritiesFile {
    pub round_id: EquityRoundId,
    pub securities: Vec<PendingSecurity>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartStagedRoundRequest {
    pub entity_id: EntityId,
    pub name: String,
    pub issuer_legal_entity_id: LegalEntityId,
    #[serde(default)]
    pub pre_money_cents: Option<i64>,
    #[serde(default)]
    pub round_price_cents: Option<i64>,
    #[serde(default)]
    pub target_raise_cents: Option<i64>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AddSecurityRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub holder_id: Option<HolderId>,
    #[serde(default)]
    pub email: Option<String>,
    pub instrument_id: InstrumentId,
    pub quantity: i64,
    #[serde(default)]
    pub principal_cents: i64,
    pub recipient_name: String,
    #[serde(default)]
    pub grant_type: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueStagedRoundRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub meeting_id: Option<MeetingId>,
    #[serde(default)]
    pub resolution_id: Option<ResolutionId>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IssueStagedRoundResponse {
    pub round: RoundResponse,
    pub positions: Vec<PositionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_id: Option<MeetingId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agenda_item_id: Option<AgendaItemId>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewConversionRequest {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    #[serde(default)]
    pub source_reference: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecuteConversionRequest {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    pub intent_id: IntentId,
    #[serde(default)]
    pub source_reference: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateWorkflowDocsRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub documents: Vec<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmitTransferReviewRequest {
    pub entity_id: EntityId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordTransferReviewRequest {
    pub entity_id: EntityId,
    pub approved: bool,
    pub notes: String,
    pub reviewer: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordTransferRofrRequest {
    pub entity_id: EntityId,
    pub offered: bool,
    pub waived: bool,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordTransferBoardApprovalRequest {
    pub entity_id: EntityId,
    pub meeting_id: MeetingId,
    pub resolution_id: ResolutionId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordTransferExecutionRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
    pub prepare_intent_id: IntentId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyFundraisingTermsRequest {
    pub entity_id: EntityId,
    pub anti_dilution_method: AntiDilutionMethod,
    #[serde(default)]
    pub conversion_precedence: Vec<InstrumentKind>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub protective_provisions: serde_json::Value,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordFundraisingBoardApprovalRequest {
    pub entity_id: EntityId,
    pub meeting_id: MeetingId,
    pub resolution_id: ResolutionId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordFundraisingAcceptanceRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
    #[serde(default)]
    pub accepted_by_contact_id: Option<ContactId>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordFundraisingCloseRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CompileWorkflowPacketRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub required_signers: Vec<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StartWorkflowSignaturesRequest {
    pub entity_id: EntityId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordWorkflowSignatureRequest {
    pub entity_id: EntityId,
    pub signer_identity: String,
    #[serde(default)]
    pub channel: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FinalizeWorkflowRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub phase: Option<String>,
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HolderResponse {
    pub holder_id: HolderId,
    pub contact_id: ContactId,
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub holder_type: HolderType,
    pub created_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LegalEntityResponse {
    pub legal_entity_id: LegalEntityId,
    pub workspace_id: WorkspaceId,
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub role: LegalEntityRole,
    pub created_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ControlLinkResponse {
    pub control_link_id: ControlLinkId,
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    pub voting_power_bps: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RuleSetResponse {
    pub rule_set_id: EquityRuleSetId,
    pub anti_dilution_method: AntiDilutionMethod,
    pub conversion_precedence: Vec<InstrumentKind>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CapTableInstrumentSummary {
    pub instrument_id: InstrumentId,
    pub symbol: String,
    pub kind: InstrumentKind,
    pub authorized_units: Option<i64>,
    pub issued_units: i64,
    pub diluted_units: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CapTableResponse {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub basis: CapTableBasis,
    pub total_units: i64,
    pub instruments: Vec<CapTableInstrumentSummary>,
    pub holders: Vec<CapTableHolderSummary>,
    pub generated_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConversionPreviewLine {
    pub source_position_id: PositionId,
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub principal_cents: i64,
    pub conversion_price_cents: i64,
    pub new_units: i64,
    pub basis: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConversionPreviewResponse {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    pub target_instrument_id: InstrumentId,
    pub lines: Vec<ConversionPreviewLine>,
    pub anti_dilution_adjustment_units: i64,
    pub total_new_units: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConversionExecuteResponse {
    pub conversion_execution_id: ConversionExecutionId,
    pub round_id: EquityRoundId,
    pub converted_positions: usize,
    pub target_positions_touched: usize,
    pub total_new_units: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ControlMapEdge {
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    pub voting_power_bps: Option<u32>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ControlMapResponse {
    pub root_entity_id: LegalEntityId,
    pub traversed_entities: Vec<LegalEntityId>,
    pub edges: Vec<ControlMapEdge>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DilutionPreviewResponse {
    pub round_id: EquityRoundId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub pre_round_outstanding_units: i64,
    pub projected_new_units: i64,
    pub projected_post_outstanding_units: i64,
    pub projected_dilution_bps: u32,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TransferWorkflowResponse {
    pub transfer_workflow_id: TransferWorkflowId,
    pub transfer_id: TransferId,
    pub prepare_intent_id: IntentId,
    pub execute_intent_id: Option<IntentId>,
    pub transfer_status: TransferStatus,
    #[schema(value_type = String)]
    pub execution_status: TransferExecutionStatus,
    pub active_packet_id: Option<PacketId>,
    pub last_packet_hash: Option<String>,
    pub board_approval_meeting_id: Option<MeetingId>,
    pub board_approval_resolution_id: Option<ResolutionId>,
    pub generated_documents: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FundraisingWorkflowResponse {
    pub fundraising_workflow_id: FundraisingWorkflowId,
    pub round_id: EquityRoundId,
    pub prepare_intent_id: IntentId,
    pub accept_intent_id: Option<IntentId>,
    pub close_intent_id: Option<IntentId>,
    #[schema(value_type = String)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PacketSignatureResponse {
    pub signature_id: PacketSignatureId,
    pub signer_identity: String,
    pub channel: String,
    pub signed_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WorkflowStatusResponse {
    pub workflow_type: WorkflowType,
    pub workflow_id: String,
    pub execution_status: String,
    pub active_packet_id: Option<PacketId>,
    pub transfer_workflow: Option<TransferWorkflowResponse>,
    pub fundraising_workflow: Option<FundraisingWorkflowResponse>,
    pub packet: Option<TransactionPacketResponse>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateSafeNoteRequest {
    pub entity_id: EntityId,
    pub investor_name: String,
    #[serde(default)]
    pub investor_contact_id: Option<ContactId>,
    #[serde(default)]
    pub email: Option<String>,
    pub principal_amount_cents: i64,
    #[serde(default)]
    pub valuation_cap_cents: Option<i64>,
    #[serde(default)]
    pub discount_rate: Option<f64>,
    #[serde(default)]
    pub safe_type: Option<SafeType>,
    #[serde(default)]
    pub pro_rata_rights: bool,
    #[serde(default)]
    pub document_id: Option<DocumentId>,
    #[serde(default)]
    pub conversion_unit_type: Option<String>,
    #[serde(default)]
    pub meeting_id: Option<MeetingId>,
    #[serde(default)]
    pub resolution_id: Option<ResolutionId>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SafeNoteResponse {
    pub safe_note_id: SafeNoteId,
    pub entity_id: EntityId,
    pub investor_name: String,
    pub investor_contact_id: Option<ContactId>,
    pub principal_amount_cents: i64,
    pub valuation_cap_cents: Option<i64>,
    pub discount_rate: Option<f64>,
    pub safe_type: SafeType,
    pub pro_rata_rights: bool,
    pub status: SafeStatus,
    pub document_id: Option<DocumentId>,
    pub conversion_unit_type: String,
    pub issued_at: String,
    pub created_at: String,
    pub converted_at: Option<String>,
    pub conversion_shares: Option<i64>,
    pub conversion_price_cents: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_id: Option<MeetingId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_id: Option<ResolutionId>,
}

const MAX_FMV_PER_SHARE_CENTS: i64 = 10_000_000;
const MAX_HOLDER_NAME_LEN: usize = 256;
const MAX_ROUND_NAME_LEN: usize = 200;
const MAX_INSTRUMENT_SYMBOL_LEN: usize = 32;

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

fn ensure_entity_is_active_for_governance(
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

fn normalized_grant_type(grant_type: Option<&str>) -> Option<String> {
    grant_type.map(|value| value.trim().to_ascii_lowercase().replace('-', "_"))
}

fn expected_instrument_kinds(grant_type: Option<&str>) -> Option<&'static [InstrumentKind]> {
    match normalized_grant_type(grant_type).as_deref() {
        Some("common") | Some("common_stock") => Some(&[InstrumentKind::CommonEquity]),
        Some("preferred") | Some("preferred_stock") => Some(&[InstrumentKind::PreferredEquity]),
        Some("membership_unit") | Some("unit") => Some(&[InstrumentKind::MembershipUnit]),
        Some("option") | Some("options") | Some("stock_option") | Some("iso") | Some("nso") => {
            Some(&[InstrumentKind::OptionGrant])
        }
        Some("rsa") => Some(&[
            InstrumentKind::CommonEquity,
            InstrumentKind::PreferredEquity,
        ]),
        Some("safe") | Some("post_money") | Some("pre_money") | Some("mfn") => {
            Some(&[InstrumentKind::Safe])
        }
        _ => None,
    }
}

fn validate_grant_type_for_instrument(
    grant_type: Option<&str>,
    instrument: &Instrument,
) -> Result<(), AppError> {
    let Some(expected_kinds) = expected_instrument_kinds(grant_type) else {
        return Ok(());
    };
    if expected_kinds.contains(&instrument.kind()) {
        return Ok(());
    }
    Err(AppError::BadRequest(format!(
        "grant_type {} is not valid for instrument {} ({:?})",
        grant_type.unwrap_or_default(),
        instrument.symbol(),
        instrument.kind()
    )))
}

fn grant_requires_current_409a(grant_type: Option<&str>, instrument: &Instrument) -> bool {
    instrument.kind() == InstrumentKind::OptionGrant
        || matches!(
            normalized_grant_type(grant_type).as_deref(),
            Some("option") | Some("options") | Some("stock_option") | Some("iso") | Some("nso")
        )
}

fn ensure_current_409a_exists(store: &EntityStore<'_>) -> Result<(), AppError> {
    let has_current = read_all::<Valuation>(store)?
        .into_iter()
        .any(|valuation| valuation.is_current_409a());
    if has_current {
        return Ok(());
    }
    Err(AppError::BadRequest(
        "stock option issuances require a current approved 409A valuation".to_owned(),
    ))
}

fn ensure_instrument_has_capacity(
    positions: &[Position],
    pending: &[PendingSecurity],
    instrument: &Instrument,
    extra_quantity: i64,
) -> Result<(), AppError> {
    let Some(authorized_units) = instrument.authorized_units() else {
        return Ok(());
    };
    let issued_units = positions
        .iter()
        .filter(|position| position.instrument_id() == instrument.instrument_id())
        .map(|position| position.quantity_units().max(0))
        .sum::<i64>();
    let staged_units = pending
        .iter()
        .filter(|security| security.instrument_id == instrument.instrument_id())
        .map(|security| security.quantity.max(0))
        .sum::<i64>();
    let total_units = issued_units
        .checked_add(staged_units)
        .and_then(|sum| sum.checked_add(extra_quantity))
        .ok_or_else(|| AppError::BadRequest("issued quantity would overflow".to_owned()))?;
    if total_units > authorized_units {
        return Err(AppError::BadRequest(format!(
            "issuing {} additional units would exceed authorized units for instrument {} (issued={}, staged={}, authorized={})",
            extra_quantity.max(0),
            instrument.symbol(),
            issued_units,
            staged_units,
            authorized_units
        )));
    }
    Ok(())
}

fn resolve_contact_reference(
    store: &EntityStore<'_>,
    reference: &str,
) -> Result<Contact, AppError> {
    let ids = store
        .list_ids::<Contact>("main")
        .map_err(|e| AppError::Internal(format!("list contacts: {e}")))?;
    let trimmed = reference.trim();
    let normalized = trimmed.to_ascii_lowercase();
    let mut name_matches = Vec::new();
    for id in ids {
        let contact = store
            .read::<Contact>("main", id)
            .map_err(|e| AppError::Internal(format!("read contact {id}: {e}")))?;
        if contact.contact_id().to_string() == trimmed {
            return Ok(contact);
        }
        if contact.name().trim().to_ascii_lowercase() == normalized {
            name_matches.push(contact);
        }
    }
    match name_matches.len() {
        0 => Err(AppError::NotFound(format!(
            "contact not found: {reference}"
        ))),
        1 => Ok(name_matches.remove(0)),
        _ => Err(AppError::BadRequest(format!(
            "contact reference is ambiguous: {reference}; use a contact_id"
        ))),
    }
}

fn resolve_or_prepare_investor_contact(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    investor_name: &str,
    investor_contact_id: Option<ContactId>,
    email: Option<&str>,
) -> Result<(Option<ContactId>, Vec<FileWrite>), AppError> {
    if let Some(contact_id) = investor_contact_id {
        let contact = store
            .read::<Contact>("main", contact_id)
            .map_err(|_| AppError::NotFound(format!("contact {contact_id} not found")))?;
        if contact.entity_id() != entity_id {
            return Err(AppError::BadRequest(format!(
                "contact {contact_id} does not belong to entity {entity_id}"
            )));
        }
        return Ok((Some(contact_id), Vec::new()));
    }

    let trimmed_email = email.map(str::trim).filter(|value| !value.is_empty());
    if let Some(email) = trimmed_email {
        let contacts = read_all::<Contact>(store)?;
        if let Some(contact) = contacts.iter().find(|contact| {
            contact
                .email()
                .map(|value| value.eq_ignore_ascii_case(email))
                .unwrap_or(false)
        }) {
            return Ok((Some(contact.contact_id()), Vec::new()));
        }
    }

    let contact_id = ContactId::new();
    let contact = Contact::new(
        contact_id,
        entity_id,
        workspace_id,
        ContactType::Individual,
        investor_name.trim().to_owned(),
        trimmed_email.map(ToOwned::to_owned),
        ContactCategory::Investor,
    )
    .map_err(AppError::BadRequest)?;
    let file = FileWrite::json(format!("contacts/{}.json", contact_id), &contact)
        .map_err(|e| AppError::Internal(format!("serialize contact {contact_id}: {e}")))?;
    Ok((Some(contact_id), vec![file]))
}

fn resolve_transfer_sender_contact(
    store: &EntityStore<'_>,
    reference: &str,
) -> Result<Contact, AppError> {
    if let Ok(grant_id) = reference.parse::<crate::domain::ids::EquityGrantId>() {
        let grant = store
            .read::<EquityGrant>("main", grant_id)
            .map_err(|_| AppError::NotFound(format!("grant not found: {reference}")))?;
        if let Some(contact_id) = grant.contact_id() {
            return store.read::<Contact>("main", contact_id).map_err(|_| {
                AppError::NotFound(format!("contact {} not found for grant", contact_id))
            });
        }
        return resolve_contact_reference(store, grant.recipient_name());
    }
    resolve_contact_reference(store, reference)
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

fn validate_holder_name(name: &str) -> Result<String, AppError> {
    require_safe_single_line_max(name, "name", MAX_HOLDER_NAME_LEN)
}

fn validate_round_name(name: &str) -> Result<String, AppError> {
    require_safe_single_line_max(name, "name", MAX_ROUND_NAME_LEN)
}

fn validate_instrument_symbol(symbol: &str) -> Result<String, AppError> {
    let trimmed = require_non_empty_trimmed_max(symbol, "symbol", MAX_INSTRUMENT_SYMBOL_LEN)?;
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err(AppError::BadRequest(
            "symbol must use only letters, numbers, '_' or '-'".to_owned(),
        ));
    }
    Ok(trimmed)
}

fn ensure_equity_resolution_unused(
    store: &EntityStore<'_>,
    resolution_id: ResolutionId,
) -> Result<(), AppError> {
    for round in read_all::<EquityRound>(store)? {
        if round.board_approval_resolution_id() == Some(resolution_id) {
            return Err(AppError::Conflict(format!(
                "resolution {} is already bound to equity round {}",
                resolution_id,
                round.equity_round_id()
            )));
        }
    }
    for workflow in read_all::<FundraisingWorkflow>(store)? {
        if workflow.board_approval_resolution_id() == Some(resolution_id) {
            return Err(AppError::Conflict(format!(
                "resolution {} is already bound to fundraising workflow {}",
                resolution_id,
                workflow.fundraising_workflow_id()
            )));
        }
    }
    for workflow in read_all::<TransferWorkflow>(store)? {
        if workflow.board_approval_resolution_id() == Some(resolution_id) {
            return Err(AppError::Conflict(format!(
                "resolution {} is already bound to transfer workflow {}",
                resolution_id,
                workflow.transfer_workflow_id()
            )));
        }
    }
    for transfer in read_all::<ShareTransfer>(store)? {
        if transfer.board_approval_resolution_id() == Some(resolution_id) {
            return Err(AppError::Conflict(format!(
                "resolution {} is already bound to share transfer {}",
                resolution_id,
                transfer.transfer_id()
            )));
        }
    }
    for note in read_all::<SafeNote>(store)? {
        if note.board_approval_resolution_id() == Some(resolution_id) {
            return Err(AppError::Conflict(format!(
                "resolution {} is already bound to SAFE note {}",
                resolution_id,
                note.safe_note_id()
            )));
        }
    }
    for valuation in read_all::<Valuation>(store)? {
        if valuation.board_approval_resolution_id() == Some(resolution_id) {
            return Err(AppError::Conflict(format!(
                "resolution {} is already bound to valuation {}",
                resolution_id,
                valuation.valuation_id()
            )));
        }
    }
    Ok(())
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
            AppError::NotFound("no linked legal entity exists for this entity".to_owned())
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
    ensure_equity_resolution_unused(store, resolution_id)?;

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
    ensure_equity_resolution_unused(store, resolution_id)?;

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

fn entity_profile_for_docs(
    store: &EntityStore<'_>,
) -> Result<(Entity, GovernanceProfile), AppError> {
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    let profile = store
        .read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH)
        .unwrap_or_else(|_| GovernanceProfile::default_for_entity(&entity));
    Ok((entity, profile))
}

fn contact_name_email(store: &EntityStore<'_>, contact_id: ContactId) -> (String, Option<String>) {
    let (name, email, _) = contact_details(store, contact_id);
    (name, email)
}

fn contact_details(
    store: &EntityStore<'_>,
    contact_id: ContactId,
) -> (String, Option<String>, Option<String>) {
    store
        .read::<Contact>("main", contact_id)
        .map(|c| {
            (
                c.name().to_owned(),
                c.email().map(ToOwned::to_owned),
                c.mailing_address().map(ToOwned::to_owned),
            )
        })
        .unwrap_or_else(|_| (contact_id.to_string(), None, None))
}

fn signature_req(
    role: &str,
    signer_name: String,
    signer_email: Option<String>,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "role": role,
        "signer_name": signer_name,
        "required": true
    });
    if let Some(email) = signer_email {
        value["signer_email"] = serde_json::json!(email);
    }
    value
}

fn format_units(value: i64) -> String {
    let digits = value.abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let rendered: String = out.chars().rev().collect();
    if value < 0 {
        format!("-{rendered}")
    } else {
        rendered
    }
}

fn metadata_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|entry| match entry {
            serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
            serde_json::Value::Number(number) => Some(number.to_string()),
            _ => None,
        })
    })
}

fn round_instrument_label(round: &EquityRound) -> String {
    metadata_string(round.metadata(), &["instrument"])
        .unwrap_or_else(|| "Equity Securities".to_owned())
}

fn uses_principal_amount(instrument: &str) -> bool {
    let normalized = instrument.to_ascii_lowercase();
    normalized.contains("safe")
        || normalized.contains("note")
        || normalized.contains("debt")
        || normalized.contains("convertible")
        || normalized.contains("loan")
}

fn round_purchase_amount(round: &EquityRound) -> Result<String, AppError> {
    metadata_string(round.metadata(), &["purchase_amount", "amount_authorized"])
        .or_else(|| round.target_raise_cents().map(doc_generator::format_usd))
        .ok_or_else(|| {
            AppError::UnprocessableEntity(
                "round is missing purchase amount or target_raise_cents for production documents"
                    .to_owned(),
            )
        })
}

fn round_quantity_or_principal_amount(round: &EquityRound) -> Result<String, AppError> {
    if let Some(raw) = round
        .metadata()
        .get("quantity_or_principal_amount")
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
    {
        return Ok(raw);
    }

    if let Some(quantity_units) = round
        .metadata()
        .get("quantity_units")
        .and_then(serde_json::Value::as_i64)
        .filter(|q| *q > 0)
    {
        return Ok(format!("{} shares", format_units(quantity_units)));
    }

    if let Some((raise, price)) = round
        .target_raise_cents()
        .and_then(|raise| round.round_price_cents().map(|price| (raise, price)))
        .filter(|(_, price)| *price > 0)
    {
        if raise % price == 0 {
            return Ok(format!("{} shares", format_units(raise / price)));
        }

        let shares = (raise as f64) / (price as f64);
        let mut rendered = format!("{shares:.4}");
        while rendered.contains('.') && rendered.ends_with('0') {
            rendered.pop();
        }
        if rendered.ends_with('.') {
            rendered.pop();
        }
        return Ok(format!("{rendered} shares"));
    }

    let instrument = round_instrument_label(round);
    if uses_principal_amount(&instrument) {
        return round_purchase_amount(round);
    }

    Err(AppError::UnprocessableEntity(format!(
        "round is missing quantity_units or round_price_cents for equity instrument '{}'",
        instrument
    )))
}

fn validate_governance_document_content(
    store: &EntityStore<'_>,
    governance_tag: &str,
    content: &serde_json::Value,
) -> Result<(), AppError> {
    let ast = doc_ast::default_doc_ast();
    let doc_def = ast
        .documents
        .iter()
        .find(|doc| doc.id == governance_tag || doc.path.contains(governance_tag))
        .ok_or_else(|| {
            AppError::Internal(format!(
                "no AST document definition matches governance_tag '{}'",
                governance_tag
            ))
        })?;
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    let profile = store
        .read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH)
        .unwrap_or_else(|_| GovernanceProfile::default_for_entity(&entity));
    let entity_type = match entity.entity_type() {
        EntityType::CCorp => doc_ast::EntityTypeKey::Corporation,
        EntityType::Llc => doc_ast::EntityTypeKey::Llc,
    };
    let rendered = doc_generator::render_document_from_ast_with_context(
        doc_def,
        ast,
        entity_type,
        &profile,
        content,
    );
    let warnings = doc_generator::detect_placeholder_warnings_for_text(governance_tag, &rendered);
    if warnings.is_empty() {
        Ok(())
    } else {
        Err(AppError::UnprocessableEntity(format!(
            "document '{}' is incomplete for production use: {}",
            governance_tag,
            warnings.join("; ")
        )))
    }
}

fn create_governance_document(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    document_type: DocumentType,
    title: String,
    governance_tag: &str,
    fields: serde_json::Value,
    signature_requirements: Vec<serde_json::Value>,
) -> Result<DocumentId, AppError> {
    let document_id = DocumentId::new();
    let mut content = serde_json::json!({ "fields": fields });
    if !signature_requirements.is_empty() {
        content["signature_requirements"] = serde_json::Value::Array(signature_requirements);
    }
    validate_governance_document_content(store, governance_tag, &content)?;
    let doc = Document::new(
        document_id,
        entity_id,
        workspace_id,
        document_type,
        title,
        content,
        Some(governance_tag.to_owned()),
        None,
    );
    store
        .write_document(
            "main",
            &doc,
            &format!("Generate document {document_type:?} {document_id}"),
        )
        .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
    Ok(document_id)
}

fn to_packet_items_with_store(store: &EntityStore<'_>, documents: &[String]) -> Vec<PacketItem> {
    documents
        .iter()
        .enumerate()
        .map(|(idx, doc)| {
            let title = doc
                .parse::<DocumentId>()
                .ok()
                .and_then(|document_id| store.read_document("main", document_id).ok())
                .map(|d| d.title().to_owned())
                .unwrap_or_else(|| doc.rsplit('/').next().unwrap_or(doc).to_owned());
            PacketItem {
                item_id: format!("item-{}", idx + 1),
                title,
                document_path: doc.clone(),
                required: true,
            }
        })
        .collect()
}

fn default_required_signers() -> Vec<String> {
    vec!["officer".to_owned(), "board".to_owned()]
}

fn generate_transfer_documents(
    store: &EntityStore<'_>,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    transfer: &ShareTransfer,
) -> Result<Vec<String>, AppError> {
    let (entity, _profile) = entity_profile_for_docs(store)?;
    let (transferor_name, transferor_email) =
        contact_name_email(store, transfer.sender_contact_id());
    let (transferee_name, transferee_email) = contact_name_email(store, transfer.to_contact_id());
    let share_class = store
        .read::<ShareClass>("main", transfer.share_class_id())
        .ok();
    let class_label = share_class
        .as_ref()
        .map(|c| c.class_code().to_owned())
        .unwrap_or_else(|| transfer.share_class_id().to_string());
    let (consideration, per_share) = match transfer.price_per_share_cents() {
        Some(price) => (
            doc_generator::format_usd(price.raw() * transfer.share_count().raw()),
            doc_generator::format_usd(price.raw()),
        ),
        None => match transfer.transfer_type() {
            TransferType::Gift => (
                "No cash consideration; transfer designated as a gift".to_owned(),
                "No cash price".to_owned(),
            ),
            TransferType::TrustTransfer => (
                "No cash consideration; transfer into trust".to_owned(),
                "No cash price".to_owned(),
            ),
            TransferType::Estate => (
                "No cash consideration; transfer pursuant to estate administration".to_owned(),
                "No cash price".to_owned(),
            ),
            TransferType::SecondarySale | TransferType::Other => {
                return Err(AppError::UnprocessableEntity(
                    "price_per_share_cents is required for sale transfers and other priced transfers"
                        .to_owned(),
                ));
            }
        },
    };
    let transfer_doc = create_governance_document(
        store,
        entity_id,
        workspace_id,
        DocumentType::StockTransferAgreement,
        format!(
            "Stock Transfer Agreement — {} to {}",
            transferor_name, transferee_name
        ),
        "stock_transfer_agreement",
        serde_json::json!({
            "effective_date": chrono::Utc::now().date_naive().to_string(),
            "entity_legal_name": entity.legal_name(),
            "transferor_name": transferor_name,
            "transferee_name": transferee_name,
            "security_class": class_label,
            "units_transferred": transfer.share_count().raw().to_string(),
            "consideration": consideration,
            "price_per_share": per_share,
            "closing_date": chrono::Utc::now().date_naive().to_string(),
            "ledger_reference": transfer.transfer_id().to_string(),
            "transfer_type": format!("{:?}", transfer.transfer_type()),
        }),
        vec![
            signature_req("Transferor", transferor_name, transferor_email),
            signature_req("Transferee", transferee_name, transferee_email),
            signature_req("Company", entity.legal_name().to_owned(), None),
        ],
    )?;
    let consent_doc = create_governance_document(
        store,
        entity_id,
        workspace_id,
        DocumentType::TransferBoardConsent,
        format!("Transfer Approval Consent — {}", entity.legal_name()),
        "transfer_board_consent",
        serde_json::json!({
            "effective_date": chrono::Utc::now().date_naive().to_string(),
            "entity_legal_name": entity.legal_name(),
            "transferor_name": contact_name_email(store, transfer.sender_contact_id()).0,
            "transferee_name": contact_name_email(store, transfer.to_contact_id()).0,
            "units_transferred": transfer.share_count().raw().to_string(),
            "security_class": share_class.map(|c| c.class_code().to_owned()).unwrap_or_else(|| transfer.share_class_id().to_string()),
            "consideration": consideration.clone(),
            "closing_target_date": chrono::Utc::now().date_naive().to_string(),
        }),
        vec![signature_req(
            "Approving Signatory",
            entity.legal_name().to_owned(),
            None,
        )],
    )?;
    Ok(vec![transfer_doc.to_string(), consent_doc.to_string()])
}

fn generate_fundraising_packet_documents(
    store: &EntityStore<'_>,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    round: &EquityRound,
    closing: bool,
) -> Result<Vec<String>, AppError> {
    let (entity, _profile) = entity_profile_for_docs(store)?;
    let (accepted_investor_name, accepted_investor_email, accepted_investor_address) = round
        .accepted_by_contact_id()
        .map(|contact_id| contact_details(store, contact_id))
        .unwrap_or_else(|| (String::new(), None, None));
    let investor = (!accepted_investor_name.is_empty())
        .then_some(accepted_investor_name.clone())
        .or_else(|| metadata_string(round.metadata(), &["investor_name", "subscriber_name"]))
        .unwrap_or_else(|| "Investor".to_owned());
    let investor_email = accepted_investor_email.or_else(|| {
        metadata_string(
            round.metadata(),
            &["investor_email", "subscriber_email", "email"],
        )
    });
    let investor_address = accepted_investor_address.or_else(|| {
        metadata_string(
            round.metadata(),
            &["investor_address", "subscriber_address", "mailing_address"],
        )
    });
    let amount = round_purchase_amount(round)?;
    let quantity_or_principal_amount = round_quantity_or_principal_amount(round)?;
    let instrument = round_instrument_label(round);
    if closing {
        let subscription_doc = create_governance_document(
            store,
            entity_id,
            workspace_id,
            DocumentType::SubscriptionAgreement,
            format!("Subscription Agreement — {}", investor),
            "subscription_agreement",
            serde_json::json!({
                "effective_date": chrono::Utc::now().date_naive().to_string(),
                "entity_legal_name": entity.legal_name(),
                "subscriber_name": investor,
                "subscriber_address": investor_address.clone(),
                "subscriber_email": investor_email.clone(),
                "security_subscribed_for": instrument,
                "quantity_or_principal_amount": quantity_or_principal_amount,
                "purchase_amount": amount,
                "closing_date": chrono::Utc::now().date_naive().to_string(),
            }),
            vec![
                signature_req("Subscriber", investor.clone(), None),
                signature_req("Company", entity.legal_name().to_owned(), None),
            ],
        )?;
        let rights_doc = create_governance_document(
            store,
            entity_id,
            workspace_id,
            DocumentType::InvestorRightsAgreement,
            format!("Investor Rights Agreement — {}", investor),
            "investor_rights_agreement",
            serde_json::json!({
                "effective_date": chrono::Utc::now().date_naive().to_string(),
                "entity_legal_name": entity.legal_name(),
                "investor_name": investor,
                "investor_address": investor_address,
            }),
            vec![
                signature_req("Investor", investor, investor_email),
                signature_req("Company", entity.legal_name().to_owned(), None),
            ],
        )?;
        Ok(vec![subscription_doc.to_string(), rights_doc.to_string()])
    } else {
        let board_doc = create_governance_document(
            store,
            entity_id,
            workspace_id,
            DocumentType::FinancingBoardConsent,
            format!("Board Consent for Financing — {}", round.name()),
            "financing_board_consent",
            serde_json::json!({
                "effective_date": chrono::Utc::now().date_naive().to_string(),
                "entity_legal_name": entity.legal_name(),
                "transaction_type": round.name(),
                "counterparty_name": investor,
                "amount_authorized": amount,
                "instrument_name": instrument,
                "closing_window_start": chrono::Utc::now().date_naive().to_string(),
                "closing_window_end": chrono::Utc::now().date_naive().to_string(),
            }),
            vec![signature_req(
                "Approving Signatory",
                entity.legal_name().to_owned(),
                None,
            )],
        )?;
        let issuance_doc = create_governance_document(
            store,
            entity_id,
            workspace_id,
            DocumentType::EquityIssuanceApproval,
            format!("Equity Issuance Approval — {}", round.name()),
            "equity_issuance_approval",
            serde_json::json!({
                "effective_date": chrono::Utc::now().date_naive().to_string(),
                "entity_legal_name": entity.legal_name(),
                "recipient_names": investor,
                "security_type_and_class": instrument,
                "quantity_or_principal_amount": quantity_or_principal_amount,
                "purchase_price_or_consideration": amount,
                "vesting_or_milestone_conditions": "None",
                "securities_exemption_basis": "Private placement exemption",
            }),
            vec![signature_req(
                "Approving Signatory",
                entity.legal_name().to_owned(),
                None,
            )],
        )?;
        Ok(vec![board_doc.to_string(), issuance_doc.to_string()])
    }
}

// ── Handlers ─────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/equity/holders",
    tag = "equity",
    request_body = CreateHolderRequest,
    responses(
        (status = 200, description = "Holder created", body = HolderResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_holder(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateHolderRequest>,
) -> Result<Json<HolderResponse>, AppError> {
    let holder_name = validate_holder_name(&req.name)?;

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    state.enforce_creation_rate_limit("equity.holder.create", workspace_id, 120, 60)?;

    let holder = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let holder_name = holder_name.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let holder = Holder::new(
                HolderId::new(),
                req.contact_id,
                req.linked_entity_id,
                holder_name,
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

#[utoipa::path(
    post,
    path = "/v1/equity/entities",
    tag = "equity",
    request_body = CreateLegalEntityRequest,
    responses(
        (status = 200, description = "Legal entity created", body = LegalEntityResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
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
    state.enforce_creation_rate_limit("equity.legal_entity.create", workspace_id, 30, 60)?;

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

#[utoipa::path(
    post,
    path = "/v1/equity/control-links",
    tag = "equity",
    request_body = CreateControlLinkRequest,
    responses(
        (status = 200, description = "Control link created", body = ControlLinkResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/instruments",
    tag = "equity",
    request_body = CreateInstrumentRequest,
    responses(
        (status = 200, description = "Instrument created", body = InstrumentResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_instrument(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateInstrumentRequest>,
) -> Result<Json<InstrumentResponse>, AppError> {
    let symbol = validate_instrument_symbol(&req.symbol)?;
    if req.authorized_units.is_some_and(|v| v <= 0) {
        return Err(AppError::BadRequest(
            "authorized_units must be positive when provided".to_owned(),
        ));
    }
    if req.issue_price_cents.is_some_and(|v| v < 0) {
        return Err(AppError::BadRequest(
            "issue_price_cents cannot be negative".to_owned(),
        ));
    }
    reject_dangerous_param_keys(&req.terms)?;

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    state.enforce_creation_rate_limit("equity.instrument.create", workspace_id, 120, 60)?;

    let instrument = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let symbol = symbol.clone();
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
                symbol,
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

#[utoipa::path(
    post,
    path = "/v1/equity/positions/adjust",
    tag = "equity",
    request_body = AdjustPositionRequest,
    responses(
        (status = 200, description = "Position adjusted", body = PositionResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/rounds",
    tag = "equity",
    request_body = CreateRoundRequest,
    responses(
        (status = 200, description = "Equity round created", body = RoundResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateRoundRequest>,
) -> Result<Json<RoundResponse>, AppError> {
    let round_name = validate_round_name(&req.name)?;
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    state.enforce_creation_rate_limit("equity.round.create", workspace_id, 120, 60)?;

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let round_name = round_name.clone();
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
                round_name,
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

#[utoipa::path(
    post,
    path = "/v1/equity/rounds/{round_id}/apply-terms",
    tag = "equity",
    params(
        ("round_id" = EquityRoundId, Path, description = "Equity round ID"),
    ),
    request_body = ApplyRoundTermsRequest,
    responses(
        (status = 200, description = "Round terms applied", body = RuleSetResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Round not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/rounds/{round_id}/board-approve",
    tag = "equity",
    params(
        ("round_id" = EquityRoundId, Path, description = "Equity round ID"),
    ),
    request_body = BoardApproveRoundRequest,
    responses(
        (status = 200, description = "Round board-approved", body = RoundResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Round not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/rounds/{round_id}/accept",
    tag = "equity",
    params(
        ("round_id" = EquityRoundId, Path, description = "Equity round ID"),
    ),
    request_body = AcceptRoundRequest,
    responses(
        (status = 200, description = "Round accepted", body = RoundResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Round not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows",
    tag = "equity",
    request_body = CreateTransferWorkflowRequest,
    responses(
        (status = 200, description = "Transfer workflow created", body = TransferWorkflowResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
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
    ShareCount::new(req.share_count)
        .require_positive()
        .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    if req.from_contact_id == req.to_contact_id {
        return Err(AppError::BadRequest(
            "from_contact_id and to_contact_id must be different".to_owned(),
        ));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    state.enforce_creation_rate_limit("equity.transfer_workflow.create", workspace_id, 120, 60)?;

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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/generate-docs",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = GenerateWorkflowDocsRequest,
    responses(
        (status = 200, description = "Transfer workflow documents generated", body = TransferWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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
            let transfer = store
                .read::<ShareTransfer>("main", workflow.transfer_id())
                .map_err(|_| {
                    AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                })?;

            let docs = if req.documents.is_empty() {
                generate_transfer_documents(&store, workspace_id, entity_id, &transfer)?
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/submit-review",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = SubmitTransferReviewRequest,
    responses(
        (status = 200, description = "Transfer workflow submitted for review", body = TransferWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/record-review",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = RecordTransferReviewRequest,
    responses(
        (status = 200, description = "Transfer workflow review recorded", body = TransferWorkflowResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/record-rofr",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = RecordTransferRofrRequest,
    responses(
        (status = 200, description = "Transfer workflow ROFR recorded", body = TransferWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/record-board-approval",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = RecordTransferBoardApprovalRequest,
    responses(
        (status = 200, description = "Transfer workflow board approval recorded", body = TransferWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/record-execution",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = RecordTransferExecutionRequest,
    responses(
        (status = 200, description = "Transfer workflow execution recorded", body = TransferWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    get,
    path = "/v1/equity/transfer-workflows/{workflow_id}",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Transfer workflow details", body = TransferWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows",
    tag = "equity",
    request_body = CreateFundraisingWorkflowRequest,
    responses(
        (status = 200, description = "Fundraising workflow created", body = FundraisingWorkflowResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_fundraising_workflow(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateFundraisingWorkflowRequest>,
) -> Result<Json<FundraisingWorkflowResponse>, AppError> {
    let round_name = validate_round_name(&req.name)?;
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    state.enforce_creation_rate_limit(
        "equity.fundraising_workflow.create",
        workspace_id,
        60,
        60,
    )?;

    let workflow = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let round_name = round_name.clone();
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
                round_name,
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/apply-terms",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = ApplyFundraisingTermsRequest,
    responses(
        (status = 200, description = "Fundraising workflow terms applied", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/generate-board-packet",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = GenerateWorkflowDocsRequest,
    responses(
        (status = 200, description = "Board packet generated", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

            let round = store
                .read::<EquityRound>("main", workflow.round_id())
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", workflow.round_id()))
                })?;
            let docs = if req.documents.is_empty() {
                generate_fundraising_packet_documents(
                    &store,
                    workspace_id,
                    entity_id,
                    &round,
                    false,
                )?
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/record-board-approval",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = RecordFundraisingBoardApprovalRequest,
    responses(
        (status = 200, description = "Fundraising board approval recorded", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/record-investor-acceptance",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = RecordFundraisingAcceptanceRequest,
    responses(
        (status = 200, description = "Investor acceptance recorded", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/generate-closing-packet",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = GenerateWorkflowDocsRequest,
    responses(
        (status = 200, description = "Closing packet generated", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

            let round = store
                .read::<EquityRound>("main", workflow.round_id())
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", workflow.round_id()))
                })?;
            let docs = if req.documents.is_empty() {
                generate_fundraising_packet_documents(
                    &store,
                    workspace_id,
                    entity_id,
                    &round,
                    true,
                )?
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/record-close",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = RecordFundraisingCloseRequest,
    responses(
        (status = 200, description = "Fundraising close recorded", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    get,
    path = "/v1/equity/fundraising-workflows/{workflow_id}",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Fundraising workflow details", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/prepare-execution",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = PrepareWorkflowExecutionRequest,
    responses(
        (status = 200, description = "Transfer workflow execution prepared", body = TransferWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/prepare-execution",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = PrepareWorkflowExecutionRequest,
    responses(
        (status = 200, description = "Fundraising workflow execution prepared", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/compile-packet",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = CompileWorkflowPacketRequest,
    responses(
        (status = 200, description = "Transfer workflow packet compiled", body = TransactionPacketResponse),
        (status = 404, description = "Workflow not found"),
        (status = 422, description = "Workflow not ready for packet compilation"),
    ),
)]
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
                let transfer = store
                    .read::<ShareTransfer>("main", workflow.transfer_id())
                    .map_err(|_| {
                        AppError::NotFound(format!("transfer {} not found", workflow.transfer_id()))
                    })?;
                generate_transfer_documents(&store, workspace_id, entity_id, &transfer)?
            } else {
                workflow.generated_documents().to_vec()
            };
            let packet = TransactionPacket::new(
                PacketId::new(),
                entity_id,
                intent_id,
                WorkflowType::Transfer,
                workflow_id.to_string(),
                to_packet_items_with_store(&store, &documents),
                if req.required_signers.is_empty() {
                    default_required_signers()
                } else {
                    req.required_signers
                },
            );
            workflow.add_generated_documents(documents.clone());
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/compile-packet",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = CompileWorkflowPacketRequest,
    responses(
        (status = 200, description = "Fundraising workflow packet compiled", body = TransactionPacketResponse),
        (status = 404, description = "Workflow not found"),
        (status = 422, description = "Workflow not ready for packet compilation"),
    ),
)]
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

            let round = store
                .read::<EquityRound>("main", workflow.round_id())
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", workflow.round_id()))
                })?;
            let documents = if is_close {
                if workflow.closing_packet_documents().is_empty() {
                    generate_fundraising_packet_documents(
                        &store,
                        workspace_id,
                        entity_id,
                        &round,
                        true,
                    )?
                } else {
                    workflow.closing_packet_documents().to_vec()
                }
            } else if workflow.board_packet_documents().is_empty() {
                generate_fundraising_packet_documents(
                    &store,
                    workspace_id,
                    entity_id,
                    &round,
                    false,
                )?
            } else {
                workflow.board_packet_documents().to_vec()
            };

            let packet = TransactionPacket::new(
                PacketId::new(),
                entity_id,
                intent_id,
                WorkflowType::Fundraising,
                workflow_id.to_string(),
                to_packet_items_with_store(&store, &documents),
                if req.required_signers.is_empty() {
                    default_required_signers()
                } else {
                    req.required_signers
                },
            );
            if is_close {
                workflow.add_closing_packet_documents(documents.clone());
            } else {
                workflow.add_board_packet_documents(documents.clone());
            }
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/start-signatures",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = StartWorkflowSignaturesRequest,
    responses(
        (status = 200, description = "Signature collection started", body = TransactionPacketResponse),
        (status = 404, description = "Workflow not found"),
        (status = 422, description = "Workflow has no compiled packet"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/start-signatures",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = StartWorkflowSignaturesRequest,
    responses(
        (status = 200, description = "Signature collection started", body = TransactionPacketResponse),
        (status = 404, description = "Workflow not found"),
        (status = 422, description = "Workflow has no compiled packet"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/record-signature",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = RecordWorkflowSignatureRequest,
    responses(
        (status = 200, description = "Signature recorded", body = TransactionPacketResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/record-signature",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = RecordWorkflowSignatureRequest,
    responses(
        (status = 200, description = "Signature recorded", body = TransactionPacketResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/transfer-workflows/{workflow_id}/finalize",
    tag = "equity",
    params(
        ("workflow_id" = TransferWorkflowId, Path, description = "Transfer workflow ID"),
    ),
    request_body = FinalizeWorkflowRequest,
    responses(
        (status = 200, description = "Transfer workflow finalized", body = TransferWorkflowResponse),
        (status = 404, description = "Workflow not found"),
        (status = 422, description = "Workflow not ready for finalization"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/fundraising-workflows/{workflow_id}/finalize",
    tag = "equity",
    params(
        ("workflow_id" = FundraisingWorkflowId, Path, description = "Fundraising workflow ID"),
    ),
    request_body = FinalizeWorkflowRequest,
    responses(
        (status = 200, description = "Fundraising workflow finalized", body = FundraisingWorkflowResponse),
        (status = 404, description = "Workflow not found"),
        (status = 422, description = "Workflow not ready for finalization"),
    ),
)]
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

#[utoipa::path(
    get,
    path = "/v1/equity/workflows/{workflow_type}/{workflow_id}/status",
    tag = "equity",
    params(
        ("workflow_type" = String, Path, description = "Workflow type (transfer or fundraising)"),
        ("workflow_id" = String, Path, description = "Workflow ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Workflow status", body = WorkflowStatusResponse),
        (status = 400, description = "Invalid workflow type"),
        (status = 404, description = "Workflow not found"),
    ),
)]
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

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/cap-table",
    tag = "equity",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        CapTableQuery,
    ),
    responses(
        (status = 200, description = "Cap table", body = CapTableResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/conversions/preview",
    tag = "equity",
    request_body = PreviewConversionRequest,
    responses(
        (status = 200, description = "Conversion preview", body = ConversionPreviewResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Round not found"),
    ),
)]
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

#[utoipa::path(
    post,
    path = "/v1/equity/conversions/execute",
    tag = "equity",
    request_body = ExecuteConversionRequest,
    responses(
        (status = 200, description = "Conversion executed", body = ConversionExecuteResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Round not found"),
    ),
)]
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

#[utoipa::path(
    get,
    path = "/v1/equity/control-map",
    tag = "equity",
    params(ControlMapQuery),
    responses(
        (status = 200, description = "Control map", body = ControlMapResponse),
        (status = 404, description = "Root entity not found"),
    ),
)]
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

#[utoipa::path(
    get,
    path = "/v1/equity/dilution/preview",
    tag = "equity",
    params(DilutionPreviewQuery),
    responses(
        (status = 200, description = "Dilution preview", body = DilutionPreviewResponse),
        (status = 404, description = "Round not found"),
    ),
)]
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

// ── Staged equity round handlers ────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/equity/rounds/staged",
    tag = "equity",
    request_body = StartStagedRoundRequest,
    responses(
        (status = 200, description = "Staged round started", body = RoundResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn start_staged_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<StartStagedRoundRequest>,
) -> Result<Json<RoundResponse>, AppError> {
    let round_name = validate_round_name(&req.name)?;
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let round_name = round_name.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Validate issuer legal entity exists
            let entities = read_all::<LegalEntity>(&store)?;
            if !entities
                .iter()
                .any(|e| e.legal_entity_id() == req.issuer_legal_entity_id)
            {
                return Err(AppError::BadRequest(
                    "issuer_legal_entity_id does not exist".to_owned(),
                ));
            }

            let round = EquityRound::new(
                EquityRoundId::new(),
                req.issuer_legal_entity_id,
                round_name,
                req.pre_money_cents,
                req.round_price_cents,
                req.target_raise_cents,
                None,
                req.metadata,
            );

            let pending = PendingSecuritiesFile {
                round_id: round.equity_round_id(),
                securities: Vec::new(),
            };

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )?,
                FileWrite::json(
                    format!(
                        "cap-table/pending_securities/{}.json",
                        round.equity_round_id()
                    ),
                    &pending,
                )?,
            ];
            store
                .commit(
                    "main",
                    &format!("Start staged equity round {}", round.equity_round_id()),
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

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/equity-rounds",
    tag = "equity",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of equity rounds", body = Vec<RoundResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_equity_rounds(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<RoundResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let rounds = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let all = read_all::<EquityRound>(&store)?;
            Ok::<_, AppError>(all.iter().map(round_to_response).collect::<Vec<_>>())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(rounds))
}

#[utoipa::path(
    post,
    path = "/v1/equity/rounds/{round_id}/securities",
    tag = "equity",
    params(
        ("round_id" = EquityRoundId, Path, description = "Equity round ID"),
    ),
    request_body = AddSecurityRequest,
    responses(
        (status = 200, description = "Security added to staged round", body = PendingSecurity),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Round not found"),
    ),
)]
async fn add_round_security(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(round_id): Path<EquityRoundId>,
    Json(req): Json<AddSecurityRequest>,
) -> Result<Json<PendingSecurity>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    if req.quantity <= 0 {
        return Err(AppError::BadRequest("quantity must be positive".to_owned()));
    }

    let security = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Verify round exists and is Draft
            let round = store
                .read::<EquityRound>("main", round_id)
                .map_err(|e| AppError::NotFound(format!("round {round_id} not found: {e}")))?;
            if !matches!(
                round.status(),
                EquityRoundStatus::Draft | EquityRoundStatus::BoardApproved
            ) {
                return Err(AppError::BadRequest(
                    "round must be in Draft or BoardApproved status".to_owned(),
                ));
            }

            // Validate instrument exists
            let instruments = read_all::<Instrument>(&store)?;
            let instrument = instruments
                .iter()
                .find(|i| i.instrument_id() == req.instrument_id)
                .ok_or_else(|| AppError::BadRequest("instrument_id does not exist".to_owned()))?;
            validate_grant_type_for_instrument(req.grant_type.as_deref(), instrument)?;
            if grant_requires_current_409a(req.grant_type.as_deref(), instrument) {
                ensure_current_409a_exists(&store)?;
            }

            // Resolve holder
            let holders = read_all::<Holder>(&store)?;
            let holder_id = if let Some(hid) = req.holder_id {
                // Direct holder_id — validate it exists
                if !holders.iter().any(|h| h.holder_id() == hid) {
                    return Err(AppError::BadRequest("holder_id does not exist".to_owned()));
                }
                hid
            } else if let Some(ref email) = req.email {
                // Look up contact by email, then find linked holder
                let contacts = read_all::<Contact>(&store)?;
                let contact = contacts.iter().find(|c| {
                    c.email()
                        .map(|e| e.eq_ignore_ascii_case(email))
                        .unwrap_or(false)
                });

                if let Some(contact) = contact {
                    // Find holder linked to this contact
                    let cid = contact.contact_id();
                    if let Some(h) = holders.iter().find(|h| h.contact_id() == cid) {
                        h.holder_id()
                    } else {
                        // Contact exists but no holder — create one
                        let new_holder = Holder::new(
                            HolderId::new(),
                            cid,
                            None,
                            req.recipient_name.clone(),
                            HolderType::Individual,
                            None,
                        );
                        let hid = new_holder.holder_id();
                        store
                            .write_json(
                                "main",
                                &format!("cap-table/holders/{}.json", hid),
                                &new_holder,
                                &format!("Create holder {} for {}", hid, req.recipient_name),
                            )
                            .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
                        hid
                    }
                } else {
                    // No contact found — create new contact + holder
                    let contact_id = ContactId::new();
                    let contact = Contact::new(
                        contact_id,
                        entity_id,
                        workspace_id,
                        ContactType::Individual,
                        req.recipient_name.clone(),
                        Some(email.clone()),
                        ContactCategory::Investor,
                    )
                    .map_err(AppError::BadRequest)?;
                    let new_holder = Holder::new(
                        HolderId::new(),
                        contact_id,
                        None,
                        req.recipient_name.clone(),
                        HolderType::Individual,
                        None,
                    );
                    let hid = new_holder.holder_id();
                    let files = vec![
                        FileWrite::json(format!("contacts/{}.json", contact_id), &contact)?,
                        FileWrite::json(format!("cap-table/holders/{}.json", hid), &new_holder)?,
                    ];
                    store
                        .commit(
                            "main",
                            &format!("Create contact + holder for {}", req.recipient_name),
                            files,
                        )
                        .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
                    hid
                }
            } else {
                // Neither holder_id nor email — create new contact + holder from recipient_name
                let contact_id = ContactId::new();
                let contact = Contact::new(
                    contact_id,
                    entity_id,
                    workspace_id,
                    ContactType::Individual,
                    req.recipient_name.clone(),
                    None,
                    ContactCategory::Investor,
                )
                .map_err(AppError::BadRequest)?;
                let new_holder = Holder::new(
                    HolderId::new(),
                    contact_id,
                    None,
                    req.recipient_name.clone(),
                    HolderType::Individual,
                    None,
                );
                let hid = new_holder.holder_id();
                let files = vec![
                    FileWrite::json(format!("contacts/{}.json", contact_id), &contact)?,
                    FileWrite::json(format!("cap-table/holders/{}.json", hid), &new_holder)?,
                ];
                store
                    .commit(
                        "main",
                        &format!("Create contact + holder for {}", req.recipient_name),
                        files,
                    )
                    .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
                hid
            };

            let security = PendingSecurity {
                holder_id,
                instrument_id: req.instrument_id,
                quantity: req.quantity,
                principal_cents: req.principal_cents,
                recipient_name: req.recipient_name,
                grant_type: req.grant_type,
            };

            // Read pending securities, append, write back
            let pending_path = format!("cap-table/pending_securities/{}.json", round_id);
            let mut pending: PendingSecuritiesFile =
                store.read_json("main", &pending_path).map_err(|e| {
                    AppError::NotFound(format!("pending securities file not found: {e}"))
                })?;
            let all_positions = read_all::<Position>(&store)?;
            ensure_instrument_has_capacity(
                &all_positions,
                &pending.securities,
                instrument,
                req.quantity,
            )?;
            pending.securities.push(security.clone());

            store
                .write_json(
                    "main",
                    &pending_path,
                    &pending,
                    &format!(
                        "Add security for {} to round {}",
                        security.recipient_name, round_id
                    ),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(security)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(security))
}

#[utoipa::path(
    post,
    path = "/v1/equity/rounds/{round_id}/issue",
    tag = "equity",
    params(
        ("round_id" = EquityRoundId, Path, description = "Equity round ID"),
    ),
    request_body = IssueStagedRoundRequest,
    responses(
        (status = 200, description = "Staged round issued", body = IssueStagedRoundResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Round not found"),
    ),
)]
async fn issue_staged_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(round_id): Path<EquityRoundId>,
    Json(req): Json<IssueStagedRoundRequest>,
) -> Result<Json<IssueStagedRoundResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let (round, positions, board_meeting_id, board_agenda_item_id) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Read and validate round
            let mut round = store
                .read::<EquityRound>("main", round_id)
                .map_err(|e| AppError::NotFound(format!("round {round_id} not found: {e}")))?;
            if round.status() != EquityRoundStatus::Draft {
                return Err(AppError::BadRequest(
                    "round is not in Draft status".to_owned(),
                ));
            }

            // Read pending securities
            let pending_path = format!("cap-table/pending_securities/{}.json", round_id);
            let pending: PendingSecuritiesFile =
                store.read_json("main", &pending_path).map_err(|e| {
                    AppError::NotFound(format!("pending securities file not found: {e}"))
                })?;
            if pending.securities.is_empty() {
                return Err(AppError::BadRequest(
                    "no pending securities to issue".to_owned(),
                ));
            }

            // Read all existing positions and instruments.
            let all_positions = read_all::<Position>(&store)?;
            let instrument_map: HashMap<InstrumentId, Instrument> = read_all::<Instrument>(&store)?
                .into_iter()
                .map(|instrument| (instrument.instrument_id(), instrument))
                .collect();
            for security in &pending.securities {
                let instrument = instrument_map.get(&security.instrument_id).ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "instrument {} does not exist",
                        security.instrument_id
                    ))
                })?;
                validate_grant_type_for_instrument(security.grant_type.as_deref(), instrument)?;
            }
            if pending.securities.iter().any(|security| {
                instrument_map
                    .get(&security.instrument_id)
                    .map(|instrument| {
                        grant_requires_current_409a(security.grant_type.as_deref(), instrument)
                    })
                    .unwrap_or(false)
            }) {
                ensure_current_409a_exists(&store)?;
            }
            for instrument in instrument_map.values() {
                ensure_instrument_has_capacity(&all_positions, &pending.securities, instrument, 0)?;
            }

            let mut files: Vec<FileWrite> = Vec::new();
            let mut result_positions: Vec<Position> = Vec::new();

            for sec in &pending.securities {
                // Find existing position for this holder+instrument
                let existing = all_positions.iter().find(|p| {
                    p.issuer_legal_entity_id() == round.issuer_legal_entity_id()
                        && p.holder_id() == sec.holder_id
                        && p.instrument_id() == sec.instrument_id
                });

                let position = if let Some(existing) = existing {
                    let mut p = existing.clone();
                    p.apply_delta(
                        sec.quantity,
                        sec.principal_cents,
                        Some(format!("round:{}", round_id)),
                        None,
                        None,
                    )?;
                    p
                } else {
                    Position::new(
                        PositionId::new(),
                        round.issuer_legal_entity_id(),
                        sec.holder_id,
                        sec.instrument_id,
                        sec.quantity,
                        sec.principal_cents,
                        Some(format!("round:{}", round_id)),
                        None,
                        None,
                    )?
                };

                files.push(FileWrite::json(
                    format!("cap-table/positions/{}.json", position.position_id()),
                    &position,
                )?);
                result_positions.push(position);
            }

            // Require board approval when a board exists; otherwise preserve the
            // boardless draft-close path used for pre-governance entities.
            let has_board = read_all::<GovernanceBody>(&store)?
                .into_iter()
                .any(|body| body.body_type() == BodyType::BoardOfDirectors);
            let mut board_meeting_id = None;
            if has_board {
                if round.status() == EquityRoundStatus::Draft {
                    let meeting_id = req.meeting_id.ok_or_else(|| {
                        AppError::BadRequest(
                            "meeting_id is required to issue a round when a board exists"
                                .to_owned(),
                        )
                    })?;
                    let resolution_id = req.resolution_id.ok_or_else(|| {
                        AppError::BadRequest(
                            "resolution_id is required to issue a round when a board exists"
                                .to_owned(),
                        )
                    })?;
                    validate_board_resolution_for_round(
                        &store,
                        entity_id,
                        meeting_id,
                        resolution_id,
                    )?;
                    round
                        .record_board_approval(meeting_id, resolution_id)
                        .map_err(|e| {
                            AppError::BadRequest(format!("failed to record board approval: {e}"))
                        })?;
                }
                board_meeting_id = round.board_approval_meeting_id();
                round.accept(None).map_err(|e| {
                    AppError::BadRequest(format!("failed to accept approved round: {e}"))
                })?;
                round
                    .close()
                    .map_err(|e| AppError::BadRequest(format!("failed to close round: {e}")))?;
            } else {
                round
                    .close_from_draft()
                    .map_err(|e| AppError::BadRequest(format!("failed to close round: {e}")))?;
            }
            files.push(FileWrite::json(
                format!("cap-table/rounds/{}.json", round.equity_round_id()),
                &round,
            )?);
            let board_agenda_item_id = None;

            store
                .commit(
                    "main",
                    &format!(
                        "Issue {} securities and close round {}",
                        pending.securities.len(),
                        round_id,
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>((
                round,
                result_positions,
                board_meeting_id,
                board_agenda_item_id,
            ))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(IssueStagedRoundResponse {
        round: round_to_response(&round),
        positions: positions.iter().map(position_to_response).collect(),
        meeting_id: board_meeting_id,
        agenda_item_id: board_agenda_item_id,
    }))
}

// ── Valuation types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateValuationRequest {
    pub entity_id: EntityId,
    pub valuation_type: ValuationType,
    pub effective_date: NaiveDate,
    #[serde(default)]
    pub fmv_per_share_cents: Option<i64>,
    #[serde(default)]
    pub enterprise_value_cents: Option<i64>,
    #[serde(default)]
    pub hurdle_amount_cents: Option<i64>,
    pub methodology: ValuationMethodology,
    #[serde(default)]
    pub provider_contact_id: Option<ContactId>,
    #[serde(default)]
    pub report_document_id: Option<DocumentId>,
    #[serde(default)]
    pub dlom: Option<String>,
    #[serde(default)]
    pub report_date: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmitValuationForApprovalRequest {
    pub entity_id: EntityId,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApproveValuationRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub resolution_id: Option<ResolutionId>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateLegacyGrantRequest {
    pub entity_id: EntityId,
    pub grant_type: GrantType,
    pub shares: i64,
    pub recipient_name: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateLegacyShareTransferRequest {
    pub entity_id: EntityId,
    pub share_class_id: ShareClassId,
    pub from_holder: String,
    pub to_holder: String,
    pub shares: i64,
    pub transfer_type: TransferType,
    #[serde(default)]
    pub transferee_rights: Option<TransfereeRights>,
    #[serde(default)]
    pub governing_doc_type: Option<GoverningDocType>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ValuationResponse {
    pub valuation_id: ValuationId,
    pub entity_id: EntityId,
    pub valuation_type: ValuationType,
    pub effective_date: NaiveDate,
    pub expiration_date: Option<NaiveDate>,
    pub fmv_per_share_cents: Option<i64>,
    pub enterprise_value_cents: Option<i64>,
    pub hurdle_amount_cents: Option<i64>,
    pub methodology: ValuationMethodology,
    pub provider_contact_id: Option<ContactId>,
    pub report_document_id: Option<DocumentId>,
    pub board_approval_resolution_id: Option<ResolutionId>,
    pub status: ValuationStatus,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_id: Option<MeetingId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agenda_item_id: Option<AgendaItemId>,
}

fn valuation_to_response(v: &Valuation) -> ValuationResponse {
    ValuationResponse {
        valuation_id: v.valuation_id(),
        entity_id: v.entity_id(),
        valuation_type: v.valuation_type(),
        effective_date: v.effective_date(),
        expiration_date: v.expiration_date(),
        fmv_per_share_cents: v.fmv_per_share_cents().map(|c| c.raw()),
        enterprise_value_cents: v.enterprise_value_cents().map(|c| c.raw()),
        hurdle_amount_cents: v.hurdle_amount_cents().map(|c| c.raw()),
        methodology: v.methodology(),
        provider_contact_id: v.provider_contact_id(),
        report_document_id: v.report_document_id(),
        board_approval_resolution_id: v.board_approval_resolution_id(),
        status: v.status(),
        created_at: v.created_at().to_rfc3339(),
        meeting_id: v.board_approval_meeting_id(),
        agenda_item_id: v.board_approval_agenda_item_id(),
    }
}

fn safe_note_to_response(note: &SafeNote) -> SafeNoteResponse {
    SafeNoteResponse {
        safe_note_id: note.safe_note_id(),
        entity_id: note.entity_id(),
        investor_name: note.investor_name().to_owned(),
        investor_contact_id: note.investor_id(),
        principal_amount_cents: note.principal_amount_cents().raw(),
        valuation_cap_cents: note.valuation_cap_cents().map(|value| value.raw()),
        discount_rate: note.discount_rate(),
        safe_type: note.safe_type(),
        pro_rata_rights: note.pro_rata_rights(),
        status: note.status(),
        document_id: note.document_id(),
        conversion_unit_type: note.conversion_unit_type().to_owned(),
        issued_at: note.issued_at().to_rfc3339(),
        created_at: note.created_at().to_rfc3339(),
        converted_at: note.converted_at().map(|value| value.to_rfc3339()),
        conversion_shares: note.conversion_shares().map(|value| value.raw()),
        conversion_price_cents: note.conversion_price_cents().map(|value| value.raw()),
        meeting_id: note.board_approval_meeting_id(),
        resolution_id: note.board_approval_resolution_id(),
    }
}

// ── SAFE note handlers ──────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/safe-notes",
    tag = "equity",
    request_body = CreateSafeNoteRequest,
    responses(
        (status = 200, description = "SAFE note issued", body = SafeNoteResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_safe_note(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateSafeNoteRequest>,
) -> Result<Json<SafeNoteResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    if req.investor_name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "investor_name must not be empty".to_owned(),
        ));
    }
    if req
        .email
        .as_deref()
        .is_some_and(|email| email.trim().is_empty())
    {
        return Err(AppError::BadRequest("email cannot be empty".to_owned()));
    }
    if req
        .conversion_unit_type
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(AppError::BadRequest(
            "conversion_unit_type cannot be empty".to_owned(),
        ));
    }
    if req.meeting_id.is_some() ^ req.resolution_id.is_some() {
        return Err(AppError::BadRequest(
            "meeting_id and resolution_id must be provided together".to_owned(),
        ));
    }
    Cents::new(req.principal_amount_cents)
        .require_positive()
        .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    if let Some(cap) = req.valuation_cap_cents {
        Cents::new(cap)
            .require_positive()
            .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    }
    if let Some(rate) = req.discount_rate
        && !(0.0..=1.0).contains(&rate)
    {
        return Err(AppError::BadRequest(
            "discount_rate must be between 0.0 and 1.0".to_owned(),
        ));
    }
    state.enforce_creation_rate_limit("equity.safe_note.create", workspace_id, 60, 60)?;

    let safe_note = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let (investor_contact_id, mut files) = resolve_or_prepare_investor_contact(
                &store,
                entity_id,
                workspace_id,
                &req.investor_name,
                req.investor_contact_id,
                req.email.as_deref(),
            )?;

            let approval_required = read_all::<GovernanceBody>(&store)?.into_iter().any(|body| {
                body.status() == BodyStatus::Active
                    && matches!(
                        body.body_type(),
                        BodyType::BoardOfDirectors | BodyType::LlcMemberVote
                    )
            });

            let safe_note_id = SafeNoteId::new();
            let mut safe_note = SafeNote::new(
                safe_note_id,
                entity_id,
                req.investor_name.trim().to_owned(),
                investor_contact_id,
                Cents::new(req.principal_amount_cents),
                req.valuation_cap_cents.map(Cents::new),
                req.discount_rate,
                req.safe_type.unwrap_or(SafeType::PostMoney),
                req.pro_rata_rights,
                req.document_id,
                req.conversion_unit_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("preferred_equity")
                    .to_owned(),
            )
            .map_err(|e| AppError::BadRequest(format!("{e}")))?;

            if let (Some(meeting_id), Some(resolution_id)) = (req.meeting_id, req.resolution_id) {
                validate_resolution_for_equity_workflow(
                    &store,
                    entity_id,
                    meeting_id,
                    resolution_id,
                )?;
                safe_note.record_board_approval(meeting_id, resolution_id);
            } else if approval_required {
                return Err(AppError::BadRequest(
                    "meeting_id and resolution_id are required to issue a SAFE when an active board or member-vote body exists".to_owned(),
                ));
            }

            files.push(
                FileWrite::json(SafeNote::storage_path(safe_note_id), &safe_note)
                    .map_err(|e| AppError::Internal(format!("serialize safe note: {e}")))?,
            );
            store
                .commit("main", &format!("Issue SAFE note {safe_note_id}"), files)
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(safe_note)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(safe_note_to_response(&safe_note)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/safe-notes",
    tag = "equity",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List SAFE notes", body = Vec<SafeNoteResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_safe_notes(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<SafeNoteResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let safe_notes = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut notes = read_all::<SafeNote>(&store)?;
            notes.sort_by_key(|note| note.created_at());
            Ok::<_, AppError>(
                notes
                    .iter()
                    .rev()
                    .map(safe_note_to_response)
                    .collect::<Vec<_>>(),
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(safe_notes))
}

// ── Valuation handlers ──────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/valuations",
    tag = "equity",
    request_body = CreateValuationRequest,
    responses(
        (status = 200, description = "Valuation created", body = ValuationResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_valuation(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateValuationRequest>,
) -> Result<Json<ValuationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    state.enforce_creation_rate_limit("equity.valuation.create", workspace_id, 60, 60)?;
    if let Some(amount) = req.fmv_per_share_cents {
        Cents::new(amount)
            .require_positive()
            .map_err(|e| AppError::BadRequest(e.to_owned()))?;
        if amount > MAX_FMV_PER_SHARE_CENTS {
            return Err(AppError::BadRequest(format!(
                "fmv_per_share_cents exceeds sanity limit (max {} cents per share)",
                MAX_FMV_PER_SHARE_CENTS
            )));
        }
    }
    if let Some(amount) = req.enterprise_value_cents {
        Cents::new(amount)
            .require_positive()
            .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    }
    if let Some(amount) = req.hurdle_amount_cents {
        Cents::new(amount)
            .require_positive()
            .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    }
    let today = Utc::now().date_naive();
    if req.effective_date > today {
        return Err(AppError::BadRequest(
            "effective_date cannot be in the future".to_owned(),
        ));
    }
    let max_days_past = if req.valuation_type == ValuationType::FourOhNineA {
        365
    } else {
        730
    };
    validate_not_too_far_past("effective_date", req.effective_date, max_days_past)?;
    if req.valuation_type == ValuationType::FourOhNineA {
        if req.fmv_per_share_cents.is_none() {
            return Err(AppError::BadRequest(
                "fmv_per_share_cents is required for a 409A valuation report".to_owned(),
            ));
        }
        if req.enterprise_value_cents.is_none() {
            return Err(AppError::BadRequest(
                "enterprise_value_cents is required for a 409A valuation report".to_owned(),
            ));
        }
    }

    let valuation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let (entity, _profile) = entity_profile_for_docs(&store)?;
            if let Some(formation_date) = entity.formation_date()
                && req.effective_date < formation_date.date_naive()
            {
                return Err(AppError::BadRequest(format!(
                    "effective_date {} cannot be before entity formation date {}",
                    req.effective_date,
                    formation_date.date_naive()
                )));
            }
            let valuation_id = ValuationId::new();
            let report_document_id = if let Some(document_id) = req.report_document_id {
                Some(document_id)
            } else if req.valuation_type == ValuationType::FourOhNineA {
                let provider_name = req
                    .provider_contact_id
                    .map(|contact_id| contact_name_email(&store, contact_id).0)
                    .unwrap_or_else(|| "Independent Valuation Provider".to_owned());
                Some(create_governance_document(
                    &store,
                    entity_id,
                    workspace_id,
                    DocumentType::FourOhNineAValuationReport,
                    format!("409A Valuation Report — {}", entity.legal_name()),
                    "four_oh_nine_a_valuation_report",
                    serde_json::json!({
                        "effective_date": req.effective_date.to_string(),
                        "entity_legal_name": entity.legal_name(),
                        "valuation_type": "409A",
                        "methodology": format!("{:?}", req.methodology),
                        "provider_name": provider_name,
                        "fmv_per_share": doc_generator::format_usd(req.fmv_per_share_cents.unwrap_or_default()),
                        "enterprise_value": doc_generator::format_usd(req.enterprise_value_cents.unwrap_or_default()),
                        "expiration_date": (req.effective_date + chrono::Duration::days(365)).to_string(),
                        "dlom": req.dlom.as_deref().unwrap_or("N/A"),
                        "report_date": req.report_date.as_deref().unwrap_or(&req.effective_date.to_string()),
                    }),
                    Vec::new(),
                )?)
            } else {
                None
            };
            let valuation = Valuation::new(
                valuation_id,
                entity_id,
                workspace_id,
                req.valuation_type,
                req.effective_date,
                req.fmv_per_share_cents.map(Cents::new),
                req.enterprise_value_cents.map(Cents::new),
                req.hurdle_amount_cents.map(Cents::new),
                req.methodology,
                req.provider_contact_id,
                report_document_id,
            );
            store
                .write::<Valuation>(
                    "main",
                    valuation_id,
                    &valuation,
                    &format!("Create valuation {valuation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(valuation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(valuation_to_response(&valuation)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/valuations",
    tag = "equity",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of valuations", body = Vec<ValuationResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_valuations(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ValuationResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let valuations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let all = read_all::<Valuation>(&store)?;
            Ok::<_, AppError>(all.iter().map(valuation_to_response).collect::<Vec<_>>())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(valuations))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/current-409a",
    tag = "equity",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Current 409A valuation", body = ValuationResponse),
        (status = 404, description = "No current 409A valuation found"),
    ),
)]
async fn get_current_409a(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<ValuationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let valuation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let all = read_all::<Valuation>(&store)?;
            all.into_iter()
                .find(|v| v.is_current_409a())
                .ok_or_else(|| {
                    AppError::NotFound("no current approved 409A valuation found".to_owned())
                })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(valuation_to_response(&valuation)))
}

#[utoipa::path(
    post,
    path = "/v1/valuations/{valuation_id}/submit-for-approval",
    tag = "equity",
    params(
        ("valuation_id" = ValuationId, Path, description = "Valuation ID"),
    ),
    request_body = SubmitValuationForApprovalRequest,
    responses(
        (status = 200, description = "Valuation submitted for approval", body = ValuationResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Valuation not found"),
    ),
)]
async fn submit_valuation_for_approval(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(valuation_id): Path<ValuationId>,
    Json(req): Json<SubmitValuationForApprovalRequest>,
) -> Result<Json<ValuationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            ensure_entity_is_active_for_governance(&store, "valuation approval submission")?;

            // Read and transition the valuation.
            let mut valuation = store
                .read::<Valuation>("main", valuation_id)
                .map_err(|_| AppError::NotFound(format!("valuation {} not found", valuation_id)))?;
            valuation
                .submit_for_approval()
                .map_err(|e| AppError::BadRequest(format!("{e}")))?;

            // Find the board body for this entity.
            let bodies = read_all::<GovernanceBody>(&store)?;
            let board_body = bodies
                .iter()
                .find(|b| b.body_type() == BodyType::BoardOfDirectors)
                .ok_or_else(|| {
                    AppError::NotFound("no board governance body found for entity".to_owned())
                })?;
            let body_id = board_body.body_id();

            // Look for an existing Draft or Noticed meeting for this board body.
            let meetings = read_all::<Meeting>(&store)?;
            let existing_meeting = meetings.iter().find(|m| {
                m.body_id() == body_id
                    && (m.status() == MeetingStatus::Draft || m.status() == MeetingStatus::Noticed)
            });

            let agenda_item_id = AgendaItemId::new();
            let effective = valuation.effective_date();
            let agenda_title = format!("Approve 409A Valuation ({effective})");

            let (meeting_id, new_meeting) = if let Some(m) = existing_meeting {
                (m.meeting_id(), None)
            } else {
                let mid = MeetingId::new();
                let meeting = Meeting::new(
                    mid,
                    body_id,
                    MeetingType::BoardMeeting,
                    format!("Board Meeting — 409A Approval ({effective})"),
                    None,
                    String::new(),
                    0,
                );
                (mid, Some(meeting))
            };

            // Determine the next sequence number by counting existing agenda items.
            let existing_item_ids = store
                .list_agenda_item_ids("main", meeting_id)
                .unwrap_or_default();
            let next_seq = (existing_item_ids.len() as u32) + 1;

            let agenda_item = AgendaItem::new(
                agenda_item_id,
                meeting_id,
                next_seq,
                agenda_title,
                None,
                AgendaItemType::Resolution,
            );
            valuation.record_submission_for_approval(meeting_id, agenda_item_id);

            // Build atomic commit.
            let mut files = vec![
                FileWrite::json(Valuation::storage_path(valuation_id), &valuation)
                    .map_err(|e| AppError::Internal(format!("serialize valuation: {e}")))?,
                FileWrite::json(
                    format!(
                        "governance/meetings/{}/agenda/{}.json",
                        meeting_id, agenda_item_id
                    ),
                    &agenda_item,
                )
                .map_err(|e| AppError::Internal(format!("serialize agenda item: {e}")))?,
            ];
            if let Some(ref meeting) = new_meeting {
                files.push(
                    FileWrite::json(Meeting::storage_path(meeting_id), meeting)
                        .map_err(|e| AppError::Internal(format!("serialize meeting: {e}")))?,
                );
            }

            store
                .commit(
                    "main",
                    &format!("Submit valuation {valuation_id} for board approval"),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            let mut resp = valuation_to_response(&valuation);
            resp.meeting_id = Some(meeting_id);
            resp.agenda_item_id = Some(agenda_item_id);
            Ok::<_, AppError>(resp)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/v1/valuations/{valuation_id}/approve",
    tag = "equity",
    params(
        ("valuation_id" = ValuationId, Path, description = "Valuation ID"),
    ),
    request_body = ApproveValuationRequest,
    responses(
        (status = 200, description = "Valuation approved", body = ValuationResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Valuation not found"),
    ),
)]
async fn approve_valuation(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(valuation_id): Path<ValuationId>,
    Json(req): Json<ApproveValuationRequest>,
) -> Result<Json<ValuationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let valuation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let mut valuation = store
                .read::<Valuation>("main", valuation_id)
                .map_err(|_| AppError::NotFound(format!("valuation {} not found", valuation_id)))?;
            if let Some(resolution_id) = req.resolution_id {
                let meetings = read_all::<Meeting>(&store)?;
                let mut found = false;
                for meeting in meetings {
                    let resolution_ids = store
                        .list_resolution_ids("main", meeting.meeting_id())
                        .unwrap_or_default();
                    if !resolution_ids.contains(&resolution_id) {
                        continue;
                    }
                    let resolution = store
                        .read_resolution("main", meeting.meeting_id(), resolution_id)
                        .map_err(|e| AppError::Internal(format!("read resolution: {e}")))?;
                    let body = store
                        .read::<GovernanceBody>("main", meeting.body_id())
                        .map_err(|_| {
                            AppError::NotFound(format!(
                                "governance body {} not found",
                                meeting.body_id()
                            ))
                        })?;
                    if body.entity_id() != entity_id {
                        return Err(AppError::BadRequest(
                            "approval resolution must belong to the same entity".to_owned(),
                        ));
                    }
                    if !resolution.passed()
                        || !matches!(
                            meeting.status(),
                            MeetingStatus::Convened | MeetingStatus::Adjourned
                        )
                    {
                        return Err(AppError::BadRequest(
                            "approval resolution must be passed in a convened or adjourned meeting"
                                .to_owned(),
                        ));
                    }
                    found = true;
                    break;
                }
                if !found {
                    return Err(AppError::NotFound(format!(
                        "resolution {} not found for entity",
                        resolution_id
                    )));
                }
                ensure_equity_resolution_unused(&store, resolution_id)?;
            }
            valuation
                .approve(req.resolution_id)
                .map_err(|e| AppError::BadRequest(format!("{e}")))?;

            // Auto-supersede previous approved 409A valuations.
            let mut files = vec![
                FileWrite::json(Valuation::storage_path(valuation_id), &valuation)
                    .map_err(|e| AppError::Internal(format!("serialize valuation: {e}")))?,
            ];

            if valuation.valuation_type() == ValuationType::FourOhNineA {
                let all = read_all::<Valuation>(&store)?;
                for prev in all {
                    if prev.valuation_id() != valuation_id
                        && prev.valuation_type() == ValuationType::FourOhNineA
                        && prev.status() == ValuationStatus::Approved
                    {
                        let mut prev = prev;
                        if prev.supersede().is_ok() {
                            files.push(
                                FileWrite::json(
                                    Valuation::storage_path(prev.valuation_id()),
                                    &prev,
                                )
                                .map_err(|e| {
                                    AppError::Internal(format!("serialize prev valuation: {e}"))
                                })?,
                            );
                        }
                    }
                }
            }

            store
                .commit("main", &format!("Approve valuation {valuation_id}"), files)
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(valuation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(valuation_to_response(&valuation)))
}

#[utoipa::path(
    post,
    path = "/v1/equity/grants",
    tag = "equity",
    request_body = CreateLegacyGrantRequest,
    security(("bearer_auth" = [])),
    responses((status = 501, description = "Not implemented")),
)]
async fn create_legacy_grant(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(_state): State<AppState>,
    Json(req): Json<CreateLegacyGrantRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    let _ = req;
    Err(AppError::NotImplemented(
        "legacy equity grant issuance is disabled; use the governed equity issuance workflow"
            .to_owned(),
    ))
}

#[utoipa::path(
    post,
    path = "/v1/share-transfers",
    tag = "equity",
    request_body = CreateLegacyShareTransferRequest,
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Transfer created")),
)]
async fn create_legacy_share_transfer(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateLegacyShareTransferRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    let shares = ShareCount::new(req.shares)
        .require_positive()
        .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    let from_holder = req.from_holder.clone();
    let to_holder = req.to_holder.clone();
    if from_holder.trim() == to_holder.trim() {
        return Err(AppError::BadRequest(
            "from_holder and to_holder must be different".to_owned(),
        ));
    }
    let share_class_id = req.share_class_id;

    let transfer = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let from_contact = resolve_transfer_sender_contact(&store, &from_holder)?;
            let to_contact = resolve_contact_reference(&store, &to_holder)?;
            if from_contact.contact_id() == to_contact.contact_id() {
                return Err(AppError::BadRequest(
                    "from_holder and to_holder must resolve to different contacts".to_owned(),
                ));
            }
            let share_class = read_all::<ShareClass>(&store)?
                .into_iter()
                .find(|sc| sc.share_class_id() == share_class_id)
                .ok_or_else(|| {
                    AppError::BadRequest(format!("share class {} not found", share_class_id))
                })?;
            let transfer_id = TransferId::new();
            let mut transfer = ShareTransfer::new(
                transfer_id,
                entity_id,
                workspace_id,
                share_class.share_class_id(),
                from_contact.contact_id(),
                to_contact.contact_id(),
                req.transfer_type,
                shares,
                None,
                None,
                req.governing_doc_type.unwrap_or(GoverningDocType::Other),
                req.transferee_rights.unwrap_or(TransfereeRights::Limited),
            )
            .map_err(|e| AppError::BadRequest(format!("{e}")))?;
            transfer
                .submit_for_review()
                .map_err(|e| AppError::BadRequest(format!("{e}")))?;
            let path = format!("cap-table/transfers/{}.json", transfer_id);
            store
                .write_json(
                    "main",
                    &path,
                    &transfer,
                    &format!("Create share transfer {transfer_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(transfer)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(serde_json::json!({
        "transfer_id": transfer.transfer_id(),
        "entity_id": transfer.entity_id(),
        "from_holder": req.from_holder,
        "to_holder": req.to_holder,
        "from_contact_id": transfer.sender_contact_id(),
        "to_contact_id": transfer.to_contact_id(),
        "shares": transfer.share_count().raw(),
        "share_count": transfer.share_count().raw(),
        "transfer_type": transfer.transfer_type(),
        "status": transfer.status(),
    })))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/share-transfers",
    tag = "equity",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "List of share transfers")),
)]
async fn list_legacy_share_transfers(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let transfers = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let all = read_all::<ShareTransfer>(&store)?;
            Ok::<_, AppError>(
                all.into_iter()
                    .filter(|transfer| transfer.entity_id() == entity_id)
                    .map(|transfer| {
                        let from_holder = store
                            .read::<Contact>("main", transfer.sender_contact_id())
                            .map(|contact| contact.name().to_owned())
                            .unwrap_or_else(|_| transfer.sender_contact_id().to_string());
                        let to_holder = store
                            .read::<Contact>("main", transfer.to_contact_id())
                            .map(|contact| contact.name().to_owned())
                            .unwrap_or_else(|_| transfer.to_contact_id().to_string());
                        serde_json::json!({
                            "transfer_id": transfer.transfer_id(),
                            "entity_id": transfer.entity_id(),
                            "from_holder": from_holder,
                            "to_holder": to_holder,
                            "shares": transfer.share_count().raw(),
                            "share_count": transfer.share_count().raw(),
                            "transfer_type": transfer.transfer_type(),
                            "status": transfer.status(),
                        })
                    })
                    .collect::<Vec<_>>(),
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(transfers))
}

// ── OpenAPI ──────────────────────────────────────────────────────────

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create_holder,
        create_legal_entity,
        create_control_link,
        create_instrument,
        adjust_position,
        create_round,
        start_staged_round,
        list_equity_rounds,
        add_round_security,
        issue_staged_round,
        apply_round_terms,
        board_approve_round,
        accept_round,
        create_transfer_workflow,
        get_transfer_workflow,
        generate_transfer_workflow_docs,
        submit_transfer_workflow_for_review,
        record_transfer_workflow_review,
        record_transfer_workflow_rofr,
        record_transfer_workflow_board_approval,
        record_transfer_workflow_execution,
        prepare_transfer_workflow_execution,
        compile_transfer_workflow_packet,
        start_transfer_workflow_signatures,
        record_transfer_workflow_signature,
        finalize_transfer_workflow,
        create_fundraising_workflow,
        get_fundraising_workflow,
        apply_fundraising_workflow_terms,
        generate_fundraising_board_packet,
        record_fundraising_workflow_board_approval,
        record_fundraising_workflow_acceptance,
        generate_fundraising_closing_packet,
        record_fundraising_workflow_close,
        prepare_fundraising_workflow_execution,
        compile_fundraising_workflow_packet,
        start_fundraising_workflow_signatures,
        record_fundraising_workflow_signature,
        finalize_fundraising_workflow,
        get_workflow_status,
        preview_conversion,
        execute_conversion,
        get_cap_table,
        get_control_map,
        get_dilution_preview,
        create_safe_note,
        list_safe_notes,
        create_valuation,
        list_valuations,
        get_current_409a,
        submit_valuation_for_approval,
        approve_valuation,
        create_legacy_grant,
        create_legacy_share_transfer,
        list_legacy_share_transfers,
    ),
    components(schemas(
        CapTableBasis,
        CapTableQuery,
        ControlMapQuery,
        DilutionPreviewQuery,
        CreateHolderRequest,
        CreateLegalEntityRequest,
        CreateControlLinkRequest,
        CreateInstrumentRequest,
        AdjustPositionRequest,
        CreateRoundRequest,
        ApplyRoundTermsRequest,
        BoardApproveRoundRequest,
        AcceptRoundRequest,
        PendingSecurity,
        PendingSecuritiesFile,
        StartStagedRoundRequest,
        AddSecurityRequest,
        IssueStagedRoundRequest,
        IssueStagedRoundResponse,
        PreviewConversionRequest,
        ExecuteConversionRequest,
        CreateTransferWorkflowRequest,
        GenerateWorkflowDocsRequest,
        SubmitTransferReviewRequest,
        RecordTransferReviewRequest,
        RecordTransferRofrRequest,
        RecordTransferBoardApprovalRequest,
        RecordTransferExecutionRequest,
        CreateFundraisingWorkflowRequest,
        ApplyFundraisingTermsRequest,
        RecordFundraisingBoardApprovalRequest,
        RecordFundraisingAcceptanceRequest,
        RecordFundraisingCloseRequest,
        PrepareWorkflowExecutionRequest,
        CompileWorkflowPacketRequest,
        StartWorkflowSignaturesRequest,
        RecordWorkflowSignatureRequest,
        FinalizeWorkflowRequest,
        CreateValuationRequest,
        SubmitValuationForApprovalRequest,
        ApproveValuationRequest,
        HolderResponse,
        LegalEntityResponse,
        ControlLinkResponse,
        InstrumentResponse,
        PositionResponse,
        RoundResponse,
        RuleSetResponse,
        CapTableInstrumentSummary,
        CapTableHolderSummary,
        CapTableResponse,
        ConversionPreviewLine,
        ConversionPreviewResponse,
        ConversionExecuteResponse,
        ControlMapEdge,
        ControlMapResponse,
        DilutionPreviewResponse,
        TransferWorkflowResponse,
        FundraisingWorkflowResponse,
        TransactionPacketResponse,
        PacketSignatureResponse,
        WorkflowStatusResponse,
        CreateSafeNoteRequest,
        SafeNoteResponse,
        ValuationResponse,
        CreateLegacyGrantRequest,
        CreateLegacyShareTransferRequest,
    )),
    tags((name = "equity", description = "Cap table, instruments, rounds, and conversions")),
)]
pub struct EquityApi;

// ── Router ───────────────────────────────────────────────────────────

pub fn equity_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/equity/holders", post(create_holder))
        .route("/v1/equity/entities", post(create_legal_entity))
        .route("/v1/equity/control-links", post(create_control_link))
        .route("/v1/equity/instruments", post(create_instrument))
        .route("/v1/equity/positions/adjust", post(adjust_position))
        .route("/v1/equity/rounds", post(create_round))
        .route("/v1/equity/rounds/staged", post(start_staged_round))
        .route(
            "/v1/entities/{entity_id}/equity-rounds",
            get(list_equity_rounds),
        )
        .route(
            "/v1/equity/rounds/{round_id}/securities",
            post(add_round_security),
        )
        .route(
            "/v1/equity/rounds/{round_id}/issue",
            post(issue_staged_round),
        )
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
        .route("/v1/equity/grants", post(create_legacy_grant))
        .route("/v1/share-transfers", post(create_legacy_share_transfer))
        .route(
            "/v1/entities/{entity_id}/share-transfers",
            get(list_legacy_share_transfers),
        )
        .route("/v1/entities/{entity_id}/cap-table", get(get_cap_table))
        .route("/v1/equity/control-map", get(get_control_map))
        .route("/v1/equity/dilution/preview", get(get_dilution_preview))
        .route("/v1/safe-notes", post(create_safe_note))
        .route("/v1/entities/{entity_id}/safe-notes", get(list_safe_notes))
        .route("/v1/valuations", post(create_valuation))
        .route("/v1/entities/{entity_id}/valuations", get(list_valuations))
        .route(
            "/v1/entities/{entity_id}/current-409a",
            get(get_current_409a),
        )
        .route(
            "/v1/valuations/{valuation_id}/submit-for-approval",
            post(submit_valuation_for_approval),
        )
        .route(
            "/v1/valuations/{valuation_id}/approve",
            post(approve_valuation),
        )
}
