//! Compliance evidence links (stored as `compliance/evidence-links/{id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ComplianceEscalationId, ComplianceEvidenceLinkId, EntityId, PacketId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceEvidenceLink {
    evidence_link_id: ComplianceEvidenceLinkId,
    entity_id: EntityId,
    escalation_id: ComplianceEscalationId,
    evidence_type: String,
    packet_id: Option<PacketId>,
    filing_reference: Option<String>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
}

impl ComplianceEvidenceLink {
    pub fn new(
        evidence_link_id: ComplianceEvidenceLinkId,
        entity_id: EntityId,
        escalation_id: ComplianceEscalationId,
        evidence_type: String,
        packet_id: Option<PacketId>,
        filing_reference: Option<String>,
        notes: Option<String>,
    ) -> Self {
        Self {
            evidence_link_id,
            entity_id,
            escalation_id,
            evidence_type,
            packet_id,
            filing_reference,
            notes,
            created_at: Utc::now(),
        }
    }

    pub fn evidence_link_id(&self) -> ComplianceEvidenceLinkId {
        self.evidence_link_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn escalation_id(&self) -> ComplianceEscalationId {
        self.escalation_id
    }
    pub fn evidence_type(&self) -> &str {
        &self.evidence_type
    }
    pub fn packet_id(&self) -> Option<PacketId> {
        self.packet_id
    }
    pub fn filing_reference(&self) -> Option<&str> {
        self.filing_reference.as_deref()
    }
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
