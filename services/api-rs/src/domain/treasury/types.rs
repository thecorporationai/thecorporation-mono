//! Treasury domain types — monetary newtypes and enums.
//!
//! All monetary values are stored as integer cents. Distinct newtypes prevent
//! unit confusion: you cannot add `Cents` to `ShareCount`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

// ── Monetary newtypes ──────────────────────────────────────────────────

/// Integer cents (USD). $1.00 = 100 cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cents(i64);

impl Cents {
    /// Zero cents.
    pub const ZERO: Self = Self(0);

    /// Create a new `Cents` value.
    #[inline]
    pub const fn new(cents: i64) -> Self {
        Self(cents)
    }

    /// Return the raw integer value.
    #[inline]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Whether the amount is negative.
    #[inline]
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    /// Whether the amount is exactly zero.
    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Whether the amount is positive.
    #[inline]
    pub fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Convert to `Decimal` dollars for display or precise calculations.
    pub fn to_dollars(self) -> Decimal {
        Decimal::new(self.0, 2)
    }

    /// Multiply by an integer scaling factor.
    #[inline]
    pub fn scale(self, multiplier: i64) -> Self {
        Self(
            self.0
                .checked_mul(multiplier)
                .expect("Cents scale overflow"),
        )
    }

    /// Checked addition — returns `None` on overflow.
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    /// Checked subtraction — returns `None` on underflow.
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    /// Require a positive value (> 0).
    #[inline]
    pub fn require_positive(self) -> Result<Self, &'static str> {
        if self.0 > 0 {
            Ok(self)
        } else {
            Err("amount must be positive")
        }
    }

    /// Require a non-negative value (>= 0).
    #[inline]
    pub fn require_non_negative(self) -> Result<Self, &'static str> {
        if self.0 >= 0 {
            Ok(self)
        } else {
            Err("amount must be non-negative")
        }
    }
}

impl Add for Cents {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.checked_add(rhs.0).expect("Cents addition overflow"))
    }
}

impl Sub for Cents {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_sub(rhs.0)
                .expect("Cents subtraction overflow"),
        )
    }
}

impl Neg for Cents {
    type Output = Self;
    fn neg(self) -> Self {
        Self(self.0.checked_neg().expect("Cents negation overflow"))
    }
}

impl AddAssign for Cents {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.checked_add(rhs.0).expect("Cents addition overflow");
    }
}

impl SubAssign for Cents {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self
            .0
            .checked_sub(rhs.0)
            .expect("Cents subtraction overflow");
    }
}

impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dollars = self.0 / 100;
        let cents = (self.0 % 100).abs();
        if self.0 < 0 {
            write!(f, "-${}.{:02}", dollars.abs(), cents)
        } else {
            write!(f, "${}.{:02}", dollars, cents)
        }
    }
}

// ── Ledger Amount ──────────────────────────────────────────────────────

/// A ledger amount that carries its side (debit or credit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerAmount {
    cents: Cents,
    side: Side,
}

impl LedgerAmount {
    /// Create a debit-side ledger amount.
    pub fn debit(cents: Cents) -> Self {
        Self {
            cents,
            side: Side::Debit,
        }
    }

    /// Create a credit-side ledger amount.
    pub fn credit(cents: Cents) -> Self {
        Self {
            cents,
            side: Side::Credit,
        }
    }

    /// The cents value of this amount.
    pub fn cents(self) -> Cents {
        self.cents
    }

    /// The side (debit or credit) of this amount.
    pub fn side(self) -> Side {
        self.side
    }

    /// Signed value: debits positive, credits negative.
    pub fn signed(self) -> i64 {
        match self.side {
            Side::Debit => self.cents.raw(),
            Side::Credit => -self.cents.raw(),
        }
    }
}

// ── Currency ───────────────────────────────────────────────────────────

/// Supported currencies. Currently USD only.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Currency {
    /// United States Dollar.
    #[default]
    Usd,
}

// ── Account Types ──────────────────────────────────────────────────────

/// The five fundamental accounting categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    /// Assets owned by the entity.
    Asset,
    /// Debts and obligations owed.
    Liability,
    /// Owner's residual interest.
    Equity,
    /// Income earned.
    Revenue,
    /// Costs incurred.
    Expense,
}

impl AccountType {
    /// The normal balance side for this account type.
    /// Assets and Expenses normally have debit balances.
    /// Liabilities, Equity, and Revenue normally have credit balances.
    pub const fn normal_balance(self) -> Side {
        match self {
            Self::Asset | Self::Expense => Side::Debit,
            Self::Liability | Self::Equity | Self::Revenue => Side::Credit,
        }
    }
}

// ── Side ───────────────────────────────────────────────────────────────

/// Debit or credit side of a ledger entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Left side of the T-account.
    Debit,
    /// Right side of the T-account.
    Credit,
}

impl Side {
    /// Return the opposite side.
    pub fn opposite(self) -> Self {
        match self {
            Self::Debit => Self::Credit,
            Self::Credit => Self::Debit,
        }
    }
}

// ── Chart of Accounts ──────────────────────────────────────────────────

/// Standard GL account codes with integer discriminants matching the code number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GlAccountCode {
    /// 1000 — Cash and cash equivalents.
    Cash = 1000,
    /// 1100 — Accounts Receivable.
    AccountsReceivable = 1100,
    /// 2000 — Accounts Payable.
    AccountsPayable = 2000,
    /// 2100 — Accrued Expenses.
    AccruedExpenses = 2100,
    /// 3000 — Founder Capital / Retained Earnings.
    FounderCapital = 3000,
    /// 4000 — Revenue.
    Revenue = 4000,
    /// 5000 — Operating Expenses.
    OperatingExpenses = 5000,
    /// 5100 — Cost of Goods Sold.
    Cogs = 5100,
}

impl GlAccountCode {
    /// Return the accounting category for this GL code.
    pub fn account_type(self) -> AccountType {
        match self {
            Self::Cash | Self::AccountsReceivable => AccountType::Asset,
            Self::AccountsPayable | Self::AccruedExpenses => AccountType::Liability,
            Self::FounderCapital => AccountType::Equity,
            Self::Revenue => AccountType::Revenue,
            Self::OperatingExpenses | Self::Cogs => AccountType::Expense,
        }
    }

    /// The numeric code (e.g. 1000, 2000).
    pub fn code(self) -> u16 {
        self as u16
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Cash => "Cash",
            Self::AccountsReceivable => "Accounts Receivable",
            Self::AccountsPayable => "Accounts Payable",
            Self::AccruedExpenses => "Accrued Expenses",
            Self::FounderCapital => "Founder Capital / Retained Earnings",
            Self::Revenue => "Revenue",
            Self::OperatingExpenses => "Operating Expenses",
            Self::Cogs => "Cost of Goods Sold",
        }
    }
}

// ── Bank Account Types ─────────────────────────────────────────────────

/// Type of bank account held by the entity.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankAccountType {
    /// Standard checking account.
    #[default]
    Checking,
    /// Savings account.
    Savings,
}

// ── Payment Method ─────────────────────────────────────────────────────

/// How a payment is made or received.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    /// Bank-to-bank transfer.
    BankTransfer,
    /// Credit or debit card.
    Card,
    /// Paper check.
    Check,
    /// Wire transfer.
    Wire,
    /// ACH transfer.
    Ach,
}

// ── Bank Account Status ────────────────────────────────────────────────

/// Lifecycle status of a bank account connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankAccountStatus {
    /// Awaiting review / compliance check.
    PendingReview,
    /// Active and usable.
    Active,
    /// Permanently closed.
    Closed,
}

impl fmt::Display for BankAccountStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PendingReview => write!(f, "pending_review"),
            Self::Active => write!(f, "active"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

// ── KYB Status ─────────────────────────────────────────────────────────

/// Know Your Business verification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KybStatus {
    /// Submitted, awaiting verification.
    PendingVerification,
    /// All documents collected.
    Complete,
    /// Identity and business verified.
    Verified,
    /// Verification failed.
    Rejected,
}

impl fmt::Display for KybStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PendingVerification => write!(f, "pending_verification"),
            Self::Complete => write!(f, "complete"),
            Self::Verified => write!(f, "verified"),
            Self::Rejected => write!(f, "rejected"),
        }
    }
}

// ── Invoice Status ─────────────────────────────────────────────────────

/// Lifecycle status of an invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    /// Not yet sent.
    Draft,
    /// Delivered to the recipient.
    Sent,
    /// Payment received.
    Paid,
    /// Cancelled / voided.
    Voided,
}

impl fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Sent => write!(f, "sent"),
            Self::Paid => write!(f, "paid"),
            Self::Voided => write!(f, "voided"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents_arithmetic() {
        let a = Cents::new(1000);
        let b = Cents::new(500);
        assert_eq!((a + b).raw(), 1500);
        assert_eq!((a - b).raw(), 500);
        assert_eq!((-a).raw(), -1000);
    }

    #[test]
    fn cents_add_assign() {
        let mut a = Cents::new(100);
        a += Cents::new(50);
        assert_eq!(a.raw(), 150);
    }

    #[test]
    fn cents_sub_assign() {
        let mut a = Cents::new(200);
        a -= Cents::new(75);
        assert_eq!(a.raw(), 125);
    }

    #[test]
    fn cents_scale() {
        let price = Cents::new(500);
        assert_eq!(price.scale(3).raw(), 1500);
        assert_eq!(price.scale(0).raw(), 0);
        assert_eq!(price.scale(-2).raw(), -1000);
    }

    #[test]
    fn cents_display() {
        assert_eq!(Cents::new(12345).to_string(), "$123.45");
        assert_eq!(Cents::new(-500).to_string(), "-$5.00");
        assert_eq!(Cents::ZERO.to_string(), "$0.00");
        assert_eq!(Cents::new(1).to_string(), "$0.01");
        assert_eq!(Cents::new(-1).to_string(), "-$0.01");
    }

    #[test]
    fn cents_to_dollars() {
        let c = Cents::new(12345);
        let d = c.to_dollars();
        assert_eq!(d.to_string(), "123.45");
    }

    #[test]
    fn cents_predicates() {
        assert!(Cents::new(1).is_positive());
        assert!(!Cents::new(1).is_negative());
        assert!(!Cents::new(1).is_zero());
        assert!(Cents::ZERO.is_zero());
        assert!(Cents::new(-1).is_negative());
    }

    #[test]
    fn cents_serde_roundtrip() {
        let c = Cents::new(42);
        let json = serde_json::to_string(&c).expect("serialize Cents");
        assert_eq!(json, "42");
        let parsed: Cents = serde_json::from_str(&json).expect("deserialize Cents");
        assert_eq!(c, parsed);
    }

    #[test]
    fn normal_balance_correctness() {
        assert_eq!(AccountType::Asset.normal_balance(), Side::Debit);
        assert_eq!(AccountType::Expense.normal_balance(), Side::Debit);
        assert_eq!(AccountType::Liability.normal_balance(), Side::Credit);
        assert_eq!(AccountType::Equity.normal_balance(), Side::Credit);
        assert_eq!(AccountType::Revenue.normal_balance(), Side::Credit);
    }

    #[test]
    fn side_opposite() {
        assert_eq!(Side::Debit.opposite(), Side::Credit);
        assert_eq!(Side::Credit.opposite(), Side::Debit);
    }

    #[test]
    fn ledger_amount_signed() {
        let debit = LedgerAmount::debit(Cents::new(100));
        let credit = LedgerAmount::credit(Cents::new(100));
        assert_eq!(debit.signed(), 100);
        assert_eq!(credit.signed(), -100);
    }

    #[test]
    fn ledger_amount_accessors() {
        let amt = LedgerAmount::debit(Cents::new(250));
        assert_eq!(amt.cents(), Cents::new(250));
        assert_eq!(amt.side(), Side::Debit);
    }

    #[test]
    fn gl_account_code_roundtrip() {
        assert_eq!(GlAccountCode::Cash.code(), 1000);
        assert_eq!(GlAccountCode::Cogs.code(), 5100);
        assert_eq!(GlAccountCode::Cash.label(), "Cash");
    }

    #[test]
    fn bank_account_type_default() {
        assert_eq!(BankAccountType::default(), BankAccountType::Checking);
    }

    #[test]
    fn currency_default() {
        assert_eq!(Currency::default(), Currency::Usd);
    }

    #[test]
    fn invoice_status_serde() {
        let status = InvoiceStatus::Draft;
        let json = serde_json::to_string(&status).expect("serialize InvoiceStatus");
        assert_eq!(json, "\"draft\"");
        let parsed: InvoiceStatus = serde_json::from_str(&json).expect("deserialize InvoiceStatus");
        assert_eq!(status, parsed);
    }

    #[test]
    fn cannot_add_cents_to_non_cents() {
        // This test documents that the type system prevents unit confusion.
        // The following would not compile:
        // let cents = Cents::new(100);
        // let shares = crate::domain::equity::types::ShareCount::new(10);
        // let _ = cents + shares; // ERROR: mismatched types
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn cents_addition_overflow() {
        let _ = Cents::new(i64::MAX) + Cents::new(1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn cents_subtraction_overflow() {
        let _ = Cents::new(i64::MIN) - Cents::new(1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn cents_negation_overflow() {
        let _ = -Cents::new(i64::MIN);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn cents_add_assign_overflow() {
        let mut c = Cents::new(i64::MAX);
        c += Cents::new(1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn cents_sub_assign_overflow() {
        let mut c = Cents::new(i64::MIN);
        c -= Cents::new(1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn cents_scale_overflow() {
        let _ = Cents::new(i64::MAX).scale(2);
    }
}
