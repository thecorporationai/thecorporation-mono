//! Legal entity records used in the equity and control-link graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{EntityId, LegalEntityId, WorkspaceId};

// ── LegalEntityRole ───────────────────────────────────────────────────────────

/// The functional role of a legal entity within the corporate structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegalEntityRole {
    /// The primary operating company.
    Operating,
    /// A holding or control company.
    Control,
    /// An investment vehicle (fund, LP, etc.).
    Investment,
    /// A nonprofit or public-benefit entity.
    Nonprofit,
    /// A special-purpose vehicle.
    Spv,
    Other,
}

// ── LegalEntity ───────────────────────────────────────────────────────────────

/// A registered legal entity, which may be a holding company, operating
/// company, SPV, fund, or other legal structure.
///
/// `linked_entity_id` optionally connects this record to a formation
/// [`Entity`][corp_core::formation::Entity] when one exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalEntity {
    pub legal_entity_id: LegalEntityId,
    pub workspace_id: WorkspaceId,
    /// Optional link to a formation-domain `Entity` record.
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub role: LegalEntityRole,
    pub created_at: DateTime<Utc>,
}

impl LegalEntity {
    /// Create a new legal entity record.
    pub fn new(
        workspace_id: WorkspaceId,
        linked_entity_id: Option<EntityId>,
        name: impl Into<String>,
        role: LegalEntityRole,
    ) -> Self {
        Self {
            legal_entity_id: LegalEntityId::new(),
            workspace_id,
            linked_entity_id,
            name: name.into(),
            role,
            created_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_legal_entity(role: LegalEntityRole) -> LegalEntity {
        LegalEntity::new(
            WorkspaceId::new(),
            Some(EntityId::new()),
            "Acme Corp",
            role,
        )
    }

    #[test]
    fn new_legal_entity_stores_name() {
        let le = make_legal_entity(LegalEntityRole::Operating);
        assert_eq!(le.name, "Acme Corp");
    }

    #[test]
    fn new_legal_entity_stores_role() {
        let le = make_legal_entity(LegalEntityRole::Operating);
        assert_eq!(le.role, LegalEntityRole::Operating);
    }

    #[test]
    fn new_legal_entity_stores_linked_entity_id() {
        let le = make_legal_entity(LegalEntityRole::Operating);
        assert!(le.linked_entity_id.is_some());
    }

    #[test]
    fn new_legal_entity_without_linked_entity() {
        let le = LegalEntity::new(WorkspaceId::new(), None, "HoldCo", LegalEntityRole::Control);
        assert!(le.linked_entity_id.is_none());
    }

    #[test]
    fn new_legal_entity_has_unique_id() {
        let a = make_legal_entity(LegalEntityRole::Spv);
        let b = make_legal_entity(LegalEntityRole::Spv);
        assert_ne!(a.legal_entity_id, b.legal_entity_id);
    }

    #[test]
    fn legal_entity_serde_roundtrip() {
        let le = make_legal_entity(LegalEntityRole::Investment);
        let json = serde_json::to_string(&le).unwrap();
        let de: LegalEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(de.legal_entity_id, le.legal_entity_id);
        assert_eq!(de.role, LegalEntityRole::Investment);
        assert_eq!(de.name, "Acme Corp");
    }

    // ── LegalEntityRole serde ─────────────────────────────────────────────────

    #[test]
    fn role_serde_operating() {
        let json = serde_json::to_string(&LegalEntityRole::Operating).unwrap();
        assert_eq!(json, r#""operating""#);
        let de: LegalEntityRole = serde_json::from_str(&json).unwrap();
        assert_eq!(de, LegalEntityRole::Operating);
    }

    #[test]
    fn role_serde_control() {
        let json = serde_json::to_string(&LegalEntityRole::Control).unwrap();
        assert_eq!(json, r#""control""#);
    }

    #[test]
    fn role_serde_investment() {
        let json = serde_json::to_string(&LegalEntityRole::Investment).unwrap();
        assert_eq!(json, r#""investment""#);
    }

    #[test]
    fn role_serde_nonprofit() {
        let json = serde_json::to_string(&LegalEntityRole::Nonprofit).unwrap();
        assert_eq!(json, r#""nonprofit""#);
    }

    #[test]
    fn role_serde_spv() {
        let json = serde_json::to_string(&LegalEntityRole::Spv).unwrap();
        assert_eq!(json, r#""spv""#);
    }

    #[test]
    fn role_serde_other() {
        let json = serde_json::to_string(&LegalEntityRole::Other).unwrap();
        assert_eq!(json, r#""other""#);
    }
}
