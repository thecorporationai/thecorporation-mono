//! Compliance deadline record (stored as `deadlines/{deadline_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{DeadlineId, EntityId};

/// Recurrence pattern for a deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Recurrence {
    OneTime,
    Monthly,
    Quarterly,
    Annual,
}

/// Status of a deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineStatus {
    Upcoming,
    Due,
    Completed,
    Overdue,
}

/// Risk severity of missing a deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// A compliance or filing deadline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deadline {
    deadline_id: DeadlineId,
    entity_id: EntityId,
    deadline_type: String,
    due_date: NaiveDate,
    description: String,
    recurrence: Recurrence,
    severity: DeadlineSeverity,
    status: DeadlineStatus,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl Deadline {
    pub fn new(
        deadline_id: DeadlineId,
        entity_id: EntityId,
        deadline_type: String,
        due_date: NaiveDate,
        description: String,
        recurrence: Recurrence,
        severity: DeadlineSeverity,
    ) -> Self {
        let today = Utc::now().date_naive();
        let status = if due_date < today {
            DeadlineStatus::Overdue
        } else if due_date == today {
            DeadlineStatus::Due
        } else {
            DeadlineStatus::Upcoming
        };
        Self {
            deadline_id,
            entity_id,
            deadline_type,
            due_date,
            description,
            recurrence,
            severity,
            status,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn mark_completed(&mut self) {
        self.status = DeadlineStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Compute the current status dynamically from `due_date` vs today.
    ///
    /// Unlike the stored `status` field (which is set once at creation and only
    /// updated by explicit transitions like `mark_completed`), this method
    /// always reflects the real-time relationship between the due date and the
    /// current date. Use this when you need an up-to-date status after
    /// deserialization.
    pub fn current_status(&self) -> DeadlineStatus {
        // If explicitly completed, honour that regardless of date.
        if self.status == DeadlineStatus::Completed {
            return DeadlineStatus::Completed;
        }
        let today = Utc::now().date_naive();
        if self.due_date < today {
            DeadlineStatus::Overdue
        } else if self.due_date == today {
            DeadlineStatus::Due
        } else {
            DeadlineStatus::Upcoming
        }
    }

    // Accessors
    pub fn deadline_id(&self) -> DeadlineId {
        self.deadline_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn deadline_type(&self) -> &str {
        &self.deadline_type
    }
    pub fn due_date(&self) -> NaiveDate {
        self.due_date
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn recurrence(&self) -> Recurrence {
        self.recurrence
    }
    pub fn severity(&self) -> DeadlineSeverity {
        self.severity
    }
    pub fn status(&self) -> DeadlineStatus {
        self.status
    }
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_deadline_is_upcoming() {
        let d = Deadline::new(
            DeadlineId::new(),
            EntityId::new(),
            "annual_report".to_owned(),
            NaiveDate::from_ymd_opt(2026, 4, 15).unwrap(),
            "File annual report".to_owned(),
            Recurrence::Annual,
            DeadlineSeverity::Medium,
        );
        assert_eq!(d.status(), DeadlineStatus::Upcoming);
        assert_eq!(d.recurrence(), Recurrence::Annual);
        assert_eq!(d.severity(), DeadlineSeverity::Medium);
    }

    #[test]
    fn serde_roundtrip() {
        let d = Deadline::new(
            DeadlineId::new(),
            EntityId::new(),
            "quarterly_tax".to_owned(),
            NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            "Q2 estimated tax".to_owned(),
            Recurrence::Quarterly,
            DeadlineSeverity::High,
        );
        let json = serde_json::to_string(&d).unwrap();
        let parsed: Deadline = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.deadline_id(), d.deadline_id());
        assert_eq!(parsed.severity(), DeadlineSeverity::High);
    }

    #[test]
    fn current_status_overdue_for_past_date() {
        // Use a date well in the past so it is always overdue when the test runs.
        let d = Deadline::new(
            DeadlineId::new(),
            EntityId::new(),
            "old_filing".to_owned(),
            NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            "Ancient filing deadline".to_owned(),
            Recurrence::OneTime,
            DeadlineSeverity::Low,
        );
        assert_eq!(d.current_status(), DeadlineStatus::Overdue);
    }

    #[test]
    fn current_status_upcoming_for_future_date() {
        // Use a date far in the future so it is always upcoming when the test runs.
        let d = Deadline::new(
            DeadlineId::new(),
            EntityId::new(),
            "future_filing".to_owned(),
            NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
            "Far-future filing deadline".to_owned(),
            Recurrence::OneTime,
            DeadlineSeverity::Low,
        );
        assert_eq!(d.current_status(), DeadlineStatus::Upcoming);
    }
}
