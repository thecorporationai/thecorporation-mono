// AUTO-GENERATED from OpenAPI spec — do not edit
// Regenerate with: npm run generate:types

export const AccountType = ["asset","liability","equity","revenue","expense"] as const;
export type AccountType = (typeof AccountType)[number];

export const AgendaItemStatus = ["pending","discussed","voted","tabled","withdrawn"] as const;
export type AgendaItemStatus = (typeof AgendaItemStatus)[number];

export const AgendaItemType = ["resolution","discussion","report","election"] as const;
export type AgendaItemType = (typeof AgendaItemType)[number];

export const AgentStatus = ["active","paused","disabled"] as const;
export type AgentStatus = (typeof AgentStatus)[number];

export const AntiDilutionMethod = ["none","broad_based_weighted_average","narrow_based_weighted_average","full_ratchet"] as const;
export type AntiDilutionMethod = (typeof AntiDilutionMethod)[number];

export const AssigneeType = ["internal","third_party","human"] as const;
export type AssigneeType = (typeof AssigneeType)[number];

export const AuthoritySource = ["law","charter","governance_docs","resolution","directive","standing_instruction","delegation_schedule","heuristic"] as const;
export type AuthoritySource = (typeof AuthoritySource)[number];

export const AuthorityTier = ["tier_1","tier_2","tier_3"] as const;
export type AuthorityTier = (typeof AuthorityTier)[number];

export const BankAccountStatus = ["pending_review","active","closed"] as const;
export type BankAccountStatus = (typeof BankAccountStatus)[number];

export const BankAccountType = ["checking","savings"] as const;
export type BankAccountType = (typeof BankAccountType)[number];

export const BodyStatus = ["active","inactive"] as const;
export type BodyStatus = (typeof BodyStatus)[number];

export const BodyType = ["board_of_directors","llc_member_vote"] as const;
export type BodyType = (typeof BodyType)[number];

export const CapTableAccess = ["none","summary","detailed"] as const;
export type CapTableAccess = (typeof CapTableAccess)[number];

export const CapTableBasis = ["outstanding","as_converted","fully_diluted"] as const;
export type CapTableBasis = (typeof CapTableBasis)[number];

export const ClassificationResult = ["independent","employee","uncertain"] as const;
export type ClassificationResult = (typeof ClassificationResult)[number];

export const ContactCategory = ["employee","contractor","board_member","law_firm","valuation_firm","accounting_firm","investor","officer","founder","member","other"] as const;
export type ContactCategory = (typeof ContactCategory)[number];

export const ContactStatus = ["active","inactive"] as const;
export type ContactStatus = (typeof ContactStatus)[number];

export const ContactType = ["individual","organization"] as const;
export type ContactType = (typeof ContactType)[number];

export const ContractStatus = ["draft","active","expired","terminated"] as const;
export type ContractStatus = (typeof ContractStatus)[number];

export const ContractTemplateType = ["consulting_agreement","employment_offer","contractor_agreement","nda","safe_agreement","custom"] as const;
export type ContractTemplateType = (typeof ContractTemplateType)[number];

export const ControlType = ["voting","board","economic","contractual"] as const;
export type ControlType = (typeof ControlType)[number];

export const Currency = ["usd"] as const;
export type Currency = (typeof Currency)[number];

export const DeadlineSeverity = ["low","medium","high","critical"] as const;
export type DeadlineSeverity = (typeof DeadlineSeverity)[number];

export const DeadlineStatus = ["upcoming","due","completed","overdue"] as const;
export type DeadlineStatus = (typeof DeadlineStatus)[number];

export const DistributionStatus = ["pending","approved","distributed"] as const;
export type DistributionStatus = (typeof DistributionStatus)[number];

export const DistributionType = ["dividend","return","liquidation"] as const;
export type DistributionType = (typeof DistributionType)[number];

export const DocumentRequestStatus = ["requested","provided","not_applicable","waived"] as const;
export type DocumentRequestStatus = (typeof DocumentRequestStatus)[number];

export const DocumentStatus = ["draft","signed","amended","filed"] as const;
export type DocumentStatus = (typeof DocumentStatus)[number];

export const DocumentType = ["articles_of_incorporation","articles_of_organization","bylaws","incorporator_action","initial_board_consent","operating_agreement","initial_written_consent","ss4_application","meeting_notice","resolution","consulting_agreement","employment_offer_letter","contractor_services_agreement","mutual_nondisclosure_agreement","safe_agreement","four_oh_nine_a_valuation_report","stock_transfer_agreement","transfer_board_consent","financing_board_consent","equity_issuance_approval","subscription_agreement","investor_rights_agreement","restricted_stock_purchase_agreement","ip_assignment_agreement","contract"] as const;
export type DocumentType = (typeof DocumentType)[number];

export const EntityType = ["c_corp","llc"] as const;
export type EntityType = (typeof EntityType)[number];

export const EquityRoundStatus = ["draft","open","board_approved","accepted","closed","cancelled"] as const;
export type EquityRoundStatus = (typeof EquityRoundStatus)[number];

export const EscalationStatus = ["open","resolved"] as const;
export type EscalationStatus = (typeof EscalationStatus)[number];

export const FormationState = ["forming","active"] as const;
export type FormationState = (typeof FormationState)[number];

export const FormationStatus = ["pending","documents_generated","documents_signed","filing_submitted","filed","ein_applied","active","rejected","dissolved"] as const;
export type FormationStatus = (typeof FormationStatus)[number];

export const GlAccountCode = ["Cash","AccountsReceivable","AccountsPayable","AccruedExpenses","FounderCapital","Revenue","OperatingExpenses","Cogs"] as const;
export type GlAccountCode = (typeof GlAccountCode)[number];

export const GovernanceAuditEventType = ["mode_changed","lockdown_trigger_applied","manual_event","checkpoint_written","chain_verified","chain_verification_failed"] as const;
export type GovernanceAuditEventType = (typeof GovernanceAuditEventType)[number];

export const GovernanceMode = ["normal","principal_unavailable","incident_lockdown"] as const;
export type GovernanceMode = (typeof GovernanceMode)[number];

export const GovernanceTriggerSource = ["compliance_scanner","execution_gate","external_ingestion"] as const;
export type GovernanceTriggerSource = (typeof GovernanceTriggerSource)[number];

export const GovernanceTriggerType = ["external_signal","policy_evidence_mismatch","compliance_deadline_missed_d_plus_1","audit_chain_verification_failed"] as const;
export type GovernanceTriggerType = (typeof GovernanceTriggerType)[number];

export const GoverningDocType = ["bylaws","operating_agreement","shareholder_agreement","other"] as const;
export type GoverningDocType = (typeof GoverningDocType)[number];

export const GrantType = ["common_stock","preferred_stock","membership_unit","stock_option","iso","nso","rsa","svu"] as const;
export type GrantType = (typeof GrantType)[number];

export const HolderType = ["individual","organization","fund","nonprofit","trust","other"] as const;
export type HolderType = (typeof HolderType)[number];

export const HttpMethod = ["GET","POST","PUT","PATCH","DELETE","HEAD","OPTIONS"] as const;
export type HttpMethod = (typeof HttpMethod)[number];

export const IncidentSeverity = ["low","medium","high","critical"] as const;
export type IncidentSeverity = (typeof IncidentSeverity)[number];

export const IncidentStatus = ["open","resolved"] as const;
export type IncidentStatus = (typeof IncidentStatus)[number];

export const InstrumentKind = ["common_equity","preferred_equity","membership_unit","option_grant","safe","convertible_note","warrant"] as const;
export type InstrumentKind = (typeof InstrumentKind)[number];

export const InstrumentStatus = ["active","closed","cancelled"] as const;
export type InstrumentStatus = (typeof InstrumentStatus)[number];

export const IntentStatus = ["pending","evaluated","authorized","executed","failed","cancelled"] as const;
export type IntentStatus = (typeof IntentStatus)[number];

export const InvestorType = ["natural_person","agent","entity"] as const;
export type InvestorType = (typeof InvestorType)[number];

export const InvoiceStatus = ["draft","sent","paid","voided"] as const;
export type InvoiceStatus = (typeof InvoiceStatus)[number];

export const JournalEntryStatus = ["draft","posted","voided"] as const;
export type JournalEntryStatus = (typeof JournalEntryStatus)[number];

export const LegalEntityRole = ["operating","control","investment","nonprofit","spv","other"] as const;
export type LegalEntityRole = (typeof LegalEntityRole)[number];

export const MeetingStatus = ["draft","noticed","convened","adjourned","cancelled"] as const;
export type MeetingStatus = (typeof MeetingStatus)[number];

export const MeetingType = ["board_meeting","shareholder_meeting","written_consent","member_meeting"] as const;
export type MeetingType = (typeof MeetingType)[number];

export const MemberRole = ["director","officer","manager","member","chair"] as const;
export type MemberRole = (typeof MemberRole)[number];

export const NetworkEgress = ["restricted","open"] as const;
export type NetworkEgress = (typeof NetworkEgress)[number];

export const ObligationStatus = ["required","in_progress","fulfilled","waived","expired"] as const;
export type ObligationStatus = (typeof ObligationStatus)[number];

export const OfficerTitle = ["ceo","cfo","cto","coo","secretary","treasurer","president","vp","other"] as const;
export type OfficerTitle = (typeof OfficerTitle)[number];

export const PaymentMethod = ["bank_transfer","card","check","wire","ach"] as const;
export type PaymentMethod = (typeof PaymentMethod)[number];

export const PaymentStatus = ["submitted","processing","completed","failed"] as const;
export type PaymentStatus = (typeof PaymentStatus)[number];

export const PayrollStatus = ["pending","processing","completed"] as const;
export type PayrollStatus = (typeof PayrollStatus)[number];

export const PositionStatus = ["active","closed"] as const;
export type PositionStatus = (typeof PositionStatus)[number];

export const QuorumStatus = ["unknown","met","not_met"] as const;
export type QuorumStatus = (typeof QuorumStatus)[number];

export const QuorumThreshold = ["majority","supermajority","unanimous"] as const;
export type QuorumThreshold = (typeof QuorumThreshold)[number];

export const ReceiptStatus = ["pending","executed","failed"] as const;
export type ReceiptStatus = (typeof ReceiptStatus)[number];

export const ReconciliationStatus = ["balanced","discrepancy"] as const;
export type ReconciliationStatus = (typeof ReconciliationStatus)[number];

export const Recurrence = ["one_time","monthly","quarterly","annual"] as const;
export type Recurrence = (typeof Recurrence)[number];

export const ReferenceKind = ["entity","contact","share_transfer","invoice","bank_account","payment","payroll_run","distribution","reconciliation","tax_filing","deadline","classification","body","meeting","seat","agenda_item","resolution","document","work_item","agent","valuation","safe_note","instrument","share_class","round"] as const;
export type ReferenceKind = (typeof ReferenceKind)[number];

export const ResolutionType = ["ordinary","special","unanimous_written_consent"] as const;
export type ResolutionType = (typeof ResolutionType)[number];

export const RiskLevel = ["low","medium","high"] as const;
export type RiskLevel = (typeof RiskLevel)[number];

export const SafeStatus = ["issued","converted","cancelled"] as const;
export type SafeStatus = (typeof SafeStatus)[number];

export const SafeType = ["post_money","pre_money","mfn"] as const;
export type SafeType = (typeof SafeType)[number];

export const Scope = ["formation_create","formation_read","formation_sign","equity_read","equity_write","equity_transfer","governance_read","governance_write","governance_vote","treasury_read","treasury_write","treasury_approve","contacts_read","contacts_write","execution_read","execution_write","branch_create","branch_merge","branch_delete","admin","internal_worker_read","internal_worker_write","secrets_manage","all"] as const;
export type Scope = (typeof Scope)[number];

export const SeatRole = ["chair","member","officer","observer"] as const;
export type SeatRole = (typeof SeatRole)[number];

export const SeatStatus = ["active","resigned","expired"] as const;
export type SeatStatus = (typeof SeatStatus)[number];

export const Side = ["debit","credit"] as const;
export type Side = (typeof Side)[number];

export const TaxFilingStatus = ["pending","filed","accepted","rejected"] as const;
export type TaxFilingStatus = (typeof TaxFilingStatus)[number];

export const TransactionPacketStatus = ["drafted","ready_for_signature","fully_signed","executable","executed","failed"] as const;
export type TransactionPacketStatus = (typeof TransactionPacketStatus)[number];

export const TransferStatus = ["draft","pending_bylaws_review","pending_rofr","pending_board_approval","approved","executed","denied","cancelled"] as const;
export type TransferStatus = (typeof TransferStatus)[number];

export const TransferType = ["gift","trust_transfer","secondary_sale","estate","other"] as const;
export type TransferType = (typeof TransferType)[number];

export const TransfereeRights = ["full_member","economic_only","limited"] as const;
export type TransfereeRights = (typeof TransfereeRights)[number];

export const Transport = ["stdio","http"] as const;
export type Transport = (typeof Transport)[number];

export const ValuationMethodology = ["income","market","asset","backsolve","hybrid","other"] as const;
export type ValuationMethodology = (typeof ValuationMethodology)[number];

export const ValuationStatus = ["draft","pending_approval","approved","expired","superseded"] as const;
export type ValuationStatus = (typeof ValuationStatus)[number];

export const ValuationType = ["four_oh_nine_a","llc_profits_interest","fair_market_value","gift","estate","other"] as const;
export type ValuationType = (typeof ValuationType)[number];

export const VoteValue = ["for","against","abstain","recusal"] as const;
export type VoteValue = (typeof VoteValue)[number];

export const VotingMethod = ["per_capita","per_unit"] as const;
export type VotingMethod = (typeof VotingMethod)[number];

export const WorkItemActorTypeValue = ["contact","agent"] as const;
export type WorkItemActorTypeValue = (typeof WorkItemActorTypeValue)[number];

export const WorkItemStatus = ["open","claimed","completed","cancelled"] as const;
export type WorkItemStatus = (typeof WorkItemStatus)[number];

export const WorkflowType = ["transfer","fundraising"] as const;
export type WorkflowType = (typeof WorkflowType)[number];
