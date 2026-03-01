//! Equity domain errors.

use super::{
    round::EquityRoundStatus,
    types::{
        FundingRoundStatus, GrantStatus, SafeStatus, ShareCount, TransferStatus, ValuationStatus,
    },
};
use crate::domain::ids::{
    CapTableId, EquityGrantId, FundingRoundId, RepurchaseRightId, SafeNoteId, ShareClassId,
    TransferId, ValuationId,
};
use crate::domain::treasury::types::Cents;
use thiserror::Error;

/// Errors that can occur in the equity domain.
#[derive(Debug, Error)]
pub enum EquityError {
    /// Outstanding shares exceed the authorized limit for a share class.
    #[error(
        "outstanding shares exceed authorized for share class {share_class_id}: \
         outstanding={outstanding}, authorized={authorized}"
    )]
    OutstandingExceedsAuthorized {
        share_class_id: ShareClassId,
        outstanding: ShareCount,
        authorized: ShareCount,
    },

    /// Not enough shares available for the requested operation.
    #[error("insufficient shares: available={available}, requested={requested}")]
    InsufficientShares {
        available: ShareCount,
        requested: ShareCount,
    },

    /// The requested equity grant does not exist.
    #[error("equity grant {0} not found")]
    GrantNotFound(EquityGrantId),

    /// The requested share class does not exist.
    #[error("share class {0} not found")]
    ShareClassNotFound(ShareClassId),

    /// The requested cap table does not exist.
    #[error("cap table {0} not found")]
    CapTableNotFound(CapTableId),

    /// The requested SAFE note does not exist.
    #[error("SAFE note {0} not found")]
    SafeNotFound(SafeNoteId),

    /// The requested valuation does not exist.
    #[error("valuation {0} not found")]
    ValuationNotFound(ValuationId),

    /// The requested transfer does not exist.
    #[error("transfer {0} not found")]
    TransferNotFound(TransferId),

    /// The requested funding round does not exist.
    #[error("funding round {0} not found")]
    FundingRoundNotFound(FundingRoundId),

    /// A grant cannot transition between the given states.
    #[error("invalid grant transition from {from} to {to}")]
    InvalidGrantTransition { from: GrantStatus, to: GrantStatus },

    /// A SAFE cannot transition between the given states.
    #[error("invalid SAFE transition from {from} to {to}")]
    InvalidSafeTransition { from: SafeStatus, to: SafeStatus },

    /// A transfer cannot transition between the given states.
    #[error("invalid transfer transition from {from} to {to}")]
    InvalidTransferTransition {
        from: TransferStatus,
        to: TransferStatus,
    },

    /// A valuation cannot transition between the given states.
    #[error("invalid valuation transition from {from} to {to}")]
    InvalidValuationTransition {
        from: ValuationStatus,
        to: ValuationStatus,
    },

    /// A funding round cannot transition between the given states.
    #[error("invalid funding round transition from {from} to {to}")]
    InvalidFundingRoundTransition {
        from: FundingRoundStatus,
        to: FundingRoundStatus,
    },

    /// An equity round cannot transition between the given states.
    #[error("invalid equity round transition from {from} to {to}")]
    InvalidRoundTransition {
        from: EquityRoundStatus,
        to: EquityRoundStatus,
    },

    /// The valuation has expired and can no longer be used.
    #[error("valuation {0} has expired")]
    ValuationExpired(ValuationId),

    /// Valuation cap is below the principal amount.
    #[error("valuation cap {cap} is below principal amount {principal}")]
    ValuationCapBelowPrincipal { cap: Cents, principal: Cents },

    /// Exercise price is below the current fair market value (409A violation risk).
    #[error("exercise price {exercise_price} is below FMV {fmv}")]
    ExercisePriceBelowFmv { exercise_price: Cents, fmv: Cents },

    /// The requested repurchase right does not exist.
    #[error("repurchase right {0} not found")]
    RepurchaseNotFound(RepurchaseRightId),

    /// General validation error.
    #[error("equity validation error: {0}")]
    Validation(String),
}
