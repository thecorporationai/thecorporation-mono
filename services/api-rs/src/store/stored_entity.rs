//! `StoredEntity` trait — maps domain types to their git storage paths.
//!
//! Each domain type that follows the `{dir}/{id}.json` convention implements
//! this trait, enabling generic `read`, `list_ids`, and `write` methods on
//! [`EntityStore`] and eliminating per-type boilerplate.

use std::fmt;
use std::str::FromStr;

use serde::de::DeserializeOwned;
use serde::Serialize;

/// A domain type stored as `{storage_dir}/{id}.json` in a git repo.
pub trait StoredEntity: DeserializeOwned + Serialize {
    /// The ID type for this entity (e.g., `EquityGrantId`).
    type Id: fmt::Display + FromStr + Copy;

    /// The directory path where entities of this type are stored.
    fn storage_dir() -> &'static str;

    /// Full path to a specific entity's JSON file.
    fn storage_path(id: Self::Id) -> String {
        format!("{}/{}.json", Self::storage_dir(), id)
    }
}

// ── Equity ──────────────────────────────────────────────────────────────

use crate::domain::equity::funding_round::FundingRound;
use crate::domain::equity::grant::EquityGrant;
use crate::domain::equity::safe_note::SafeNote;
use crate::domain::equity::share_class::ShareClass;
use crate::domain::equity::transfer::ShareTransfer;
use crate::domain::equity::valuation::Valuation;
use crate::domain::ids::{
    AccountId, BankAccountId, ClassificationId, ContactId, ContractId, DeadlineId,
    DistributionId, EquityGrantId, FundingRoundId, GovernanceBodyId, GovernanceSeatId,
    IntentId, InvoiceId, JournalEntryId, MeetingId, ObligationId, PaymentId, PayrollRunId,
    ReceiptId, ReconciliationId, SafeNoteId, ShareClassId, TaxFilingId, TransferId,
    ValuationId,
};

impl StoredEntity for ShareClass {
    type Id = ShareClassId;
    fn storage_dir() -> &'static str { "cap-table/classes" }
}

impl StoredEntity for EquityGrant {
    type Id = EquityGrantId;
    fn storage_dir() -> &'static str { "cap-table/grants" }
}

impl StoredEntity for SafeNote {
    type Id = SafeNoteId;
    fn storage_dir() -> &'static str { "safe-notes" }
}

impl StoredEntity for Valuation {
    type Id = ValuationId;
    fn storage_dir() -> &'static str { "valuations" }
}

impl StoredEntity for ShareTransfer {
    type Id = TransferId;
    fn storage_dir() -> &'static str { "cap-table/transfers" }
}

impl StoredEntity for FundingRound {
    type Id = FundingRoundId;
    fn storage_dir() -> &'static str { "funding-rounds" }
}

// ── Governance ──────────────────────────────────────────────────────────

use crate::domain::governance::body::GovernanceBody;
use crate::domain::governance::meeting::Meeting;
use crate::domain::governance::seat::GovernanceSeat;

impl StoredEntity for GovernanceBody {
    type Id = GovernanceBodyId;
    fn storage_dir() -> &'static str { "governance/bodies" }
}

impl StoredEntity for GovernanceSeat {
    type Id = GovernanceSeatId;
    fn storage_dir() -> &'static str { "governance/seats" }
}

impl StoredEntity for Meeting {
    type Id = MeetingId;
    fn storage_dir() -> &'static str { "governance/meetings" }
    fn storage_path(id: Self::Id) -> String {
        format!("governance/meetings/{}/meeting.json", id)
    }
}

// ── Treasury ────────────────────────────────────────────────────────────

use crate::domain::treasury::account::Account;
use crate::domain::treasury::bank_account::BankAccount;
use crate::domain::treasury::distribution::Distribution;
use crate::domain::treasury::invoice::Invoice;
use crate::domain::treasury::journal_entry::JournalEntry;
use crate::domain::treasury::payment::Payment;
use crate::domain::treasury::payroll::PayrollRun;
use crate::domain::treasury::reconciliation::Reconciliation;

impl StoredEntity for Account {
    type Id = AccountId;
    fn storage_dir() -> &'static str { "treasury/accounts" }
}

impl StoredEntity for JournalEntry {
    type Id = JournalEntryId;
    fn storage_dir() -> &'static str { "treasury/journal-entries" }
}

impl StoredEntity for Invoice {
    type Id = InvoiceId;
    fn storage_dir() -> &'static str { "treasury/invoices" }
}

impl StoredEntity for BankAccount {
    type Id = BankAccountId;
    fn storage_dir() -> &'static str { "treasury/bank-accounts" }
}

impl StoredEntity for Payment {
    type Id = PaymentId;
    fn storage_dir() -> &'static str { "treasury/payments" }
}

impl StoredEntity for PayrollRun {
    type Id = PayrollRunId;
    fn storage_dir() -> &'static str { "treasury/payroll" }
}

impl StoredEntity for Distribution {
    type Id = DistributionId;
    fn storage_dir() -> &'static str { "treasury/distributions" }
}

impl StoredEntity for Reconciliation {
    type Id = ReconciliationId;
    fn storage_dir() -> &'static str { "treasury/reconciliations" }
}

// ── Execution ───────────────────────────────────────────────────────────

use crate::domain::execution::intent::Intent;
use crate::domain::execution::obligation::Obligation;
use crate::domain::execution::receipt::Receipt;

impl StoredEntity for Intent {
    type Id = IntentId;
    fn storage_dir() -> &'static str { "execution/intents" }
}

impl StoredEntity for Obligation {
    type Id = ObligationId;
    fn storage_dir() -> &'static str { "execution/obligations" }
}

impl StoredEntity for Receipt {
    type Id = ReceiptId;
    fn storage_dir() -> &'static str { "execution/receipts" }
}

// ── Contacts ────────────────────────────────────────────────────────────

use crate::domain::contacts::contact::Contact;

impl StoredEntity for Contact {
    type Id = ContactId;
    fn storage_dir() -> &'static str { "contacts" }
}

// ── Formation: Contracts, Tax Filings, Deadlines, Contractors ───────────

use crate::domain::formation::contract::Contract;
use crate::domain::formation::contractor::ContractorClassification;
use crate::domain::formation::deadline::Deadline;
use crate::domain::formation::tax_filing::TaxFiling;

impl StoredEntity for Contract {
    type Id = ContractId;
    fn storage_dir() -> &'static str { "contracts" }
}

impl StoredEntity for TaxFiling {
    type Id = TaxFilingId;
    fn storage_dir() -> &'static str { "tax/filings" }
}

impl StoredEntity for Deadline {
    type Id = DeadlineId;
    fn storage_dir() -> &'static str { "deadlines" }
}

impl StoredEntity for ContractorClassification {
    type Id = ClassificationId;
    fn storage_dir() -> &'static str { "contractors" }
}
