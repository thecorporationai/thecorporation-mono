//! Entity record — the top-level corporate entity metadata.
//!
//! Stored as `corp.json` in the entity's git repository.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::FormationError;
use super::types::{EntityType, FormationState, FormationStatus};
use crate::domain::ids::{EntityId, WorkspaceId};

/// Maximum length for a legal entity name.
const MAX_LEGAL_NAME_LEN: usize = 500;

/// The top-level corporate entity record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    legal_name: String,
    entity_type: EntityType,
    jurisdiction: String,
    formation_state: FormationState,
    formation_status: FormationStatus,
    registered_agent_name: Option<String>,
    registered_agent_address: Option<String>,
    created_at: DateTime<Utc>,
}

impl Entity {
    /// Create a new entity with validated fields.
    ///
    /// `formation_state` is set to `Forming` and `formation_status` to `Pending`.
    pub fn new(
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        legal_name: String,
        entity_type: EntityType,
        jurisdiction: String,
        registered_agent_name: Option<String>,
        registered_agent_address: Option<String>,
    ) -> Result<Self, FormationError> {
        if legal_name.is_empty() || legal_name.len() > MAX_LEGAL_NAME_LEN {
            return Err(FormationError::Validation(format!(
                "legal_name must be between 1 and {MAX_LEGAL_NAME_LEN} characters, got {}",
                legal_name.len()
            )));
        }

        Ok(Self {
            entity_id,
            workspace_id,
            legal_name,
            entity_type,
            jurisdiction,
            formation_state: FormationState::Forming,
            formation_status: FormationStatus::Pending,
            registered_agent_name,
            registered_agent_address,
            created_at: Utc::now(),
        })
    }

    /// Advance the formation status, checking the FSM for valid transitions.
    pub fn advance_status(&mut self, to: FormationStatus) -> Result<(), FormationError> {
        if !self.formation_status.allowed_transitions().contains(&to) {
            return Err(FormationError::InvalidTransition {
                from: self.formation_status,
                to,
            });
        }
        self.formation_status = to;
        Ok(())
    }

    /// Change the entity type (e.g., LLC → Corporation conversion).
    ///
    /// Only allowed when the entity is in `Pending` or `Active` formation status.
    pub fn set_entity_type(&mut self, entity_type: EntityType) -> Result<(), FormationError> {
        match self.formation_status {
            FormationStatus::Pending | FormationStatus::Active => {
                self.entity_type = entity_type;
                Ok(())
            }
            _ => Err(FormationError::Validation(format!(
                "entity type conversion only allowed in Pending or Active status, currently {}",
                self.formation_status
            ))),
        }
    }

    /// Dissolve the entity — transitions from Active to Dissolved.
    pub fn dissolve(&mut self) -> Result<(), FormationError> {
        self.advance_status(FormationStatus::Dissolved)
    }

    /// Activate the entity — sets both `formation_state` to `Active` and
    /// `formation_status` to `Active` (terminal).
    pub fn activate(&mut self) {
        self.formation_state = FormationState::Active;
        self.formation_status = FormationStatus::Active;
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn legal_name(&self) -> &str {
        &self.legal_name
    }

    pub fn entity_type(&self) -> EntityType {
        self.entity_type
    }

    pub fn jurisdiction(&self) -> &str {
        &self.jurisdiction
    }

    pub fn formation_state(&self) -> FormationState {
        self.formation_state
    }

    pub fn formation_status(&self) -> FormationStatus {
        self.formation_status
    }

    pub fn registered_agent_name(&self) -> Option<&str> {
        self.registered_agent_name.as_deref()
    }

    pub fn registered_agent_address(&self) -> Option<&str> {
        self.registered_agent_address.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity() -> Entity {
        Entity::new(
            EntityId::new(),
            WorkspaceId::new(),
            "Acme Corp".into(),
            EntityType::Corporation,
            "US-DE".into(),
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn new_entity_has_correct_defaults() {
        let e = make_entity();
        assert_eq!(e.formation_state(), FormationState::Forming);
        assert_eq!(e.formation_status(), FormationStatus::Pending);
        assert_eq!(e.legal_name(), "Acme Corp");
    }

    #[test]
    fn rejects_empty_legal_name() {
        let result = Entity::new(
            EntityId::new(),
            WorkspaceId::new(),
            "".into(),
            EntityType::Llc,
            "US-WY".into(),
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_oversized_legal_name() {
        let long_name = "X".repeat(501);
        let result = Entity::new(
            EntityId::new(),
            WorkspaceId::new(),
            long_name,
            EntityType::Llc,
            "US-WY".into(),
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn advance_status_follows_fsm() {
        let mut e = make_entity();
        assert!(e
            .advance_status(FormationStatus::DocumentsGenerated)
            .is_ok());
        assert_eq!(e.formation_status(), FormationStatus::DocumentsGenerated);
    }

    #[test]
    fn advance_status_rejects_invalid() {
        let mut e = make_entity();
        // Cannot jump from Pending to Filed
        assert!(e.advance_status(FormationStatus::Filed).is_err());
    }

    #[test]
    fn activate_sets_terminal_state() {
        let mut e = make_entity();
        e.activate();
        assert_eq!(e.formation_state(), FormationState::Active);
        assert_eq!(e.formation_status(), FormationStatus::Active);
    }

    #[test]
    fn serde_roundtrip() {
        let e = make_entity();
        let json = serde_json::to_string(&e).unwrap();
        let parsed: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entity_id(), e.entity_id());
        assert_eq!(parsed.legal_name(), e.legal_name());
    }
}
