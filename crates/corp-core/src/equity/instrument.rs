//! Financial instruments (shares, options, SAFEs, convertible notes, warrants).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{InstrumentId, LegalEntityId};

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

/// A canonical instrument definition for a legal entity's cap table.
///
/// An `Instrument` is the *class* of security — positions are individual
/// holdings of that instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub instrument_id: InstrumentId,
    pub issuer_legal_entity_id: LegalEntityId,
    /// Short ticker or series label, e.g. `"CS-1"` or `"Series A Preferred"`.
    pub symbol: String,
    pub kind: InstrumentKind,
    /// Total units authorised for this instrument, if applicable.
    pub authorized_units: Option<i64>,
    /// Par / issue price in whole cents, if applicable.
    pub issue_price_cents: Option<i64>,
    /// Flexible JSON blob for instrument-specific terms (liquidation preference,
    /// conversion ratio, etc.).
    pub terms: serde_json::Value,
    pub status: InstrumentStatus,
    pub created_at: DateTime<Utc>,
}

impl Instrument {
    /// Create a new instrument in the `Active` state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        issuer_legal_entity_id: LegalEntityId,
        symbol: impl Into<String>,
        kind: InstrumentKind,
        authorized_units: Option<i64>,
        issue_price_cents: Option<i64>,
        terms: serde_json::Value,
    ) -> Self {
        Self {
            instrument_id: InstrumentId::new(),
            issuer_legal_entity_id,
            symbol: symbol.into(),
            kind,
            authorized_units,
            issue_price_cents,
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
            LegalEntityId::new(),
            "CS-1",
            kind,
            Some(10_000_000),
            Some(1),
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
