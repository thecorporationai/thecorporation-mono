//! Execution domain errors.

use super::types::{IntentStatus, ObligationStatus};
use crate::domain::ids::{IntentId, ObligationId, ReceiptId};
use thiserror::Error;

/// Errors that can occur in the execution domain.
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// The requested intent does not exist.
    #[error("intent {0} not found")]
    IntentNotFound(IntentId),

    /// The requested receipt does not exist.
    #[error("receipt {0} not found")]
    ReceiptNotFound(ReceiptId),

    /// The requested obligation does not exist.
    #[error("obligation {0} not found")]
    ObligationNotFound(ObligationId),

    /// The intent cannot transition between the given states.
    #[error("invalid intent transition from {from} to {to}")]
    InvalidIntentTransition {
        from: IntentStatus,
        to: IntentStatus,
    },

    /// The obligation cannot transition between the given states.
    #[error("invalid obligation transition from {from} to {to}")]
    InvalidObligationTransition {
        from: ObligationStatus,
        to: ObligationStatus,
    },

    /// An execution has already been recorded for this intent.
    #[error("duplicate execution for intent {intent_id}")]
    DuplicateExecution { intent_id: IntentId },
}
