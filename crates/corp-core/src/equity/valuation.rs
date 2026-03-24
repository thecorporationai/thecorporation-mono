//! 409A and fair-market-value valuation records.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::{ValuationMethodology, ValuationStatus, ValuationType};
use crate::ids::{CapTableId, EntityId, ValuationId};

/// A point-in-time valuation of the company (e.g. a 409A appraisal).
///
/// State machine:
/// ```text
/// Draft → PendingApproval (submit_for_approval)
///       → Approved        (approve)
///       → Expired         (expire)
///       → Superseded      (supersede)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Valuation {
    pub valuation_id: ValuationId,
    pub entity_id: EntityId,
    pub cap_table_id: CapTableId,
    pub valuation_type: ValuationType,
    pub methodology: ValuationMethodology,
    /// Total enterprise / FMV in whole cents.
    pub valuation_amount_cents: i64,
    /// Date as of which the valuation is effective.
    pub effective_date: NaiveDate,
    /// Name or identifier of the firm / person who prepared the valuation.
    pub prepared_by: Option<String>,
    pub status: ValuationStatus,
    pub approved_at: Option<DateTime<Utc>>,
    pub approved_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Valuation {
    /// Create a new valuation in `Draft` state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        cap_table_id: CapTableId,
        valuation_type: ValuationType,
        methodology: ValuationMethodology,
        valuation_amount_cents: i64,
        effective_date: NaiveDate,
        prepared_by: Option<String>,
    ) -> Self {
        Self {
            valuation_id: ValuationId::new(),
            entity_id,
            cap_table_id,
            valuation_type,
            methodology,
            valuation_amount_cents,
            effective_date,
            prepared_by,
            status: ValuationStatus::Draft,
            approved_at: None,
            approved_by: None,
            created_at: Utc::now(),
        }
    }

    /// Move from `Draft` to `PendingApproval`.
    pub fn submit_for_approval(&mut self) -> Result<(), ValuationError> {
        if self.status == ValuationStatus::Draft {
            self.status = ValuationStatus::PendingApproval;
            Ok(())
        } else {
            Err(ValuationError::InvalidTransition {
                from: self.status.clone(),
                to: ValuationStatus::PendingApproval,
            })
        }
    }

    /// Move from `PendingApproval` to `Approved`.
    pub fn approve(&mut self, approved_by: impl Into<String>) -> Result<(), ValuationError> {
        if self.status == ValuationStatus::PendingApproval {
            self.status = ValuationStatus::Approved;
            self.approved_at = Some(Utc::now());
            self.approved_by = Some(approved_by.into());
            Ok(())
        } else {
            Err(ValuationError::InvalidTransition {
                from: self.status.clone(),
                to: ValuationStatus::Approved,
            })
        }
    }

    /// Move from `Approved` to `Expired`.
    pub fn expire(&mut self) -> Result<(), ValuationError> {
        if self.status == ValuationStatus::Approved {
            self.status = ValuationStatus::Expired;
            Ok(())
        } else {
            Err(ValuationError::InvalidTransition {
                from: self.status.clone(),
                to: ValuationStatus::Expired,
            })
        }
    }

    /// Move from `Approved` to `Superseded` (e.g. when a newer valuation is
    /// approved).
    pub fn supersede(&mut self) -> Result<(), ValuationError> {
        if self.status == ValuationStatus::Approved {
            self.status = ValuationStatus::Superseded;
            Ok(())
        } else {
            Err(ValuationError::InvalidTransition {
                from: self.status.clone(),
                to: ValuationStatus::Superseded,
            })
        }
    }
}

/// Errors produced by [`Valuation`] state transitions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValuationError {
    #[error("cannot transition valuation from {from:?} to {to:?}")]
    InvalidTransition {
        from: ValuationStatus,
        to: ValuationStatus,
    },
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valuation(
        valuation_type: ValuationType,
        methodology: ValuationMethodology,
    ) -> Valuation {
        Valuation::new(
            EntityId::new(),
            CapTableId::new(),
            valuation_type,
            methodology,
            10_000_000_00, // $10M
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            Some("Acme Valuation Firm".to_string()),
        )
    }

    fn make_409a() -> Valuation {
        make_valuation(ValuationType::FourOhNineA, ValuationMethodology::Backsolve)
    }

    // ── Valuation::new() ──────────────────────────────────────────────────────

    #[test]
    fn new_valuation_status_is_draft() {
        let v = make_409a();
        assert_eq!(v.status, ValuationStatus::Draft);
    }

    #[test]
    fn new_valuation_stores_amount() {
        let v = make_409a();
        assert_eq!(v.valuation_amount_cents, 10_000_000_00);
    }

    #[test]
    fn new_valuation_stores_prepared_by() {
        let v = make_409a();
        assert_eq!(v.prepared_by.as_deref(), Some("Acme Valuation Firm"));
    }

    #[test]
    fn new_valuation_has_no_approved_at() {
        let v = make_409a();
        assert!(v.approved_at.is_none());
    }

    #[test]
    fn new_valuation_has_no_approved_by() {
        let v = make_409a();
        assert!(v.approved_by.is_none());
    }

    #[test]
    fn new_valuation_four_oh_nine_a_type() {
        let v = make_valuation(ValuationType::FourOhNineA, ValuationMethodology::Income);
        assert_eq!(v.valuation_type, ValuationType::FourOhNineA);
    }

    #[test]
    fn new_valuation_fair_market_value_type() {
        let v = make_valuation(ValuationType::FairMarketValue, ValuationMethodology::Market);
        assert_eq!(v.valuation_type, ValuationType::FairMarketValue);
    }

    #[test]
    fn new_valuation_other_type() {
        let v = make_valuation(ValuationType::Other, ValuationMethodology::Other);
        assert_eq!(v.valuation_type, ValuationType::Other);
    }

    #[test]
    fn new_valuation_all_methodologies() {
        for methodology in [
            ValuationMethodology::Income,
            ValuationMethodology::Market,
            ValuationMethodology::Asset,
            ValuationMethodology::Backsolve,
            ValuationMethodology::Hybrid,
            ValuationMethodology::Other,
        ] {
            let v = make_valuation(ValuationType::FourOhNineA, methodology.clone());
            assert_eq!(v.methodology, methodology);
        }
    }

    // ── submit_for_approval() ─────────────────────────────────────────────────

    #[test]
    fn submit_for_approval_from_draft() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        assert_eq!(v.status, ValuationStatus::PendingApproval);
    }

    #[test]
    fn submit_for_approval_from_pending_approval_fails() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        assert!(matches!(
            v.submit_for_approval(),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn submit_for_approval_from_approved_fails() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("CFO").unwrap();
        assert!(matches!(
            v.submit_for_approval(),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn submit_for_approval_from_expired_fails() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("CFO").unwrap();
        v.expire().unwrap();
        assert!(matches!(
            v.submit_for_approval(),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    // ── approve() ─────────────────────────────────────────────────────────────

    #[test]
    fn approve_from_pending_approval() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("CFO").unwrap();
        assert_eq!(v.status, ValuationStatus::Approved);
    }

    #[test]
    fn approve_records_approved_at() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("CFO").unwrap();
        assert!(v.approved_at.is_some());
    }

    #[test]
    fn approve_records_approved_by() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("Jane CFO").unwrap();
        assert_eq!(v.approved_by.as_deref(), Some("Jane CFO"));
    }

    #[test]
    fn approve_from_draft_fails() {
        let mut v = make_409a();
        assert!(matches!(
            v.approve("CFO"),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn approve_from_expired_fails() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("CFO").unwrap();
        v.expire().unwrap();
        assert!(matches!(
            v.approve("CFO"),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    // ── expire() ──────────────────────────────────────────────────────────────

    #[test]
    fn expire_from_approved() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("CFO").unwrap();
        v.expire().unwrap();
        assert_eq!(v.status, ValuationStatus::Expired);
    }

    #[test]
    fn expire_from_draft_fails() {
        let mut v = make_409a();
        assert!(matches!(
            v.expire(),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn expire_from_pending_approval_fails() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        assert!(matches!(
            v.expire(),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    // ── supersede() ───────────────────────────────────────────────────────────

    #[test]
    fn supersede_from_approved() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("CFO").unwrap();
        v.supersede().unwrap();
        assert_eq!(v.status, ValuationStatus::Superseded);
    }

    #[test]
    fn supersede_from_draft_fails() {
        let mut v = make_409a();
        assert!(matches!(
            v.supersede(),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn supersede_from_pending_approval_fails() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        assert!(matches!(
            v.supersede(),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn supersede_from_expired_fails() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("CFO").unwrap();
        v.expire().unwrap();
        assert!(matches!(
            v.supersede(),
            Err(ValuationError::InvalidTransition { .. })
        ));
    }

    // ── Full lifecycle ─────────────────────────────────────────────────────────

    #[test]
    fn full_lifecycle_draft_to_approved() {
        let mut v = make_409a();
        assert_eq!(v.status, ValuationStatus::Draft);
        v.submit_for_approval().unwrap();
        assert_eq!(v.status, ValuationStatus::PendingApproval);
        v.approve("Board").unwrap();
        assert_eq!(v.status, ValuationStatus::Approved);
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn valuation_serde_roundtrip_draft() {
        let v = make_409a();
        let json = serde_json::to_string(&v).unwrap();
        let de: Valuation = serde_json::from_str(&json).unwrap();
        assert_eq!(de.valuation_id, v.valuation_id);
        assert_eq!(de.status, ValuationStatus::Draft);
    }

    #[test]
    fn valuation_serde_roundtrip_approved() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve("Board").unwrap();
        let json = serde_json::to_string(&v).unwrap();
        let de: Valuation = serde_json::from_str(&json).unwrap();
        assert_eq!(de.status, ValuationStatus::Approved);
        assert!(de.approved_at.is_some());
    }
}
