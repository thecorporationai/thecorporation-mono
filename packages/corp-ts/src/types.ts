// ── Domain types ─────────────────────────────────────────────────────────────
// Mirror the Rust domain types for TypeScript consumers.

// ── IDs ──────────────────────────────────────────────────────────────────────

/** All IDs are UUID v4 strings. */
export type EntityId = string;
export type WorkspaceId = string;
export type DocumentId = string;
export type ContactId = string;
export type FilingId = string;
export type TaxProfileId = string;
export type CapTableId = string;
export type InstrumentId = string;
export type EquityGrantId = string;
export type SafeNoteId = string;
export type ValuationId = string;
export type TransferId = string;
export type FundingRoundId = string;
export type HolderId = string;
export type GovernanceBodyId = string;
export type GovernanceSeatId = string;
export type MeetingId = string;
export type AgendaItemId = string;
export type VoteId = string;
export type ResolutionId = string;
export type AccountId = string;
export type JournalEntryId = string;
export type InvoiceId = string;
export type PaymentId = string;
export type BankAccountId = string;
export type PayrollRunId = string;
export type IntentId = string;
export type ObligationId = string;
export type ReceiptId = string;
export type AgentId = string;
export type WorkItemId = string;
export type ServiceRequestId = string;
export type ApiKeyId = string;
export type OptionExerciseId = string;
export type PositionId = string;
export type VestingScheduleId = string;
export type VestingEventId = string;
export type RepurchaseRightId = string;

// ── Formation ────────────────────────────────────────────────────────────────

export type EntityType = "c_corp" | "llc";

export type FormationStatus =
  | "pending"
  | "documents_generated"
  | "documents_signed"
  | "filing_submitted"
  | "filed"
  | "ein_applied"
  | "active"
  | "rejected"
  | "dissolved";

export type DocumentStatus = "draft" | "signed" | "amended" | "filed";

export type DocumentType =
  | "articles_of_incorporation"
  | "articles_of_organization"
  | "bylaws"
  | "incorporator_action"
  | "initial_board_consent"
  | "operating_agreement"
  | "initial_written_consent"
  | "ss4_application"
  | "resolution"
  | "safe_agreement"
  | "stock_transfer_agreement"
  | "other";

export interface Entity {
  entity_id: EntityId;
  workspace_id: WorkspaceId;
  legal_name: string;
  entity_type: EntityType;
  jurisdiction: string;
  formation_status: FormationStatus;
  registered_agent_name: string | null;
  registered_agent_address: string | null;
  formation_date: string | null;
  dissolution_effective_date: string | null;
  created_at: string;
}

export interface Signature {
  signature_id: string;
  document_id: DocumentId;
  signer_name: string;
  signer_role: string;
  signer_email: string;
  signature_text: string;
  signature_svg: string | null;
  document_hash_at_signing: string;
  signed_at: string;
}

export interface Document {
  document_id: DocumentId;
  entity_id: EntityId;
  workspace_id: WorkspaceId;
  document_type: DocumentType;
  title: string;
  content_hash: string;
  content: unknown;
  status: DocumentStatus;
  version: number;
  signatures: Signature[];
  created_at: string;
}

export interface Filing {
  filing_id: FilingId;
  entity_id: EntityId;
  workspace_id: WorkspaceId;
  filing_type: string;
  jurisdiction: string;
  status: string;
  confirmation_number: string | null;
  created_at: string;
}

export interface TaxProfile {
  tax_profile_id: TaxProfileId;
  entity_id: EntityId;
  workspace_id: WorkspaceId;
  ein: string | null;
  ein_status: "pending" | "active";
  classification: string;
  created_at: string;
}

// ── Equity ───────────────────────────────────────────────────────────────────

export interface CapTable {
  cap_table_id: CapTableId;
  entity_id: EntityId;
  status: "active" | "frozen";
  created_at: string;
}

export interface Instrument {
  instrument_id: InstrumentId;
  cap_table_id: CapTableId;
  symbol: string;
  kind: "common_equity" | "preferred_equity" | "membership_unit" | "option_grant" | "safe";
  par_value: string;
  authorized_units: number;
  liquidation_preference: string | null;
  created_at: string;
}

export interface EquityGrant {
  grant_id: EquityGrantId;
  entity_id: EntityId;
  cap_table_id: CapTableId;
  instrument_id: InstrumentId;
  recipient_contact_id: ContactId;
  recipient_name: string;
  grant_type: string;
  shares: number;
  status: string;
  created_at: string;
}

export type ExerciseType = "full" | "partial" | "early";

export interface OptionExercise {
  exercise_id: OptionExerciseId;
  entity_id: EntityId;
  grant_id: EquityGrantId;
  holder_id: HolderId;
  shares_exercised: number;
  strike_price_cents: number;
  total_cost_cents: number;
  exercise_date: string;
  exercise_type: ExerciseType;
  created_at: string;
}

export interface SafeNote {
  safe_note_id: SafeNoteId;
  entity_id: EntityId;
  investor_name: string;
  safe_type: "post_money" | "pre_money" | "mfn";
  investment_amount_cents: number;
  valuation_cap_cents: number | null;
  status: "issued" | "converted" | "cancelled";
  created_at: string;
}

export interface Valuation {
  valuation_id: ValuationId;
  entity_id: EntityId;
  valuation_type: string;
  methodology: string;
  valuation_amount_cents: number;
  effective_date: string;
  status: string;
  created_at: string;
}

export type HolderType = "individual" | "entity" | "trust";

export interface Holder {
  holder_id: HolderId;
  entity_id: EntityId;
  contact_id: ContactId | null;
  name: string;
  holder_type: HolderType;
  created_at: string;
}

export type TransferStatus = "draft" | "pending_board_approval" | "approved" | "executed" | "denied" | "cancelled";
export type TransferType = "secondary_sale" | "gift" | "trust_transfer" | "estate" | "other";

export interface ShareTransfer {
  transfer_id: TransferId;
  entity_id: EntityId;
  cap_table_id: CapTableId;
  from_holder_id: HolderId;
  to_holder_id: HolderId;
  instrument_id: InstrumentId;
  shares: number;
  transfer_type: TransferType;
  price_per_share_cents: number | null;
  status: TransferStatus;
  created_at: string;
}

export type PositionStatus = "active" | "closed";

export interface Position {
  position_id: string;
  entity_id: EntityId;
  holder_id: HolderId;
  instrument_id: InstrumentId;
  quantity_units: number;
  principal_cents: number;
  source_reference: string | null;
  status: PositionStatus;
  updated_at: string;
  created_at: string;
}

export type FundingRoundStatus = "term_sheet" | "diligence" | "closing" | "closed";

export interface FundingRound {
  round_id: FundingRoundId;
  entity_id: EntityId;
  cap_table_id: CapTableId;
  name: string;
  target_amount_cents: number;
  price_per_share_cents: number | null;
  status: FundingRoundStatus;
  created_at: string;
}

export type VestingScheduleId = string;
export type VestingEventId = string;

export interface VestingSchedule {
  schedule_id: VestingScheduleId;
  grant_id: EquityGrantId;
  entity_id: EntityId;
  total_shares: number;
  vesting_start_date: string;
  template: string;
  cliff_months: number;
  total_months: number;
  status: string;
  created_at: string;
}

export interface VestingEvent {
  event_id: VestingEventId;
  schedule_id: VestingScheduleId;
  event_date: string;
  shares: number;
  status: string;
}

// ── Governance ───────────────────────────────────────────────────────────────

export interface GovernanceBody {
  body_id: GovernanceBodyId;
  entity_id: EntityId;
  body_type: "board_of_directors" | "llc_member_vote";
  name: string;
  quorum_rule: "majority" | "supermajority" | "unanimous";
  voting_method: "per_capita" | "per_unit";
  status: "active" | "inactive";
  created_at: string;
}

export interface GovernanceSeat {
  seat_id: GovernanceSeatId;
  body_id: GovernanceBodyId;
  holder_id: ContactId;
  role: "chair" | "member" | "officer" | "observer";
  status: "active" | "resigned" | "expired";
  created_at: string;
}

export interface Meeting {
  meeting_id: MeetingId;
  body_id: GovernanceBodyId;
  meeting_type: "board_meeting" | "shareholder_meeting" | "written_consent" | "member_meeting";
  title: string;
  status: "draft" | "noticed" | "convened" | "adjourned" | "cancelled";
  created_at: string;
}

// ── Treasury ─────────────────────────────────────────────────────────────────

export interface Invoice {
  invoice_id: InvoiceId;
  entity_id: EntityId;
  customer_name: string;
  amount_cents: number;
  status: "draft" | "sent" | "paid" | "voided";
  created_at: string;
}

export interface Payment {
  payment_id: PaymentId;
  entity_id: EntityId;
  recipient_name: string;
  amount_cents: number;
  method: string;
  created_at: string;
}

// ── Contacts ─────────────────────────────────────────────────────────────────

export interface Contact {
  contact_id: ContactId;
  entity_id: EntityId;
  name: string;
  email: string | null;
  category: string;
  status: "active" | "inactive";
  created_at: string;
}

// ── Agents ───────────────────────────────────────────────────────────────────

export interface Agent {
  agent_id: AgentId;
  workspace_id: WorkspaceId;
  name: string;
  status: "active" | "inactive";
  created_at: string;
}

// ── Work Items ───────────────────────────────────────────────────────────────

export interface WorkItem {
  work_item_id: WorkItemId;
  entity_id: EntityId;
  title: string;
  status: "open" | "claimed" | "completed" | "cancelled";
  created_at: string;
}

// ── API responses ────────────────────────────────────────────────────────────

export interface ApiError {
  error: string;
}

export interface ApiKeyResponse {
  key_id: ApiKeyId;
  name: string;
  raw_key: string;
  scopes: string[];
}
