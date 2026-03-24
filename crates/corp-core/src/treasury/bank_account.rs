//! Bank account records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::types::{BankAccountStatus, BankAccountType};
use crate::ids::{BankAccountId, EntityId};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BankAccountError {
    #[error("bank account must be in PendingReview to be activated; current status: {0:?}")]
    NotPendingReview(BankAccountStatus),
    #[error("bank account must be Active to be closed; current status: {0:?}")]
    NotActive(BankAccountStatus),
}

// ── BankAccount ───────────────────────────────────────────────────────────────

/// A bank account associated with a legal entity. Follows the FSM:
/// ```text
/// PendingReview → Active → Closed
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankAccount {
    pub bank_account_id: BankAccountId,
    pub entity_id: EntityId,
    pub institution: String,
    pub account_type: BankAccountType,
    pub status: BankAccountStatus,
    /// Last 4 digits of the account number, if available.
    pub account_number_last4: Option<String>,
    /// Last 4 digits of the routing number, if available.
    pub routing_number_last4: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl BankAccount {
    /// Create a new bank account in `PendingReview` status.
    pub fn new(
        entity_id: EntityId,
        institution: impl Into<String>,
        account_type: BankAccountType,
        account_number_last4: Option<String>,
        routing_number_last4: Option<String>,
    ) -> Self {
        Self {
            bank_account_id: BankAccountId::new(),
            entity_id,
            institution: institution.into(),
            account_type,
            status: BankAccountStatus::PendingReview,
            account_number_last4,
            routing_number_last4,
            created_at: Utc::now(),
        }
    }

    /// Transition `PendingReview → Active`.
    pub fn activate(&mut self) -> Result<(), BankAccountError> {
        match self.status {
            BankAccountStatus::PendingReview => {
                self.status = BankAccountStatus::Active;
                Ok(())
            }
            s => Err(BankAccountError::NotPendingReview(s)),
        }
    }

    /// Transition `Active → Closed`.
    pub fn close(&mut self) -> Result<(), BankAccountError> {
        match self.status {
            BankAccountStatus::Active => {
                self.status = BankAccountStatus::Closed;
                Ok(())
            }
            s => Err(BankAccountError::NotActive(s)),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntityId;

    fn make_checking() -> BankAccount {
        BankAccount::new(
            EntityId::new(),
            "First National Bank",
            BankAccountType::Checking,
            Some("1234".into()),
            Some("0210".into()),
        )
    }

    fn make_savings() -> BankAccount {
        BankAccount::new(
            EntityId::new(),
            "Savings Bank",
            BankAccountType::Savings,
            None,
            None,
        )
    }

    #[test]
    fn new_checking_starts_pending_review() {
        let ba = make_checking();
        assert_eq!(ba.status, BankAccountStatus::PendingReview);
        assert_eq!(ba.account_type, BankAccountType::Checking);
    }

    #[test]
    fn new_savings_starts_pending_review() {
        let ba = make_savings();
        assert_eq!(ba.status, BankAccountStatus::PendingReview);
        assert_eq!(ba.account_type, BankAccountType::Savings);
    }

    #[test]
    fn new_stores_institution() {
        let ba = make_checking();
        assert_eq!(ba.institution, "First National Bank");
    }

    #[test]
    fn new_stores_account_numbers() {
        let ba = make_checking();
        assert_eq!(ba.account_number_last4.as_deref(), Some("1234"));
        assert_eq!(ba.routing_number_last4.as_deref(), Some("0210"));
    }

    #[test]
    fn new_without_numbers() {
        let ba = make_savings();
        assert!(ba.account_number_last4.is_none());
        assert!(ba.routing_number_last4.is_none());
    }

    #[test]
    fn activate_from_pending_review() {
        let mut ba = make_checking();
        assert!(ba.activate().is_ok());
        assert_eq!(ba.status, BankAccountStatus::Active);
    }

    #[test]
    fn close_from_active() {
        let mut ba = make_checking();
        ba.activate().unwrap();
        assert!(ba.close().is_ok());
        assert_eq!(ba.status, BankAccountStatus::Closed);
    }

    #[test]
    fn full_lifecycle_pending_active_closed() {
        let mut ba = make_savings();
        ba.activate().unwrap();
        ba.close().unwrap();
        assert_eq!(ba.status, BankAccountStatus::Closed);
    }

    #[test]
    fn activate_from_active_is_error() {
        let mut ba = make_checking();
        ba.activate().unwrap();
        assert!(matches!(
            ba.activate(),
            Err(BankAccountError::NotPendingReview(
                BankAccountStatus::Active
            ))
        ));
    }

    #[test]
    fn activate_from_closed_is_error() {
        let mut ba = make_checking();
        ba.activate().unwrap();
        ba.close().unwrap();
        assert!(matches!(
            ba.activate(),
            Err(BankAccountError::NotPendingReview(
                BankAccountStatus::Closed
            ))
        ));
    }

    #[test]
    fn close_from_pending_review_is_error() {
        let mut ba = make_checking();
        assert!(matches!(
            ba.close(),
            Err(BankAccountError::NotActive(
                BankAccountStatus::PendingReview
            ))
        ));
    }

    #[test]
    fn close_from_closed_is_error() {
        let mut ba = make_checking();
        ba.activate().unwrap();
        ba.close().unwrap();
        assert!(matches!(
            ba.close(),
            Err(BankAccountError::NotActive(BankAccountStatus::Closed))
        ));
    }

    #[test]
    fn bank_account_ids_are_unique() {
        let a = make_checking();
        let b = make_checking();
        assert_ne!(a.bank_account_id, b.bank_account_id);
    }
}
