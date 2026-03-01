//! Governance mode change events (stored as `governance/mode-history/{event_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::mode::GovernanceMode;
use crate::domain::ids::{
    ContactId, EntityId, GovernanceModeEventId, GovernanceTriggerId, IncidentId,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceModeChangeEvent {
    mode_event_id: GovernanceModeEventId,
    entity_id: EntityId,
    from_mode: GovernanceMode,
    to_mode: GovernanceMode,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    incident_ids: Vec<IncidentId>,
    #[serde(default)]
    evidence_refs: Vec<String>,
    #[serde(default)]
    trigger_id: Option<GovernanceTriggerId>,
    #[serde(default)]
    updated_by: Option<ContactId>,
    created_at: DateTime<Utc>,
}

impl GovernanceModeChangeEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode_event_id: GovernanceModeEventId,
        entity_id: EntityId,
        from_mode: GovernanceMode,
        to_mode: GovernanceMode,
        reason: Option<String>,
        incident_ids: Vec<IncidentId>,
        evidence_refs: Vec<String>,
        trigger_id: Option<GovernanceTriggerId>,
        updated_by: Option<ContactId>,
    ) -> Self {
        Self {
            mode_event_id,
            entity_id,
            from_mode,
            to_mode,
            reason,
            incident_ids,
            evidence_refs,
            trigger_id,
            updated_by,
            created_at: Utc::now(),
        }
    }

    pub fn mode_event_id(&self) -> GovernanceModeEventId {
        self.mode_event_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn from_mode(&self) -> GovernanceMode {
        self.from_mode
    }
    pub fn to_mode(&self) -> GovernanceMode {
        self.to_mode
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn incident_ids(&self) -> &[IncidentId] {
        &self.incident_ids
    }
    pub fn evidence_refs(&self) -> &[String] {
        &self.evidence_refs
    }
    pub fn trigger_id(&self) -> Option<GovernanceTriggerId> {
        self.trigger_id
    }
    pub fn updated_by(&self) -> Option<ContactId> {
        self.updated_by
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_mode_event() {
        let event = GovernanceModeChangeEvent::new(
            GovernanceModeEventId::new(),
            EntityId::new(),
            GovernanceMode::Normal,
            GovernanceMode::IncidentLockdown,
            Some("test".to_owned()),
            vec![IncidentId::new()],
            vec!["evidence:a".to_owned()],
            Some(GovernanceTriggerId::new()),
            Some(ContactId::new()),
        );
        let bytes = serde_json::to_vec(&event).expect("serialize");
        let parsed: GovernanceModeChangeEvent =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(parsed.mode_event_id(), event.mode_event_id());
        assert_eq!(parsed.to_mode(), GovernanceMode::IncidentLockdown);
    }
}
