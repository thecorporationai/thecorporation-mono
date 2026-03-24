//! Execution domain — intents, obligations, and receipts.

pub mod intent;
pub mod obligation;
pub mod receipt;
pub mod types;

pub use intent::Intent;
pub use obligation::Obligation;
pub use receipt::Receipt;
pub use types::{
    AssigneeType, DocumentRequestStatus, IntentStatus, ObligationStatus, ReceiptStatus,
};
