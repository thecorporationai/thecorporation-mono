//! Treasury domain errors.

use super::types::{BankAccountStatus, Cents, InvoiceStatus, KybStatus};
use crate::domain::ids::{
    AccountId, BankAccountId, InvoiceId, JournalEntryId, KybPackageId, PaymentId,
};
use thiserror::Error;

/// Errors that can occur in the treasury domain.
#[derive(Debug, Error)]
pub enum TreasuryError {
    /// A journal entry's debits and credits do not balance.
    #[error("journal entry is not balanced: debits={debits}, credits={credits}")]
    UnbalancedEntry { debits: Cents, credits: Cents },

    /// Attempted to post a journal entry that was already posted.
    #[error("journal entry {0} is already posted")]
    AlreadyPosted(JournalEntryId),

    /// Attempted to void a journal entry that was already voided.
    #[error("journal entry {0} is already voided")]
    AlreadyVoided(JournalEntryId),

    /// Cannot void a journal entry that is still in draft state.
    #[error("cannot void a draft journal entry")]
    CannotVoidDraft,

    /// The requested invoice does not exist.
    #[error("invoice {0} not found")]
    InvoiceNotFound(InvoiceId),

    /// The invoice cannot transition between the given states.
    #[error("invalid invoice transition from {from} to {to}")]
    InvalidInvoiceTransition {
        from: InvoiceStatus,
        to: InvoiceStatus,
    },

    /// The requested bank account does not exist.
    #[error("bank account {0} not found")]
    BankAccountNotFound(BankAccountId),

    /// The bank account exists but is not in active status.
    #[error("bank account {0} is not active")]
    BankAccountNotActive(BankAccountId),

    /// The bank account cannot transition between the given states.
    #[error("invalid bank account transition from {from} to {to}")]
    InvalidBankAccountTransition {
        from: BankAccountStatus,
        to: BankAccountStatus,
    },

    /// The requested KYB package does not exist.
    #[error("KYB package {0} not found")]
    KybNotFound(KybPackageId),

    /// The KYB package cannot transition between the given states.
    #[error("invalid KYB transition from {from} to {to}")]
    InvalidKybTransition { from: KybStatus, to: KybStatus },

    /// A spending request exceeds the authorized limit.
    #[error("spending limit exceeded: requested={requested}, authorized={authorized}")]
    SpendingLimitExceeded {
        requested: Cents,
        authorized: Cents,
    },

    /// No spending policy has been configured for the entity.
    #[error("no spending policy found for entity")]
    NoSpendingPolicy,

    /// The requested GL account does not exist.
    #[error("account {0} not found")]
    AccountNotFound(AccountId),

    /// A GL account with this code already exists.
    #[error("duplicate GL account code: {0}")]
    DuplicateAccount(u16),

    /// A payment attempt failed.
    #[error("payment {0} failed: {1}")]
    PaymentFailed(PaymentId, String),

    /// An error from an external banking connector.
    #[error("bank feed provider error: {0}")]
    ConnectorError(String),
}
