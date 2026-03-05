//! Governance trigger events (stored as `governance/triggers/{trigger_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::incident::IncidentSeverity;
use crate::domain::ids::{
    ComplianceEscalationId, EntityId, GovernanceModeEventId, GovernanceTriggerId, IncidentId,
    IntentId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceTriggerType {
    ExternalSignal,
    PolicyEvidenceMismatch,
    #[serde(rename = "compliance_deadline_missed_d_plus_1")]
    ComplianceDeadlineMissedDPlus1,
    AuditChainVerificationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceTriggerSource {
    ComplianceScanner,
    ExecutionGate,
    ExternalIngestion,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GovernanceTriggerEvent {
    trigger_id: GovernanceTriggerId,
    entity_id: EntityId,
    trigger_type: GovernanceTriggerType,
    source: GovernanceTriggerSource,
    severity: IncidentSeverity,
    title: String,
    description: String,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    linked_intent_id: Option<IntentId>,
    #[serde(default)]
    linked_escalation_id: Option<ComplianceEscalationId>,
    incident_id: IncidentId,
    mode_event_id: GovernanceModeEventId,
    #[serde(default)]
    idempotency_key_hash: Option<String>,
    created_at: DateTime<Utc>,
}

impl GovernanceTriggerEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        trigger_id: GovernanceTriggerId,
        entity_id: EntityId,
        trigger_type: GovernanceTriggerType,
        source: GovernanceTriggerSource,
        severity: IncidentSeverity,
        title: String,
        description: String,
        evidence_refs: Vec<String>,
        linked_intent_id: Option<IntentId>,
        linked_escalation_id: Option<ComplianceEscalationId>,
        incident_id: IncidentId,
        mode_event_id: GovernanceModeEventId,
        idempotency_key_hash: Option<String>,
    ) -> Self {
        Self {
            trigger_id,
            entity_id,
            trigger_type,
            source,
            severity,
            title,
            description,
            evidence_refs,
            linked_intent_id,
            linked_escalation_id,
            incident_id,
            mode_event_id,
            idempotency_key_hash,
            created_at: Utc::now(),
        }
    }

    pub fn trigger_id(&self) -> GovernanceTriggerId {
        self.trigger_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn trigger_type(&self) -> GovernanceTriggerType {
        self.trigger_type
    }
    pub fn source(&self) -> GovernanceTriggerSource {
        self.source
    }
    pub fn severity(&self) -> IncidentSeverity {
        self.severity
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn evidence_refs(&self) -> &[String] {
        &self.evidence_refs
    }
    pub fn linked_intent_id(&self) -> Option<IntentId> {
        self.linked_intent_id
    }
    pub fn linked_escalation_id(&self) -> Option<ComplianceEscalationId> {
        self.linked_escalation_id
    }
    pub fn incident_id(&self) -> IncidentId {
        self.incident_id
    }
    pub fn mode_event_id(&self) -> GovernanceModeEventId {
        self.mode_event_id
    }
    pub fn idempotency_key_hash(&self) -> Option<&str> {
        self.idempotency_key_hash.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceTriggerIdempotencyRecord {
    idempotency_key_hash: String,
    trigger_id: GovernanceTriggerId,
    incident_id: IncidentId,
    mode_event_id: GovernanceModeEventId,
    created_at: DateTime<Utc>,
}

impl GovernanceTriggerIdempotencyRecord {
    pub fn new(
        idempotency_key_hash: String,
        trigger_id: GovernanceTriggerId,
        incident_id: IncidentId,
        mode_event_id: GovernanceModeEventId,
    ) -> Self {
        Self {
            idempotency_key_hash,
            trigger_id,
            incident_id,
            mode_event_id,
            created_at: Utc::now(),
        }
    }

    pub fn idempotency_key_hash(&self) -> &str {
        &self.idempotency_key_hash
    }
    pub fn trigger_id(&self) -> GovernanceTriggerId {
        self.trigger_id
    }
    pub fn incident_id(&self) -> IncidentId {
        self.incident_id
    }
    pub fn mode_event_id(&self) -> GovernanceModeEventId {
        self.mode_event_id
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_roundtrip() {
        let trigger = GovernanceTriggerEvent::new(
            GovernanceTriggerId::new(),
            EntityId::new(),
            GovernanceTriggerType::ExternalSignal,
            GovernanceTriggerSource::ExternalIngestion,
            IncidentSeverity::High,
            "signal".to_owned(),
            "signal detail".to_owned(),
            vec!["evidence:1".to_owned()],
            None,
            None,
            IncidentId::new(),
            GovernanceModeEventId::new(),
            Some("abc".to_owned()),
        );
        let bytes = serde_json::to_vec(&trigger).expect("serialize trigger");
        let parsed: GovernanceTriggerEvent =
            serde_json::from_slice(&bytes).expect("deserialize trigger");
        assert_eq!(parsed.trigger_id(), trigger.trigger_id());
        assert_eq!(parsed.trigger_type(), GovernanceTriggerType::ExternalSignal);
    }
}
