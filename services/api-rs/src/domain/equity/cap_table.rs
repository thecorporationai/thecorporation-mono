//! Cap table record (stored as `cap-table/cap-table.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::CapTableStatus;
use crate::domain::ids::{CapTableId, EntityId};

/// The root cap-table record for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapTable {
    cap_table_id: CapTableId,
    entity_id: EntityId,
    status: CapTableStatus,
    as_of_version: u32,
    created_at: DateTime<Utc>,
}

impl CapTable {
    /// Create a new active cap table.
    pub fn new(cap_table_id: CapTableId, entity_id: EntityId) -> Self {
        Self {
            cap_table_id,
            entity_id,
            status: CapTableStatus::Active,
            as_of_version: 1,
            created_at: Utc::now(),
        }
    }

    pub fn cap_table_id(&self) -> CapTableId {
        self.cap_table_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn status(&self) -> CapTableStatus {
        self.status
    }

    pub fn as_of_version(&self) -> u32 {
        self.as_of_version
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_cap_table() {
        let ct = CapTable::new(CapTableId::new(), EntityId::new());
        assert_eq!(ct.status(), CapTableStatus::Active);
        assert_eq!(ct.as_of_version(), 1);
    }

    #[test]
    fn serde_roundtrip() {
        let ct = CapTable::new(CapTableId::new(), EntityId::new());
        let json = serde_json::to_string(&ct).unwrap();
        let parsed: CapTable = serde_json::from_str(&json).unwrap();
        assert_eq!(ct.cap_table_id(), parsed.cap_table_id());
        assert_eq!(ct.entity_id(), parsed.entity_id());
        assert_eq!(ct.status(), parsed.status());
    }
}
