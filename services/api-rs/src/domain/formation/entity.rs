//! Entity record — the top-level corporate entity metadata.
//!
//! Stored as `corp.json` in the entity's git repository.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::FormationError;
use super::types::{EntityType, FormationState, FormationStatus, Jurisdiction};
use crate::domain::ids::{ContractId, DocumentId, EntityId, WorkspaceId};

/// Maximum length for a legal entity name.
const MAX_LEGAL_NAME_LEN: usize = 500;

/// Validate data-integrity invariants shared by `new()` and `TryFrom<RawEntity>`.
fn validate_legal_name(legal_name: &str) -> Result<(), FormationError> {
    if legal_name.is_empty() || legal_name.len() > MAX_LEGAL_NAME_LEN {
        return Err(FormationError::Validation(format!(
            "legal_name must be between 1 and {MAX_LEGAL_NAME_LEN} characters, got {}",
            legal_name.len()
        )));
    }
    Ok(())
}

// ── Raw mirror for deserialization ──────────────────────────────────────

#[derive(Deserialize)]
struct RawEntity {
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    legal_name: String,
    entity_type: EntityType,
    jurisdiction: Jurisdiction,
    formation_state: FormationState,
    formation_status: FormationStatus,
    registered_agent_name: Option<String>,
    registered_agent_address: Option<String>,
    #[serde(default)]
    formation_date: Option<DateTime<Utc>>,
    #[serde(default)]
    service_agreement_executed: bool,
    #[serde(default)]
    service_agreement_executed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    service_agreement_contract_id: Option<ContractId>,
    #[serde(default)]
    service_agreement_document_id: Option<DocumentId>,
    #[serde(default)]
    service_agreement_notes: Option<String>,
    created_at: DateTime<Utc>,
}

impl TryFrom<RawEntity> for Entity {
    type Error = FormationError;

    fn try_from(raw: RawEntity) -> Result<Self, Self::Error> {
        validate_legal_name(&raw.legal_name)?;
        Ok(Entity {
            entity_id: raw.entity_id,
            workspace_id: raw.workspace_id,
            legal_name: raw.legal_name,
            entity_type: raw.entity_type,
            jurisdiction: raw.jurisdiction,
            formation_state: raw.formation_state,
            formation_status: raw.formation_status,
            registered_agent_name: raw.registered_agent_name,
            registered_agent_address: raw.registered_agent_address,
            formation_date: raw.formation_date,
            service_agreement_executed: raw.service_agreement_executed,
            service_agreement_executed_at: raw.service_agreement_executed_at,
            service_agreement_contract_id: raw.service_agreement_contract_id,
            service_agreement_document_id: raw.service_agreement_document_id,
            service_agreement_notes: raw.service_agreement_notes,
            created_at: raw.created_at,
        })
    }
}

// ── Entity ──────────────────────────────────────────────────────────────

/// The top-level corporate entity record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RawEntity")]
pub struct Entity {
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    legal_name: String,
    entity_type: EntityType,
    jurisdiction: Jurisdiction,
    formation_state: FormationState,
    formation_status: FormationStatus,
    registered_agent_name: Option<String>,
    registered_agent_address: Option<String>,
    formation_date: Option<DateTime<Utc>>,
    service_agreement_executed: bool,
    service_agreement_executed_at: Option<DateTime<Utc>>,
    service_agreement_contract_id: Option<ContractId>,
    service_agreement_document_id: Option<DocumentId>,
    service_agreement_notes: Option<String>,
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
        jurisdiction: Jurisdiction,
        registered_agent_name: Option<String>,
        registered_agent_address: Option<String>,
    ) -> Result<Self, FormationError> {
        validate_legal_name(&legal_name)?;

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
            formation_date: None,
            service_agreement_executed: false,
            service_agreement_executed_at: None,
            service_agreement_contract_id: None,
            service_agreement_document_id: None,
            service_agreement_notes: None,
            created_at: Utc::now(),
        })
    }

    /// Advance the formation status, checking the FSM for valid transitions.
    ///
    /// When transitioning to `Active`, also sets `formation_state` to `Active`.
    pub fn advance_status(&mut self, to: FormationStatus) -> Result<(), FormationError> {
        if !self.formation_status.allowed_transitions().contains(&to) {
            return Err(FormationError::InvalidTransition {
                from: self.formation_status,
                to,
            });
        }
        self.formation_status = to;
        if to == FormationStatus::Active {
            self.formation_state = FormationState::Active;
            if self.formation_date.is_none() {
                self.formation_date = Some(Utc::now());
            }
        }
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

    pub fn set_jurisdiction(&mut self, jurisdiction: Jurisdiction) -> Result<(), FormationError> {
        match self.formation_status {
            FormationStatus::Pending => {
                self.jurisdiction = jurisdiction;
                Ok(())
            }
            _ => Err(FormationError::Validation(format!(
                "jurisdiction change only allowed in Pending status, currently {}",
                self.formation_status
            ))),
        }
    }

    pub fn set_registered_agent(
        &mut self,
        registered_agent_name: Option<String>,
        registered_agent_address: Option<String>,
    ) -> Result<(), FormationError> {
        match self.formation_status {
            FormationStatus::Pending => {
                self.registered_agent_name = registered_agent_name;
                self.registered_agent_address = registered_agent_address;
                Ok(())
            }
            _ => Err(FormationError::Validation(format!(
                "registered agent changes only allowed in Pending status, currently {}",
                self.formation_status
            ))),
        }
    }

    /// Dissolve the entity — transitions from Active to Dissolved.
    pub fn dissolve(&mut self) -> Result<(), FormationError> {
        self.advance_status(FormationStatus::Dissolved)
    }

    pub fn record_service_agreement_execution(
        &mut self,
        contract_id: Option<ContractId>,
        document_id: Option<DocumentId>,
        notes: Option<String>,
    ) {
        self.service_agreement_executed = true;
        self.service_agreement_executed_at = Some(Utc::now());
        self.service_agreement_contract_id = contract_id;
        self.service_agreement_document_id = document_id;
        self.service_agreement_notes = notes;
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

    pub fn jurisdiction(&self) -> &Jurisdiction {
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

    pub fn formation_date(&self) -> Option<DateTime<Utc>> {
        self.formation_date
    }

    pub fn set_formation_date(&mut self, date: DateTime<Utc>) {
        self.formation_date = Some(date);
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn service_agreement_executed(&self) -> bool {
        self.service_agreement_executed
    }

    pub fn service_agreement_executed_at(&self) -> Option<DateTime<Utc>> {
        self.service_agreement_executed_at
    }

    pub fn service_agreement_contract_id(&self) -> Option<ContractId> {
        self.service_agreement_contract_id
    }

    pub fn service_agreement_document_id(&self) -> Option<DocumentId> {
        self.service_agreement_document_id
    }

    pub fn service_agreement_notes(&self) -> Option<&str> {
        self.service_agreement_notes.as_deref()
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
            EntityType::CCorp,
            Jurisdiction::new("US-DE").unwrap(),
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
            Jurisdiction::new("US-WY").unwrap(),
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
            Jurisdiction::new("US-WY").unwrap(),
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn advance_status_follows_fsm() {
        let mut e = make_entity();
        assert!(
            e.advance_status(FormationStatus::DocumentsGenerated)
                .is_ok()
        );
        assert_eq!(e.formation_status(), FormationStatus::DocumentsGenerated);
    }

    #[test]
    fn advance_status_rejects_invalid() {
        let mut e = make_entity();
        // Cannot jump from Pending to Filed
        assert!(e.advance_status(FormationStatus::Filed).is_err());
    }

    #[test]
    fn advance_to_active_sets_formation_state() {
        let mut e = make_entity();
        // Walk the FSM to Active
        e.advance_status(FormationStatus::DocumentsGenerated)
            .unwrap();
        e.advance_status(FormationStatus::DocumentsSigned).unwrap();
        e.advance_status(FormationStatus::FilingSubmitted).unwrap();
        e.advance_status(FormationStatus::Filed).unwrap();
        e.advance_status(FormationStatus::EinApplied).unwrap();
        e.advance_status(FormationStatus::Active).unwrap();
        assert_eq!(e.formation_state(), FormationState::Active);
        assert_eq!(e.formation_status(), FormationStatus::Active);
    }

    #[test]
    fn advance_to_active_auto_sets_formation_date() {
        let mut e = make_entity();
        assert!(e.formation_date().is_none());
        // Walk the FSM to Active
        e.advance_status(FormationStatus::DocumentsGenerated)
            .unwrap();
        e.advance_status(FormationStatus::DocumentsSigned).unwrap();
        e.advance_status(FormationStatus::FilingSubmitted).unwrap();
        e.advance_status(FormationStatus::Filed).unwrap();
        e.advance_status(FormationStatus::EinApplied).unwrap();
        e.advance_status(FormationStatus::Active).unwrap();
        assert!(e.formation_date().is_some());
    }

    #[test]
    fn explicit_formation_date_preserved_on_advance() {
        let mut e = make_entity();
        let explicit_date = chrono::Utc::now() - chrono::Duration::days(30);
        e.set_formation_date(explicit_date);
        // Walk the FSM to Active
        e.advance_status(FormationStatus::DocumentsGenerated)
            .unwrap();
        e.advance_status(FormationStatus::DocumentsSigned).unwrap();
        e.advance_status(FormationStatus::FilingSubmitted).unwrap();
        e.advance_status(FormationStatus::Filed).unwrap();
        e.advance_status(FormationStatus::EinApplied).unwrap();
        e.advance_status(FormationStatus::Active).unwrap();
        // Should preserve the explicitly set date, not override it
        assert_eq!(e.formation_date(), Some(explicit_date));
    }

    #[test]
    fn registered_agent_can_change_while_pending() {
        let mut e = make_entity();
        e.set_registered_agent(
            Some("Delaware RA".to_owned()),
            Some("123 Main St, Dover, DE 19901".to_owned()),
        )
        .unwrap();
        assert_eq!(e.registered_agent_name(), Some("Delaware RA"));
        assert_eq!(
            e.registered_agent_address(),
            Some("123 Main St, Dover, DE 19901")
        );
    }

    #[test]
    fn backward_compat_deserialization_without_formation_date() {
        // Simulate old JSON without formation_date field
        let e = make_entity();
        let mut json: serde_json::Value = serde_json::to_value(&e).unwrap();
        // Remove formation_date to simulate old data
        json.as_object_mut().unwrap().remove("formation_date");
        let parsed: Entity = serde_json::from_value(json).unwrap();
        assert!(parsed.formation_date().is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let e = make_entity();
        let json = serde_json::to_string(&e).unwrap();
        let parsed: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entity_id(), e.entity_id());
        assert_eq!(parsed.legal_name(), e.legal_name());
    }

    #[test]
    fn deserialize_rejects_empty_legal_name() {
        let e = make_entity();
        let mut json: serde_json::Value = serde_json::to_value(&e).unwrap();
        json["legal_name"] = serde_json::json!("");
        let result: Result<Entity, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_rejects_oversized_legal_name() {
        let e = make_entity();
        let mut json: serde_json::Value = serde_json::to_value(&e).unwrap();
        json["legal_name"] = serde_json::json!("X".repeat(501));
        let result: Result<Entity, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
