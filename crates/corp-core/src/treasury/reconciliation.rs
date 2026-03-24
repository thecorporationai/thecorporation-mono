//! Bank reconciliation records.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{AccountId, EntityId, ReconciliationId};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReconciliationError {
    #[error("reconciliation is already marked as reconciled")]
    AlreadyReconciled,
}

// ── Reconciliation ────────────────────────────────────────────────────────────

/// A bank reconciliation comparing statement balance to book balance for a
/// given account and period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reconciliation {
    pub reconciliation_id: ReconciliationId,
    pub entity_id: EntityId,
    pub account_id: AccountId,
    pub period_end: NaiveDate,
    pub statement_balance_cents: i64,
    pub book_balance_cents: i64,
    /// `statement_balance_cents - book_balance_cents`. Populated automatically
    /// by [`Reconciliation::new`].
    pub difference_cents: i64,
    pub reconciled: bool,
    pub created_at: DateTime<Utc>,
}

impl Reconciliation {
    /// Create a new reconciliation. `difference_cents` is computed as
    /// `statement_balance_cents - book_balance_cents`.
    pub fn new(
        entity_id: EntityId,
        account_id: AccountId,
        period_end: NaiveDate,
        statement_balance_cents: i64,
        book_balance_cents: i64,
    ) -> Self {
        Self {
            reconciliation_id: ReconciliationId::new(),
            entity_id,
            account_id,
            period_end,
            statement_balance_cents,
            book_balance_cents,
            difference_cents: statement_balance_cents - book_balance_cents,
            reconciled: false,
            created_at: Utc::now(),
        }
    }

    /// Mark this reconciliation as complete.
    ///
    /// Returns `Err` if already reconciled.
    pub fn mark_reconciled(&mut self) -> Result<(), ReconciliationError> {
        if self.reconciled {
            return Err(ReconciliationError::AlreadyReconciled);
        }
        self.reconciled = true;
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AccountId, EntityId};

    #[test]
    fn difference_computed_on_new() {
        let r = Reconciliation::new(
            EntityId::new(),
            AccountId::new(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            10_000,
            9_800,
        );
        assert_eq!(r.difference_cents, 200);
    }

    #[test]
    fn mark_reconciled_twice_is_error() {
        let mut r = Reconciliation::new(
            EntityId::new(),
            AccountId::new(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            10_000,
            10_000,
        );
        assert!(r.mark_reconciled().is_ok());
        assert_eq!(
            r.mark_reconciled(),
            Err(ReconciliationError::AlreadyReconciled)
        );
    }
}
