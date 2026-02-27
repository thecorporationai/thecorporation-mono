//! Reconciliation record (stored as `treasury/reconciliations/{reconciliation_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::Cents;
use crate::domain::ids::{EntityId, ReconciliationId};

/// Status of a reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    Balanced,
    Discrepancy,
}

/// A ledger reconciliation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reconciliation {
    reconciliation_id: ReconciliationId,
    entity_id: EntityId,
    as_of_date: NaiveDate,
    total_debits_cents: Cents,
    total_credits_cents: Cents,
    difference_cents: Cents,
    status: ReconciliationStatus,
    created_at: DateTime<Utc>,
}

impl Reconciliation {
    pub fn new(
        reconciliation_id: ReconciliationId,
        entity_id: EntityId,
        as_of_date: NaiveDate,
        total_debits_cents: Cents,
        total_credits_cents: Cents,
    ) -> Self {
        let difference_cents = total_debits_cents - total_credits_cents;
        let status = if difference_cents.is_zero() {
            ReconciliationStatus::Balanced
        } else {
            ReconciliationStatus::Discrepancy
        };
        Self {
            reconciliation_id,
            entity_id,
            as_of_date,
            total_debits_cents,
            total_credits_cents,
            difference_cents,
            status,
            created_at: Utc::now(),
        }
    }

    // Accessors
    pub fn reconciliation_id(&self) -> ReconciliationId { self.reconciliation_id }
    pub fn entity_id(&self) -> EntityId { self.entity_id }
    pub fn as_of_date(&self) -> NaiveDate { self.as_of_date }
    pub fn total_debits_cents(&self) -> Cents { self.total_debits_cents }
    pub fn total_credits_cents(&self) -> Cents { self.total_credits_cents }
    pub fn difference_cents(&self) -> Cents { self.difference_cents }
    pub fn status(&self) -> ReconciliationStatus { self.status }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_reconciliation() {
        let r = Reconciliation::new(
            ReconciliationId::new(),
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            Cents::new(100000),
            Cents::new(100000),
        );
        assert_eq!(r.status(), ReconciliationStatus::Balanced);
        assert!(r.difference_cents().is_zero());
    }

    #[test]
    fn discrepancy_reconciliation() {
        let r = Reconciliation::new(
            ReconciliationId::new(),
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 1, 31).unwrap(),
            Cents::new(100000),
            Cents::new(99500),
        );
        assert_eq!(r.status(), ReconciliationStatus::Discrepancy);
        assert_eq!(r.difference_cents().raw(), 500);
    }

    #[test]
    fn serde_roundtrip() {
        let r = Reconciliation::new(
            ReconciliationId::new(),
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            Cents::new(50000),
            Cents::new(50000),
        );
        let json = serde_json::to_string(&r).unwrap();
        let parsed: Reconciliation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.reconciliation_id(), r.reconciliation_id());
    }
}
