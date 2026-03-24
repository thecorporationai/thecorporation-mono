//! Share class definitions within a cap table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{ShareCount, StockType};
use crate::ids::{CapTableId, EntityId, ShareClassId};

/// A class of shares (e.g. "Common A", "Series Seed Preferred") within a cap
/// table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareClass {
    pub share_class_id: ShareClassId,
    pub entity_id: EntityId,
    pub cap_table_id: CapTableId,
    /// Short identifier used in documents, e.g. `"CS-A"` or `"PREF-SEED"`.
    pub class_code: String,
    pub stock_type: StockType,
    /// Par value per share expressed as a formatted string, e.g. `"0.00001"`.
    pub par_value: String,
    pub authorized_shares: ShareCount,
    /// Liquidation preference description, e.g. `"1x non-participating"`.
    pub liquidation_preference: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ShareClass {
    /// Create a new share class. `liquidation_preference` is only relevant for
    /// preferred stock; pass `None` for common shares.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        cap_table_id: CapTableId,
        class_code: impl Into<String>,
        stock_type: StockType,
        par_value: impl Into<String>,
        authorized_shares: ShareCount,
        liquidation_preference: Option<String>,
    ) -> Self {
        Self {
            share_class_id: ShareClassId::new(),
            entity_id,
            cap_table_id,
            class_code: class_code.into(),
            stock_type,
            par_value: par_value.into(),
            authorized_shares,
            liquidation_preference,
            created_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_common_class() -> ShareClass {
        ShareClass::new(
            EntityId::new(),
            CapTableId::new(),
            "CS-A",
            StockType::Common,
            "0.00001",
            ShareCount::new(10_000_000),
            None,
        )
    }

    fn make_preferred_class() -> ShareClass {
        ShareClass::new(
            EntityId::new(),
            CapTableId::new(),
            "PREF-SEED",
            StockType::Preferred,
            "0.001",
            ShareCount::new(5_000_000),
            Some("1x non-participating".to_string()),
        )
    }

    fn make_membership_unit_class() -> ShareClass {
        ShareClass::new(
            EntityId::new(),
            CapTableId::new(),
            "UNIT-A",
            StockType::MembershipUnit,
            "0.01",
            ShareCount::new(1_000_000),
            None,
        )
    }

    #[test]
    fn new_common_class_stores_type() {
        let sc = make_common_class();
        assert_eq!(sc.stock_type, StockType::Common);
    }

    #[test]
    fn new_common_class_stores_class_code() {
        let sc = make_common_class();
        assert_eq!(sc.class_code, "CS-A");
    }

    #[test]
    fn new_common_class_stores_par_value() {
        let sc = make_common_class();
        assert_eq!(sc.par_value, "0.00001");
    }

    #[test]
    fn new_common_class_stores_authorized_shares() {
        let sc = make_common_class();
        assert_eq!(sc.authorized_shares, ShareCount::new(10_000_000));
    }

    #[test]
    fn new_common_class_no_liquidation_preference() {
        let sc = make_common_class();
        assert!(sc.liquidation_preference.is_none());
    }

    #[test]
    fn new_preferred_class_stores_type() {
        let sc = make_preferred_class();
        assert_eq!(sc.stock_type, StockType::Preferred);
    }

    #[test]
    fn new_preferred_class_stores_liquidation_preference() {
        let sc = make_preferred_class();
        assert_eq!(
            sc.liquidation_preference.as_deref(),
            Some("1x non-participating")
        );
    }

    #[test]
    fn new_membership_unit_class_stores_type() {
        let sc = make_membership_unit_class();
        assert_eq!(sc.stock_type, StockType::MembershipUnit);
    }

    #[test]
    fn new_share_class_has_unique_id() {
        let a = make_common_class();
        let b = make_common_class();
        assert_ne!(a.share_class_id, b.share_class_id);
    }

    #[test]
    fn share_class_serde_roundtrip_common() {
        let sc = make_common_class();
        let json = serde_json::to_string(&sc).unwrap();
        let de: ShareClass = serde_json::from_str(&json).unwrap();
        assert_eq!(de.share_class_id, sc.share_class_id);
        assert_eq!(de.stock_type, StockType::Common);
    }

    #[test]
    fn share_class_serde_roundtrip_preferred() {
        let sc = make_preferred_class();
        let json = serde_json::to_string(&sc).unwrap();
        let de: ShareClass = serde_json::from_str(&json).unwrap();
        assert_eq!(de.stock_type, StockType::Preferred);
        assert_eq!(
            de.liquidation_preference.as_deref(),
            Some("1x non-participating")
        );
    }

    #[test]
    fn share_class_serde_roundtrip_membership_unit() {
        let sc = make_membership_unit_class();
        let json = serde_json::to_string(&sc).unwrap();
        let de: ShareClass = serde_json::from_str(&json).unwrap();
        assert_eq!(de.stock_type, StockType::MembershipUnit);
    }
}
