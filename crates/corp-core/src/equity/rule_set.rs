//! Equity rule sets: anti-dilution, conversion precedence, and protective provisions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::EquityRuleSetId;
use super::instrument::InstrumentKind;

// ── AntiDilutionMethod ────────────────────────────────────────────────────────

/// The anti-dilution method that applies to preferred stockholders on a
/// down round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiDilutionMethod {
    /// No anti-dilution protection.
    None,
    /// Broad-based weighted average (most investor-friendly to founders).
    BroadBasedWeightedAverage,
    /// Narrow-based weighted average.
    NarrowBasedWeightedAverage,
    /// Full ratchet (most investor-friendly).
    FullRatchet,
}

// ── EquityRuleSet ─────────────────────────────────────────────────────────────

/// A set of governance rules attached to an equity class or funding round.
///
/// `conversion_precedence` determines the order in which instrument classes
/// convert during a liquidity event (highest priority first).
///
/// `protective_provisions` is a flexible JSON blob for additional terms
/// (e.g. drag-along thresholds, pay-to-play, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityRuleSet {
    pub rule_set_id: EquityRuleSetId,
    pub anti_dilution_method: AntiDilutionMethod,
    /// Ordered list of instrument kinds, first = highest precedence.
    pub conversion_precedence: Vec<InstrumentKind>,
    /// Flexible JSON blob for additional protective provisions.
    pub protective_provisions: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl EquityRuleSet {
    /// Create a new equity rule set.
    pub fn new(
        anti_dilution_method: AntiDilutionMethod,
        conversion_precedence: Vec<InstrumentKind>,
        protective_provisions: serde_json::Value,
    ) -> Self {
        Self {
            rule_set_id: EquityRuleSetId::new(),
            anti_dilution_method,
            conversion_precedence,
            protective_provisions,
            created_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule_set() -> EquityRuleSet {
        EquityRuleSet::new(
            AntiDilutionMethod::BroadBasedWeightedAverage,
            vec![
                InstrumentKind::PreferredEquity,
                InstrumentKind::CommonEquity,
            ],
            serde_json::json!({ "drag_along_threshold_bps": 6700 }),
        )
    }

    #[test]
    fn new_rule_set_stores_anti_dilution() {
        let rs = make_rule_set();
        assert_eq!(
            rs.anti_dilution_method,
            AntiDilutionMethod::BroadBasedWeightedAverage
        );
    }

    #[test]
    fn new_rule_set_stores_conversion_precedence() {
        let rs = make_rule_set();
        assert_eq!(rs.conversion_precedence.len(), 2);
        assert_eq!(rs.conversion_precedence[0], InstrumentKind::PreferredEquity);
        assert_eq!(rs.conversion_precedence[1], InstrumentKind::CommonEquity);
    }

    #[test]
    fn new_rule_set_has_unique_id() {
        let a = make_rule_set();
        let b = make_rule_set();
        assert_ne!(a.rule_set_id, b.rule_set_id);
    }

    #[test]
    fn rule_set_serde_roundtrip() {
        let rs = make_rule_set();
        let json = serde_json::to_string(&rs).unwrap();
        let de: EquityRuleSet = serde_json::from_str(&json).unwrap();
        assert_eq!(de.rule_set_id, rs.rule_set_id);
        assert_eq!(
            de.anti_dilution_method,
            AntiDilutionMethod::BroadBasedWeightedAverage
        );
        assert_eq!(de.conversion_precedence.len(), 2);
    }

    // ── AntiDilutionMethod serde ──────────────────────────────────────────────

    #[test]
    fn anti_dilution_serde_none() {
        let json = serde_json::to_string(&AntiDilutionMethod::None).unwrap();
        assert_eq!(json, r#""none""#);
        let de: AntiDilutionMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(de, AntiDilutionMethod::None);
    }

    #[test]
    fn anti_dilution_serde_broad_based() {
        let json =
            serde_json::to_string(&AntiDilutionMethod::BroadBasedWeightedAverage).unwrap();
        assert_eq!(json, r#""broad_based_weighted_average""#);
    }

    #[test]
    fn anti_dilution_serde_narrow_based() {
        let json =
            serde_json::to_string(&AntiDilutionMethod::NarrowBasedWeightedAverage).unwrap();
        assert_eq!(json, r#""narrow_based_weighted_average""#);
    }

    #[test]
    fn anti_dilution_serde_full_ratchet() {
        let json = serde_json::to_string(&AntiDilutionMethod::FullRatchet).unwrap();
        assert_eq!(json, r#""full_ratchet""#);
    }

    #[test]
    fn empty_conversion_precedence_is_valid() {
        let rs = EquityRuleSet::new(
            AntiDilutionMethod::None,
            vec![],
            serde_json::Value::Null,
        );
        assert!(rs.conversion_precedence.is_empty());
    }
}
