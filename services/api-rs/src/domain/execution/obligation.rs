//! Obligation record (stored as `execution/obligations/{obligation_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::ExecutionError;
use super::types::{AssigneeType, ObligationStatus, ObligationType};
use crate::domain::ids::{ContactId, EntityId, IntentId, ObligationId};

/// An obligation that must be fulfilled as part of a corporate action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    obligation_id: ObligationId,
    entity_id: EntityId,
    intent_id: Option<IntentId>,
    obligation_type: ObligationType,
    assignee_type: AssigneeType,
    assignee_id: Option<ContactId>,
    description: String,
    due_date: Option<NaiveDate>,
    status: ObligationStatus,
    fulfilled_at: Option<DateTime<Utc>>,
    waived_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl Obligation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        obligation_id: ObligationId,
        entity_id: EntityId,
        intent_id: Option<IntentId>,
        obligation_type: ObligationType,
        assignee_type: AssigneeType,
        assignee_id: Option<ContactId>,
        description: String,
        due_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            obligation_id,
            entity_id,
            intent_id,
            obligation_type,
            assignee_type,
            assignee_id,
            description,
            due_date,
            status: ObligationStatus::Required,
            fulfilled_at: None,
            waived_at: None,
            created_at: Utc::now(),
        }
    }

    /// Start work. Required -> InProgress.
    pub fn start(&mut self) -> Result<(), ExecutionError> {
        if self.status != ObligationStatus::Required {
            return Err(ExecutionError::InvalidObligationTransition {
                from: self.status,
                to: ObligationStatus::InProgress,
            });
        }
        self.status = ObligationStatus::InProgress;
        Ok(())
    }

    /// Fulfill. Required or InProgress -> Fulfilled.
    pub fn fulfill(&mut self) -> Result<(), ExecutionError> {
        match self.status {
            ObligationStatus::Required | ObligationStatus::InProgress => {
                self.status = ObligationStatus::Fulfilled;
                self.fulfilled_at = Some(Utc::now());
                Ok(())
            }
            _ => Err(ExecutionError::InvalidObligationTransition {
                from: self.status,
                to: ObligationStatus::Fulfilled,
            }),
        }
    }

    /// Waive. Required or InProgress -> Waived.
    pub fn waive(&mut self) -> Result<(), ExecutionError> {
        match self.status {
            ObligationStatus::Required | ObligationStatus::InProgress => {
                self.status = ObligationStatus::Waived;
                self.waived_at = Some(Utc::now());
                Ok(())
            }
            _ => Err(ExecutionError::InvalidObligationTransition {
                from: self.status,
                to: ObligationStatus::Waived,
            }),
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────

    pub fn obligation_id(&self) -> ObligationId {
        self.obligation_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn intent_id(&self) -> Option<IntentId> {
        self.intent_id
    }
    pub fn obligation_type(&self) -> &ObligationType {
        &self.obligation_type
    }
    pub fn assignee_type(&self) -> AssigneeType {
        self.assignee_type
    }
    pub fn assignee_id(&self) -> Option<ContactId> {
        self.assignee_id
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn due_date(&self) -> Option<NaiveDate> {
        self.due_date
    }
    pub fn status(&self) -> ObligationStatus {
        self.status
    }
    pub fn fulfilled_at(&self) -> Option<DateTime<Utc>> {
        self.fulfilled_at
    }
    pub fn waived_at(&self) -> Option<DateTime<Utc>> {
        self.waived_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Assign or reassign an obligation to a contact.
    pub fn assign(&mut self, assignee_id: ContactId) -> Result<(), ExecutionError> {
        match self.status {
            ObligationStatus::Required | ObligationStatus::InProgress => {
                self.assignee_id = Some(assignee_id);
                Ok(())
            }
            _ => Err(ExecutionError::InvalidObligationTransition {
                from: self.status,
                to: self.status,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_obligation() -> Obligation {
        Obligation::new(
            ObligationId::new(),
            EntityId::new(),
            Some(IntentId::new()),
            ObligationType::from("annual_report"),
            AssigneeType::Internal,
            None,
            "File annual report with Secretary of State".to_owned(),
            Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        )
    }

    #[test]
    fn fsm_required_to_in_progress_to_fulfilled() {
        let mut obl = make_obligation();
        assert_eq!(obl.status(), ObligationStatus::Required);

        obl.start().unwrap();
        assert_eq!(obl.status(), ObligationStatus::InProgress);

        obl.fulfill().unwrap();
        assert_eq!(obl.status(), ObligationStatus::Fulfilled);
        assert!(obl.fulfilled_at().is_some());
    }

    #[test]
    fn fulfill_directly_from_required() {
        let mut obl = make_obligation();
        obl.fulfill().unwrap();
        assert_eq!(obl.status(), ObligationStatus::Fulfilled);
    }

    #[test]
    fn waive_from_required() {
        let mut obl = make_obligation();
        obl.waive().unwrap();
        assert_eq!(obl.status(), ObligationStatus::Waived);
        assert!(obl.waived_at().is_some());
    }

    #[test]
    fn waive_from_in_progress() {
        let mut obl = make_obligation();
        obl.start().unwrap();
        obl.waive().unwrap();
        assert_eq!(obl.status(), ObligationStatus::Waived);
    }

    #[test]
    fn cannot_start_from_fulfilled() {
        let mut obl = make_obligation();
        obl.fulfill().unwrap();
        assert!(obl.start().is_err());
    }

    #[test]
    fn cannot_fulfill_from_waived() {
        let mut obl = make_obligation();
        obl.waive().unwrap();
        assert!(obl.fulfill().is_err());
    }

    #[test]
    fn cannot_waive_from_fulfilled() {
        let mut obl = make_obligation();
        obl.fulfill().unwrap();
        assert!(obl.waive().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let mut obl = make_obligation();
        obl.start().unwrap();

        let json = serde_json::to_string_pretty(&obl).expect("serialize");
        let parsed: Obligation = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.obligation_id(), obl.obligation_id());
        assert_eq!(parsed.status(), ObligationStatus::InProgress);
        assert_eq!(parsed.obligation_type().as_str(), "annual_report");
        assert_eq!(parsed.due_date(), Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()));
    }
}
