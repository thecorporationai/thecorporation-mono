//! GL account record (stored as `treasury/accounts/{account_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{AccountType, Currency, GlAccountCode, Side};
use crate::domain::ids::{AccountId, EntityId};

/// A general ledger account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    account_id: AccountId,
    entity_id: EntityId,
    account_code: GlAccountCode,
    account_name: String,
    account_type: AccountType,
    normal_balance: Side,
    currency: Currency,
    is_active: bool,
    created_at: DateTime<Utc>,
}

impl Account {
    /// Create a new GL account. Account type and normal balance are derived
    /// from the `GlAccountCode`.
    pub fn new(
        account_id: AccountId,
        entity_id: EntityId,
        account_code: GlAccountCode,
    ) -> Self {
        let account_type = account_code.account_type();
        Self {
            account_id,
            entity_id,
            account_code,
            account_name: account_code.label().to_string(),
            account_type,
            normal_balance: account_type.normal_balance(),
            currency: Currency::default(),
            is_active: true,
            created_at: Utc::now(),
        }
    }

    /// Deactivate the account.
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn account_code(&self) -> GlAccountCode {
        self.account_code
    }

    pub fn account_name(&self) -> &str {
        &self.account_name
    }

    pub fn account_type(&self) -> AccountType {
        self.account_type
    }

    pub fn normal_balance(&self) -> Side {
        self.normal_balance
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_account() -> Account {
        Account::new(
            AccountId::new(),
            EntityId::new(),
            GlAccountCode::Cash,
        )
    }

    #[test]
    fn new_derives_type_and_normal_balance() {
        let acct = make_account();
        assert_eq!(acct.account_type(), AccountType::Asset);
        assert_eq!(acct.normal_balance(), Side::Debit);
        assert_eq!(acct.account_name(), "Cash");
        assert!(acct.is_active());
    }

    #[test]
    fn revenue_account_has_credit_normal_balance() {
        let acct = Account::new(
            AccountId::new(),
            EntityId::new(),
            GlAccountCode::Revenue,
        );
        assert_eq!(acct.account_type(), AccountType::Revenue);
        assert_eq!(acct.normal_balance(), Side::Credit);
    }

    #[test]
    fn deactivate() {
        let mut acct = make_account();
        assert!(acct.is_active());
        acct.deactivate();
        assert!(!acct.is_active());
    }

    #[test]
    fn serde_roundtrip() {
        let acct = make_account();
        let json = serde_json::to_string(&acct).unwrap();
        let parsed: Account = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.account_id(), acct.account_id());
        assert_eq!(parsed.account_code(), acct.account_code());
        assert_eq!(parsed.account_name(), acct.account_name());
        assert_eq!(parsed.is_active(), acct.is_active());
    }
}
