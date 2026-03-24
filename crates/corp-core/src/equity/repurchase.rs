//! Repurchase rights: the company's right to buy back unvested shares.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::{RepurchaseStatus, ShareCount};
use crate::ids::{EntityId, EquityGrantId, RepurchaseRightId};

// ── RepurchaseRight ───────────────────────────────────────────────────────────

/// The company's repurchase right over unvested shares for a specific grant.
///
/// Typically attached to RSAs (restricted stock agreements) where shares vest
/// over time and the company may repurchase unvested shares upon termination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepurchaseRight {
    pub repurchase_right_id: RepurchaseRightId,
    pub entity_id: EntityId,
    pub grant_id: EquityGrantId,
    pub share_count: ShareCount,
    /// Repurchase price in whole cents per share.
    pub price_per_share_cents: i64,
    /// Date after which the repurchase right lapses, if any.
    pub expiration_date: Option<NaiveDate>,
    pub status: RepurchaseStatus,
    pub created_at: DateTime<Utc>,
}

impl RepurchaseRight {
    /// Create a new repurchase right in the `Pending` state.
    pub fn new(
        entity_id: EntityId,
        grant_id: EquityGrantId,
        share_count: ShareCount,
        price_per_share_cents: i64,
        expiration_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            repurchase_right_id: RepurchaseRightId::new(),
            entity_id,
            grant_id,
            share_count,
            price_per_share_cents,
            expiration_date,
            status: RepurchaseStatus::Pending,
            created_at: Utc::now(),
        }
    }

    /// Activate the repurchase right (`Pending` → `Active`).
    pub fn activate(&mut self) {
        self.status = RepurchaseStatus::Active;
    }

    /// Close the repurchase right once exercised (`Active` → `Closed`).
    pub fn close(&mut self) {
        self.status = RepurchaseStatus::Closed;
    }

    /// Waive the repurchase right (`Pending` or `Active` → `Waived`).
    pub fn waive(&mut self) {
        self.status = RepurchaseStatus::Waived;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_repurchase() -> RepurchaseRight {
        RepurchaseRight::new(
            EntityId::new(),
            EquityGrantId::new(),
            ShareCount::new(1_000_000),
            1, // $0.01 per share
            Some(NaiveDate::from_ymd_opt(2030, 1, 1).unwrap()),
        )
    }

    #[test]
    fn new_repurchase_status_is_pending() {
        let r = make_repurchase();
        assert_eq!(r.status, RepurchaseStatus::Pending);
    }

    #[test]
    fn new_repurchase_stores_share_count() {
        let r = make_repurchase();
        assert_eq!(r.share_count.raw(), 1_000_000);
    }

    #[test]
    fn new_repurchase_stores_price() {
        let r = make_repurchase();
        assert_eq!(r.price_per_share_cents, 1);
    }

    #[test]
    fn new_repurchase_stores_expiration() {
        let r = make_repurchase();
        assert_eq!(
            r.expiration_date,
            Some(NaiveDate::from_ymd_opt(2030, 1, 1).unwrap())
        );
    }

    #[test]
    fn activate_transitions_to_active() {
        let mut r = make_repurchase();
        r.activate();
        assert_eq!(r.status, RepurchaseStatus::Active);
    }

    #[test]
    fn close_transitions_to_closed() {
        let mut r = make_repurchase();
        r.activate();
        r.close();
        assert_eq!(r.status, RepurchaseStatus::Closed);
    }

    #[test]
    fn waive_from_pending_transitions_to_waived() {
        let mut r = make_repurchase();
        r.waive();
        assert_eq!(r.status, RepurchaseStatus::Waived);
    }

    #[test]
    fn waive_from_active_transitions_to_waived() {
        let mut r = make_repurchase();
        r.activate();
        r.waive();
        assert_eq!(r.status, RepurchaseStatus::Waived);
    }

    #[test]
    fn no_expiration_date_is_valid() {
        let r = RepurchaseRight::new(
            EntityId::new(),
            EquityGrantId::new(),
            ShareCount::new(100_000),
            1,
            None,
        );
        assert!(r.expiration_date.is_none());
    }

    #[test]
    fn new_repurchase_has_unique_id() {
        let a = make_repurchase();
        let b = make_repurchase();
        assert_ne!(a.repurchase_right_id, b.repurchase_right_id);
    }

    #[test]
    fn repurchase_serde_roundtrip() {
        let r = make_repurchase();
        let json = serde_json::to_string(&r).unwrap();
        let de: RepurchaseRight = serde_json::from_str(&json).unwrap();
        assert_eq!(de.repurchase_right_id, r.repurchase_right_id);
        assert_eq!(de.status, RepurchaseStatus::Pending);
        assert_eq!(de.share_count.raw(), 1_000_000);
    }

    // ── RepurchaseStatus serde ────────────────────────────────────────────────

    #[test]
    fn repurchase_status_serde_pending() {
        let json = serde_json::to_string(&RepurchaseStatus::Pending).unwrap();
        assert_eq!(json, r#""pending""#);
        let de: RepurchaseStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, RepurchaseStatus::Pending);
    }

    #[test]
    fn repurchase_status_serde_active() {
        let json = serde_json::to_string(&RepurchaseStatus::Active).unwrap();
        assert_eq!(json, r#""active""#);
    }

    #[test]
    fn repurchase_status_serde_closed() {
        let json = serde_json::to_string(&RepurchaseStatus::Closed).unwrap();
        assert_eq!(json, r#""closed""#);
    }

    #[test]
    fn repurchase_status_serde_waived() {
        let json = serde_json::to_string(&RepurchaseStatus::Waived).unwrap();
        assert_eq!(json, r#""waived""#);
    }
}
