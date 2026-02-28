//! Contact record (stored as `contacts/{contact_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{CapTableAccess, ContactCategory, ContactStatus, ContactType};
use crate::domain::ids::{ContactId, EntityId, WorkspaceId};

/// A person or organization that interacts with the entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    contact_id: ContactId,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    contact_type: ContactType,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    category: ContactCategory,
    cap_table_access: CapTableAccess,
    notes: Option<String>,
    status: ContactStatus,
    created_at: DateTime<Utc>,
}

impl Contact {
    pub fn new(
        contact_id: ContactId,
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        contact_type: ContactType,
        name: String,
        email: Option<String>,
        category: ContactCategory,
    ) -> Self {
        Self {
            contact_id,
            entity_id,
            workspace_id,
            contact_type,
            name,
            email,
            phone: None,
            category,
            cap_table_access: CapTableAccess::None_,
            notes: None,
            status: ContactStatus::Active,
            created_at: Utc::now(),
        }
    }

    pub fn deactivate(&mut self) {
        self.status = ContactStatus::Inactive;
    }

    pub fn set_cap_table_access(&mut self, access: CapTableAccess) {
        self.cap_table_access = access;
    }

    pub fn set_phone(&mut self, phone: String) {
        self.phone = Some(phone);
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn set_email(&mut self, email: Option<String>) {
        self.email = email;
    }

    pub fn set_notes(&mut self, notes: String) {
        self.notes = Some(notes);
    }

    // Accessors
    pub fn contact_id(&self) -> ContactId {
        self.contact_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }
    pub fn contact_type(&self) -> ContactType {
        self.contact_type
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
    pub fn phone(&self) -> Option<&str> {
        self.phone.as_deref()
    }
    pub fn category(&self) -> ContactCategory {
        self.category
    }
    pub fn cap_table_access(&self) -> CapTableAccess {
        self.cap_table_access
    }
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
    pub fn status(&self) -> ContactStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_contact() -> Contact {
        Contact::new(
            ContactId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            ContactType::Individual,
            "Jane Doe".to_owned(),
            Some("jane@example.com".to_owned()),
            ContactCategory::Officer,
        )
    }

    #[test]
    fn new_defaults_to_active_and_none_access() {
        let c = make_contact();
        assert_eq!(c.status(), ContactStatus::Active);
        assert_eq!(c.cap_table_access(), CapTableAccess::None_);
        assert_eq!(c.name(), "Jane Doe");
        assert_eq!(c.email(), Some("jane@example.com"));
        assert!(c.phone().is_none());
        assert!(c.notes().is_none());
    }

    #[test]
    fn deactivate() {
        let mut c = make_contact();
        assert_eq!(c.status(), ContactStatus::Active);
        c.deactivate();
        assert_eq!(c.status(), ContactStatus::Inactive);
    }

    #[test]
    fn set_cap_table_access() {
        let mut c = make_contact();
        assert_eq!(c.cap_table_access(), CapTableAccess::None_);
        c.set_cap_table_access(CapTableAccess::Detailed);
        assert_eq!(c.cap_table_access(), CapTableAccess::Detailed);
    }

    #[test]
    fn set_phone_and_notes() {
        let mut c = make_contact();
        c.set_phone("555-1234".to_owned());
        c.set_notes("VIP contact".to_owned());
        assert_eq!(c.phone(), Some("555-1234"));
        assert_eq!(c.notes(), Some("VIP contact"));
    }

    #[test]
    fn serde_roundtrip() {
        let mut c = make_contact();
        c.set_cap_table_access(CapTableAccess::Summary);
        c.set_phone("555-0000".to_owned());

        let json = serde_json::to_string_pretty(&c).expect("serialize");
        let parsed: Contact = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.contact_id(), c.contact_id());
        assert_eq!(parsed.entity_id(), c.entity_id());
        assert_eq!(parsed.name(), c.name());
        assert_eq!(parsed.email(), c.email());
        assert_eq!(parsed.phone(), c.phone());
        assert_eq!(parsed.category(), c.category());
        assert_eq!(parsed.cap_table_access(), CapTableAccess::Summary);
        assert_eq!(parsed.status(), c.status());
        assert_eq!(parsed.contact_type(), ContactType::Individual);
    }
}
