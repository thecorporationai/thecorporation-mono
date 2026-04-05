//! Financial instruments: the single primitive for all security types on a cap table.
//!
//! An `Instrument` represents a class of security — common shares, preferred shares,
//! membership units, SAFEs, convertible notes, warrants, or option pools. Positions
//! track individual holdings of an instrument.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{CapTableId, EntityId, InstrumentId};

// ── InstrumentKind ────────────────────────────────────────────────────────────

/// The type of financial instrument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    CommonEquity,
    PreferredEquity,
    MembershipUnit,
    OptionGrant,
    Safe,
    ConvertibleNote,
    Warrant,
}

// ── InstrumentStatus ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentStatus {
    Active,
    Closed,
    Cancelled,
}

// ── Instrument ────────────────────────────────────────────────────────────────

/// A canonical instrument definition on a legal entity's cap table.
///
/// An `Instrument` is the *class* of security — positions are individual
/// holdings of that instrument. This is the single primitive that replaces
/// the former ShareClass + Instrument split.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub instrument_id: InstrumentId,
    pub entity_id: EntityId,
    pub cap_table_id: CapTableId,
    /// Short ticker or series label, e.g. `"CS-A"` or `"Series A Preferred"`.
    pub symbol: String,
    pub kind: InstrumentKind,
    /// Total units authorised for this instrument, if applicable.
    pub authorized_units: Option<i64>,
    /// Par value per unit as a formatted string, e.g. `"0.00001"`.
    /// Only relevant for equity instruments (common/preferred/membership units).
    pub par_value: Option<String>,
    /// Issue / strike price in whole cents, if applicable.
    pub issue_price_cents: Option<i64>,
    /// Liquidation preference description, e.g. `"1x non-participating"`.
    /// Only relevant for preferred equity.
    pub liquidation_preference: Option<String>,
    /// Flexible JSON blob for instrument-specific terms (conversion ratio,
    /// anti-dilution provisions, etc.).
    pub terms: serde_json::Value,
    pub status: InstrumentStatus,
    pub created_at: DateTime<Utc>,
}

impl Instrument {
    /// Create a new instrument in the `Active` state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        cap_table_id: CapTableId,
        symbol: impl Into<String>,
        kind: InstrumentKind,
        authorized_units: Option<i64>,
        par_value: Option<String>,
        issue_price_cents: Option<i64>,
        liquidation_preference: Option<String>,
        terms: serde_json::Value,
    ) -> Self {
        Self {
            instrument_id: InstrumentId::new(),
            entity_id,
            cap_table_id,
            symbol: symbol.into(),
            kind,
            authorized_units,
            par_value,
            issue_price_cents,
            liquidation_preference,
            terms,
            status: InstrumentStatus::Active,
            created_at: Utc::now(),
        }
    }

    /// Update the instrument's status.
    pub fn set_status(&mut self, status: InstrumentStatus) {
        self.status = status;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_instrument(kind: InstrumentKind) -> Instrument {
        Instrument::new(
            EntityId::new(),
            CapTableId::new(),
            "CS-1",
            kind,
            Some(10_000_000),
            Some("0.00001".to_string()),
            Some(1),
            None,
            serde_json::Value::Null,
        )
    }

    #[test]
    fn new_instrument_status_is_active() {
        let i = make_instrument(InstrumentKind::CommonEquity);
        assert_eq!(i.status, InstrumentStatus::Active);
    }

    #[test]
    fn new_instrument_stores_symbol() {
        let i = make_instrument(InstrumentKind::CommonEquity);
        assert_eq!(i.symbol, "CS-1");
    }

    #[test]
    fn new_instrument_stores_kind() {
        let i = make_instrument(InstrumentKind::PreferredEquity);
        assert_eq!(i.kind, InstrumentKind::PreferredEquity);
    }

    #[test]
    fn new_instrument_stores_authorized_units() {
        let i = make_instrument(InstrumentKind::CommonEquity);
        assert_eq!(i.authorized_units, Some(10_000_000));
    }

    #[test]
    fn new_instrument_stores_issue_price() {
        let i = make_instrument(InstrumentKind::CommonEquity);
        assert_eq!(i.issue_price_cents, Some(1));
    }

    #[test]
    fn new_instrument_stores_par_value() {
        let i = make_instrument(InstrumentKind::CommonEquity);
        assert_eq!(i.par_value, Some("0.00001".to_string()));
    }

    #[test]
    fn new_instrument_preferred_with_liquidation() {
        let i = Instrument::new(
            EntityId::new(),
            CapTableId::new(),
            "PREF-A",
            InstrumentKind::PreferredEquity,
            Some(5_000_000),
            Some("0.001".to_string()),
            Some(100),
            Some("1x non-participating".to_string()),
            serde_json::Value::Null,
        );
        assert_eq!(i.liquidation_preference, Some("1x non-participating".to_string()));
    }

    #[test]
    fn set_status_to_closed() {
        let mut i = make_instrument(InstrumentKind::CommonEquity);
        i.set_status(InstrumentStatus::Closed);
        assert_eq!(i.status, InstrumentStatus::Closed);
    }

    #[test]
    fn set_status_to_cancelled() {
        let mut i = make_instrument(InstrumentKind::Warrant);
        i.set_status(InstrumentStatus::Cancelled);
        assert_eq!(i.status, InstrumentStatus::Cancelled);
    }

    #[test]
    fn new_instrument_has_unique_id() {
        let a = make_instrument(InstrumentKind::Safe);
        let b = make_instrument(InstrumentKind::Safe);
        assert_ne!(a.instrument_id, b.instrument_id);
    }

    #[test]
    fn instrument_serde_roundtrip() {
        let i = make_instrument(InstrumentKind::ConvertibleNote);
        let json = serde_json::to_string(&i).unwrap();
        let de: Instrument = serde_json::from_str(&json).unwrap();
        assert_eq!(de.instrument_id, i.instrument_id);
        assert_eq!(de.kind, InstrumentKind::ConvertibleNote);
        assert_eq!(de.status, InstrumentStatus::Active);
    }

    // ── InstrumentKind serde ──────────────────────────────────────────────────

    #[test]
    fn instrument_kind_serde_common_equity() {
        let json = serde_json::to_string(&InstrumentKind::CommonEquity).unwrap();
        assert_eq!(json, r#""common_equity""#);
        let de: InstrumentKind = serde_json::from_str(&json).unwrap();
        assert_eq!(de, InstrumentKind::CommonEquity);
    }

    #[test]
    fn instrument_kind_serde_preferred_equity() {
        let json = serde_json::to_string(&InstrumentKind::PreferredEquity).unwrap();
        assert_eq!(json, r#""preferred_equity""#);
    }

    #[test]
    fn instrument_kind_serde_membership_unit() {
        let json = serde_json::to_string(&InstrumentKind::MembershipUnit).unwrap();
        assert_eq!(json, r#""membership_unit""#);
    }

    #[test]
    fn instrument_kind_serde_option_grant() {
        let json = serde_json::to_string(&InstrumentKind::OptionGrant).unwrap();
        assert_eq!(json, r#""option_grant""#);
    }

    #[test]
    fn instrument_kind_serde_safe() {
        let json = serde_json::to_string(&InstrumentKind::Safe).unwrap();
        assert_eq!(json, r#""safe""#);
    }

    #[test]
    fn instrument_kind_serde_convertible_note() {
        let json = serde_json::to_string(&InstrumentKind::ConvertibleNote).unwrap();
        assert_eq!(json, r#""convertible_note""#);
    }

    #[test]
    fn instrument_kind_serde_warrant() {
        let json = serde_json::to_string(&InstrumentKind::Warrant).unwrap();
        assert_eq!(json, r#""warrant""#);
    }

    // ── InstrumentStatus serde ────────────────────────────────────────────────

    #[test]
    fn instrument_status_serde_active() {
        let json = serde_json::to_string(&InstrumentStatus::Active).unwrap();
        assert_eq!(json, r#""active""#);
    }

    #[test]
    fn instrument_status_serde_closed() {
        let json = serde_json::to_string(&InstrumentStatus::Closed).unwrap();
        assert_eq!(json, r#""closed""#);
    }

    #[test]
    fn instrument_status_serde_cancelled() {
        let json = serde_json::to_string(&InstrumentStatus::Cancelled).unwrap();
        assert_eq!(json, r#""cancelled""#);
    }
}
