//! Payroll run record (stored as `treasury/payroll/{payroll_run_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{EntityId, PayrollRunId};

/// Status of a payroll run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PayrollStatus {
    Pending,
    Processing,
    Completed,
}

/// A payroll run for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayrollRun {
    payroll_run_id: PayrollRunId,
    entity_id: EntityId,
    pay_period_start: NaiveDate,
    pay_period_end: NaiveDate,
    status: PayrollStatus,
    created_at: DateTime<Utc>,
}

impl PayrollRun {
    pub fn new(
        payroll_run_id: PayrollRunId,
        entity_id: EntityId,
        pay_period_start: NaiveDate,
        pay_period_end: NaiveDate,
    ) -> Self {
        Self {
            payroll_run_id,
            entity_id,
            pay_period_start,
            pay_period_end,
            status: PayrollStatus::Pending,
            created_at: Utc::now(),
        }
    }

    pub fn mark_processing(&mut self) {
        self.status = PayrollStatus::Processing;
    }

    pub fn mark_completed(&mut self) {
        self.status = PayrollStatus::Completed;
    }

    // Accessors
    pub fn payroll_run_id(&self) -> PayrollRunId {
        self.payroll_run_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn pay_period_start(&self) -> NaiveDate {
        self.pay_period_start
    }
    pub fn pay_period_end(&self) -> NaiveDate {
        self.pay_period_end
    }
    pub fn status(&self) -> PayrollStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_payroll_run_is_pending() {
        let pr = PayrollRun::new(
            PayrollRunId::new(),
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
        );
        assert_eq!(pr.status(), PayrollStatus::Pending);
    }

    #[test]
    fn serde_roundtrip() {
        let pr = PayrollRun::new(
            PayrollRunId::new(),
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 2, 15).unwrap(),
        );
        let json = serde_json::to_string(&pr).unwrap();
        let parsed: PayrollRun = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.payroll_run_id(), pr.payroll_run_id());
    }
}
