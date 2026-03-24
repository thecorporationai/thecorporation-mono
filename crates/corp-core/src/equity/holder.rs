//! Equity holder records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ContactId, EntityId, HolderId};

/// The classification of an equity holder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HolderType {
    Individual,
    Entity,
    Trust,
}

/// A person or organisation that holds equity in a company.
///
/// `contact_id` links the holder to a `Contact` record when one exists (e.g.
/// a natural person or a known investor entity). It may be `None` for
/// historical or synthetic holders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holder {
    pub holder_id: HolderId,
    pub entity_id: EntityId,
    pub contact_id: Option<ContactId>,
    pub name: String,
    pub holder_type: HolderType,
    pub created_at: DateTime<Utc>,
}

impl Holder {
    /// Create a new holder record.
    pub fn new(
        entity_id: EntityId,
        contact_id: Option<ContactId>,
        name: impl Into<String>,
        holder_type: HolderType,
    ) -> Self {
        Self {
            holder_id: HolderId::new(),
            entity_id,
            contact_id,
            name: name.into(),
            holder_type,
            created_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_individual() -> Holder {
        Holder::new(
            EntityId::new(),
            Some(ContactId::new()),
            "Jane Founder",
            HolderType::Individual,
        )
    }

    fn make_entity_holder() -> Holder {
        Holder::new(
            EntityId::new(),
            Some(ContactId::new()),
            "Acme VC Fund I",
            HolderType::Entity,
        )
    }

    fn make_trust_holder() -> Holder {
        Holder::new(
            EntityId::new(),
            None,
            "Jane Founder 2026 Trust",
            HolderType::Trust,
        )
    }

    // ── Holder::new() ─────────────────────────────────────────────────────────

    #[test]
    fn new_individual_stores_type() {
        let h = make_individual();
        assert_eq!(h.holder_type, HolderType::Individual);
    }

    #[test]
    fn new_individual_stores_name() {
        let h = make_individual();
        assert_eq!(h.name, "Jane Founder");
    }

    #[test]
    fn new_individual_stores_contact_id() {
        let cid = ContactId::new();
        let h = Holder::new(EntityId::new(), Some(cid), "Alice", HolderType::Individual);
        assert_eq!(h.contact_id, Some(cid));
    }

    #[test]
    fn new_entity_holder_stores_type() {
        let h = make_entity_holder();
        assert_eq!(h.holder_type, HolderType::Entity);
    }

    #[test]
    fn new_trust_holder_stores_type() {
        let h = make_trust_holder();
        assert_eq!(h.holder_type, HolderType::Trust);
    }

    #[test]
    fn new_trust_holder_no_contact_id() {
        let h = make_trust_holder();
        assert!(h.contact_id.is_none());
    }

    #[test]
    fn new_holder_without_contact_id() {
        let h = Holder::new(EntityId::new(), None, "Anonymous", HolderType::Individual);
        assert!(h.contact_id.is_none());
    }

    #[test]
    fn new_holder_has_unique_id() {
        let a = make_individual();
        let b = make_individual();
        assert_ne!(a.holder_id, b.holder_id);
    }

    // ── HolderType serde ──────────────────────────────────────────────────────

    #[test]
    fn holder_type_serde_individual() {
        let json = serde_json::to_string(&HolderType::Individual).unwrap();
        assert_eq!(json, r#""individual""#);
        let de: HolderType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, HolderType::Individual);
    }

    #[test]
    fn holder_type_serde_entity() {
        let json = serde_json::to_string(&HolderType::Entity).unwrap();
        assert_eq!(json, r#""entity""#);
        let de: HolderType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, HolderType::Entity);
    }

    #[test]
    fn holder_type_serde_trust() {
        let json = serde_json::to_string(&HolderType::Trust).unwrap();
        assert_eq!(json, r#""trust""#);
        let de: HolderType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, HolderType::Trust);
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn holder_serde_roundtrip_individual() {
        let h = make_individual();
        let json = serde_json::to_string(&h).unwrap();
        let de: Holder = serde_json::from_str(&json).unwrap();
        assert_eq!(de.holder_id, h.holder_id);
        assert_eq!(de.holder_type, HolderType::Individual);
        assert_eq!(de.name, "Jane Founder");
    }

    #[test]
    fn holder_serde_roundtrip_trust_no_contact() {
        let h = make_trust_holder();
        let json = serde_json::to_string(&h).unwrap();
        let de: Holder = serde_json::from_str(&json).unwrap();
        assert_eq!(de.holder_type, HolderType::Trust);
        assert!(de.contact_id.is_none());
    }
}
