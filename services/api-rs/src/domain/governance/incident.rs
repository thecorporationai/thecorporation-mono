//! Governance incident record (stored as `governance/incidents/{incident_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{EntityId, IncidentId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IncidentSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Open,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GovernanceIncident {
    incident_id: IncidentId,
    entity_id: EntityId,
    severity: IncidentSeverity,
    title: String,
    description: String,
    status: IncidentStatus,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

impl GovernanceIncident {
    pub fn new(
        incident_id: IncidentId,
        entity_id: EntityId,
        severity: IncidentSeverity,
        title: String,
        description: String,
    ) -> Self {
        Self {
            incident_id,
            entity_id,
            severity,
            title,
            description,
            status: IncidentStatus::Open,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }

    pub fn resolve(&mut self) {
        self.status = IncidentStatus::Resolved;
        self.resolved_at = Some(Utc::now());
    }

    pub fn incident_id(&self) -> IncidentId {
        self.incident_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
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
    pub fn status(&self) -> IncidentStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn resolved_at(&self) -> Option<DateTime<Utc>> {
        self.resolved_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_incident_is_open() {
        let incident = GovernanceIncident::new(
            IncidentId::new(),
            EntityId::new(),
            IncidentSeverity::High,
            "Breach".to_owned(),
            "Credential compromise".to_owned(),
        );
        assert_eq!(incident.status(), IncidentStatus::Open);
        assert!(incident.resolved_at().is_none());
    }

    #[test]
    fn resolve_sets_timestamp() {
        let mut incident = GovernanceIncident::new(
            IncidentId::new(),
            EntityId::new(),
            IncidentSeverity::Medium,
            "Ops incident".to_owned(),
            "Provider degraded".to_owned(),
        );
        incident.resolve();
        assert_eq!(incident.status(), IncidentStatus::Resolved);
        assert!(incident.resolved_at().is_some());
    }
}
