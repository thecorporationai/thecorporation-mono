//! Control relationships between legal entities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ControlLinkId, LegalEntityId};

/// Type of control relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControlType {
    Voting,
    Board,
    Economic,
    Contractual,
}

/// Relationship edge in the control map graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlLink {
    control_link_id: ControlLinkId,
    parent_legal_entity_id: LegalEntityId,
    child_legal_entity_id: LegalEntityId,
    control_type: ControlType,
    /// Optional voting power in basis points (10000 = 100%).
    voting_power_bps: Option<u32>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
}

impl ControlLink {
    pub fn new(
        control_link_id: ControlLinkId,
        parent_legal_entity_id: LegalEntityId,
        child_legal_entity_id: LegalEntityId,
        control_type: ControlType,
        voting_power_bps: Option<u32>,
        notes: Option<String>,
    ) -> Self {
        Self {
            control_link_id,
            parent_legal_entity_id,
            child_legal_entity_id,
            control_type,
            voting_power_bps,
            notes,
            created_at: Utc::now(),
        }
    }

    pub fn control_link_id(&self) -> ControlLinkId {
        self.control_link_id
    }

    pub fn parent_legal_entity_id(&self) -> LegalEntityId {
        self.parent_legal_entity_id
    }

    pub fn child_legal_entity_id(&self) -> LegalEntityId {
        self.child_legal_entity_id
    }

    pub fn control_type(&self) -> ControlType {
        self.control_type
    }

    pub fn voting_power_bps(&self) -> Option<u32> {
        self.voting_power_bps
    }

    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
