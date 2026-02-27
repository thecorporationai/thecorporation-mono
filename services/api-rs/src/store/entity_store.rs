//! Entity store — reads and writes entity data to git repos.

use serde::Serialize;

use crate::domain::contacts::contact::Contact;
use crate::domain::formation::contract::Contract;
use crate::domain::formation::contractor::ContractorClassification;
use crate::domain::formation::deadline::Deadline;
use crate::domain::formation::tax_filing::TaxFiling;
use crate::domain::treasury::distribution::Distribution;
use crate::domain::treasury::payment::Payment;
use crate::domain::treasury::payroll::PayrollRun;
use crate::domain::treasury::reconciliation::Reconciliation;
use crate::domain::equity::{
    cap_table::CapTable,
    funding_round::FundingRound,
    grant::EquityGrant,
    safe_note::SafeNote,
    share_class::ShareClass,
    transfer::ShareTransfer,
    valuation::Valuation,
};
use crate::domain::execution::{
    intent::Intent,
    obligation::Obligation,
    receipt::Receipt,
};
use crate::domain::formation::{
    document::Document, entity::Entity, filing::Filing, tax_profile::TaxProfile,
};
use crate::domain::governance::{
    agenda_item::AgendaItem,
    body::GovernanceBody,
    meeting::Meeting,
    resolution::Resolution,
    seat::GovernanceSeat,
    vote::Vote,
};
use crate::domain::ids::{
    AccountId, AgendaItemId, BankAccountId, ClassificationId, ContactId, ContractId, DeadlineId,
    DistributionId, DocumentId, EntityId, EquityGrantId, FundingRoundId, GovernanceBodyId,
    GovernanceSeatId, IntentId, InvoiceId, JournalEntryId, MeetingId, ObligationId, PaymentId,
    PayrollRunId, ReceiptId, ReconciliationId, ResolutionId, SafeNoteId, ShareClassId,
    TaxFilingId, TransferId, ValuationId, VoteId, WorkspaceId,
};
use crate::domain::treasury::{
    account::Account,
    bank_account::BankAccount,
    invoice::Invoice,
    journal_entry::JournalEntry,
};
use crate::git::commit::{commit_files, FileWrite};
use crate::git::error::GitStorageError;
use crate::git::repo::CorpRepo;

use super::RepoLayout;

/// Operations on a single entity's git repo.
pub struct EntityStore<'a> {
    repo: CorpRepo,
    layout: &'a RepoLayout,
}

impl<'a> EntityStore<'a> {
    /// Initialize a new entity repo with the initial entity record.
    pub fn init(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
        entity: &Entity,
    ) -> Result<Self, GitStorageError> {
        let path = layout.entity_repo_path(workspace_id, entity_id);
        let repo = CorpRepo::init(&path)?;
        let files = vec![FileWrite::json("corp.json", entity)?];
        commit_files(&repo, "main", "Initialize entity", &files)?;
        Ok(Self { repo, layout })
    }

    /// Open an existing entity repo.
    pub fn open(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
    ) -> Result<Self, GitStorageError> {
        let path = layout.entity_repo_path(workspace_id, entity_id);
        let repo = CorpRepo::open(&path)?;
        Ok(Self { repo, layout })
    }

    /// Read the entity record (corp.json) from a branch.
    pub fn read_entity(&self, branch: &str) -> Result<Entity, GitStorageError> {
        self.repo.read_json(branch, "corp.json")
    }

    /// Write the entity record.
    pub fn write_entity(
        &self,
        branch: &str,
        entity: &Entity,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let files = vec![FileWrite::json("corp.json", entity)?];
        commit_files(&self.repo, branch, message, &files)?;
        Ok(())
    }

    /// Read a document.
    pub fn read_document(
        &self,
        branch: &str,
        doc_id: DocumentId,
    ) -> Result<Document, GitStorageError> {
        self.repo
            .read_json(branch, &format!("formation/{}.json", doc_id))
    }

    /// Write a document.
    pub fn write_document(
        &self,
        branch: &str,
        doc: &Document,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let path = format!("formation/{}.json", doc.document_id());
        let files = vec![FileWrite::json(path, doc)?];
        commit_files(&self.repo, branch, message, &files)?;
        Ok(())
    }

    /// Write multiple files atomically.
    pub fn commit(
        &self,
        branch: &str,
        message: &str,
        files: Vec<FileWrite>,
    ) -> Result<(), GitStorageError> {
        commit_files(&self.repo, branch, message, &files)?;
        Ok(())
    }

    /// Read filing record.
    pub fn read_filing(&self, branch: &str) -> Result<Filing, GitStorageError> {
        self.repo.read_json(branch, "formation/filing.json")
    }

    /// Read tax profile.
    pub fn read_tax_profile(&self, branch: &str) -> Result<TaxProfile, GitStorageError> {
        self.repo.read_json(branch, "tax/profile.json")
    }

    /// List all document IDs in the formation/ directory.
    pub fn list_document_ids(&self, branch: &str) -> Result<Vec<DocumentId>, GitStorageError> {
        let entries = self.repo.list_dir(branch, "formation")?;
        let mut ids = Vec::new();
        for (name, is_dir) in entries {
            if is_dir {
                continue;
            }
            if name == "filing.json" {
                continue;
            }
            // Parse "{uuid}.json" -> DocumentId
            if let Some(uuid_str) = name.strip_suffix(".json") {
                if let Ok(id) = uuid_str.parse() {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
    }

    /// Get the underlying repo for advanced operations.
    pub fn repo(&self) -> &CorpRepo {
        &self.repo
    }

    /// Get the layout reference.
    pub fn layout(&self) -> &RepoLayout {
        self.layout
    }

    // ── Equity: Cap table ────────────────────────────────────────────

    /// Read the cap table record.
    pub fn read_cap_table(&self, branch: &str) -> Result<CapTable, GitStorageError> {
        self.repo.read_json(branch, "cap-table/cap-table.json")
    }

    // ── Equity: Share classes ────────────────────────────────────────

    /// Read a share class by ID.
    pub fn read_share_class(
        &self,
        branch: &str,
        id: ShareClassId,
    ) -> Result<ShareClass, GitStorageError> {
        self.repo
            .read_json(branch, &format!("cap-table/classes/{}.json", id))
    }

    /// List all share class IDs.
    pub fn list_share_class_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<ShareClassId>, GitStorageError> {
        self.list_ids_in_dir(branch, "cap-table/classes")
    }

    // ── Equity: Grants ───────────────────────────────────────────────

    /// Read an equity grant by ID.
    pub fn read_grant(
        &self,
        branch: &str,
        id: EquityGrantId,
    ) -> Result<EquityGrant, GitStorageError> {
        self.repo
            .read_json(branch, &format!("cap-table/grants/{}.json", id))
    }

    /// List all grant IDs.
    pub fn list_grant_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<EquityGrantId>, GitStorageError> {
        self.list_ids_in_dir(branch, "cap-table/grants")
    }

    // ── Equity: SAFE notes ───────────────────────────────────────────

    /// Read a SAFE note by ID.
    pub fn read_safe_note(
        &self,
        branch: &str,
        id: SafeNoteId,
    ) -> Result<SafeNote, GitStorageError> {
        self.repo
            .read_json(branch, &format!("safe-notes/{}.json", id))
    }

    /// List all SAFE note IDs.
    pub fn list_safe_note_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<SafeNoteId>, GitStorageError> {
        self.list_ids_in_dir(branch, "safe-notes")
    }

    // ── Equity: Valuations ───────────────────────────────────────────

    /// Read a valuation by ID.
    pub fn read_valuation(
        &self,
        branch: &str,
        id: ValuationId,
    ) -> Result<Valuation, GitStorageError> {
        self.repo
            .read_json(branch, &format!("valuations/{}.json", id))
    }

    /// List all valuation IDs.
    pub fn list_valuation_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<ValuationId>, GitStorageError> {
        self.list_ids_in_dir(branch, "valuations")
    }

    // ── Equity: Transfers ────────────────────────────────────────────

    /// Read a share transfer by ID.
    pub fn read_transfer(
        &self,
        branch: &str,
        id: TransferId,
    ) -> Result<ShareTransfer, GitStorageError> {
        self.repo
            .read_json(branch, &format!("cap-table/transfers/{}.json", id))
    }

    /// List all transfer IDs.
    pub fn list_transfer_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<TransferId>, GitStorageError> {
        self.list_ids_in_dir(branch, "cap-table/transfers")
    }

    // ── Equity: Funding rounds ───────────────────────────────────────

    /// Read a funding round by ID.
    pub fn read_funding_round(
        &self,
        branch: &str,
        id: FundingRoundId,
    ) -> Result<FundingRound, GitStorageError> {
        self.repo
            .read_json(branch, &format!("funding-rounds/{}.json", id))
    }

    /// List all funding round IDs.
    pub fn list_funding_round_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<FundingRoundId>, GitStorageError> {
        self.list_ids_in_dir(branch, "funding-rounds")
    }

    // ── Governance: Bodies ─────────────────────────────────────────────

    /// Read a governance body by ID.
    pub fn read_governance_body(
        &self,
        branch: &str,
        id: GovernanceBodyId,
    ) -> Result<GovernanceBody, GitStorageError> {
        self.repo
            .read_json(branch, &format!("governance/bodies/{}.json", id))
    }

    /// List all governance body IDs.
    pub fn list_governance_body_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<GovernanceBodyId>, GitStorageError> {
        self.list_ids_in_dir(branch, "governance/bodies")
    }

    // ── Governance: Seats ──────────────────────────────────────────────

    /// Read a governance seat by ID.
    pub fn read_governance_seat(
        &self,
        branch: &str,
        id: GovernanceSeatId,
    ) -> Result<GovernanceSeat, GitStorageError> {
        self.repo
            .read_json(branch, &format!("governance/seats/{}.json", id))
    }

    /// List all governance seat IDs.
    pub fn list_governance_seat_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<GovernanceSeatId>, GitStorageError> {
        self.list_ids_in_dir(branch, "governance/seats")
    }

    // ── Governance: Meetings ───────────────────────────────────────────

    /// Read a meeting by ID.
    pub fn read_meeting(
        &self,
        branch: &str,
        id: MeetingId,
    ) -> Result<Meeting, GitStorageError> {
        self.repo
            .read_json(branch, &format!("governance/meetings/{}/meeting.json", id))
    }

    /// List all meeting IDs.
    pub fn list_meeting_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<MeetingId>, GitStorageError> {
        self.list_ids_in_dir(branch, "governance/meetings")
    }

    // ── Governance: Agenda items ───────────────────────────────────────

    /// Read an agenda item from a meeting.
    pub fn read_agenda_item(
        &self,
        branch: &str,
        meeting_id: MeetingId,
        id: AgendaItemId,
    ) -> Result<AgendaItem, GitStorageError> {
        self.repo.read_json(
            branch,
            &format!("governance/meetings/{}/agenda/{}.json", meeting_id, id),
        )
    }

    /// List all agenda item IDs for a meeting.
    pub fn list_agenda_item_ids(
        &self,
        branch: &str,
        meeting_id: MeetingId,
    ) -> Result<Vec<AgendaItemId>, GitStorageError> {
        self.list_ids_in_dir(
            branch,
            &format!("governance/meetings/{}/agenda", meeting_id),
        )
    }

    // ── Governance: Votes ──────────────────────────────────────────────

    /// Read a vote from a meeting.
    pub fn read_vote(
        &self,
        branch: &str,
        meeting_id: MeetingId,
        id: VoteId,
    ) -> Result<Vote, GitStorageError> {
        self.repo.read_json(
            branch,
            &format!("governance/meetings/{}/votes/{}.json", meeting_id, id),
        )
    }

    /// List all vote IDs for a meeting.
    pub fn list_vote_ids(
        &self,
        branch: &str,
        meeting_id: MeetingId,
    ) -> Result<Vec<VoteId>, GitStorageError> {
        self.list_ids_in_dir(
            branch,
            &format!("governance/meetings/{}/votes", meeting_id),
        )
    }

    // ── Governance: Resolutions ────────────────────────────────────────

    /// Read a resolution from a meeting.
    pub fn read_resolution(
        &self,
        branch: &str,
        meeting_id: MeetingId,
        id: ResolutionId,
    ) -> Result<Resolution, GitStorageError> {
        self.repo.read_json(
            branch,
            &format!(
                "governance/meetings/{}/resolutions/{}.json",
                meeting_id, id
            ),
        )
    }

    /// List all resolution IDs for a meeting.
    pub fn list_resolution_ids(
        &self,
        branch: &str,
        meeting_id: MeetingId,
    ) -> Result<Vec<ResolutionId>, GitStorageError> {
        self.list_ids_in_dir(
            branch,
            &format!("governance/meetings/{}/resolutions", meeting_id),
        )
    }

    // ── Treasury: Accounts ────────────────────────────────────────────
    pub fn read_account(&self, branch: &str, id: AccountId) -> Result<Account, GitStorageError> {
        self.repo
            .read_json(branch, &format!("treasury/accounts/{}.json", id))
    }
    pub fn list_account_ids(&self, branch: &str) -> Result<Vec<AccountId>, GitStorageError> {
        self.list_ids_in_dir(branch, "treasury/accounts")
    }

    // ── Treasury: Journal Entries ─────────────────────────────────────
    pub fn read_journal_entry(
        &self,
        branch: &str,
        id: JournalEntryId,
    ) -> Result<JournalEntry, GitStorageError> {
        self.repo
            .read_json(branch, &format!("treasury/journal-entries/{}.json", id))
    }
    pub fn list_journal_entry_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<JournalEntryId>, GitStorageError> {
        self.list_ids_in_dir(branch, "treasury/journal-entries")
    }

    // ── Treasury: Invoices ────────────────────────────────────────────
    pub fn read_invoice(
        &self,
        branch: &str,
        id: InvoiceId,
    ) -> Result<Invoice, GitStorageError> {
        self.repo
            .read_json(branch, &format!("treasury/invoices/{}.json", id))
    }
    pub fn list_invoice_ids(&self, branch: &str) -> Result<Vec<InvoiceId>, GitStorageError> {
        self.list_ids_in_dir(branch, "treasury/invoices")
    }

    // ── Treasury: Bank Accounts ───────────────────────────────────────
    pub fn read_bank_account(
        &self,
        branch: &str,
        id: BankAccountId,
    ) -> Result<BankAccount, GitStorageError> {
        self.repo
            .read_json(branch, &format!("treasury/bank-accounts/{}.json", id))
    }
    pub fn list_bank_account_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<BankAccountId>, GitStorageError> {
        self.list_ids_in_dir(branch, "treasury/bank-accounts")
    }

    // ── Execution: Intents ────────────────────────────────────────────
    pub fn read_intent(&self, branch: &str, id: IntentId) -> Result<Intent, GitStorageError> {
        self.repo
            .read_json(branch, &format!("execution/intents/{}.json", id))
    }
    pub fn list_intent_ids(&self, branch: &str) -> Result<Vec<IntentId>, GitStorageError> {
        self.list_ids_in_dir(branch, "execution/intents")
    }

    // ── Execution: Obligations ────────────────────────────────────────
    pub fn read_obligation(
        &self,
        branch: &str,
        id: ObligationId,
    ) -> Result<Obligation, GitStorageError> {
        self.repo
            .read_json(branch, &format!("execution/obligations/{}.json", id))
    }
    pub fn list_obligation_ids(
        &self,
        branch: &str,
    ) -> Result<Vec<ObligationId>, GitStorageError> {
        self.list_ids_in_dir(branch, "execution/obligations")
    }

    // ── Execution: Receipts ───────────────────────────────────────────
    pub fn read_receipt(&self, branch: &str, id: ReceiptId) -> Result<Receipt, GitStorageError> {
        self.repo
            .read_json(branch, &format!("execution/receipts/{}.json", id))
    }
    pub fn list_receipt_ids(&self, branch: &str) -> Result<Vec<ReceiptId>, GitStorageError> {
        self.list_ids_in_dir(branch, "execution/receipts")
    }

    // ── Contacts ──────────────────────────────────────────────────────
    pub fn read_contact(&self, branch: &str, id: ContactId) -> Result<Contact, GitStorageError> {
        self.repo
            .read_json(branch, &format!("contacts/{}.json", id))
    }
    pub fn list_contact_ids(&self, branch: &str) -> Result<Vec<ContactId>, GitStorageError> {
        self.list_ids_in_dir(branch, "contacts")
    }

    // ── Contracts ────────────────────────────────────────────────────
    pub fn read_contract(&self, branch: &str, id: ContractId) -> Result<Contract, GitStorageError> {
        self.repo.read_json(branch, &format!("contracts/{}.json", id))
    }
    pub fn list_contract_ids(&self, branch: &str) -> Result<Vec<ContractId>, GitStorageError> {
        self.list_ids_in_dir(branch, "contracts")
    }

    // ── Tax Filings ─────────────────────────────────────────────────
    pub fn read_tax_filing(&self, branch: &str, id: TaxFilingId) -> Result<TaxFiling, GitStorageError> {
        self.repo.read_json(branch, &format!("tax/filings/{}.json", id))
    }
    pub fn list_tax_filing_ids(&self, branch: &str) -> Result<Vec<TaxFilingId>, GitStorageError> {
        self.list_ids_in_dir(branch, "tax/filings")
    }

    // ── Deadlines ───────────────────────────────────────────────────
    pub fn read_deadline(&self, branch: &str, id: DeadlineId) -> Result<Deadline, GitStorageError> {
        self.repo.read_json(branch, &format!("deadlines/{}.json", id))
    }
    pub fn list_deadline_ids(&self, branch: &str) -> Result<Vec<DeadlineId>, GitStorageError> {
        self.list_ids_in_dir(branch, "deadlines")
    }

    // ── Contractor Classifications ──────────────────────────────────
    pub fn read_contractor_classification(&self, branch: &str, id: ClassificationId) -> Result<ContractorClassification, GitStorageError> {
        self.repo.read_json(branch, &format!("contractors/{}.json", id))
    }
    pub fn list_contractor_classification_ids(&self, branch: &str) -> Result<Vec<ClassificationId>, GitStorageError> {
        self.list_ids_in_dir(branch, "contractors")
    }

    // ── Treasury: Payments ──────────────────────────────────────────
    pub fn read_payment(&self, branch: &str, id: PaymentId) -> Result<Payment, GitStorageError> {
        self.repo.read_json(branch, &format!("treasury/payments/{}.json", id))
    }
    pub fn list_payment_ids(&self, branch: &str) -> Result<Vec<PaymentId>, GitStorageError> {
        self.list_ids_in_dir(branch, "treasury/payments")
    }

    // ── Treasury: Payroll ───────────────────────────────────────────
    pub fn read_payroll_run(&self, branch: &str, id: PayrollRunId) -> Result<PayrollRun, GitStorageError> {
        self.repo.read_json(branch, &format!("treasury/payroll/{}.json", id))
    }
    pub fn list_payroll_run_ids(&self, branch: &str) -> Result<Vec<PayrollRunId>, GitStorageError> {
        self.list_ids_in_dir(branch, "treasury/payroll")
    }

    // ── Treasury: Distributions ─────────────────────────────────────
    pub fn read_distribution(&self, branch: &str, id: DistributionId) -> Result<Distribution, GitStorageError> {
        self.repo.read_json(branch, &format!("treasury/distributions/{}.json", id))
    }
    pub fn list_distribution_ids(&self, branch: &str) -> Result<Vec<DistributionId>, GitStorageError> {
        self.list_ids_in_dir(branch, "treasury/distributions")
    }

    // ── Treasury: Reconciliations ───────────────────────────────────
    pub fn read_reconciliation(&self, branch: &str, id: ReconciliationId) -> Result<Reconciliation, GitStorageError> {
        self.repo.read_json(branch, &format!("treasury/reconciliations/{}.json", id))
    }
    pub fn list_reconciliation_ids(&self, branch: &str) -> Result<Vec<ReconciliationId>, GitStorageError> {
        self.list_ids_in_dir(branch, "treasury/reconciliations")
    }

    // ── Access Manifest ────────────────────────────────────────────

    /// Read the access manifest, returning a default empty one if not found.
    pub fn read_access_manifest(
        &self,
        branch: &str,
    ) -> Result<crate::git::projection::AccessManifest, GitStorageError> {
        match self.repo.read_json(branch, ".corp/access-manifest.json") {
            Ok(manifest) => Ok(manifest),
            Err(GitStorageError::NotFound(_)) => Ok(crate::git::projection::AccessManifest::new()),
            Err(e) => Err(e),
        }
    }

    /// Write the access manifest.
    pub fn write_access_manifest(
        &self,
        branch: &str,
        manifest: &crate::git::projection::AccessManifest,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let files = vec![FileWrite::json(".corp/access-manifest.json", manifest)?];
        commit_files(&self.repo, branch, message, &files)?;
        Ok(())
    }

    // ── Generic helpers ──────────────────────────────────────────────

    /// Read any deserializable JSON from a path.
    pub fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        branch: &str,
        path: &str,
    ) -> Result<T, GitStorageError> {
        self.repo.read_json(branch, path)
    }

    /// Write any serializable value to a JSON path and commit it.
    pub fn write_json<T: Serialize>(
        &self,
        branch: &str,
        path: &str,
        value: &T,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let files = vec![FileWrite::json(path, value)?];
        commit_files(&self.repo, branch, message, &files)?;
        Ok(())
    }

    /// List UUID-style IDs from files in a directory.
    ///
    /// Expects files named `{uuid}.json`. Returns parsed IDs for all
    /// matching entries, silently skipping non-UUID filenames.
    pub fn list_ids_in_dir<T: std::str::FromStr>(
        &self,
        branch: &str,
        dir_path: &str,
    ) -> Result<Vec<T>, GitStorageError> {
        let entries = match self.repo.list_dir(branch, dir_path) {
            Ok(entries) => entries,
            Err(GitStorageError::NotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut ids = Vec::new();
        for (name, is_dir) in entries {
            if is_dir {
                continue;
            }
            if let Some(uuid_str) = name.strip_suffix(".json") {
                if let Ok(id) = uuid_str.parse() {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
    }
}
