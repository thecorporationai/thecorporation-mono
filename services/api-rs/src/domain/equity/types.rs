//! Equity domain types — share counts, prices, percentages, and status enums.

use crate::domain::treasury::types::Cents;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};

// ── ShareCount ─────────────────────────────────────────────────────────

/// A count of shares. Distinct from `Cents` to prevent cross-type arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShareCount(i64);

impl ShareCount {
    /// Zero shares.
    pub const ZERO: Self = Self(0);

    /// Create a new share count.
    #[inline]
    pub const fn new(count: i64) -> Self {
        Self(count)
    }

    /// Return the raw integer value.
    #[inline]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Whether this count is exactly zero.
    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0
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
            Err("share count must be positive")
        }
    }
}

impl Add for ShareCount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_add(rhs.0)
                .expect("ShareCount addition overflow"),
        )
    }
}

impl Sub for ShareCount {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(
            self.0
                .checked_sub(rhs.0)
                .expect("ShareCount subtraction overflow"),
        )
    }
}

impl fmt::Display for ShareCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::iter::Sum for ShareCount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        // Uses the checked Add impl above, so overflow will panic.
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

// ── PositiveShareCount ─────────────────────────────────────────────────

/// A share count guaranteed to be positive (> 0).
///
/// Used in contexts where zero or negative shares are invalid (grants,
/// transfers, SAFE notes). Deserializes via `TryFrom<i64>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PositiveShareCount(ShareCount);

impl TryFrom<i64> for PositiveShareCount {
    type Error = String;
    fn try_from(v: i64) -> Result<Self, Self::Error> {
        if v <= 0 {
            Err(format!("share count must be positive, got {v}"))
        } else {
            Ok(Self(ShareCount::new(v)))
        }
    }
}

impl<'de> Deserialize<'de> for PositiveShareCount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = i64::deserialize(deserializer)?;
        PositiveShareCount::try_from(v).map_err(serde::de::Error::custom)
    }
}

impl PositiveShareCount {
    /// Create a new positive share count.
    ///
    /// Returns `Err` if `count <= 0`.
    pub fn new(count: i64) -> Result<Self, String> {
        Self::try_from(count)
    }

    /// Return the inner `ShareCount`.
    pub fn into_inner(self) -> ShareCount {
        self.0
    }

    /// Return the raw integer value.
    pub fn raw(self) -> i64 {
        self.0.raw()
    }
}

impl From<PositiveShareCount> for ShareCount {
    fn from(p: PositiveShareCount) -> Self {
        p.0
    }
}

// ── PricePerShare ──────────────────────────────────────────────────────

/// Price per share, stored as `Cents`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PricePerShare(Cents);

impl PricePerShare {
    /// Create a new price per share.
    #[inline]
    pub fn new(cents: Cents) -> Self {
        Self(cents)
    }

    /// Return the inner `Cents` value.
    #[inline]
    pub fn as_cents(self) -> Cents {
        self.0
    }
}

impl fmt::Display for PricePerShare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ── ValuationCap ───────────────────────────────────────────────────────

/// A valuation cap on a SAFE note, stored as `Cents`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValuationCap(Cents);

impl ValuationCap {
    /// Create a new valuation cap.
    #[inline]
    pub fn new(cents: Cents) -> Self {
        Self(cents)
    }

    /// Return the inner `Cents` value.
    #[inline]
    pub fn as_cents(self) -> Cents {
        self.0
    }
}

impl fmt::Display for ValuationCap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ── Percentage ─────────────────────────────────────────────────────────

/// A percentage stored as basis points (10000 = 100%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Percentage(u32);

impl TryFrom<u32> for Percentage {
    type Error = String;
    fn try_from(basis_points: u32) -> Result<Self, Self::Error> {
        if basis_points > 10_000 {
            Err(format!(
                "percentage cannot exceed 100% (10000 basis points), got {basis_points}"
            ))
        } else {
            Ok(Self(basis_points))
        }
    }
}

impl<'de> Deserialize<'de> for Percentage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = u32::deserialize(deserializer)?;
        Percentage::try_from(v).map_err(serde::de::Error::custom)
    }
}

impl Percentage {
    /// Zero percent.
    pub const ZERO: Self = Self(0);

    /// One hundred percent.
    pub const ONE_HUNDRED: Self = Self(10_000);

    /// Create a percentage from basis points (10000 = 100%).
    ///
    /// Returns `Err` if `basis_points > 10_000`.
    #[inline]
    pub fn new(basis_points: u32) -> Result<Self, String> {
        Self::try_from(basis_points)
    }

    /// Return the raw basis points value.
    #[inline]
    pub const fn basis_points(self) -> u32 {
        self.0
    }

    /// Convert to a `Decimal` fraction (e.g. 5000 bps -> 0.5000).
    pub fn to_decimal(self) -> Decimal {
        Decimal::new(self.0 as i64, 4)
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / 100;
        let frac = self.0 % 100;
        write!(f, "{whole}.{frac:02}%")
    }
}

// ── VotingRights ──────────────────────────────────────────────────────

/// Whether an equity grant carries voting rights.
///
/// Replaces `Option<bool>` for clearer semantics.
/// Backward-compatible deserialization from `Option<bool>` via `From`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VotingRights {
    /// Voting rights have not been specified.
    Unspecified,
    /// Voting rights are granted.
    Granted,
    /// Voting rights are withheld.
    Withheld,
}

impl From<Option<bool>> for VotingRights {
    fn from(v: Option<bool>) -> Self {
        match v {
            None => Self::Unspecified,
            Some(true) => Self::Granted,
            Some(false) => Self::Withheld,
        }
    }
}

impl VotingRights {
    /// Whether voting rights are granted.
    pub fn is_granted(self) -> bool {
        self == Self::Granted
    }
}

// ── Enums ──────────────────────────────────────────────────────────────

/// The class of stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockType {
    /// Common stock (C-Corp).
    Common,
    /// Preferred stock (C-Corp).
    Preferred,
    /// Membership unit (LLC).
    MembershipUnit,
}

/// The type of equity grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantType {
    /// Common stock grant.
    CommonStock,
    /// Preferred stock grant.
    PreferredStock,
    /// LLC membership unit grant.
    MembershipUnit,
    /// Stock option (unspecified ISO/NSO).
    StockOption,
    /// Incentive Stock Option (IRC Section 422).
    Iso,
    /// Non-Qualified Stock Option.
    Nso,
    /// Restricted Stock Award.
    Rsa,
    /// Simulated Vesting Unit (profits interest proxy).
    Svu,
}

/// Lifecycle status of an equity grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantStatus {
    /// Grant has been issued but not yet vested.
    Issued,
    /// Grant has vested (or partially vested).
    Vested,
    /// Option has been exercised.
    Exercised,
    /// Grant was forfeited (e.g. departure before vesting).
    Forfeited,
    /// Grant was cancelled.
    Cancelled,
}

impl fmt::Display for GrantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Issued => write!(f, "issued"),
            Self::Vested => write!(f, "vested"),
            Self::Exercised => write!(f, "exercised"),
            Self::Forfeited => write!(f, "forfeited"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Who is receiving the equity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipientType {
    /// A human individual.
    NaturalPerson,
    /// An investor entity.
    Investor,
    /// A corporate entity.
    Entity,
    /// A secondary transferee.
    Transferee,
}

/// Status of a vesting schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VestingStatus {
    /// Vesting is actively running.
    Active,
    /// Vesting was terminated (voluntary or involuntary).
    Terminated,
}

/// Type of vesting event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VestingEventType {
    /// Initial cliff vesting.
    Cliff,
    /// Monthly pro-rata vesting.
    Monthly,
    /// Manually triggered vesting event.
    Manual,
    /// Accelerated vesting (e.g. change of control).
    Acceleration,
}

/// Status of an individual vesting event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VestingEventStatus {
    /// Scheduled for a future date.
    Scheduled,
    /// Shares have vested.
    Vested,
    /// Shares were forfeited.
    Forfeited,
    /// Event was cancelled.
    Cancelled,
}

/// Reason for termination of employment/service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminationType {
    /// Employee chose to leave.
    Voluntary,
    /// Employer-initiated without cause.
    Involuntary,
    /// Termination for cause.
    ForCause,
    /// Death or disability.
    DeathDisability,
}

/// Status of a cap table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapTableStatus {
    /// Cap table is active and in use.
    Active,
    /// Cap table has been frozen (e.g. during acquisition).
    Frozen,
}

impl fmt::Display for CapTableStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Frozen => write!(f, "frozen"),
        }
    }
}

/// The type of governing document for a share transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoverningDocType {
    /// Company bylaws.
    Bylaws,
    /// Operating agreement (LLC).
    OperatingAgreement,
    /// Shareholder agreement.
    ShareholderAgreement,
    /// Other document.
    Other,
}

/// Status of a bylaws review for a transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BylawsReviewStatus {
    /// Review approved the transfer.
    Approved,
    /// Review denied the transfer.
    Denied,
}

/// Rights granted to the transferee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransfereeRights {
    /// Full membership/shareholder rights.
    FullMember,
    /// Economic rights only (no voting).
    EconomicOnly,
    /// Limited rights as defined by agreement.
    Limited,
}

/// Type of SAFE note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeType {
    /// Post-money SAFE (YC standard).
    PostMoney,
    /// Pre-money SAFE.
    PreMoney,
    /// Most Favored Nation SAFE.
    Mfn,
}

/// Lifecycle status of a SAFE note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeStatus {
    /// SAFE has been issued and is outstanding.
    Issued,
    /// SAFE has been converted into equity.
    Converted,
    /// SAFE was cancelled.
    Cancelled,
}

impl fmt::Display for SafeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Issued => write!(f, "issued"),
            Self::Converted => write!(f, "converted"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Lifecycle status of a funding round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FundingRoundStatus {
    /// Term sheet signed, diligence not yet started.
    TermSheet,
    /// Due diligence in progress.
    Diligence,
    /// Documents being signed, funds being wired.
    Closing,
    /// Round is fully closed.
    Closed,
}

impl fmt::Display for FundingRoundStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TermSheet => write!(f, "term_sheet"),
            Self::Diligence => write!(f, "diligence"),
            Self::Closing => write!(f, "closing"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// Type of 409A or equivalent valuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationType {
    /// IRC Section 409A valuation (C-Corp).
    FourOhNineA,
    /// LLC profits interest valuation.
    LlcProfitsInterest,
    /// Fair market value determination.
    FairMarketValue,
    /// Gift valuation.
    Gift,
    /// Estate valuation.
    Estate,
    /// Other valuation type.
    Other,
}

/// Lifecycle status of a valuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationStatus {
    /// Valuation report is being drafted.
    Draft,
    /// Submitted for board/management approval.
    PendingApproval,
    /// Approved and effective.
    Approved,
    /// Past its effective date.
    Expired,
    /// Replaced by a newer valuation.
    Superseded,
}

impl fmt::Display for ValuationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::PendingApproval => write!(f, "pending_approval"),
            Self::Approved => write!(f, "approved"),
            Self::Expired => write!(f, "expired"),
            Self::Superseded => write!(f, "superseded"),
        }
    }
}

/// Methodology used for a valuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValuationMethodology {
    /// Discounted cash flow / income approach.
    Income,
    /// Comparable company / market approach.
    Market,
    /// Net asset value approach.
    Asset,
    /// Option pricing / backsolve method.
    Backsolve,
    /// Combination of multiple methods.
    Hybrid,
    /// Other methodology.
    Other,
}

/// Type of share transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferType {
    /// Gifted shares.
    Gift,
    /// Transfer into a trust.
    TrustTransfer,
    /// Secondary market sale.
    SecondarySale,
    /// Estate transfer.
    Estate,
    /// Other transfer type.
    Other,
}

/// Lifecycle status of a share transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    /// Transfer request drafted.
    Draft,
    /// Pending review under bylaws/operating agreement.
    PendingBylawsReview,
    /// Pending right of first refusal exercise period.
    PendingRofr,
    /// Pending board approval.
    PendingBoardApproval,
    /// Transfer approved.
    Approved,
    /// Transfer executed and recorded.
    Executed,
    /// Transfer denied.
    Denied,
    /// Transfer cancelled by requester.
    Cancelled,
}

impl fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::PendingBylawsReview => write!(f, "pending_bylaws_review"),
            Self::PendingRofr => write!(f, "pending_rofr"),
            Self::PendingBoardApproval => write!(f, "pending_board_approval"),
            Self::Approved => write!(f, "approved"),
            Self::Executed => write!(f, "executed"),
            Self::Denied => write!(f, "denied"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Status of a company repurchase right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepurchaseStatus {
    /// Repurchase right exists but not yet exercised.
    Pending,
    /// Repurchase is being actively exercised.
    Active,
    /// Repurchase completed.
    Closed,
    /// Company waived its repurchase right.
    Waived,
}

/// Type of entry in an investor's ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvestorLedgerEntryType {
    /// Initial SAFE investment.
    SafeInvestment,
    /// Priced round investment.
    PricedRoundInvestment,
    /// SAFE converting into equity.
    SafeConversion,
    /// Pro-rata right exercise.
    ProRataExercise,
}

/// Type of tax election.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElectionType {
    /// 83(b) election for RSA grant.
    RsaGrant,
    /// 83(b) election for early exercise.
    EarlyExercise,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_count_arithmetic() {
        let a = ShareCount::new(1000);
        let b = ShareCount::new(300);
        assert_eq!((a + b).raw(), 1300);
        assert_eq!((a - b).raw(), 700);
    }

    #[test]
    fn share_count_sum() {
        let counts = vec![
            ShareCount::new(100),
            ShareCount::new(200),
            ShareCount::new(300),
        ];
        let total: ShareCount = counts.into_iter().sum();
        assert_eq!(total.raw(), 600);
    }

    #[test]
    fn share_count_display() {
        assert_eq!(ShareCount::new(42).to_string(), "42");
        assert_eq!(ShareCount::new(-10).to_string(), "-10");
    }

    #[test]
    fn share_count_serde_roundtrip() {
        let sc = ShareCount::new(999);
        let json = serde_json::to_string(&sc).expect("serialize ShareCount");
        assert_eq!(json, "999");
        let parsed: ShareCount = serde_json::from_str(&json).expect("deserialize ShareCount");
        assert_eq!(sc, parsed);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn share_count_addition_overflow() {
        let _ = ShareCount::new(i64::MAX) + ShareCount::new(1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn share_count_subtraction_overflow() {
        let _ = ShareCount::new(i64::MIN) - ShareCount::new(1);
    }

    #[test]
    fn price_per_share_display() {
        let pps = PricePerShare::new(Cents::new(1050));
        assert_eq!(pps.to_string(), "$10.50");
    }

    #[test]
    fn percentage_display() {
        assert_eq!(Percentage::new(10000).unwrap().to_string(), "100.00%");
        assert_eq!(Percentage::new(2500).unwrap().to_string(), "25.00%");
        assert_eq!(Percentage::new(1).unwrap().to_string(), "0.01%");
    }

    #[test]
    fn percentage_to_decimal() {
        let pct = Percentage::new(5000).unwrap();
        let d = pct.to_decimal();
        assert_eq!(d.to_string(), "0.5000");
    }

    #[test]
    fn percentage_rejects_over_100() {
        assert!(Percentage::new(10_001).is_err());
        assert!(Percentage::new(20_000).is_err());
    }

    #[test]
    fn percentage_accepts_valid() {
        assert!(Percentage::new(0).is_ok());
        assert!(Percentage::new(5000).is_ok());
        assert!(Percentage::new(10_000).is_ok());
    }

    #[test]
    fn percentage_deserialize_rejects_invalid() {
        let result: Result<Percentage, _> = serde_json::from_str("10001");
        assert!(result.is_err());
    }

    #[test]
    fn percentage_deserialize_accepts_valid() {
        let pct: Percentage = serde_json::from_str("5000").unwrap();
        assert_eq!(pct.basis_points(), 5000);
    }

    #[test]
    fn percentage_const_helpers() {
        assert_eq!(Percentage::ZERO.basis_points(), 0);
        assert_eq!(Percentage::ONE_HUNDRED.basis_points(), 10_000);
    }

    #[test]
    fn positive_share_count_rejects_zero() {
        assert!(PositiveShareCount::new(0).is_err());
    }

    #[test]
    fn positive_share_count_rejects_negative() {
        assert!(PositiveShareCount::new(-1).is_err());
    }

    #[test]
    fn positive_share_count_accepts_positive() {
        let psc = PositiveShareCount::new(100).unwrap();
        assert_eq!(psc.raw(), 100);
        assert_eq!(psc.into_inner(), ShareCount::new(100));
    }

    #[test]
    fn positive_share_count_deserialize_rejects_invalid() {
        let result: Result<PositiveShareCount, _> = serde_json::from_str("0");
        assert!(result.is_err());
        let result: Result<PositiveShareCount, _> = serde_json::from_str("-5");
        assert!(result.is_err());
    }

    #[test]
    fn positive_share_count_deserialize_accepts_valid() {
        let psc: PositiveShareCount = serde_json::from_str("42").unwrap();
        assert_eq!(psc.raw(), 42);
    }

    #[test]
    fn grant_status_serde_roundtrip() {
        let status = GrantStatus::Issued;
        let json = serde_json::to_string(&status).expect("serialize GrantStatus");
        assert_eq!(json, "\"issued\"");
        let parsed: GrantStatus = serde_json::from_str(&json).expect("deserialize GrantStatus");
        assert_eq!(status, parsed);
    }

    #[test]
    fn transfer_status_serde() {
        let status = TransferStatus::PendingBoardApproval;
        let json = serde_json::to_string(&status).expect("serialize TransferStatus");
        assert_eq!(json, "\"pending_board_approval\"");
    }

    #[test]
    fn valuation_status_serde() {
        let status = ValuationStatus::PendingApproval;
        let json = serde_json::to_string(&status).expect("serialize ValuationStatus");
        assert_eq!(json, "\"pending_approval\"");
    }

    #[test]
    fn all_enums_roundtrip() {
        // Spot-check a representative enum from each group
        let st = StockType::MembershipUnit;
        let json = serde_json::to_string(&st).expect("serialize StockType");
        let parsed: StockType = serde_json::from_str(&json).expect("deserialize StockType");
        assert_eq!(st, parsed);

        let gt = GrantType::Iso;
        let json = serde_json::to_string(&gt).expect("serialize GrantType");
        let parsed: GrantType = serde_json::from_str(&json).expect("deserialize GrantType");
        assert_eq!(gt, parsed);

        let safe = SafeType::Mfn;
        let json = serde_json::to_string(&safe).expect("serialize SafeType");
        let parsed: SafeType = serde_json::from_str(&json).expect("deserialize SafeType");
        assert_eq!(safe, parsed);
    }
}
