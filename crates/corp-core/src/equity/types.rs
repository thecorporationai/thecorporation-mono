//! Core equity value types and enumerations.

use serde::{Deserialize, Serialize};

// ── Numeric value types ───────────────────────────────────────────────────────

/// A count of shares. Wraps `i64` to allow arithmetic while preserving type
/// safety. Use `require_positive` when a non-zero, non-negative value is
/// needed (e.g. when issuing a grant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct ShareCount(i64);

impl ShareCount {
    pub const ZERO: ShareCount = ShareCount(0);

    /// Wrap a raw `i64` value.
    #[inline]
    pub fn new(n: i64) -> Self {
        Self(n)
    }

    /// Return the underlying `i64`.
    #[inline]
    pub fn raw(self) -> i64 {
        self.0
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn checked_add(self, rhs: ShareCount) -> Option<ShareCount> {
        self.0.checked_add(rhs.0).map(ShareCount)
    }

    pub fn checked_sub(self, rhs: ShareCount) -> Option<ShareCount> {
        self.0.checked_sub(rhs.0).map(ShareCount)
    }

    /// Return `Ok(self)` if the value is strictly greater than zero.
    pub fn require_positive(self) -> Result<ShareCount, ShareCountError> {
        if self.0 > 0 {
            Ok(self)
        } else {
            Err(ShareCountError::NotPositive(self.0))
        }
    }
}

impl std::ops::Add for ShareCount {
    type Output = ShareCount;
    fn add(self, rhs: ShareCount) -> ShareCount {
        ShareCount(self.0 + rhs.0)
    }
}

impl std::ops::Sub for ShareCount {
    type Output = ShareCount;
    fn sub(self, rhs: ShareCount) -> ShareCount {
        ShareCount(self.0 - rhs.0)
    }
}

impl std::iter::Sum for ShareCount {
    fn sum<I: Iterator<Item = ShareCount>>(iter: I) -> ShareCount {
        iter.fold(ShareCount::ZERO, |acc, x| acc + x)
    }
}

impl std::fmt::Display for ShareCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Error produced by [`ShareCount::require_positive`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShareCountError {
    #[error("share count must be positive, got {0}")]
    NotPositive(i64),
}

// ── Monetary primitives ───────────────────────────────────────────────────────

/// A monetary amount stored as whole cents (i.e. USD × 100).
pub type Cents = i64;

/// Price of a single share, expressed in whole cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PricePerShare(Cents);

impl PricePerShare {
    #[inline]
    pub fn new(cents: Cents) -> Self {
        Self(cents)
    }

    #[inline]
    pub fn as_cents(self) -> Cents {
        self.0
    }
}

/// Maximum company valuation used in a SAFE conversion, expressed in whole
/// cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ValuationCap(Cents);

impl ValuationCap {
    #[inline]
    pub fn new(cents: Cents) -> Self {
        Self(cents)
    }

    #[inline]
    pub fn as_cents(self) -> Cents {
        self.0
    }
}

// ── Percentage ────────────────────────────────────────────────────────────────

/// A percentage stored as basis points (0 – 10 000, where 10 000 = 100%).
///
/// ```
/// use corp_core::equity::types::Percentage;
///
/// let half = Percentage::new(5000).unwrap();
/// assert!((half.to_decimal() - 0.5).abs() < f64::EPSILON);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Percentage(u32);

impl Percentage {
    /// 0%
    pub const ZERO: Percentage = Percentage(0);
    /// 100%
    pub const ONE_HUNDRED: Percentage = Percentage(10_000);

    /// Construct from basis points. Returns `Err` if `bp > 10_000`.
    pub fn new(bp: u32) -> Result<Self, PercentageError> {
        if bp > 10_000 {
            Err(PercentageError::OutOfRange(bp))
        } else {
            Ok(Self(bp))
        }
    }

    /// Return the underlying basis-point value.
    #[inline]
    pub fn basis_points(self) -> u32 {
        self.0
    }

    /// Convert to a `f64` in the range `[0.0, 1.0]`.
    #[inline]
    pub fn to_decimal(self) -> f64 {
        f64::from(self.0) / 10_000.0
    }
}

/// Error produced by [`Percentage::new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PercentageError {
    #[error("percentage basis points {0} exceeds maximum of 10000")]
    OutOfRange(u32),
}

// ── VotingRights ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VotingRights {
    Unspecified,
    Granted,
    Withheld,
}

// ── Enumerations ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockType {
    Common,
    Preferred,
    MembershipUnit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantType {
    CommonStock,
    PreferredStock,
    MembershipUnit,
    StockOption,
    Iso,
    Nso,
    Rsa,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantStatus {
    Issued,
    Vested,
    Exercised,
    Forfeited,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipientType {
    NaturalPerson,
    Investor,
    Entity,
    Transferee,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeType {
    PostMoney,
    PreMoney,
    Mfn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeStatus {
    Issued,
    Converted,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationType {
    FourOhNineA,
    FairMarketValue,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationStatus {
    Draft,
    PendingApproval,
    Approved,
    Expired,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationMethodology {
    Income,
    Market,
    Asset,
    Backsolve,
    Hybrid,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferType {
    Gift,
    TrustTransfer,
    SecondarySale,
    Estate,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Draft,
    PendingBoardApproval,
    Approved,
    Executed,
    Denied,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FundingRoundStatus {
    TermSheet,
    Diligence,
    Closing,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapTableStatus {
    Active,
    Frozen,
}

// ── Vesting ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VestingStatus {
    Active,
    Terminated,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VestingEventType {
    Cliff,
    Monthly,
    Manual,
    Acceleration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VestingEventStatus {
    Scheduled,
    Vested,
    Forfeited,
    Cancelled,
}

// ── Position ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionStatus {
    Active,
    Closed,
}

// ── Investor ledger ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvestorLedgerEntryType {
    SafeInvestment,
    PricedRoundInvestment,
    SafeConversion,
    ProRataExercise,
}

// ── Repurchase ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepurchaseStatus {
    Pending,
    Active,
    Closed,
    Waived,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ShareCount ────────────────────────────────────────────────────────────

    #[test]
    fn share_count_new_and_raw() {
        let sc = ShareCount::new(42);
        assert_eq!(sc.raw(), 42);
    }

    #[test]
    fn share_count_zero_constant() {
        assert_eq!(ShareCount::ZERO.raw(), 0);
        assert!(ShareCount::ZERO.is_zero());
    }

    #[test]
    fn share_count_is_zero_true() {
        assert!(ShareCount::new(0).is_zero());
    }

    #[test]
    fn share_count_is_zero_false() {
        assert!(!ShareCount::new(1).is_zero());
    }

    #[test]
    fn share_count_is_zero_negative() {
        assert!(!ShareCount::new(-1).is_zero());
    }

    #[test]
    fn share_count_add() {
        let a = ShareCount::new(100);
        let b = ShareCount::new(50);
        assert_eq!((a + b).raw(), 150);
    }

    #[test]
    fn share_count_sub() {
        let a = ShareCount::new(100);
        let b = ShareCount::new(30);
        assert_eq!((a - b).raw(), 70);
    }

    #[test]
    fn share_count_sub_goes_negative() {
        let a = ShareCount::new(10);
        let b = ShareCount::new(20);
        assert_eq!((a - b).raw(), -10);
    }

    #[test]
    fn share_count_sum_iterator() {
        let counts = vec![
            ShareCount::new(100),
            ShareCount::new(200),
            ShareCount::new(300),
        ];
        let total: ShareCount = counts.into_iter().sum();
        assert_eq!(total.raw(), 600);
    }

    #[test]
    fn share_count_sum_empty_iterator() {
        let counts: Vec<ShareCount> = vec![];
        let total: ShareCount = counts.into_iter().sum();
        assert_eq!(total.raw(), 0);
    }

    #[test]
    fn share_count_checked_add_no_overflow() {
        let a = ShareCount::new(i64::MAX - 1);
        let b = ShareCount::new(1);
        assert_eq!(a.checked_add(b).unwrap().raw(), i64::MAX);
    }

    #[test]
    fn share_count_checked_add_overflow_returns_none() {
        let a = ShareCount::new(i64::MAX);
        let b = ShareCount::new(1);
        assert!(a.checked_add(b).is_none());
    }

    #[test]
    fn share_count_checked_sub_no_underflow() {
        let a = ShareCount::new(i64::MIN + 1);
        let b = ShareCount::new(1);
        assert_eq!(a.checked_sub(b).unwrap().raw(), i64::MIN);
    }

    #[test]
    fn share_count_checked_sub_underflow_returns_none() {
        let a = ShareCount::new(i64::MIN);
        let b = ShareCount::new(1);
        assert!(a.checked_sub(b).is_none());
    }

    #[test]
    fn share_count_require_positive_with_positive_value() {
        assert!(ShareCount::new(1).require_positive().is_ok());
        assert!(ShareCount::new(1000).require_positive().is_ok());
    }

    #[test]
    fn share_count_require_positive_with_zero_fails() {
        let err = ShareCount::new(0).require_positive().unwrap_err();
        assert_eq!(err, ShareCountError::NotPositive(0));
    }

    #[test]
    fn share_count_require_positive_with_negative_fails() {
        let err = ShareCount::new(-1).require_positive().unwrap_err();
        assert_eq!(err, ShareCountError::NotPositive(-1));
    }

    #[test]
    fn share_count_display() {
        assert_eq!(ShareCount::new(42).to_string(), "42");
    }

    #[test]
    fn share_count_serde_roundtrip() {
        let sc = ShareCount::new(500_000);
        let json = serde_json::to_string(&sc).unwrap();
        let de: ShareCount = serde_json::from_str(&json).unwrap();
        assert_eq!(de, sc);
    }

    #[test]
    fn share_count_ordering() {
        assert!(ShareCount::new(10) < ShareCount::new(20));
        assert!(ShareCount::new(20) > ShareCount::new(10));
        assert_eq!(ShareCount::new(5), ShareCount::new(5));
    }

    // ── Percentage ────────────────────────────────────────────────────────────

    #[test]
    fn percentage_zero_is_valid() {
        let p = Percentage::new(0).unwrap();
        assert_eq!(p.basis_points(), 0);
        assert!((p.to_decimal() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentage_5000_is_50_percent() {
        let p = Percentage::new(5000).unwrap();
        assert_eq!(p.basis_points(), 5000);
        assert!((p.to_decimal() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn percentage_10000_is_100_percent() {
        let p = Percentage::new(10_000).unwrap();
        assert_eq!(p.basis_points(), 10_000);
        assert!((p.to_decimal() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percentage_10001_is_invalid() {
        let err = Percentage::new(10_001).unwrap_err();
        assert_eq!(err, PercentageError::OutOfRange(10_001));
    }

    #[test]
    fn percentage_u32_max_is_invalid() {
        assert!(Percentage::new(u32::MAX).is_err());
    }

    #[test]
    fn percentage_serde_roundtrip() {
        let p = Percentage::new(2500).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let de: Percentage = serde_json::from_str(&json).unwrap();
        assert_eq!(de, p);
    }

    #[test]
    fn percentage_constants() {
        assert_eq!(Percentage::ZERO.basis_points(), 0);
        assert_eq!(Percentage::ONE_HUNDRED.basis_points(), 10_000);
    }

    // ── PricePerShare ─────────────────────────────────────────────────────────

    #[test]
    fn price_per_share_new_and_as_cents() {
        let p = PricePerShare::new(100);
        assert_eq!(p.as_cents(), 100);
    }

    #[test]
    fn price_per_share_zero() {
        let p = PricePerShare::new(0);
        assert_eq!(p.as_cents(), 0);
    }

    #[test]
    fn price_per_share_large_value() {
        let p = PricePerShare::new(1_000_000_00); // $1,000,000
        assert_eq!(p.as_cents(), 100_000_000);
    }

    #[test]
    fn price_per_share_serde_roundtrip() {
        let p = PricePerShare::new(50_00); // $50.00
        let json = serde_json::to_string(&p).unwrap();
        let de: PricePerShare = serde_json::from_str(&json).unwrap();
        assert_eq!(de, p);
    }

    #[test]
    fn price_per_share_ordering() {
        assert!(PricePerShare::new(100) < PricePerShare::new(200));
    }

    // ── ValuationCap ──────────────────────────────────────────────────────────

    #[test]
    fn valuation_cap_new_and_as_cents() {
        let v = ValuationCap::new(5_000_000_00); // $5M
        assert_eq!(v.as_cents(), 500_000_000);
    }

    #[test]
    fn valuation_cap_serde_roundtrip() {
        let v = ValuationCap::new(10_000_000_00);
        let json = serde_json::to_string(&v).unwrap();
        let de: ValuationCap = serde_json::from_str(&json).unwrap();
        assert_eq!(de, v);
    }

    #[test]
    fn valuation_cap_ordering() {
        assert!(ValuationCap::new(1_000) < ValuationCap::new(2_000));
    }

    // ── StockType serde ───────────────────────────────────────────────────────

    #[test]
    fn stock_type_serde_common() {
        let json = serde_json::to_string(&StockType::Common).unwrap();
        assert_eq!(json, r#""common""#);
        let de: StockType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, StockType::Common);
    }

    #[test]
    fn stock_type_serde_preferred() {
        let json = serde_json::to_string(&StockType::Preferred).unwrap();
        assert_eq!(json, r#""preferred""#);
        let de: StockType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, StockType::Preferred);
    }

    #[test]
    fn stock_type_serde_membership_unit() {
        let json = serde_json::to_string(&StockType::MembershipUnit).unwrap();
        assert_eq!(json, r#""membership_unit""#);
        let de: StockType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, StockType::MembershipUnit);
    }

    // ── GrantType serde ───────────────────────────────────────────────────────

    #[test]
    fn grant_type_serde_common_stock() {
        let json = serde_json::to_string(&GrantType::CommonStock).unwrap();
        assert_eq!(json, r#""common_stock""#);
        let de: GrantType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantType::CommonStock);
    }

    #[test]
    fn grant_type_serde_preferred_stock() {
        let json = serde_json::to_string(&GrantType::PreferredStock).unwrap();
        assert_eq!(json, r#""preferred_stock""#);
    }

    #[test]
    fn grant_type_serde_membership_unit() {
        let json = serde_json::to_string(&GrantType::MembershipUnit).unwrap();
        assert_eq!(json, r#""membership_unit""#);
    }

    #[test]
    fn grant_type_serde_stock_option() {
        let json = serde_json::to_string(&GrantType::StockOption).unwrap();
        assert_eq!(json, r#""stock_option""#);
    }

    #[test]
    fn grant_type_serde_iso() {
        let json = serde_json::to_string(&GrantType::Iso).unwrap();
        assert_eq!(json, r#""iso""#);
        let de: GrantType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantType::Iso);
    }

    #[test]
    fn grant_type_serde_nso() {
        let json = serde_json::to_string(&GrantType::Nso).unwrap();
        assert_eq!(json, r#""nso""#);
        let de: GrantType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantType::Nso);
    }

    #[test]
    fn grant_type_serde_rsa() {
        let json = serde_json::to_string(&GrantType::Rsa).unwrap();
        assert_eq!(json, r#""rsa""#);
        let de: GrantType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantType::Rsa);
    }

    // ── GrantStatus serde ─────────────────────────────────────────────────────

    #[test]
    fn grant_status_serde_issued() {
        let json = serde_json::to_string(&GrantStatus::Issued).unwrap();
        assert_eq!(json, r#""issued""#);
        let de: GrantStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantStatus::Issued);
    }

    #[test]
    fn grant_status_serde_vested() {
        let json = serde_json::to_string(&GrantStatus::Vested).unwrap();
        assert_eq!(json, r#""vested""#);
    }

    #[test]
    fn grant_status_serde_exercised() {
        let json = serde_json::to_string(&GrantStatus::Exercised).unwrap();
        assert_eq!(json, r#""exercised""#);
    }

    #[test]
    fn grant_status_serde_forfeited() {
        let json = serde_json::to_string(&GrantStatus::Forfeited).unwrap();
        assert_eq!(json, r#""forfeited""#);
    }

    #[test]
    fn grant_status_serde_cancelled() {
        let json = serde_json::to_string(&GrantStatus::Cancelled).unwrap();
        assert_eq!(json, r#""cancelled""#);
        let de: GrantStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, GrantStatus::Cancelled);
    }

    // ── SafeType serde ────────────────────────────────────────────────────────

    #[test]
    fn safe_type_serde_post_money() {
        let json = serde_json::to_string(&SafeType::PostMoney).unwrap();
        assert_eq!(json, r#""post_money""#);
        let de: SafeType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, SafeType::PostMoney);
    }

    #[test]
    fn safe_type_serde_pre_money() {
        let json = serde_json::to_string(&SafeType::PreMoney).unwrap();
        assert_eq!(json, r#""pre_money""#);
        let de: SafeType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, SafeType::PreMoney);
    }

    #[test]
    fn safe_type_serde_mfn() {
        let json = serde_json::to_string(&SafeType::Mfn).unwrap();
        assert_eq!(json, r#""mfn""#);
        let de: SafeType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, SafeType::Mfn);
    }

    // ── SafeStatus serde ──────────────────────────────────────────────────────

    #[test]
    fn safe_status_serde_issued() {
        let json = serde_json::to_string(&SafeStatus::Issued).unwrap();
        assert_eq!(json, r#""issued""#);
        let de: SafeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, SafeStatus::Issued);
    }

    #[test]
    fn safe_status_serde_converted() {
        let json = serde_json::to_string(&SafeStatus::Converted).unwrap();
        assert_eq!(json, r#""converted""#);
        let de: SafeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, SafeStatus::Converted);
    }

    #[test]
    fn safe_status_serde_cancelled() {
        let json = serde_json::to_string(&SafeStatus::Cancelled).unwrap();
        assert_eq!(json, r#""cancelled""#);
        let de: SafeStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, SafeStatus::Cancelled);
    }

    // ── ValuationType serde ───────────────────────────────────────────────────

    #[test]
    fn valuation_type_serde_four_oh_nine_a() {
        let json = serde_json::to_string(&ValuationType::FourOhNineA).unwrap();
        assert_eq!(json, r#""four_oh_nine_a""#);
        let de: ValuationType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, ValuationType::FourOhNineA);
    }

    #[test]
    fn valuation_type_serde_fair_market_value() {
        let json = serde_json::to_string(&ValuationType::FairMarketValue).unwrap();
        assert_eq!(json, r#""fair_market_value""#);
        let de: ValuationType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, ValuationType::FairMarketValue);
    }

    #[test]
    fn valuation_type_serde_other() {
        let json = serde_json::to_string(&ValuationType::Other).unwrap();
        assert_eq!(json, r#""other""#);
    }

    // ── ValuationStatus serde ─────────────────────────────────────────────────

    #[test]
    fn valuation_status_serde_draft() {
        let json = serde_json::to_string(&ValuationStatus::Draft).unwrap();
        assert_eq!(json, r#""draft""#);
        let de: ValuationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, ValuationStatus::Draft);
    }

    #[test]
    fn valuation_status_serde_pending_approval() {
        let json = serde_json::to_string(&ValuationStatus::PendingApproval).unwrap();
        assert_eq!(json, r#""pending_approval""#);
        let de: ValuationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, ValuationStatus::PendingApproval);
    }

    #[test]
    fn valuation_status_serde_approved() {
        let json = serde_json::to_string(&ValuationStatus::Approved).unwrap();
        assert_eq!(json, r#""approved""#);
    }

    #[test]
    fn valuation_status_serde_expired() {
        let json = serde_json::to_string(&ValuationStatus::Expired).unwrap();
        assert_eq!(json, r#""expired""#);
    }

    #[test]
    fn valuation_status_serde_superseded() {
        let json = serde_json::to_string(&ValuationStatus::Superseded).unwrap();
        assert_eq!(json, r#""superseded""#);
    }

    // ── TransferType serde ────────────────────────────────────────────────────

    #[test]
    fn transfer_type_serde_gift() {
        let json = serde_json::to_string(&TransferType::Gift).unwrap();
        assert_eq!(json, r#""gift""#);
        let de: TransferType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, TransferType::Gift);
    }

    #[test]
    fn transfer_type_serde_trust_transfer() {
        let json = serde_json::to_string(&TransferType::TrustTransfer).unwrap();
        assert_eq!(json, r#""trust_transfer""#);
    }

    #[test]
    fn transfer_type_serde_secondary_sale() {
        let json = serde_json::to_string(&TransferType::SecondarySale).unwrap();
        assert_eq!(json, r#""secondary_sale""#);
    }

    #[test]
    fn transfer_type_serde_estate() {
        let json = serde_json::to_string(&TransferType::Estate).unwrap();
        assert_eq!(json, r#""estate""#);
    }

    #[test]
    fn transfer_type_serde_other() {
        let json = serde_json::to_string(&TransferType::Other).unwrap();
        assert_eq!(json, r#""other""#);
    }

    // ── TransferStatus serde ──────────────────────────────────────────────────

    #[test]
    fn transfer_status_serde_draft() {
        let json = serde_json::to_string(&TransferStatus::Draft).unwrap();
        assert_eq!(json, r#""draft""#);
        let de: TransferStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, TransferStatus::Draft);
    }

    #[test]
    fn transfer_status_serde_pending_board_approval() {
        let json = serde_json::to_string(&TransferStatus::PendingBoardApproval).unwrap();
        assert_eq!(json, r#""pending_board_approval""#);
    }

    #[test]
    fn transfer_status_serde_approved() {
        let json = serde_json::to_string(&TransferStatus::Approved).unwrap();
        assert_eq!(json, r#""approved""#);
    }

    #[test]
    fn transfer_status_serde_executed() {
        let json = serde_json::to_string(&TransferStatus::Executed).unwrap();
        assert_eq!(json, r#""executed""#);
    }

    #[test]
    fn transfer_status_serde_denied() {
        let json = serde_json::to_string(&TransferStatus::Denied).unwrap();
        assert_eq!(json, r#""denied""#);
    }

    #[test]
    fn transfer_status_serde_cancelled() {
        let json = serde_json::to_string(&TransferStatus::Cancelled).unwrap();
        assert_eq!(json, r#""cancelled""#);
    }

    // ── FundingRoundStatus serde ──────────────────────────────────────────────

    #[test]
    fn funding_round_status_serde_term_sheet() {
        let json = serde_json::to_string(&FundingRoundStatus::TermSheet).unwrap();
        assert_eq!(json, r#""term_sheet""#);
        let de: FundingRoundStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FundingRoundStatus::TermSheet);
    }

    #[test]
    fn funding_round_status_serde_diligence() {
        let json = serde_json::to_string(&FundingRoundStatus::Diligence).unwrap();
        assert_eq!(json, r#""diligence""#);
        let de: FundingRoundStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FundingRoundStatus::Diligence);
    }

    #[test]
    fn funding_round_status_serde_closing() {
        let json = serde_json::to_string(&FundingRoundStatus::Closing).unwrap();
        assert_eq!(json, r#""closing""#);
    }

    #[test]
    fn funding_round_status_serde_closed() {
        let json = serde_json::to_string(&FundingRoundStatus::Closed).unwrap();
        assert_eq!(json, r#""closed""#);
        let de: FundingRoundStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FundingRoundStatus::Closed);
    }

    // ── CapTableStatus serde ──────────────────────────────────────────────────

    #[test]
    fn cap_table_status_serde_active() {
        let json = serde_json::to_string(&CapTableStatus::Active).unwrap();
        assert_eq!(json, r#""active""#);
        let de: CapTableStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, CapTableStatus::Active);
    }

    #[test]
    fn cap_table_status_serde_frozen() {
        let json = serde_json::to_string(&CapTableStatus::Frozen).unwrap();
        assert_eq!(json, r#""frozen""#);
        let de: CapTableStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, CapTableStatus::Frozen);
    }
}
