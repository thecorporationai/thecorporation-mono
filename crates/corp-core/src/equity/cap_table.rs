//! Cap table aggregate root.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{CapTableId, EntityId};
use super::types::CapTableStatus;

/// The cap table for a single legal entity. All equity instruments — share
/// classes, grants, SAFEs, funding rounds — are scoped to a `CapTable`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapTable {
    pub cap_table_id: CapTableId,
    pub entity_id: EntityId,
    pub status: CapTableStatus,
    pub created_at: DateTime<Utc>,
}

impl CapTable {
    /// Create a new, active cap table for the given entity.
    pub fn new(entity_id: EntityId) -> Self {
        Self {
            cap_table_id: CapTableId::new(),
            entity_id,
            status: CapTableStatus::Active,
            created_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cap_table() -> CapTable {
        CapTable::new(EntityId::new())
    }

    #[test]
    fn new_cap_table_status_is_active() {
        let ct = make_cap_table();
        assert_eq!(ct.status, CapTableStatus::Active);
    }

    #[test]
    fn new_cap_table_has_unique_id() {
        let a = make_cap_table();
        let b = make_cap_table();
        assert_ne!(a.cap_table_id, b.cap_table_id);
    }

    #[test]
    fn new_cap_table_stores_entity_id() {
        let eid = EntityId::new();
        let ct = CapTable::new(eid);
        assert_eq!(ct.entity_id, eid);
    }

    #[test]
    fn cap_table_serde_roundtrip() {
        let ct = make_cap_table();
        let json = serde_json::to_string(&ct).unwrap();
        let de: CapTable = serde_json::from_str(&json).unwrap();
        assert_eq!(de.cap_table_id, ct.cap_table_id);
        assert_eq!(de.status, CapTableStatus::Active);
    }

    #[test]
    fn two_cap_tables_for_same_entity() {
        let eid = EntityId::new();
        let a = CapTable::new(eid);
        let b = CapTable::new(eid);
        assert_ne!(a.cap_table_id, b.cap_table_id);
        assert_eq!(a.entity_id, b.entity_id);
    }
}
