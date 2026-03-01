//! Compliance deadline record (stored as `deadlines/{deadline_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{DeadlineId, EntityId};

/// Recurrence pattern for a deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recurrence {
    OneTime,
    Monthly,
    Quarterly,
    Annual,
}

/// Status of a deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeadlineStatus {
    Upcoming,
    Due,
    Completed,
    Overdue,
}

/// Risk severity of missing a deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        Self {
            deadline_id,
            entity_id,
            deadline_type,
            due_date,
            description,
            recurrence,
            severity,
            status: DeadlineStatus::Upcoming,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn mark_completed(&mut self) {
        self.status = DeadlineStatus::Completed;
        self.completed_at = Some(Utc::now());
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
}
