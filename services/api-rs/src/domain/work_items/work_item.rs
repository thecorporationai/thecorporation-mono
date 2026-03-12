use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::WorkItemError;
use super::types::WorkItemStatus;
use crate::domain::ids::{EntityId, WorkItemId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemActorType {
    Contact,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemActor {
    actor_type: WorkItemActorType,
    actor_id: String,
    label: String,
}

impl WorkItemActor {
    pub fn new(actor_type: WorkItemActorType, actor_id: String, label: String) -> Self {
        Self {
            actor_type,
            actor_id,
            label,
        }
    }

    pub fn actor_type(&self) -> WorkItemActorType {
        self.actor_type
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn same_identity(&self, other: &Self) -> bool {
        self.actor_type == other.actor_type && self.actor_id == other.actor_id
    }
}

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
    claimed_by_actor: Option<WorkItemActor>,
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
    completed_by_actor: Option<WorkItemActor>,
    #[serde(default)]
    completed_by: Option<String>,
    #[serde(default)]
    result: Option<String>,
    #[serde(default = "default_metadata")]
    metadata: serde_json::Value,
    created_at: DateTime<Utc>,
    #[serde(default)]
    created_by_actor: Option<WorkItemActor>,
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
        created_by_actor: Option<WorkItemActor>,
    ) -> Self {
        let created_by = created_by_actor.as_ref().map(|actor| actor.label().to_owned());
        Self {
            work_item_id,
            entity_id,
            title,
            description,
            category,
            deadline,
            asap,
            claimed_by_actor: None,
            claimed_by: None,
            claimed_at: None,
            claim_ttl_seconds: None,
            status: WorkItemStatus::Open,
            completed_at: None,
            completed_by_actor: None,
            completed_by: None,
            result: None,
            metadata,
            created_at: Utc::now(),
            created_by_actor,
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
        self.claimed_by_actor
            .as_ref()
            .map(WorkItemActor::label)
            .or(self.claimed_by.as_deref())
    }
    pub fn claimed_by_actor(&self) -> Option<&WorkItemActor> {
        self.claimed_by_actor.as_ref()
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
        self.completed_by_actor
            .as_ref()
            .map(WorkItemActor::label)
            .or(self.completed_by.as_deref())
    }
    pub fn completed_by_actor(&self) -> Option<&WorkItemActor> {
        self.completed_by_actor.as_ref()
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
        self.created_by_actor
            .as_ref()
            .map(WorkItemActor::label)
            .or(self.created_by.as_deref())
    }
    pub fn created_by_actor(&self) -> Option<&WorkItemActor> {
        self.created_by_actor.as_ref()
    }

    pub fn is_claimed_by_actor(&self, actor: &WorkItemActor) -> bool {
        if let Some(claimed_by_actor) = self.claimed_by_actor.as_ref() {
            return claimed_by_actor.same_identity(actor);
        }
        self.claimed_by()
            .is_some_and(|claimed_by| claimed_by == actor.label())
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
            self.claimed_by_actor = None;
            self.claimed_by = None;
            self.claimed_at = None;
            self.claim_ttl_seconds = None;
        }
    }

    pub fn claim(
        &mut self,
        by: WorkItemActor,
        ttl_seconds: Option<u64>,
    ) -> Result<(), WorkItemError> {
        if self.status != WorkItemStatus::Open {
            return Err(WorkItemError::InvalidTransition {
                from: self.status,
                to: WorkItemStatus::Claimed,
            });
        }
        self.status = WorkItemStatus::Claimed;
        self.claimed_by = Some(by.label().to_owned());
        self.claimed_by_actor = Some(by);
        self.claimed_at = Some(Utc::now());
        self.claim_ttl_seconds = ttl_seconds;
        Ok(())
    }

    pub fn release_claim(&mut self) -> Result<(), WorkItemError> {
        if self.status != WorkItemStatus::Claimed {
            return Err(WorkItemError::NotClaimed(self.work_item_id));
        }
        self.status = WorkItemStatus::Open;
        self.claimed_by_actor = None;
        self.claimed_by = None;
        self.claimed_at = None;
        self.claim_ttl_seconds = None;
        Ok(())
    }

    pub fn complete(
        &mut self,
        by: WorkItemActor,
        result: Option<String>,
    ) -> Result<(), WorkItemError> {
        match self.status {
            WorkItemStatus::Open | WorkItemStatus::Claimed => {
                self.status = WorkItemStatus::Completed;
                self.completed_at = Some(Utc::now());
                self.completed_by = Some(by.label().to_owned());
                self.completed_by_actor = Some(by);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(actor_type: WorkItemActorType, actor_id: &str, label: &str) -> WorkItemActor {
        WorkItemActor::new(actor_type, actor_id.to_owned(), label.to_owned())
    }

    #[test]
    fn claim_and_complete_store_structured_actor_identity() {
        let entity_id = EntityId::new();
        let mut item = WorkItem::new(
            WorkItemId::new(),
            entity_id,
            "Follow up".to_owned(),
            String::new(),
            "ops".to_owned(),
            None,
            false,
            serde_json::json!({}),
            Some(actor(WorkItemActorType::Agent, "agt_123", "Demo Operator")),
        );

        let claimer = actor(WorkItemActorType::Agent, "agt_123", "Demo Operator");
        item.claim(claimer.clone(), None).expect("claim succeeds");
        assert_eq!(item.claimed_by(), Some("Demo Operator"));
        assert!(item.is_claimed_by_actor(&claimer));

        item.complete(claimer.clone(), Some("done".to_owned()))
            .expect("complete succeeds");
        assert_eq!(item.completed_by(), Some("Demo Operator"));
        assert_eq!(
            item.completed_by_actor().map(WorkItemActor::actor_type),
            Some(WorkItemActorType::Agent)
        );
    }

    #[test]
    fn legacy_string_claims_still_deserialize() {
        let raw = serde_json::json!({
            "work_item_id": WorkItemId::new(),
            "entity_id": EntityId::new(),
            "title": "Legacy item",
            "description": "",
            "category": "ops",
            "asap": false,
            "claimed_by": "Alice Johnson",
            "claimed_at": null,
            "claim_ttl_seconds": null,
            "status": "claimed",
            "completed_at": null,
            "completed_by": null,
            "result": null,
            "metadata": {},
            "created_at": Utc::now().to_rfc3339(),
            "created_by": "Alice Johnson"
        });

        let item: WorkItem =
            serde_json::from_value(raw).expect("legacy work item should deserialize");
        assert_eq!(item.claimed_by(), Some("Alice Johnson"));
        assert!(item.claimed_by_actor().is_none());
        assert_eq!(item.created_by(), Some("Alice Johnson"));
    }
}
