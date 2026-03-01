//! Equity legal entities (stored as `cap-table/entities/{legal_entity_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{EntityId, LegalEntityId, WorkspaceId};

/// Role this legal entity plays in the ownership/control graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegalEntityRole {
    Operating,
    Control,
    Investment,
    Nonprofit,
    Spv,
    Other,
}

/// A legal entity node in the cap-table/control graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalEntity {
    legal_entity_id: LegalEntityId,
    workspace_id: WorkspaceId,
    /// Optional link to the formation aggregate entity for this legal entity.
    linked_entity_id: Option<EntityId>,
    name: String,
    role: LegalEntityRole,
    created_at: DateTime<Utc>,
}

impl LegalEntity {
    pub fn new(
        legal_entity_id: LegalEntityId,
        workspace_id: WorkspaceId,
        linked_entity_id: Option<EntityId>,
        name: String,
        role: LegalEntityRole,
    ) -> Self {
        Self {
            legal_entity_id,
            workspace_id,
            linked_entity_id,
            name,
            role,
            created_at: Utc::now(),
        }
    }

    pub fn legal_entity_id(&self) -> LegalEntityId {
        self.legal_entity_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn linked_entity_id(&self) -> Option<EntityId> {
        self.linked_entity_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn role(&self) -> LegalEntityRole {
        self.role
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
