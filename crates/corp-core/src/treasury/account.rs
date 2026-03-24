//! General-ledger account.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{AccountId, EntityId};
use super::types::{AccountType, Currency, GlAccountCode, Side};

// ── Account ───────────────────────────────────────────────────────────────────

/// A general-ledger account belonging to a legal entity.
///
/// The `account_type` and `normal_balance` are derived automatically from the
/// `account_code` when the account is constructed via [`Account::new`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub account_id: AccountId,
    pub entity_id: EntityId,
    pub account_code: GlAccountCode,
    pub account_name: String,
    pub account_type: AccountType,
    pub normal_balance: Side,
    pub currency: Currency,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl Account {
    /// Create a new account. `account_type` and `normal_balance` are derived
    /// from `account_code`; the caller only needs to supply the display name.
    pub fn new(
        entity_id: EntityId,
        account_code: GlAccountCode,
        account_name: impl Into<String>,
        currency: Currency,
    ) -> Self {
        Self {
            account_id: AccountId::new(),
            entity_id,
            account_type: account_code.account_type(),
            normal_balance: account_code.normal_balance(),
            account_code,
            account_name: account_name.into(),
            currency,
            is_active: true,
            created_at: Utc::now(),
        }
    }

    /// Mark this account as inactive. Inactive accounts cannot receive new
    /// journal lines.
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntityId;

    fn make_account(code: GlAccountCode) -> Account {
        Account::new(EntityId::new(), code, "Test Account", Currency::Usd)
    }

    #[test]
    fn new_derives_asset_type_for_cash() {
        let a = make_account(GlAccountCode::Cash);
        assert_eq!(a.account_type, AccountType::Asset);
        assert_eq!(a.normal_balance, Side::Debit);
        assert!(a.is_active);
    }

    #[test]
    fn new_derives_asset_type_for_ar() {
        let a = make_account(GlAccountCode::AccountsReceivable);
        assert_eq!(a.account_type, AccountType::Asset);
        assert_eq!(a.normal_balance, Side::Debit);
    }

    #[test]
    fn new_derives_liability_for_ap() {
        let a = make_account(GlAccountCode::AccountsPayable);
        assert_eq!(a.account_type, AccountType::Liability);
        assert_eq!(a.normal_balance, Side::Credit);
    }

    #[test]
    fn new_derives_liability_for_accrued() {
        let a = make_account(GlAccountCode::AccruedExpenses);
        assert_eq!(a.account_type, AccountType::Liability);
        assert_eq!(a.normal_balance, Side::Credit);
    }

    #[test]
    fn new_derives_equity_for_founder_capital() {
        let a = make_account(GlAccountCode::FounderCapital);
        assert_eq!(a.account_type, AccountType::Equity);
        assert_eq!(a.normal_balance, Side::Credit);
    }

    #[test]
    fn new_derives_revenue_type() {
        let a = make_account(GlAccountCode::Revenue);
        assert_eq!(a.account_type, AccountType::Revenue);
        assert_eq!(a.normal_balance, Side::Credit);
    }

    #[test]
    fn new_derives_expense_for_opex() {
        let a = make_account(GlAccountCode::OperatingExpenses);
        assert_eq!(a.account_type, AccountType::Expense);
        assert_eq!(a.normal_balance, Side::Debit);
    }

    #[test]
    fn new_derives_expense_for_cogs() {
        let a = make_account(GlAccountCode::Cogs);
        assert_eq!(a.account_type, AccountType::Expense);
        assert_eq!(a.normal_balance, Side::Debit);
    }

    #[test]
    fn new_stores_provided_name() {
        let a = Account::new(EntityId::new(), GlAccountCode::Cash, "Main Checking", Currency::Usd);
        assert_eq!(a.account_name, "Main Checking");
    }

    #[test]
    fn new_stores_currency() {
        let a = make_account(GlAccountCode::Cash);
        assert_eq!(a.currency, Currency::Usd);
    }

    #[test]
    fn new_account_is_active() {
        let a = make_account(GlAccountCode::Cash);
        assert!(a.is_active);
    }

    #[test]
    fn deactivate_sets_inactive() {
        let mut a = make_account(GlAccountCode::Cash);
        a.deactivate();
        assert!(!a.is_active);
    }

    #[test]
    fn deactivate_is_idempotent() {
        let mut a = make_account(GlAccountCode::Cash);
        a.deactivate();
        a.deactivate();
        assert!(!a.is_active);
    }

    #[test]
    fn account_ids_are_unique() {
        let a = make_account(GlAccountCode::Cash);
        let b = make_account(GlAccountCode::Cash);
        assert_ne!(a.account_id, b.account_id);
    }

    #[test]
    fn account_code_stored_correctly() {
        let a = make_account(GlAccountCode::Revenue);
        assert_eq!(a.account_code, GlAccountCode::Revenue);
    }
}
