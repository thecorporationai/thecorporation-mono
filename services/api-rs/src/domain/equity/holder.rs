//! Equity holder identity (stored as `cap-table/holders/{holder_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ContactId, EntityId, HolderId};

/// Type of holder represented in the cap table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HolderType {
    Individual,
    Organization,
    Fund,
    Nonprofit,
    Trust,
    Other,
}

/// Canonical ownership identity, linked to a contact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holder {
    holder_id: HolderId,
    contact_id: ContactId,
    /// Optional link to an internal formation entity representing this holder.
    linked_entity_id: Option<EntityId>,
    name: String,
    holder_type: HolderType,
    external_reference: Option<String>,
    created_at: DateTime<Utc>,
}

impl Holder {
    pub fn new(
        holder_id: HolderId,
        contact_id: ContactId,
        linked_entity_id: Option<EntityId>,
        name: String,
        holder_type: HolderType,
        external_reference: Option<String>,
    ) -> Self {
        Self {
            holder_id,
            contact_id,
            linked_entity_id,
            name,
            holder_type,
            external_reference,
            created_at: Utc::now(),
        }
    }

    pub fn holder_id(&self) -> HolderId {
        self.holder_id
    }

    pub fn contact_id(&self) -> ContactId {
        self.contact_id
    }

    pub fn linked_entity_id(&self) -> Option<EntityId> {
        self.linked_entity_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn holder_type(&self) -> HolderType {
        self.holder_type
    }

    pub fn external_reference(&self) -> Option<&str> {
        self.external_reference.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
