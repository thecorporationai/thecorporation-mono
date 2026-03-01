//! Governance audit chain models.
//!
//! Stored as append-only JSON entries under `governance/audit/*`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::domain::ids::{
    EntityId, GovernanceAuditCheckpointId, GovernanceAuditEntryId, GovernanceAuditVerificationId,
    GovernanceModeEventId, GovernanceTriggerId, IncidentId, IntentId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceAuditEventType {
    ModeChanged,
    LockdownTriggerApplied,
    ManualEvent,
    CheckpointWritten,
    ChainVerified,
    ChainVerificationFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceAuditEntry {
    audit_entry_id: GovernanceAuditEntryId,
    entity_id: EntityId,
    event_type: GovernanceAuditEventType,
    action: String,
    #[serde(default)]
    details: Value,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    linked_intent_id: Option<IntentId>,
    #[serde(default)]
    linked_incident_id: Option<IncidentId>,
    #[serde(default)]
    linked_trigger_id: Option<GovernanceTriggerId>,
    #[serde(default)]
    linked_mode_event_id: Option<GovernanceModeEventId>,
    #[serde(default)]
    previous_entry_hash: Option<String>,
    entry_hash: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct GovernanceAuditHashPayload<'a> {
    audit_entry_id: GovernanceAuditEntryId,
    entity_id: EntityId,
    event_type: GovernanceAuditEventType,
    action: &'a str,
    details: &'a Value,
    evidence_refs: &'a [String],
    linked_intent_id: Option<IntentId>,
    linked_incident_id: Option<IncidentId>,
    linked_trigger_id: Option<GovernanceTriggerId>,
    linked_mode_event_id: Option<GovernanceModeEventId>,
    previous_entry_hash: Option<&'a str>,
    created_at: DateTime<Utc>,
}

impl GovernanceAuditEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        audit_entry_id: GovernanceAuditEntryId,
        entity_id: EntityId,
        event_type: GovernanceAuditEventType,
        action: String,
        details: Value,
        evidence_refs: Vec<String>,
        linked_intent_id: Option<IntentId>,
        linked_incident_id: Option<IncidentId>,
        linked_trigger_id: Option<GovernanceTriggerId>,
        linked_mode_event_id: Option<GovernanceModeEventId>,
        previous_entry_hash: Option<String>,
    ) -> Self {
        let created_at = Utc::now();
        let entry_hash = compute_hash(
            audit_entry_id,
            entity_id,
            event_type,
            &action,
            &details,
            &evidence_refs,
            linked_intent_id,
            linked_incident_id,
            linked_trigger_id,
            linked_mode_event_id,
            previous_entry_hash.as_deref(),
            created_at,
        );
        Self {
            audit_entry_id,
            entity_id,
            event_type,
            action,
            details,
            evidence_refs,
            linked_intent_id,
            linked_incident_id,
            linked_trigger_id,
            linked_mode_event_id,
            previous_entry_hash,
            entry_hash,
            created_at,
        }
    }

    pub fn verify_integrity(&self) -> bool {
        self.entry_hash
            == compute_hash(
                self.audit_entry_id,
                self.entity_id,
                self.event_type,
                &self.action,
                &self.details,
                &self.evidence_refs,
                self.linked_intent_id,
                self.linked_incident_id,
                self.linked_trigger_id,
                self.linked_mode_event_id,
                self.previous_entry_hash.as_deref(),
                self.created_at,
            )
    }

    pub fn audit_entry_id(&self) -> GovernanceAuditEntryId {
        self.audit_entry_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn event_type(&self) -> GovernanceAuditEventType {
        self.event_type
    }
    pub fn action(&self) -> &str {
        &self.action
    }
    pub fn details(&self) -> &Value {
        &self.details
    }
    pub fn evidence_refs(&self) -> &[String] {
        &self.evidence_refs
    }
    pub fn linked_intent_id(&self) -> Option<IntentId> {
        self.linked_intent_id
    }
    pub fn linked_incident_id(&self) -> Option<IncidentId> {
        self.linked_incident_id
    }
    pub fn linked_trigger_id(&self) -> Option<GovernanceTriggerId> {
        self.linked_trigger_id
    }
    pub fn linked_mode_event_id(&self) -> Option<GovernanceModeEventId> {
        self.linked_mode_event_id
    }
    pub fn previous_entry_hash(&self) -> Option<&str> {
        self.previous_entry_hash.as_deref()
    }
    pub fn entry_hash(&self) -> &str {
        &self.entry_hash
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

fn compute_hash(
    audit_entry_id: GovernanceAuditEntryId,
    entity_id: EntityId,
    event_type: GovernanceAuditEventType,
    action: &str,
    details: &Value,
    evidence_refs: &[String],
    linked_intent_id: Option<IntentId>,
    linked_incident_id: Option<IncidentId>,
    linked_trigger_id: Option<GovernanceTriggerId>,
    linked_mode_event_id: Option<GovernanceModeEventId>,
    previous_entry_hash: Option<&str>,
    created_at: DateTime<Utc>,
) -> String {
    let payload = GovernanceAuditHashPayload {
        audit_entry_id,
        entity_id,
        event_type,
        action,
        details,
        evidence_refs,
        linked_intent_id,
        linked_incident_id,
        linked_trigger_id,
        linked_mode_event_id,
        previous_entry_hash,
        created_at,
    };
    let bytes = serde_json::to_vec(&payload).expect("serialize governance audit hash payload");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hex::encode(hasher.finalize())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceAuditCheckpoint {
    checkpoint_id: GovernanceAuditCheckpointId,
    entity_id: EntityId,
    latest_entry_id: GovernanceAuditEntryId,
    latest_entry_hash: String,
    total_entries: u64,
    created_at: DateTime<Utc>,
}

impl GovernanceAuditCheckpoint {
    pub fn new(
        checkpoint_id: GovernanceAuditCheckpointId,
        entity_id: EntityId,
        latest_entry_id: GovernanceAuditEntryId,
        latest_entry_hash: String,
        total_entries: u64,
    ) -> Self {
        Self {
            checkpoint_id,
            entity_id,
            latest_entry_id,
            latest_entry_hash,
            total_entries,
            created_at: Utc::now(),
        }
    }

    pub fn checkpoint_id(&self) -> GovernanceAuditCheckpointId {
        self.checkpoint_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn latest_entry_id(&self) -> GovernanceAuditEntryId {
        self.latest_entry_id
    }
    pub fn latest_entry_hash(&self) -> &str {
        &self.latest_entry_hash
    }
    pub fn total_entries(&self) -> u64 {
        self.total_entries
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceAuditVerificationReport {
    verification_id: GovernanceAuditVerificationId,
    entity_id: EntityId,
    ok: bool,
    total_entries: u64,
    #[serde(default)]
    anomalies: Vec<String>,
    #[serde(default)]
    latest_entry_hash: Option<String>,
    triggered_lockdown: bool,
    #[serde(default)]
    trigger_id: Option<GovernanceTriggerId>,
    #[serde(default)]
    incident_id: Option<IncidentId>,
    created_at: DateTime<Utc>,
}

impl GovernanceAuditVerificationReport {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        verification_id: GovernanceAuditVerificationId,
        entity_id: EntityId,
        ok: bool,
        total_entries: u64,
        anomalies: Vec<String>,
        latest_entry_hash: Option<String>,
        triggered_lockdown: bool,
        trigger_id: Option<GovernanceTriggerId>,
        incident_id: Option<IncidentId>,
    ) -> Self {
        Self {
            verification_id,
            entity_id,
            ok,
            total_entries,
            anomalies,
            latest_entry_hash,
            triggered_lockdown,
            trigger_id,
            incident_id,
            created_at: Utc::now(),
        }
    }

    pub fn verification_id(&self) -> GovernanceAuditVerificationId {
        self.verification_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn ok(&self) -> bool {
        self.ok
    }
    pub fn total_entries(&self) -> u64 {
        self.total_entries
    }
    pub fn anomalies(&self) -> &[String] {
        &self.anomalies
    }
    pub fn latest_entry_hash(&self) -> Option<&str> {
        self.latest_entry_hash.as_deref()
    }
    pub fn triggered_lockdown(&self) -> bool {
        self.triggered_lockdown
    }
    pub fn trigger_id(&self) -> Option<GovernanceTriggerId> {
        self.trigger_id
    }
    pub fn incident_id(&self) -> Option<IncidentId> {
        self.incident_id
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn entry_roundtrip_and_integrity() {
        let entry = GovernanceAuditEntry::new(
            GovernanceAuditEntryId::new(),
            EntityId::new(),
            GovernanceAuditEventType::ManualEvent,
            "manual event".to_owned(),
            json!({"k":"v"}),
            vec!["evidence:1".to_owned()],
            None,
            None,
            None,
            None,
            None,
        );
        assert!(entry.verify_integrity());

        let bytes = serde_json::to_vec(&entry).expect("serialize entry");
        let parsed: GovernanceAuditEntry = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(parsed.audit_entry_id(), entry.audit_entry_id());
        assert!(parsed.verify_integrity());
    }
}
