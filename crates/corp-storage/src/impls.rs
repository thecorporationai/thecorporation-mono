//! [`StoredEntity`] implementations for all `corp-core` domain types.
//!
//! Centralising these impls here avoids the Rust orphan rule: `StoredEntity`
//! is defined in this crate (`corp-storage`) so it may be implemented for
//! types defined in `corp-core`.
//!
//! Route handlers import the trait and call `store.read::<Entity>(…)` /
//! `store.write::<Entity>(…)` without needing to repeat these declarations in
//! every domain module.

use corp_core::agents::Agent;
use corp_core::contacts::Contact;
use corp_core::equity::{
    CapTable, ControlLink, EquityGrant, EquityRuleSet, FundingRound, Holder, Instrument,
    InvestorLedgerEntry, LegalEntity, Position, RepurchaseRight, SafeNote, ShareClass,
    ShareTransfer, Valuation, VestingEvent, VestingSchedule,
};
use corp_core::execution::{Intent, Obligation, Receipt};
use corp_core::formation::{Document, Entity, Filing, TaxProfile};
use corp_core::governance::{
    AgendaItem, GovernanceBody, GovernanceSeat, Meeting, Resolution, Vote,
};
use corp_core::ids::{
    AccountId, AgendaItemId, AgentId, BankAccountId, CapTableId, ContactId, ControlLinkId,
    DocumentId, EntityId, EquityGrantId, EquityRuleSetId, FilingId, FundingRoundId,
    GovernanceBodyId, GovernanceSeatId, HolderId, InstrumentId, IntentId, InvestorLedgerEntryId,
    InvoiceId, JournalEntryId, LegalEntityId, MeetingId, ObligationId, PaymentId, PayrollRunId,
    PositionId, ReceiptId, ReconciliationId, RepurchaseRightId, ResolutionId, SafeNoteId,
    ServiceRequestId, ShareClassId, TaxProfileId, TransferId, ValuationId, VestingEventId,
    VestingScheduleId, VoteId, WorkItemId,
};
use corp_core::services::ServiceRequest;
use corp_core::treasury::{
    Account, BankAccount, Invoice, JournalEntry, Payment, PayrollRun, Reconciliation,
};
use corp_core::work_items::WorkItem;

use crate::traits::StoredEntity;

// ── Formation ─────────────────────────────────────────────────────────────────

impl StoredEntity for Entity {
    type Id = EntityId;
    fn storage_dir() -> &'static str {
        "formation/entity"
    }
}

impl StoredEntity for Document {
    type Id = DocumentId;
    fn storage_dir() -> &'static str {
        "formation/documents"
    }
}

impl StoredEntity for Filing {
    type Id = FilingId;
    fn storage_dir() -> &'static str {
        "formation/filings"
    }
}

impl StoredEntity for TaxProfile {
    type Id = TaxProfileId;
    fn storage_dir() -> &'static str {
        "formation/tax"
    }
}

// ── Contacts ──────────────────────────────────────────────────────────────────

impl StoredEntity for Contact {
    type Id = ContactId;
    fn storage_dir() -> &'static str {
        "contacts"
    }
}

// ── Equity ────────────────────────────────────────────────────────────────────

impl StoredEntity for CapTable {
    type Id = CapTableId;
    fn storage_dir() -> &'static str {
        "equity/cap_tables"
    }
}

impl StoredEntity for ShareClass {
    type Id = ShareClassId;
    fn storage_dir() -> &'static str {
        "equity/share_classes"
    }
}

impl StoredEntity for EquityGrant {
    type Id = EquityGrantId;
    fn storage_dir() -> &'static str {
        "equity/grants"
    }
}

impl StoredEntity for SafeNote {
    type Id = SafeNoteId;
    fn storage_dir() -> &'static str {
        "equity/safes"
    }
}

impl StoredEntity for Valuation {
    type Id = ValuationId;
    fn storage_dir() -> &'static str {
        "equity/valuations"
    }
}

impl StoredEntity for ShareTransfer {
    type Id = TransferId;
    fn storage_dir() -> &'static str {
        "equity/transfers"
    }
}

impl StoredEntity for FundingRound {
    type Id = FundingRoundId;
    fn storage_dir() -> &'static str {
        "equity/rounds"
    }
}

impl StoredEntity for Holder {
    type Id = HolderId;
    fn storage_dir() -> &'static str {
        "equity/holders"
    }
}

impl StoredEntity for VestingSchedule {
    type Id = VestingScheduleId;
    fn storage_dir() -> &'static str {
        "equity/vesting_schedules"
    }
}

impl StoredEntity for VestingEvent {
    type Id = VestingEventId;
    fn storage_dir() -> &'static str {
        "equity/vesting_events"
    }
}

impl StoredEntity for Instrument {
    type Id = InstrumentId;
    fn storage_dir() -> &'static str {
        "equity/instruments"
    }
}

impl StoredEntity for Position {
    type Id = PositionId;
    fn storage_dir() -> &'static str {
        "equity/positions"
    }
}

impl StoredEntity for InvestorLedgerEntry {
    type Id = InvestorLedgerEntryId;
    fn storage_dir() -> &'static str {
        "equity/investor_ledger"
    }
}

impl StoredEntity for LegalEntity {
    type Id = LegalEntityId;
    fn storage_dir() -> &'static str {
        "equity/legal_entities"
    }
}

impl StoredEntity for ControlLink {
    type Id = ControlLinkId;
    fn storage_dir() -> &'static str {
        "equity/control_links"
    }
}

impl StoredEntity for EquityRuleSet {
    type Id = EquityRuleSetId;
    fn storage_dir() -> &'static str {
        "equity/rule_sets"
    }
}

impl StoredEntity for RepurchaseRight {
    type Id = RepurchaseRightId;
    fn storage_dir() -> &'static str {
        "equity/repurchase_rights"
    }
}

// ── Execution ─────────────────────────────────────────────────────────────────

impl StoredEntity for Intent {
    type Id = IntentId;
    fn storage_dir() -> &'static str {
        "execution/intents"
    }
}

impl StoredEntity for Obligation {
    type Id = ObligationId;
    fn storage_dir() -> &'static str {
        "execution/obligations"
    }
}

impl StoredEntity for Receipt {
    type Id = ReceiptId;
    fn storage_dir() -> &'static str {
        "execution/receipts"
    }
}

// ── Agents ────────────────────────────────────────────────────────────────────

impl StoredEntity for Agent {
    type Id = AgentId;
    fn storage_dir() -> &'static str {
        "agents"
    }
}

// ── Governance ────────────────────────────────────────────────────────────────

impl StoredEntity for GovernanceBody {
    type Id = GovernanceBodyId;
    fn storage_dir() -> &'static str {
        "governance/bodies"
    }
}

impl StoredEntity for GovernanceSeat {
    type Id = GovernanceSeatId;
    fn storage_dir() -> &'static str {
        "governance/seats"
    }
}

impl StoredEntity for Meeting {
    type Id = MeetingId;
    fn storage_dir() -> &'static str {
        "governance/meetings"
    }
}

impl StoredEntity for AgendaItem {
    type Id = AgendaItemId;
    fn storage_dir() -> &'static str {
        "governance/agenda_items"
    }
}

impl StoredEntity for Vote {
    type Id = VoteId;
    fn storage_dir() -> &'static str {
        "governance/votes"
    }
}

impl StoredEntity for Resolution {
    type Id = ResolutionId;
    fn storage_dir() -> &'static str {
        "governance/resolutions"
    }
}

// ── Treasury ──────────────────────────────────────────────────────────────────

impl StoredEntity for Account {
    type Id = AccountId;
    fn storage_dir() -> &'static str {
        "treasury/accounts"
    }
}

impl StoredEntity for JournalEntry {
    type Id = JournalEntryId;
    fn storage_dir() -> &'static str {
        "treasury/journal_entries"
    }
}

impl StoredEntity for Invoice {
    type Id = InvoiceId;
    fn storage_dir() -> &'static str {
        "treasury/invoices"
    }
}

impl StoredEntity for Payment {
    type Id = PaymentId;
    fn storage_dir() -> &'static str {
        "treasury/payments"
    }
}

impl StoredEntity for BankAccount {
    type Id = BankAccountId;
    fn storage_dir() -> &'static str {
        "treasury/bank_accounts"
    }
}

impl StoredEntity for PayrollRun {
    type Id = PayrollRunId;
    fn storage_dir() -> &'static str {
        "treasury/payroll_runs"
    }
}

impl StoredEntity for Reconciliation {
    type Id = ReconciliationId;
    fn storage_dir() -> &'static str {
        "treasury/reconciliations"
    }
}

// ── Work items ────────────────────────────────────────────────────────────────

impl StoredEntity for WorkItem {
    type Id = WorkItemId;
    fn storage_dir() -> &'static str {
        "work_items"
    }
}

// ── Services ──────────────────────────────────────────────────────────────────

impl StoredEntity for ServiceRequest {
    type Id = ServiceRequestId;
    fn storage_dir() -> &'static str {
        "services/requests"
    }
}
