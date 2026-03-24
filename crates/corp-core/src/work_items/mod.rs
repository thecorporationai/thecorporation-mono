//! Work items domain — open tasks claimed and completed by agents or humans.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{EntityId, WorkItemId};

// ── WorkItemStatus ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemStatus {
    Open,
    Claimed,
    Completed,
    Cancelled,
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkItemError {
    #[error("work item must be Open to be claimed; current status: {0:?}")]
    NotOpen(WorkItemStatus),
    #[error("work item must be Claimed to be completed; current status: {0:?}")]
    NotClaimed(WorkItemStatus),
    #[error("work item is already in a terminal state: {0:?}")]
    AlreadyTerminal(WorkItemStatus),
}

// ── WorkItem ──────────────────────────────────────────────────────────────────

/// An open unit of work that can be claimed and completed by an agent or human.
///
/// The FSM is:
/// ```text
/// Open → Claimed → Completed
///  ↓         ↓
/// Cancelled (from any non-terminal state)
/// ```
///
/// Claims expire if `claim_ttl_seconds` is set and the elapsed time since
/// `claimed_at` exceeds it. Use [`WorkItem::is_claim_expired`] to check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub work_item_id: WorkItemId,
    pub entity_id: EntityId,
    pub title: String,
    pub description: String,
    pub category: String,
    pub deadline: Option<NaiveDate>,
    /// When `true`, this item should be processed as soon as possible regardless
    /// of deadline.
    pub asap: bool,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    /// How many seconds a claim is valid before it is considered expired.
    pub claim_ttl_seconds: Option<u64>,
    pub status: WorkItemStatus,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<String>,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl WorkItem {
    /// Create a new work item in `Open` status.
    pub fn new(
        entity_id: EntityId,
        title: impl Into<String>,
        description: impl Into<String>,
        category: impl Into<String>,
        deadline: Option<NaiveDate>,
        asap: bool,
    ) -> Self {
        Self {
            work_item_id: WorkItemId::new(),
            entity_id,
            title: title.into(),
            description: description.into(),
            category: category.into(),
            deadline,
            asap,
            claimed_by: None,
            claimed_at: None,
            claim_ttl_seconds: None,
            status: WorkItemStatus::Open,
            completed_at: None,
            completed_by: None,
            result: None,
            created_at: Utc::now(),
        }
    }

    /// Claim this work item. Allowed only when `Open`.
    pub fn claim(&mut self, by: impl Into<String>) -> Result<(), WorkItemError> {
        match self.status {
            WorkItemStatus::Open => {
                self.status = WorkItemStatus::Claimed;
                self.claimed_by = Some(by.into());
                self.claimed_at = Some(Utc::now());
                Ok(())
            }
            s => Err(WorkItemError::NotOpen(s)),
        }
    }

    /// Release the current claim, returning the item to `Open` status.
    pub fn release_claim(&mut self) -> Result<(), WorkItemError> {
        match self.status {
            WorkItemStatus::Claimed => {
                self.status = WorkItemStatus::Open;
                self.claimed_by = None;
                self.claimed_at = None;
                Ok(())
            }
            s => Err(WorkItemError::NotOpen(s)),
        }
    }

    /// Complete this work item. Must be `Claimed`.
    pub fn complete(
        &mut self,
        by: impl Into<String>,
        result: Option<String>,
    ) -> Result<(), WorkItemError> {
        match self.status {
            WorkItemStatus::Claimed => {
                self.status = WorkItemStatus::Completed;
                self.completed_by = Some(by.into());
                self.completed_at = Some(Utc::now());
                self.result = result;
                Ok(())
            }
            s => Err(WorkItemError::NotClaimed(s)),
        }
    }

    /// Cancel this work item from any non-terminal state.
    pub fn cancel(&mut self) -> Result<(), WorkItemError> {
        if self.is_terminal() {
            return Err(WorkItemError::AlreadyTerminal(self.status));
        }
        self.status = WorkItemStatus::Cancelled;
        Ok(())
    }

    /// Returns `true` if the claim has expired based on `claim_ttl_seconds`.
    ///
    /// Always returns `false` if the item is not `Claimed`, if there is no
    /// `claimed_at` timestamp, or if `claim_ttl_seconds` is not set.
    pub fn is_claim_expired(&self) -> bool {
        if self.status != WorkItemStatus::Claimed {
            return false;
        }
        let (Some(claimed_at), Some(ttl)) = (self.claimed_at, self.claim_ttl_seconds) else {
            return false;
        };
        let elapsed = Utc::now().signed_duration_since(claimed_at);
        elapsed.num_seconds() >= ttl as i64
    }

    /// Returns `true` if the item is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            WorkItemStatus::Completed | WorkItemStatus::Cancelled
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item() -> WorkItem {
        WorkItem::new(
            EntityId::new(),
            "File 83(b) election",
            "Submit 83(b) election to IRS within 30 days",
            "tax",
            None,
            true,
        )
    }

    #[test]
    fn claim_and_complete() {
        let mut item = make_item();
        item.claim("agent-1").unwrap();
        assert_eq!(item.status, WorkItemStatus::Claimed);
        item.complete("agent-1", Some("Filed".into())).unwrap();
        assert_eq!(item.status, WorkItemStatus::Completed);
        assert!(item.result.is_some());
    }

    #[test]
    fn release_claim_returns_to_open() {
        let mut item = make_item();
        item.claim("agent-1").unwrap();
        item.release_claim().unwrap();
        assert_eq!(item.status, WorkItemStatus::Open);
    }

    #[test]
    fn cancel_from_open() {
        let mut item = make_item();
        item.cancel().unwrap();
        assert_eq!(item.status, WorkItemStatus::Cancelled);
    }

    #[test]
    fn cancel_from_terminal_fails() {
        let mut item = make_item();
        item.cancel().unwrap();
        assert!(matches!(item.cancel(), Err(WorkItemError::AlreadyTerminal(_))));
    }

    // ── Additional coverage ───────────────────────────────────────────────────

    #[test]
    fn new_work_item_is_open() {
        let item = make_item();
        assert_eq!(item.status, WorkItemStatus::Open);
    }

    #[test]
    fn new_work_item_stores_fields() {
        let item = make_item();
        assert_eq!(item.title, "File 83(b) election");
        assert_eq!(item.category, "tax");
        assert!(item.asap);
        assert!(item.deadline.is_none());
    }

    #[test]
    fn new_work_item_with_deadline() {
        let deadline = NaiveDate::from_ymd_opt(2026, 4, 30).unwrap();
        let item = WorkItem::new(
            EntityId::new(),
            "Annual report",
            "File the annual report",
            "compliance",
            Some(deadline),
            false,
        );
        assert_eq!(item.deadline, Some(deadline));
        assert!(!item.asap);
    }

    #[test]
    fn claim_sets_claimed_by() {
        let mut item = make_item();
        item.claim("agent-7").unwrap();
        assert_eq!(item.claimed_by.as_deref(), Some("agent-7"));
        assert!(item.claimed_at.is_some());
    }

    #[test]
    fn claim_already_claimed_is_error() {
        let mut item = make_item();
        item.claim("agent-1").unwrap();
        assert!(matches!(item.claim("agent-2"), Err(WorkItemError::NotOpen(_))));
    }

    #[test]
    fn complete_without_claim_is_error() {
        let mut item = make_item();
        assert!(matches!(
            item.complete("agent-1", None),
            Err(WorkItemError::NotClaimed(_))
        ));
    }

    #[test]
    fn complete_with_no_result() {
        let mut item = make_item();
        item.claim("agent-1").unwrap();
        item.complete("agent-1", None).unwrap();
        assert_eq!(item.status, WorkItemStatus::Completed);
        assert!(item.result.is_none());
        assert!(item.completed_at.is_some());
        assert_eq!(item.completed_by.as_deref(), Some("agent-1"));
    }

    #[test]
    fn complete_with_result() {
        let mut item = make_item();
        item.claim("agent-1").unwrap();
        item.complete("agent-1", Some("Done".into())).unwrap();
        assert_eq!(item.result.as_deref(), Some("Done"));
    }

    #[test]
    fn release_claim_clears_claimed_by() {
        let mut item = make_item();
        item.claim("agent-5").unwrap();
        item.release_claim().unwrap();
        assert!(item.claimed_by.is_none());
        assert!(item.claimed_at.is_none());
        assert_eq!(item.status, WorkItemStatus::Open);
    }

    #[test]
    fn release_claim_from_open_is_error() {
        let mut item = make_item();
        assert!(matches!(item.release_claim(), Err(WorkItemError::NotOpen(_))));
    }

    #[test]
    fn cancel_from_claimed_state() {
        let mut item = make_item();
        item.claim("agent-1").unwrap();
        assert!(item.cancel().is_ok());
        assert_eq!(item.status, WorkItemStatus::Cancelled);
    }

    #[test]
    fn cancel_from_completed_is_error() {
        let mut item = make_item();
        item.claim("agent-1").unwrap();
        item.complete("agent-1", None).unwrap();
        assert!(matches!(item.cancel(), Err(WorkItemError::AlreadyTerminal(_))));
    }

    #[test]
    fn is_terminal_for_completed() {
        let mut item = make_item();
        item.claim("a").unwrap();
        item.complete("a", None).unwrap();
        assert!(item.is_terminal());
    }

    #[test]
    fn is_terminal_for_cancelled() {
        let mut item = make_item();
        item.cancel().unwrap();
        assert!(item.is_terminal());
    }

    #[test]
    fn is_not_terminal_for_open() {
        assert!(!make_item().is_terminal());
    }

    #[test]
    fn is_not_terminal_for_claimed() {
        let mut item = make_item();
        item.claim("a").unwrap();
        assert!(!item.is_terminal());
    }

    #[test]
    fn claim_ttl_not_expired_when_just_set() {
        let mut item = WorkItem::new(
            EntityId::new(), "ttl-test", "desc", "cat", None, false,
        );
        item.claim("agent").unwrap();
        item.claim_ttl_seconds = Some(3600); // 1 hour TTL
        assert!(!item.is_claim_expired());
    }

    #[test]
    fn claim_ttl_expired_when_past_deadline() {
        let mut item = WorkItem::new(
            EntityId::new(), "ttl-test", "desc", "cat", None, false,
        );
        item.claim("agent").unwrap();
        // Set claimed_at to the past (1 hour ago)
        item.claimed_at = Some(chrono::Utc::now() - chrono::Duration::hours(2));
        item.claim_ttl_seconds = Some(3600); // 1 hour TTL
        assert!(item.is_claim_expired());
    }

    #[test]
    fn claim_ttl_not_expired_when_no_ttl() {
        let mut item = make_item();
        item.claim("agent").unwrap();
        // claim_ttl_seconds is None
        assert!(!item.is_claim_expired());
    }

    #[test]
    fn claim_ttl_not_expired_when_not_claimed() {
        let item = make_item();
        assert!(!item.is_claim_expired());
    }

    #[test]
    fn work_item_status_serde_roundtrip() {
        for status in [
            WorkItemStatus::Open,
            WorkItemStatus::Claimed,
            WorkItemStatus::Completed,
            WorkItemStatus::Cancelled,
        ] {
            let s = serde_json::to_string(&status).unwrap();
            let de: WorkItemStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, status);
        }
        assert_eq!(serde_json::to_string(&WorkItemStatus::Open).unwrap(), r#""open""#);
        assert_eq!(serde_json::to_string(&WorkItemStatus::Claimed).unwrap(), r#""claimed""#);
        assert_eq!(serde_json::to_string(&WorkItemStatus::Completed).unwrap(), r#""completed""#);
        assert_eq!(serde_json::to_string(&WorkItemStatus::Cancelled).unwrap(), r#""cancelled""#);
    }

    #[test]
    fn work_item_ids_are_unique() {
        let a = make_item();
        let b = make_item();
        assert_ne!(a.work_item_id, b.work_item_id);
    }
}
