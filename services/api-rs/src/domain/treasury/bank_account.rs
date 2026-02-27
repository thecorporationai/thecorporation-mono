//! Bank account record (stored as `treasury/bank-accounts/{id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::TreasuryError;
use super::types::{BankAccountStatus, BankAccountType, Currency};
use crate::domain::ids::{BankAccountId, EntityId};

/// A bank account connected to an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    bank_account_id: BankAccountId,
    entity_id: EntityId,
    bank_name: String,
    account_type: BankAccountType,
    currency: Currency,
    status: BankAccountStatus,
    created_at: DateTime<Utc>,
}

impl BankAccount {
    /// Create a new bank account in PendingReview status.
    pub fn new(
        bank_account_id: BankAccountId,
        entity_id: EntityId,
        bank_name: String,
        account_type: BankAccountType,
    ) -> Self {
        Self {
            bank_account_id,
            entity_id,
            bank_name,
            account_type,
            currency: Currency::default(),
            status: BankAccountStatus::PendingReview,
            created_at: Utc::now(),
        }
    }

    /// Activate. PendingReview -> Active.
    pub fn activate(&mut self) -> Result<(), TreasuryError> {
        if self.status != BankAccountStatus::PendingReview {
            return Err(TreasuryError::InvalidBankAccountTransition {
                from: self.status,
                to: BankAccountStatus::Active,
            });
        }
        self.status = BankAccountStatus::Active;
        Ok(())
    }

    /// Close. Active -> Closed.
    pub fn close(&mut self) -> Result<(), TreasuryError> {
        if self.status != BankAccountStatus::Active {
            return Err(TreasuryError::InvalidBankAccountTransition {
                from: self.status,
                to: BankAccountStatus::Closed,
            });
        }
        self.status = BankAccountStatus::Closed;
        Ok(())
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn bank_account_id(&self) -> BankAccountId {
        self.bank_account_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn bank_name(&self) -> &str {
        &self.bank_name
    }

    pub fn account_type(&self) -> BankAccountType {
        self.account_type
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn status(&self) -> BankAccountStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bank_account() -> BankAccount {
        BankAccount::new(
            BankAccountId::new(),
            EntityId::new(),
            "First National Bank".into(),
            BankAccountType::Checking,
        )
    }

    #[test]
    fn new_bank_account_is_pending_review() {
        let ba = make_bank_account();
        assert_eq!(ba.status(), BankAccountStatus::PendingReview);
        assert_eq!(ba.bank_name(), "First National Bank");
        assert_eq!(ba.account_type(), BankAccountType::Checking);
    }

    #[test]
    fn full_lifecycle_pending_active_closed() {
        let mut ba = make_bank_account();
        assert!(ba.activate().is_ok());
        assert_eq!(ba.status(), BankAccountStatus::Active);
        assert!(ba.close().is_ok());
        assert_eq!(ba.status(), BankAccountStatus::Closed);
    }

    #[test]
    fn cannot_activate_active() {
        let mut ba = make_bank_account();
        ba.activate().unwrap();
        assert!(ba.activate().is_err());
    }

    #[test]
    fn cannot_close_pending() {
        let mut ba = make_bank_account();
        assert!(ba.close().is_err());
    }

    #[test]
    fn cannot_close_closed() {
        let mut ba = make_bank_account();
        ba.activate().unwrap();
        ba.close().unwrap();
        assert!(ba.close().is_err());
    }

    #[test]
    fn cannot_activate_closed() {
        let mut ba = make_bank_account();
        ba.activate().unwrap();
        ba.close().unwrap();
        assert!(ba.activate().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let ba = make_bank_account();
        let json = serde_json::to_string(&ba).unwrap();
        let parsed: BankAccount = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bank_account_id(), ba.bank_account_id());
        assert_eq!(parsed.bank_name(), ba.bank_name());
        assert_eq!(parsed.status(), ba.status());
    }
}
