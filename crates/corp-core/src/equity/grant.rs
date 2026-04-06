//! Equity grant records (stock grants, options, RSAs, etc.).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::{GrantStatus, GrantType, ShareCount};
use crate::ids::{CapTableId, ContactId, EntityId, EquityGrantId, InstrumentId, ResolutionId};

/// An equity grant issued to a recipient — covering common/preferred stock,
/// ISOs, NSOs, RSAs, and membership units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquityGrant {
    pub grant_id: EquityGrantId,
    pub entity_id: EntityId,
    pub cap_table_id: CapTableId,
    pub instrument_id: InstrumentId,
    /// Contact record for the grant recipient.
    pub recipient_contact_id: ContactId,
    pub recipient_name: String,
    pub grant_type: GrantType,
    pub shares: ShareCount,
    /// Strike / exercise price in whole cents. `None` for outright grants.
    pub price_per_share: Option<i64>,
    /// Date from which the vesting schedule starts.
    pub vesting_start: Option<NaiveDate>,
    /// Total vesting duration in months.
    pub vesting_months: Option<u32>,
    /// Cliff length in months (shares before this date do not vest).
    pub cliff_months: Option<u32>,
    /// Governance resolution that authorized this grant (e.g. board approval).
    #[serde(default)]
    pub resolution_id: Option<ResolutionId>,
    /// Cumulative vested shares — updated by the vest_event handler.
    /// Defaults to zero for grants created before this field existed.
    #[serde(default)]
    pub vested_shares: ShareCount,
    pub status: GrantStatus,
    pub created_at: DateTime<Utc>,
}

impl EquityGrant {
    /// Issue a new equity grant in the `Issued` state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        cap_table_id: CapTableId,
        instrument_id: InstrumentId,
        recipient_contact_id: ContactId,
        recipient_name: impl Into<String>,
        grant_type: GrantType,
        shares: ShareCount,
        price_per_share: Option<i64>,
        vesting_start: Option<NaiveDate>,
        vesting_months: Option<u32>,
        cliff_months: Option<u32>,
        resolution_id: Option<ResolutionId>,
    ) -> Self {
        Self {
            grant_id: EquityGrantId::new(),
            entity_id,
            cap_table_id,
            instrument_id,
            recipient_contact_id,
            recipient_name: recipient_name.into(),
            grant_type,
            shares,
            price_per_share,
            vesting_start,
            vesting_months,
            cliff_months,
            resolution_id,
            vested_shares: ShareCount::ZERO,
            status: GrantStatus::Issued,
            created_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grant(grant_type: GrantType) -> EquityGrant {
        EquityGrant::new(
            EntityId::new(),
            CapTableId::new(),
            InstrumentId::new(),
            ContactId::new(),
            "Jane Founder",
            grant_type,
            ShareCount::new(1_000_000),
            None,
            None,
            None,
            None,
            None,
        )
    }

    fn make_option_grant(
        price_per_share: Option<i64>,
        vesting_start: Option<NaiveDate>,
        vesting_months: Option<u32>,
        cliff_months: Option<u32>,
    ) -> EquityGrant {
        EquityGrant::new(
            EntityId::new(),
            CapTableId::new(),
            InstrumentId::new(),
            ContactId::new(),
            "Bob Employee",
            GrantType::Iso,
            ShareCount::new(100_000),
            price_per_share,
            vesting_start,
            vesting_months,
            cliff_months,
            None,
        )
    }

    #[test]
    fn new_grant_status_is_issued() {
        let g = make_grant(GrantType::CommonStock);
        assert_eq!(g.status, GrantStatus::Issued);
    }

    #[test]
    fn new_grant_stores_recipient_name() {
        let g = make_grant(GrantType::CommonStock);
        assert_eq!(g.recipient_name, "Jane Founder");
    }

    #[test]
    fn new_grant_stores_shares() {
        let g = make_grant(GrantType::CommonStock);
        assert_eq!(g.shares, ShareCount::new(1_000_000));
    }

    #[test]
    fn new_grant_common_stock() {
        let g = make_grant(GrantType::CommonStock);
        assert_eq!(g.grant_type, GrantType::CommonStock);
    }

    #[test]
    fn new_grant_preferred_stock() {
        let g = make_grant(GrantType::PreferredStock);
        assert_eq!(g.grant_type, GrantType::PreferredStock);
    }

    #[test]
    fn new_grant_membership_unit() {
        let g = make_grant(GrantType::MembershipUnit);
        assert_eq!(g.grant_type, GrantType::MembershipUnit);
    }

    #[test]
    fn new_grant_stock_option() {
        let g = make_grant(GrantType::StockOption);
        assert_eq!(g.grant_type, GrantType::StockOption);
    }

    #[test]
    fn new_grant_iso() {
        let g = make_grant(GrantType::Iso);
        assert_eq!(g.grant_type, GrantType::Iso);
    }

    #[test]
    fn new_grant_nso() {
        let g = make_grant(GrantType::Nso);
        assert_eq!(g.grant_type, GrantType::Nso);
    }

    #[test]
    fn new_grant_rsa() {
        let g = make_grant(GrantType::Rsa);
        assert_eq!(g.grant_type, GrantType::Rsa);
    }

    #[test]
    fn new_grant_has_no_price_when_none() {
        let g = make_grant(GrantType::CommonStock);
        assert!(g.price_per_share.is_none());
    }

    #[test]
    fn new_grant_stores_price_per_share() {
        let vesting_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let g = make_option_grant(Some(100), Some(vesting_start), Some(48), Some(12));
        assert_eq!(g.price_per_share, Some(100));
    }

    #[test]
    fn new_grant_stores_vesting_parameters() {
        let vesting_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let g = make_option_grant(Some(50), Some(vesting_start), Some(48), Some(12));
        assert_eq!(g.vesting_start, Some(vesting_start));
        assert_eq!(g.vesting_months, Some(48));
        assert_eq!(g.cliff_months, Some(12));
    }

    #[test]
    fn new_grant_no_vesting_fields_none() {
        let g = make_option_grant(None, None, None, None);
        assert!(g.vesting_start.is_none());
        assert!(g.vesting_months.is_none());
        assert!(g.cliff_months.is_none());
    }

    #[test]
    fn new_grant_has_unique_id() {
        let a = make_grant(GrantType::CommonStock);
        let b = make_grant(GrantType::CommonStock);
        assert_ne!(a.grant_id, b.grant_id);
    }

    #[test]
    fn grant_serde_roundtrip() {
        let g = make_grant(GrantType::Iso);
        let json = serde_json::to_string(&g).unwrap();
        let de: EquityGrant = serde_json::from_str(&json).unwrap();
        assert_eq!(de.grant_id, g.grant_id);
        assert_eq!(de.grant_type, GrantType::Iso);
        assert_eq!(de.status, GrantStatus::Issued);
    }

    #[test]
    fn new_grant_vested_shares_is_zero() {
        let g = make_grant(GrantType::Iso);
        assert_eq!(g.vested_shares, ShareCount::ZERO);
    }

    #[test]
    fn vested_shares_defaults_when_missing_in_json() {
        // Simulate a legacy grant stored without vested_shares
        let g = make_grant(GrantType::CommonStock);
        let mut json: serde_json::Value = serde_json::to_value(&g).unwrap();
        json.as_object_mut().unwrap().remove("vested_shares");
        let de: EquityGrant = serde_json::from_value(json).unwrap();
        assert_eq!(de.vested_shares, ShareCount::ZERO);
    }
}
