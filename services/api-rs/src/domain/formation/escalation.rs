//! Compliance escalation record (stored as `compliance/escalations/{escalation_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ComplianceEscalationId, DeadlineId, EntityId, IncidentId, ObligationId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EscalationStatus {
    Open,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceEscalation {
    escalation_id: ComplianceEscalationId,
    entity_id: EntityId,
    deadline_id: DeadlineId,
    milestone: String,
    action: String,
    authority: String,
    status: EscalationStatus,
    obligation_id: Option<ObligationId>,
    incident_id: Option<IncidentId>,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

impl ComplianceEscalation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        escalation_id: ComplianceEscalationId,
        entity_id: EntityId,
        deadline_id: DeadlineId,
        milestone: String,
        action: String,
        authority: String,
        obligation_id: Option<ObligationId>,
        incident_id: Option<IncidentId>,
    ) -> Self {
        Self {
            escalation_id,
            entity_id,
            deadline_id,
            milestone,
            action,
            authority,
            status: EscalationStatus::Open,
            obligation_id,
            incident_id,
            created_at: Utc::now(),
            resolved_at: None,
        }
    }

    pub fn resolve(&mut self) {
        self.status = EscalationStatus::Resolved;
        self.resolved_at = Some(Utc::now());
    }

    pub fn escalation_id(&self) -> ComplianceEscalationId {
        self.escalation_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn deadline_id(&self) -> DeadlineId {
        self.deadline_id
    }
    pub fn milestone(&self) -> &str {
        &self.milestone
    }
    pub fn action(&self) -> &str {
        &self.action
    }
    pub fn authority(&self) -> &str {
        &self.authority
    }
    pub fn status(&self) -> EscalationStatus {
        self.status
    }
    pub fn obligation_id(&self) -> Option<ObligationId> {
        self.obligation_id
    }
    pub fn incident_id(&self) -> Option<IncidentId> {
        self.incident_id
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn resolved_at(&self) -> Option<DateTime<Utc>> {
        self.resolved_at
    }
}
