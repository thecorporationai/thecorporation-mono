//! Payroll runs.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{EntityId, PayrollRunId};
use super::types::PayrollStatus;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PayrollError {
    #[error("payroll run must be in Draft to be approved; current status: {0:?}")]
    NotDraft(PayrollStatus),
    #[error("payroll run must be Approved to be processed; current status: {0:?}")]
    NotApproved(PayrollStatus),
}

// ── PayrollRun ────────────────────────────────────────────────────────────────

/// A single payroll run covering a pay period. Follows the FSM:
/// ```text
/// Draft → Approved → Processed
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayrollRun {
    pub payroll_run_id: PayrollRunId,
    pub entity_id: EntityId,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_gross_cents: i64,
    pub total_net_cents: i64,
    pub employee_count: u32,
    pub status: PayrollStatus,
    pub created_at: DateTime<Utc>,
}

impl PayrollRun {
    /// Create a new payroll run in `Draft` status.
    pub fn new(
        entity_id: EntityId,
        period_start: NaiveDate,
        period_end: NaiveDate,
        total_gross_cents: i64,
        total_net_cents: i64,
        employee_count: u32,
    ) -> Self {
        Self {
            payroll_run_id: PayrollRunId::new(),
            entity_id,
            period_start,
            period_end,
            total_gross_cents,
            total_net_cents,
            employee_count,
            status: PayrollStatus::Draft,
            created_at: Utc::now(),
        }
    }

    /// Transition `Draft → Approved`.
    pub fn approve(&mut self) -> Result<(), PayrollError> {
        match self.status {
            PayrollStatus::Draft => {
                self.status = PayrollStatus::Approved;
                Ok(())
            }
            s => Err(PayrollError::NotDraft(s)),
        }
    }

    /// Transition `Approved → Processed`.
    pub fn process(&mut self) -> Result<(), PayrollError> {
        match self.status {
            PayrollStatus::Approved => {
                self.status = PayrollStatus::Processed;
                Ok(())
            }
            s => Err(PayrollError::NotApproved(s)),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntityId;

    fn make_run() -> PayrollRun {
        PayrollRun::new(
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            500_000,
            420_000,
            5,
        )
    }

    #[test]
    fn new_run_is_draft() {
        let run = make_run();
        assert_eq!(run.status, PayrollStatus::Draft);
    }

    #[test]
    fn new_run_stores_fields() {
        let run = make_run();
        assert_eq!(run.total_gross_cents, 500_000);
        assert_eq!(run.total_net_cents, 420_000);
        assert_eq!(run.employee_count, 5);
    }

    #[test]
    fn approve_from_draft() {
        let mut run = make_run();
        assert!(run.approve().is_ok());
        assert_eq!(run.status, PayrollStatus::Approved);
    }

    #[test]
    fn process_from_approved() {
        let mut run = make_run();
        run.approve().unwrap();
        assert!(run.process().is_ok());
        assert_eq!(run.status, PayrollStatus::Processed);
    }

    #[test]
    fn full_lifecycle_draft_approved_processed() {
        let mut run = make_run();
        run.approve().unwrap();
        run.process().unwrap();
        assert_eq!(run.status, PayrollStatus::Processed);
    }

    #[test]
    fn cannot_approve_from_approved() {
        let mut run = make_run();
        run.approve().unwrap();
        assert!(matches!(run.approve(), Err(PayrollError::NotDraft(PayrollStatus::Approved))));
    }

    #[test]
    fn cannot_approve_from_processed() {
        let mut run = make_run();
        run.approve().unwrap();
        run.process().unwrap();
        assert!(matches!(run.approve(), Err(PayrollError::NotDraft(PayrollStatus::Processed))));
    }

    #[test]
    fn cannot_process_from_draft() {
        let mut run = make_run();
        assert!(matches!(run.process(), Err(PayrollError::NotApproved(PayrollStatus::Draft))));
    }

    #[test]
    fn cannot_process_from_processed() {
        let mut run = make_run();
        run.approve().unwrap();
        run.process().unwrap();
        assert!(matches!(run.process(), Err(PayrollError::NotApproved(PayrollStatus::Processed))));
    }

    #[test]
    fn payroll_run_ids_are_unique() {
        let a = make_run();
        let b = make_run();
        assert_ne!(a.payroll_run_id, b.payroll_run_id);
    }

    #[test]
    fn payroll_run_stores_period_dates() {
        let run = make_run();
        assert_eq!(run.period_start, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
        assert_eq!(run.period_end, NaiveDate::from_ymd_opt(2026, 3, 31).unwrap());
    }
}
