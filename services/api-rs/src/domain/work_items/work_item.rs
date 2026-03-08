use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::WorkItemError;
use super::types::WorkItemStatus;
use crate::domain::ids::{EntityId, WorkItemId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    work_item_id: WorkItemId,
    entity_id: EntityId,
    title: String,
    description: String,
    category: String,
    #[serde(default)]
    deadline: Option<NaiveDate>,
    #[serde(default)]
    asap: bool,
    #[serde(default)]
    claimed_by: Option<String>,
    #[serde(default)]
    claimed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    claim_ttl_seconds: Option<u64>,
    status: WorkItemStatus,
    #[serde(default)]
    completed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    completed_by: Option<String>,
    #[serde(default)]
    result: Option<String>,
    #[serde(default = "default_metadata")]
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
    #[serde(default)]
    created_by: Option<String>,
}

fn default_metadata() -> serde_json::Value {
    serde_json::Value::Object(Default::default())
}

impl WorkItem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        work_item_id: WorkItemId,
        entity_id: EntityId,
        title: String,
        description: String,
        category: String,
        deadline: Option<NaiveDate>,
        asap: bool,
        metadata: serde_json::Value,
        created_by: Option<String>,
    ) -> Self {
        Self {
            work_item_id,
            entity_id,
            title,
            description,
            category,
            deadline,
            asap,
            claimed_by: None,
            claimed_at: None,
            claim_ttl_seconds: None,
            status: WorkItemStatus::Open,
            completed_at: None,
            completed_by: None,
            result: None,
            metadata,
            created_at: Utc::now(),
            created_by,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────

    pub fn work_item_id(&self) -> WorkItemId {
        self.work_item_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn category(&self) -> &str {
        &self.category
    }
    pub fn deadline(&self) -> Option<NaiveDate> {
        self.deadline
    }
    pub fn asap(&self) -> bool {
        self.asap
    }
    pub fn claimed_by(&self) -> Option<&str> {
        self.claimed_by.as_deref()
    }
    pub fn claimed_at(&self) -> Option<DateTime<Utc>> {
        self.claimed_at
    }
    pub fn claim_ttl_seconds(&self) -> Option<u64> {
        self.claim_ttl_seconds
    }
    pub fn status(&self) -> WorkItemStatus {
        self.status
    }
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }
    pub fn completed_by(&self) -> Option<&str> {
        self.completed_by.as_deref()
    }
    pub fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }
    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn created_by(&self) -> Option<&str> {
        self.created_by.as_deref()
    }

    // ── Claim expiry ─────────────────────────────────────────────────

    pub fn is_claim_expired(&self, now: DateTime<Utc>) -> bool {
        if self.status != WorkItemStatus::Claimed {
            return false;
        }
        match (self.claimed_at, self.claim_ttl_seconds) {
            (Some(at), Some(ttl)) => {
                let expires = at + chrono::Duration::seconds(ttl as i64);
                now > expires
            }
            _ => false, // no TTL = claim doesn't expire
        }
    }

    /// Returns the effective status, treating expired claims as Open.
    pub fn effective_status(&self, now: DateTime<Utc>) -> WorkItemStatus {
        if self.is_claim_expired(now) {
            WorkItemStatus::Open
        } else {
            self.status
        }
    }

    // ── FSM transitions ──────────────────────────────────────────────

    /// Auto-release an expired claim (mutates in place).
    pub fn auto_release_expired_claim(&mut self, now: DateTime<Utc>) {
        if self.is_claim_expired(now) {
            self.status = WorkItemStatus::Open;
            self.claimed_by = None;
            self.claimed_at = None;
            self.claim_ttl_seconds = None;
        }
    }

    pub fn claim(&mut self, by: String, ttl_seconds: Option<u64>) -> Result<(), WorkItemError> {
        if self.status != WorkItemStatus::Open {
            return Err(WorkItemError::InvalidTransition {
                from: self.status,
                to: WorkItemStatus::Claimed,
            });
        }
        self.status = WorkItemStatus::Claimed;
        self.claimed_by = Some(by);
        self.claimed_at = Some(Utc::now());
        self.claim_ttl_seconds = ttl_seconds;
        Ok(())
    }

    pub fn release_claim(&mut self) -> Result<(), WorkItemError> {
        if self.status != WorkItemStatus::Claimed {
            return Err(WorkItemError::NotClaimed(self.work_item_id));
        }
        self.status = WorkItemStatus::Open;
        self.claimed_by = None;
        self.claimed_at = None;
        self.claim_ttl_seconds = None;
        Ok(())
    }

    pub fn complete(&mut self, by: String, result: Option<String>) -> Result<(), WorkItemError> {
        match self.status {
            WorkItemStatus::Open | WorkItemStatus::Claimed => {
                self.status = WorkItemStatus::Completed;
                self.completed_at = Some(Utc::now());
                self.completed_by = Some(by);
                self.result = result;
                Ok(())
            }
            other => Err(WorkItemError::InvalidTransition {
                from: other,
                to: WorkItemStatus::Completed,
            }),
        }
    }

    pub fn cancel(&mut self) -> Result<(), WorkItemError> {
        match self.status {
            WorkItemStatus::Open | WorkItemStatus::Claimed => {
                self.status = WorkItemStatus::Cancelled;
                Ok(())
            }
            other => Err(WorkItemError::InvalidTransition {
                from: other,
                to: WorkItemStatus::Cancelled,
            }),
        }
    }
}
