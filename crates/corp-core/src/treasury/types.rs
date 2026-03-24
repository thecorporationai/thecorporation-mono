//! Core treasury value types and enumerations.

use std::fmt;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

// ── Cents ─────────────────────────────────────────────────────────────────────

/// A monetary amount stored as whole US cents ($1 = 100 cents).
///
/// Arithmetic is performed on the raw `i64`, preserving sign. Use
/// [`Cents::checked_add`] / [`Cents::checked_sub`] when overflow matters.
///
/// ```
/// use corp_core::treasury::types::Cents;
///
/// let fee = Cents::new(1_50);   // $1.50
/// let tax = Cents::new(10);     // $0.10
/// assert_eq!((fee + tax).to_dollars(), 1.6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Cents(i64);

impl Cents {
    pub const ZERO: Cents = Cents(0);

    /// Wrap a raw cent value.
    #[inline]
    pub fn new(raw: i64) -> Self {
        Self(raw)
    }

    /// Return the underlying `i64`.
    #[inline]
    pub fn raw(self) -> i64 {
        self.0
    }

    /// Absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    #[inline]
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Convert to dollars as `f64`.
    #[inline]
    pub fn to_dollars(self) -> f64 {
        self.0 as f64 / 100.0
    }

    pub fn checked_add(self, rhs: Cents) -> Option<Cents> {
        self.0.checked_add(rhs.0).map(Cents)
    }

    pub fn checked_sub(self, rhs: Cents) -> Option<Cents> {
        self.0.checked_sub(rhs.0).map(Cents)
    }
}

impl Add for Cents {
    type Output = Cents;
    fn add(self, rhs: Cents) -> Cents {
        Cents(self.0 + rhs.0)
    }
}

impl Sub for Cents {
    type Output = Cents;
    fn sub(self, rhs: Cents) -> Cents {
        Cents(self.0 - rhs.0)
    }
}

impl Neg for Cents {
    type Output = Cents;
    fn neg(self) -> Cents {
        Cents(-self.0)
    }
}

impl AddAssign for Cents {
    fn add_assign(&mut self, rhs: Cents) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Cents {
    fn sub_assign(&mut self, rhs: Cents) {
        self.0 -= rhs.0;
    }
}

impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.0.unsigned_abs();
        let dollars = abs / 100;
        let cents = abs % 100;
        if self.0 < 0 {
            write!(f, "-${dollars}.{cents:02}")
        } else {
            write!(f, "${dollars}.{cents:02}")
        }
    }
}

// ── Currency ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Currency {
    #[default]
    Usd,
}

// ── AccountType ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

// ── Side ──────────────────────────────────────────────────────────────────────

/// A debit or credit side of a double-entry bookkeeping line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Debit,
    Credit,
}

impl Side {
    /// Return the opposite side.
    pub fn opposite(self) -> Side {
        match self {
            Side::Debit => Side::Credit,
            Side::Credit => Side::Debit,
        }
    }
}

// ── GlAccountCode ─────────────────────────────────────────────────────────────

/// A chart-of-accounts code representing a standard general ledger account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlAccountCode {
    Cash,
    AccountsReceivable,
    AccountsPayable,
    AccruedExpenses,
    FounderCapital,
    Revenue,
    OperatingExpenses,
    Cogs,
}

impl GlAccountCode {
    /// The `AccountType` that this GL account belongs to.
    pub fn account_type(self) -> AccountType {
        match self {
            GlAccountCode::Cash => AccountType::Asset,
            GlAccountCode::AccountsReceivable => AccountType::Asset,
            GlAccountCode::AccountsPayable => AccountType::Liability,
            GlAccountCode::AccruedExpenses => AccountType::Liability,
            GlAccountCode::FounderCapital => AccountType::Equity,
            GlAccountCode::Revenue => AccountType::Revenue,
            GlAccountCode::OperatingExpenses => AccountType::Expense,
            GlAccountCode::Cogs => AccountType::Expense,
        }
    }

    /// The numeric chart-of-accounts code.
    pub fn code(self) -> u32 {
        match self {
            GlAccountCode::Cash => 1000,
            GlAccountCode::AccountsReceivable => 1100,
            GlAccountCode::AccountsPayable => 2000,
            GlAccountCode::AccruedExpenses => 2100,
            GlAccountCode::FounderCapital => 3000,
            GlAccountCode::Revenue => 4000,
            GlAccountCode::OperatingExpenses => 5000,
            GlAccountCode::Cogs => 5100,
        }
    }

    /// Human-readable account label.
    pub fn label(self) -> &'static str {
        match self {
            GlAccountCode::Cash => "Cash",
            GlAccountCode::AccountsReceivable => "Accounts Receivable",
            GlAccountCode::AccountsPayable => "Accounts Payable",
            GlAccountCode::AccruedExpenses => "Accrued Expenses",
            GlAccountCode::FounderCapital => "Founder Capital",
            GlAccountCode::Revenue => "Revenue",
            GlAccountCode::OperatingExpenses => "Operating Expenses",
            GlAccountCode::Cogs => "Cost of Goods Sold",
        }
    }

    /// The normal balance side for this account type.
    ///
    /// Assets and Expenses have a debit normal balance; Liabilities, Equity,
    /// and Revenue have a credit normal balance.
    pub fn normal_balance(self) -> Side {
        match self.account_type() {
            AccountType::Asset | AccountType::Expense => Side::Debit,
            AccountType::Liability | AccountType::Equity | AccountType::Revenue => Side::Credit,
        }
    }
}

// ── BankAccountType ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankAccountType {
    Checking,
    Savings,
}

// ── BankAccountStatus ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankAccountStatus {
    PendingReview,
    Active,
    Closed,
}

// ── PaymentMethod ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    BankTransfer,
    Card,
    Check,
    Wire,
    Ach,
}

// ── InvoiceStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Paid,
    Voided,
}

// ── PayrollStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayrollStatus {
    Draft,
    Approved,
    Processed,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Cents::new / raw ──────────────────────────────────────────────────────

    #[test]
    fn cents_new_and_raw() {
        assert_eq!(Cents::new(0).raw(), 0);
        assert_eq!(Cents::new(123).raw(), 123);
        assert_eq!(Cents::new(-456).raw(), -456);
        assert_eq!(Cents::new(i64::MAX).raw(), i64::MAX);
        assert_eq!(Cents::new(i64::MIN).raw(), i64::MIN);
    }

    #[test]
    fn cents_zero_constant() {
        assert_eq!(Cents::ZERO.raw(), 0);
        assert!(Cents::ZERO.is_zero());
    }

    // ── Cents::abs ────────────────────────────────────────────────────────────

    #[test]
    fn cents_abs() {
        assert_eq!(Cents::new(150).abs(), Cents::new(150));
        assert_eq!(Cents::new(-150).abs(), Cents::new(150));
        assert_eq!(Cents::new(0).abs(), Cents::new(0));
    }

    // ── Cents::is_negative/zero/positive ──────────────────────────────────────

    #[test]
    fn cents_predicates() {
        assert!(Cents::new(-1).is_negative());
        assert!(!Cents::new(0).is_negative());
        assert!(!Cents::new(1).is_negative());

        assert!(Cents::new(0).is_zero());
        assert!(!Cents::new(1).is_zero());
        assert!(!Cents::new(-1).is_zero());

        assert!(Cents::new(1).is_positive());
        assert!(!Cents::new(0).is_positive());
        assert!(!Cents::new(-1).is_positive());
    }

    // ── Cents::to_dollars ────────────────────────────────────────────────────

    #[test]
    fn cents_to_dollars() {
        assert!((Cents::new(100).to_dollars() - 1.0).abs() < f64::EPSILON);
        assert!((Cents::new(0).to_dollars() - 0.0).abs() < f64::EPSILON);
        assert!((Cents::new(150).to_dollars() - 1.5).abs() < f64::EPSILON);
        assert!((Cents::new(-200).to_dollars() - (-2.0)).abs() < f64::EPSILON);
        assert!((Cents::new(123456).to_dollars() - 1234.56).abs() < 1e-9);
    }

    // ── Cents arithmetic ──────────────────────────────────────────────────────

    #[test]
    fn cents_display() {
        assert_eq!(Cents::new(100).to_string(), "$1.00");
        assert_eq!(Cents::new(150).to_string(), "$1.50");
        assert_eq!(Cents::new(0).to_string(), "$0.00");
        assert_eq!(Cents::new(-150).to_string(), "-$1.50");
        assert_eq!(Cents::new(123).to_string(), "$1.23");
        assert_eq!(Cents::new(-567).to_string(), "-$5.67");
        assert_eq!(Cents::new(123456).to_string(), "$1234.56");
    }

    #[test]
    fn cents_arithmetic() {
        let a = Cents::new(200);
        let b = Cents::new(50);
        assert_eq!((a + b).raw(), 250);
        assert_eq!((a - b).raw(), 150);
        assert_eq!((-a).raw(), -200);
    }

    #[test]
    fn cents_add_negative() {
        let a = Cents::new(100);
        let b = Cents::new(-30);
        assert_eq!((a + b).raw(), 70);
    }

    #[test]
    fn cents_sub_to_negative() {
        let a = Cents::new(50);
        let b = Cents::new(100);
        assert_eq!((a - b).raw(), -50);
    }

    #[test]
    fn cents_neg_zero() {
        assert_eq!((-Cents::ZERO).raw(), 0);
    }

    #[test]
    fn cents_assign_ops() {
        let mut x = Cents::new(100);
        x += Cents::new(50);
        assert_eq!(x.raw(), 150);
        x -= Cents::new(30);
        assert_eq!(x.raw(), 120);
    }

    #[test]
    fn cents_add_assign_negative() {
        let mut x = Cents::new(100);
        x += Cents::new(-150);
        assert_eq!(x.raw(), -50);
    }

    #[test]
    fn cents_sub_assign_negative() {
        let mut x = Cents::new(50);
        x -= Cents::new(-50);
        assert_eq!(x.raw(), 100);
    }

    // ── Cents::checked_add/sub ────────────────────────────────────────────────

    #[test]
    fn cents_checked() {
        assert_eq!(Cents::new(i64::MAX).checked_add(Cents::new(1)), None);
        assert_eq!(
            Cents::new(10).checked_sub(Cents::new(3)),
            Some(Cents::new(7))
        );
    }

    #[test]
    fn cents_checked_add_normal() {
        assert_eq!(
            Cents::new(100).checked_add(Cents::new(23)),
            Some(Cents::new(123))
        );
    }

    #[test]
    fn cents_checked_sub_underflow() {
        assert_eq!(Cents::new(i64::MIN).checked_sub(Cents::new(1)), None);
    }

    #[test]
    fn cents_checked_sub_to_zero() {
        assert_eq!(
            Cents::new(50).checked_sub(Cents::new(50)),
            Some(Cents::ZERO)
        );
    }

    #[test]
    fn cents_checked_add_zero() {
        assert_eq!(
            Cents::new(42).checked_add(Cents::ZERO),
            Some(Cents::new(42))
        );
    }

    // ── Cents ordering ────────────────────────────────────────────────────────

    #[test]
    fn cents_ordering() {
        assert!(Cents::new(-1) < Cents::new(0));
        assert!(Cents::new(0) < Cents::new(1));
        assert!(Cents::new(100) < Cents::new(200));
        assert_eq!(Cents::new(50), Cents::new(50));
    }

    // ── GlAccountCode ─────────────────────────────────────────────────────────

    #[test]
    fn gl_account_code_properties() {
        assert_eq!(GlAccountCode::Cash.code(), 1000);
        assert_eq!(GlAccountCode::Cash.account_type(), AccountType::Asset);
        assert_eq!(GlAccountCode::Cash.normal_balance(), Side::Debit);
        assert_eq!(GlAccountCode::Revenue.normal_balance(), Side::Credit);
        assert_eq!(
            GlAccountCode::AccountsPayable.account_type(),
            AccountType::Liability
        );
    }

    #[test]
    fn gl_all_codes_and_labels() {
        let cases = [
            (
                GlAccountCode::Cash,
                1000,
                "Cash",
                AccountType::Asset,
                Side::Debit,
            ),
            (
                GlAccountCode::AccountsReceivable,
                1100,
                "Accounts Receivable",
                AccountType::Asset,
                Side::Debit,
            ),
            (
                GlAccountCode::AccountsPayable,
                2000,
                "Accounts Payable",
                AccountType::Liability,
                Side::Credit,
            ),
            (
                GlAccountCode::AccruedExpenses,
                2100,
                "Accrued Expenses",
                AccountType::Liability,
                Side::Credit,
            ),
            (
                GlAccountCode::FounderCapital,
                3000,
                "Founder Capital",
                AccountType::Equity,
                Side::Credit,
            ),
            (
                GlAccountCode::Revenue,
                4000,
                "Revenue",
                AccountType::Revenue,
                Side::Credit,
            ),
            (
                GlAccountCode::OperatingExpenses,
                5000,
                "Operating Expenses",
                AccountType::Expense,
                Side::Debit,
            ),
            (
                GlAccountCode::Cogs,
                5100,
                "Cost of Goods Sold",
                AccountType::Expense,
                Side::Debit,
            ),
        ];
        for (code, expected_num, expected_label, expected_type, expected_normal) in cases {
            assert_eq!(code.code(), expected_num, "code mismatch for {:?}", code);
            assert_eq!(
                code.label(),
                expected_label,
                "label mismatch for {:?}",
                code
            );
            assert_eq!(
                code.account_type(),
                expected_type,
                "account_type mismatch for {:?}",
                code
            );
            assert_eq!(
                code.normal_balance(),
                expected_normal,
                "normal_balance mismatch for {:?}",
                code
            );
        }
    }

    // ── Side::opposite ────────────────────────────────────────────────────────

    #[test]
    fn side_opposite() {
        assert_eq!(Side::Debit.opposite(), Side::Credit);
        assert_eq!(Side::Credit.opposite(), Side::Debit);
    }

    #[test]
    fn side_opposite_twice_is_identity() {
        assert_eq!(Side::Debit.opposite().opposite(), Side::Debit);
        assert_eq!(Side::Credit.opposite().opposite(), Side::Credit);
    }

    // ── Serde roundtrips ──────────────────────────────────────────────────────

    #[test]
    fn currency_serde_roundtrip() {
        let v = Currency::Usd;
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, r#""usd""#);
        let de: Currency = serde_json::from_str(&s).unwrap();
        assert_eq!(de, v);
    }

    #[test]
    fn account_type_serde_roundtrip() {
        for variant in [
            AccountType::Asset,
            AccountType::Liability,
            AccountType::Equity,
            AccountType::Revenue,
            AccountType::Expense,
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: AccountType = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
    }

    #[test]
    fn account_type_serde_values() {
        assert_eq!(
            serde_json::to_string(&AccountType::Asset).unwrap(),
            r#""asset""#
        );
        assert_eq!(
            serde_json::to_string(&AccountType::Liability).unwrap(),
            r#""liability""#
        );
        assert_eq!(
            serde_json::to_string(&AccountType::Equity).unwrap(),
            r#""equity""#
        );
        assert_eq!(
            serde_json::to_string(&AccountType::Revenue).unwrap(),
            r#""revenue""#
        );
        assert_eq!(
            serde_json::to_string(&AccountType::Expense).unwrap(),
            r#""expense""#
        );
    }

    #[test]
    fn side_serde_roundtrip() {
        for variant in [Side::Debit, Side::Credit] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: Side = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
        assert_eq!(serde_json::to_string(&Side::Debit).unwrap(), r#""debit""#);
        assert_eq!(serde_json::to_string(&Side::Credit).unwrap(), r#""credit""#);
    }

    #[test]
    fn bank_account_type_serde_roundtrip() {
        for variant in [BankAccountType::Checking, BankAccountType::Savings] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: BankAccountType = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
        assert_eq!(
            serde_json::to_string(&BankAccountType::Checking).unwrap(),
            r#""checking""#
        );
        assert_eq!(
            serde_json::to_string(&BankAccountType::Savings).unwrap(),
            r#""savings""#
        );
    }

    #[test]
    fn bank_account_status_serde_roundtrip() {
        for variant in [
            BankAccountStatus::PendingReview,
            BankAccountStatus::Active,
            BankAccountStatus::Closed,
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: BankAccountStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
        assert_eq!(
            serde_json::to_string(&BankAccountStatus::PendingReview).unwrap(),
            r#""pending_review""#
        );
        assert_eq!(
            serde_json::to_string(&BankAccountStatus::Active).unwrap(),
            r#""active""#
        );
        assert_eq!(
            serde_json::to_string(&BankAccountStatus::Closed).unwrap(),
            r#""closed""#
        );
    }

    #[test]
    fn payment_method_serde_roundtrip() {
        for variant in [
            PaymentMethod::BankTransfer,
            PaymentMethod::Card,
            PaymentMethod::Check,
            PaymentMethod::Wire,
            PaymentMethod::Ach,
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: PaymentMethod = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
        assert_eq!(
            serde_json::to_string(&PaymentMethod::BankTransfer).unwrap(),
            r#""bank_transfer""#
        );
        assert_eq!(
            serde_json::to_string(&PaymentMethod::Ach).unwrap(),
            r#""ach""#
        );
    }

    #[test]
    fn invoice_status_serde_roundtrip() {
        for variant in [
            InvoiceStatus::Draft,
            InvoiceStatus::Sent,
            InvoiceStatus::Paid,
            InvoiceStatus::Voided,
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: InvoiceStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
        assert_eq!(
            serde_json::to_string(&InvoiceStatus::Draft).unwrap(),
            r#""draft""#
        );
        assert_eq!(
            serde_json::to_string(&InvoiceStatus::Voided).unwrap(),
            r#""voided""#
        );
    }

    #[test]
    fn payroll_status_serde_roundtrip() {
        for variant in [
            PayrollStatus::Draft,
            PayrollStatus::Approved,
            PayrollStatus::Processed,
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: PayrollStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
        assert_eq!(
            serde_json::to_string(&PayrollStatus::Draft).unwrap(),
            r#""draft""#
        );
        assert_eq!(
            serde_json::to_string(&PayrollStatus::Approved).unwrap(),
            r#""approved""#
        );
        assert_eq!(
            serde_json::to_string(&PayrollStatus::Processed).unwrap(),
            r#""processed""#
        );
    }

    #[test]
    fn cents_serde_roundtrip() {
        let c = Cents::new(1234);
        let s = serde_json::to_string(&c).unwrap();
        let de: Cents = serde_json::from_str(&s).unwrap();
        assert_eq!(c, de);
    }

    #[test]
    fn cents_default_is_zero() {
        assert_eq!(Cents::default(), Cents::ZERO);
    }
}
