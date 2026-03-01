//! Governance execution mode state (stored as `governance/mode.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ContactId, EntityId};

/// Governance mode for policy enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceMode {
    Normal,
    PrincipalUnavailable,
    IncidentLockdown,
}

/// Current governance mode state for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceModeState {
    entity_id: EntityId,
    mode: GovernanceMode,
    reason: Option<String>,
    updated_by: Option<ContactId>,
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl GovernanceModeState {
    pub fn new(entity_id: EntityId) -> Self {
        let now = Utc::now();
        Self {
            entity_id,
            mode: GovernanceMode::Normal,
            reason: None,
            updated_by: None,
            updated_at: now,
            created_at: now,
        }
    }

    pub fn set_mode(
        &mut self,
        mode: GovernanceMode,
        reason: Option<String>,
        updated_by: Option<ContactId>,
    ) {
        self.mode = mode;
        self.reason = reason;
        self.updated_by = updated_by;
        self.updated_at = Utc::now();
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn mode(&self) -> GovernanceMode {
        self.mode
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    pub fn updated_by(&self) -> Option<ContactId> {
        self.updated_by
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_normal() {
        let state = GovernanceModeState::new(EntityId::new());
        assert_eq!(state.mode(), GovernanceMode::Normal);
        assert!(state.reason().is_none());
    }

    #[test]
    fn set_mode_updates_state() {
        let mut state = GovernanceModeState::new(EntityId::new());
        let updater = ContactId::new();
        state.set_mode(
            GovernanceMode::IncidentLockdown,
            Some("security incident".to_owned()),
            Some(updater),
        );
        assert_eq!(state.mode(), GovernanceMode::IncidentLockdown);
        assert_eq!(state.reason(), Some("security incident"));
        assert_eq!(state.updated_by(), Some(updater));
    }
}
