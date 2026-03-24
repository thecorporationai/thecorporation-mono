//! Execution obligations — trackable tasks arising from an intent.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::types::{AssigneeType, ObligationStatus};
use crate::ids::{ContactId, EntityId, IntentId, ObligationId};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ObligationError {
    #[error("obligation must be Required to be started; current status: {0:?}")]
    NotRequired(ObligationStatus),
    #[error("obligation must be Required or InProgress to be fulfilled; current status: {0:?}")]
    CannotFulfill(ObligationStatus),
    #[error("obligation is already in a terminal state: {0:?}")]
    AlreadyTerminal(ObligationStatus),
}

// ── Obligation ────────────────────────────────────────────────────────────────

/// A trackable task or requirement associated with an intent execution.
///
/// The FSM is:
/// ```text
/// Required → InProgress → Fulfilled
///     ↓            ↓
///   Waived / Expired (from any non-terminal state)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    pub obligation_id: ObligationId,
    pub entity_id: EntityId,
    pub intent_id: Option<IntentId>,
    /// Free-form tag identifying the obligation type (e.g. `"board.approval"`).
    pub obligation_type: String,
    pub assignee_type: AssigneeType,
    pub assignee_id: Option<ContactId>,
    pub description: String,
    pub due_date: Option<NaiveDate>,
    pub status: ObligationStatus,
    pub fulfilled_at: Option<DateTime<Utc>>,
    pub waived_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Obligation {
    /// Create a new obligation in `Required` status.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        intent_id: Option<IntentId>,
        obligation_type: impl Into<String>,
        assignee_type: AssigneeType,
        assignee_id: Option<ContactId>,
        description: impl Into<String>,
        due_date: Option<NaiveDate>,
    ) -> Self {
        Self {
            obligation_id: ObligationId::new(),
            entity_id,
            intent_id,
            obligation_type: obligation_type.into(),
            assignee_type,
            assignee_id,
            description: description.into(),
            due_date,
            status: ObligationStatus::Required,
            fulfilled_at: None,
            waived_at: None,
            expired_at: None,
            created_at: Utc::now(),
        }
    }

    /// Transition `Required → InProgress`.
    pub fn start(&mut self) -> Result<(), ObligationError> {
        match self.status {
            ObligationStatus::Required => {
                self.status = ObligationStatus::InProgress;
                Ok(())
            }
            s => Err(ObligationError::NotRequired(s)),
        }
    }

    /// Transition `Required | InProgress → Fulfilled`.
    pub fn fulfill(&mut self) -> Result<(), ObligationError> {
        match self.status {
            ObligationStatus::Required | ObligationStatus::InProgress => {
                self.status = ObligationStatus::Fulfilled;
                self.fulfilled_at = Some(Utc::now());
                Ok(())
            }
            s => Err(ObligationError::CannotFulfill(s)),
        }
    }

    /// Waive this obligation. Allowed from any non-terminal state.
    pub fn waive(&mut self) -> Result<(), ObligationError> {
        if self.is_terminal() {
            return Err(ObligationError::AlreadyTerminal(self.status));
        }
        self.status = ObligationStatus::Waived;
        self.waived_at = Some(Utc::now());
        Ok(())
    }

    /// Expire this obligation. Allowed from any non-terminal state.
    pub fn expire(&mut self) -> Result<(), ObligationError> {
        if self.is_terminal() {
            return Err(ObligationError::AlreadyTerminal(self.status));
        }
        self.status = ObligationStatus::Expired;
        self.expired_at = Some(Utc::now());
        Ok(())
    }

    /// Returns `true` if the obligation is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            ObligationStatus::Fulfilled | ObligationStatus::Waived | ObligationStatus::Expired
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_obligation(assignee_type: AssigneeType) -> Obligation {
        Obligation::new(
            EntityId::new(),
            None,
            "board.approval",
            assignee_type,
            None,
            "Board must approve this action",
            None,
        )
    }

    fn required_internal() -> Obligation {
        make_obligation(AssigneeType::Internal)
    }

    // ── new() ─────────────────────────────────────────────────────────────────

    #[test]
    fn new_obligation_is_required() {
        let ob = required_internal();
        assert_eq!(ob.status, ObligationStatus::Required);
    }

    #[test]
    fn new_obligation_all_timestamps_none() {
        let ob = required_internal();
        assert!(ob.fulfilled_at.is_none());
        assert!(ob.waived_at.is_none());
        assert!(ob.expired_at.is_none());
    }

    #[test]
    fn new_obligation_with_internal_assignee() {
        let ob = make_obligation(AssigneeType::Internal);
        assert_eq!(ob.assignee_type, AssigneeType::Internal);
    }

    #[test]
    fn new_obligation_with_third_party_assignee() {
        let ob = make_obligation(AssigneeType::ThirdParty);
        assert_eq!(ob.assignee_type, AssigneeType::ThirdParty);
    }

    #[test]
    fn new_obligation_with_human_assignee() {
        let ob = make_obligation(AssigneeType::Human);
        assert_eq!(ob.assignee_type, AssigneeType::Human);
    }

    #[test]
    fn new_obligation_with_contact_and_intent() {
        let ob = Obligation::new(
            EntityId::new(),
            Some(IntentId::new()),
            "legal.review",
            AssigneeType::Human,
            Some(ContactId::new()),
            "Legal review required",
            Some(NaiveDate::from_ymd_opt(2026, 4, 30).unwrap()),
        );
        assert!(ob.intent_id.is_some());
        assert!(ob.assignee_id.is_some());
        assert!(ob.due_date.is_some());
    }

    // ── start() ──────────────────────────────────────────────────────────────

    #[test]
    fn start_from_required() {
        let mut ob = required_internal();
        assert!(ob.start().is_ok());
        assert_eq!(ob.status, ObligationStatus::InProgress);
    }

    #[test]
    fn start_from_in_progress_is_error() {
        let mut ob = required_internal();
        ob.start().unwrap();
        assert!(matches!(ob.start(), Err(ObligationError::NotRequired(_))));
    }

    #[test]
    fn start_from_fulfilled_is_error() {
        let mut ob = required_internal();
        ob.fulfill().unwrap();
        assert!(matches!(ob.start(), Err(ObligationError::NotRequired(_))));
    }

    // ── fulfill() ────────────────────────────────────────────────────────────

    #[test]
    fn fulfill_from_required_directly() {
        let mut ob = required_internal();
        assert!(ob.fulfill().is_ok());
        assert_eq!(ob.status, ObligationStatus::Fulfilled);
        assert!(ob.fulfilled_at.is_some());
    }

    #[test]
    fn fulfill_from_in_progress() {
        let mut ob = required_internal();
        ob.start().unwrap();
        assert!(ob.fulfill().is_ok());
        assert_eq!(ob.status, ObligationStatus::Fulfilled);
        assert!(ob.fulfilled_at.is_some());
    }

    #[test]
    fn fulfill_from_waived_is_error() {
        let mut ob = required_internal();
        ob.waive().unwrap();
        assert!(matches!(
            ob.fulfill(),
            Err(ObligationError::CannotFulfill(_))
        ));
    }

    #[test]
    fn fulfill_from_expired_is_error() {
        let mut ob = required_internal();
        ob.expire().unwrap();
        assert!(matches!(
            ob.fulfill(),
            Err(ObligationError::CannotFulfill(_))
        ));
    }

    // ── waive() ──────────────────────────────────────────────────────────────

    #[test]
    fn waive_from_required() {
        let mut ob = required_internal();
        assert!(ob.waive().is_ok());
        assert_eq!(ob.status, ObligationStatus::Waived);
        assert!(ob.waived_at.is_some());
    }

    #[test]
    fn waive_from_in_progress() {
        let mut ob = required_internal();
        ob.start().unwrap();
        assert!(ob.waive().is_ok());
        assert_eq!(ob.status, ObligationStatus::Waived);
    }

    #[test]
    fn waive_from_fulfilled_is_error() {
        let mut ob = required_internal();
        ob.fulfill().unwrap();
        assert!(matches!(
            ob.waive(),
            Err(ObligationError::AlreadyTerminal(_))
        ));
    }

    // ── expire() ─────────────────────────────────────────────────────────────

    #[test]
    fn expire_from_required() {
        let mut ob = required_internal();
        assert!(ob.expire().is_ok());
        assert_eq!(ob.status, ObligationStatus::Expired);
        assert!(ob.expired_at.is_some());
    }

    #[test]
    fn expire_from_in_progress() {
        let mut ob = required_internal();
        ob.start().unwrap();
        assert!(ob.expire().is_ok());
        assert_eq!(ob.status, ObligationStatus::Expired);
    }

    #[test]
    fn expire_from_waived_is_error() {
        let mut ob = required_internal();
        ob.waive().unwrap();
        assert!(matches!(
            ob.expire(),
            Err(ObligationError::AlreadyTerminal(_))
        ));
    }

    #[test]
    fn expire_from_fulfilled_is_error() {
        let mut ob = required_internal();
        ob.fulfill().unwrap();
        assert!(matches!(
            ob.expire(),
            Err(ObligationError::AlreadyTerminal(_))
        ));
    }

    // ── is_terminal() ────────────────────────────────────────────────────────

    #[test]
    fn is_terminal_for_fulfilled() {
        let mut ob = required_internal();
        ob.fulfill().unwrap();
        assert!(ob.is_terminal());
    }

    #[test]
    fn is_terminal_for_waived() {
        let mut ob = required_internal();
        ob.waive().unwrap();
        assert!(ob.is_terminal());
    }

    #[test]
    fn is_terminal_for_expired() {
        let mut ob = required_internal();
        ob.expire().unwrap();
        assert!(ob.is_terminal());
    }

    #[test]
    fn is_not_terminal_for_required() {
        assert!(!required_internal().is_terminal());
    }

    #[test]
    fn is_not_terminal_for_in_progress() {
        let mut ob = required_internal();
        ob.start().unwrap();
        assert!(!ob.is_terminal());
    }

    // ── ObligationStatus serde roundtrips ────────────────────────────────────

    #[test]
    fn obligation_status_serde_roundtrip() {
        for status in [
            ObligationStatus::Required,
            ObligationStatus::InProgress,
            ObligationStatus::Fulfilled,
            ObligationStatus::Waived,
            ObligationStatus::Expired,
        ] {
            let s = serde_json::to_string(&status).unwrap();
            let de: ObligationStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, status);
        }
    }

    #[test]
    fn obligation_status_serde_values() {
        assert_eq!(
            serde_json::to_string(&ObligationStatus::Required).unwrap(),
            r#""required""#
        );
        assert_eq!(
            serde_json::to_string(&ObligationStatus::InProgress).unwrap(),
            r#""in_progress""#
        );
        assert_eq!(
            serde_json::to_string(&ObligationStatus::Fulfilled).unwrap(),
            r#""fulfilled""#
        );
        assert_eq!(
            serde_json::to_string(&ObligationStatus::Waived).unwrap(),
            r#""waived""#
        );
        assert_eq!(
            serde_json::to_string(&ObligationStatus::Expired).unwrap(),
            r#""expired""#
        );
    }

    #[test]
    fn assignee_type_serde_roundtrip() {
        for variant in [
            AssigneeType::Internal,
            AssigneeType::ThirdParty,
            AssigneeType::Human,
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: AssigneeType = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
    }

    #[test]
    fn obligation_ids_are_unique() {
        let a = required_internal();
        let b = required_internal();
        assert_ne!(a.obligation_id, b.obligation_id);
    }
}
