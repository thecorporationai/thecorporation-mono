//! Contacts domain — people and organisations associated with an entity.
//!
//! A `Contact` represents any party that interacts with the entity: founders,
//! employees, investors, advisors, professional service firms, and so on.
//! The `ContactCategory` distinguishes their relationship role while
//! `ContactType` separates individuals from organizations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{ContactId, EntityId, WorkspaceId};

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContactError {
    #[error("contact name must be between 1 and 256 characters, got {0}")]
    InvalidName(usize),

    #[error("contact is already inactive")]
    AlreadyInactive,
}

// ── ContactType ───────────────────────────────────────────────────────────────

/// Whether the contact is a natural person or a legal entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactType {
    Individual,
    Organization,
}

// ── ContactCategory ───────────────────────────────────────────────────────────

/// The relationship role this contact has with the entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactCategory {
    Employee,
    Contractor,
    BoardMember,
    LawFirm,
    ValuationFirm,
    AccountingFirm,
    Investor,
    Officer,
    Founder,
    Member,
    Other,
}

// ── CapTableAccess ────────────────────────────────────────────────────────────

/// The level of cap table visibility granted to this contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapTableAccess {
    /// No cap table visibility.
    None,
    /// Can view aggregated totals only.
    Summary,
    /// Full position-level breakdown visible.
    Detailed,
}

// ── ContactStatus ─────────────────────────────────────────────────────────────

/// Lifecycle status of a contact record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactStatus {
    Active,
    Inactive,
}

// ── Contact ───────────────────────────────────────────────────────────────────

/// A party associated with a legal entity.
///
/// Contacts are workspace-scoped and entity-scoped — a contact record belongs
/// to exactly one entity within a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub contact_id: ContactId,
    pub entity_id: EntityId,
    pub workspace_id: WorkspaceId,

    pub contact_type: ContactType,

    /// Display / legal name (1–256 characters).
    pub name: String,

    pub email: Option<String>,
    pub mailing_address: Option<String>,
    pub phone: Option<String>,

    /// Relationship role.
    pub category: ContactCategory,

    /// Cap table visibility granted to this contact.
    pub cap_table_access: CapTableAccess,

    /// Internal notes about this contact (not visible to the contact).
    pub notes: Option<String>,

    pub status: ContactStatus,

    pub created_at: DateTime<Utc>,
}

impl Contact {
    /// Create a new active contact.
    ///
    /// # Errors
    /// Returns [`ContactError::InvalidName`] if `name` is empty or longer than
    /// 256 characters.
    pub fn new(
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        contact_type: ContactType,
        name: impl Into<String>,
        category: ContactCategory,
    ) -> Result<Self, ContactError> {
        let name = name.into();
        Self::validate_name_str(&name)?;

        Ok(Self {
            contact_id: ContactId::new(),
            entity_id,
            workspace_id,
            contact_type,
            name,
            email: None,
            mailing_address: None,
            phone: None,
            category,
            cap_table_access: CapTableAccess::None,
            notes: None,
            status: ContactStatus::Active,
            created_at: Utc::now(),
        })
    }

    // ── Validation ────────────────────────────────────────────────────────────

    /// Validate that the contact's current name is within allowed bounds.
    pub fn validate_name(&self) -> Result<(), ContactError> {
        Self::validate_name_str(&self.name)
    }

    fn validate_name_str(name: &str) -> Result<(), ContactError> {
        let len = name.len();
        if len == 0 || len > 256 {
            Err(ContactError::InvalidName(len))
        } else {
            Ok(())
        }
    }

    // ── Status transitions ────────────────────────────────────────────────────

    /// Mark the contact as inactive.
    ///
    /// # Errors
    /// Returns [`ContactError::AlreadyInactive`] if the contact is already
    /// inactive.
    pub fn deactivate(&mut self) -> Result<(), ContactError> {
        if self.status == ContactStatus::Inactive {
            return Err(ContactError::AlreadyInactive);
        }
        self.status = ContactStatus::Inactive;
        Ok(())
    }

    /// Reactivate a previously deactivated contact.
    pub fn reactivate(&mut self) {
        self.status = ContactStatus::Active;
    }

    // ── Field setters ─────────────────────────────────────────────────────────

    /// Update the display name.
    ///
    /// # Errors
    /// Returns [`ContactError::InvalidName`] if the new name fails validation.
    pub fn set_name(&mut self, name: impl Into<String>) -> Result<(), ContactError> {
        let name = name.into();
        Self::validate_name_str(&name)?;
        self.name = name;
        Ok(())
    }

    pub fn set_email(&mut self, email: Option<String>) {
        self.email = email;
    }

    pub fn set_mailing_address(&mut self, address: Option<String>) {
        self.mailing_address = address;
    }

    pub fn set_phone(&mut self, phone: Option<String>) {
        self.phone = phone;
    }

    pub fn set_category(&mut self, category: ContactCategory) {
        self.category = category;
    }

    pub fn set_cap_table_access(&mut self, access: CapTableAccess) {
        self.cap_table_access = access;
    }

    pub fn set_notes(&mut self, notes: Option<String>) {
        self.notes = notes;
    }

    pub fn set_contact_type(&mut self, contact_type: ContactType) {
        self.contact_type = contact_type;
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn is_active(&self) -> bool {
        self.status == ContactStatus::Active
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_contact() -> Contact {
        Contact::new(
            EntityId::new(),
            WorkspaceId::new(),
            ContactType::Individual,
            "Jane Founder",
            ContactCategory::Founder,
        )
        .unwrap()
    }

    #[test]
    fn new_contact_defaults() {
        let c = make_contact();
        assert_eq!(c.status, ContactStatus::Active);
        assert_eq!(c.cap_table_access, CapTableAccess::None);
        assert!(c.email.is_none());
        assert!(c.is_active());
    }

    #[test]
    fn deactivate_sets_inactive() {
        let mut c = make_contact();
        c.deactivate().unwrap();
        assert_eq!(c.status, ContactStatus::Inactive);
        assert!(!c.is_active());
    }

    #[test]
    fn deactivate_twice_fails() {
        let mut c = make_contact();
        c.deactivate().unwrap();
        assert!(matches!(c.deactivate(), Err(ContactError::AlreadyInactive)));
    }

    #[test]
    fn reactivate_after_deactivate() {
        let mut c = make_contact();
        c.deactivate().unwrap();
        c.reactivate();
        assert!(c.is_active());
    }

    #[test]
    fn set_name_validates_length() {
        let mut c = make_contact();
        assert!(c.set_name("").is_err());
        assert!(c.set_name("A".repeat(256)).is_ok());
        assert!(c.set_name("A".repeat(257)).is_err());
    }

    #[test]
    fn new_contact_name_empty_fails() {
        let result = Contact::new(
            EntityId::new(),
            WorkspaceId::new(),
            ContactType::Individual,
            "",
            ContactCategory::Other,
        );
        assert!(matches!(result, Err(ContactError::InvalidName(0))));
    }

    #[test]
    fn new_contact_name_too_long_fails() {
        let result = Contact::new(
            EntityId::new(),
            WorkspaceId::new(),
            ContactType::Organization,
            "X".repeat(257),
            ContactCategory::LawFirm,
        );
        assert!(matches!(result, Err(ContactError::InvalidName(257))));
    }

    #[test]
    fn setters_update_fields() {
        let mut c = make_contact();
        c.set_email(Some("jane@example.com".into()));
        c.set_phone(Some("+1-555-0100".into()));
        c.set_mailing_address(Some("123 Main St, Wilmington, DE 19801".into()));
        c.set_cap_table_access(CapTableAccess::Detailed);
        c.set_notes(Some("Founding CEO".into()));

        assert_eq!(c.email.as_deref(), Some("jane@example.com"));
        assert_eq!(c.phone.as_deref(), Some("+1-555-0100"));
        assert_eq!(c.cap_table_access, CapTableAccess::Detailed);
        assert_eq!(c.notes.as_deref(), Some("Founding CEO"));
    }

    #[test]
    fn category_serializes_as_snake_case() {
        let json = serde_json::to_string(&ContactCategory::BoardMember).unwrap();
        assert_eq!(json, r#""board_member""#);
        let json = serde_json::to_string(&ContactCategory::ValuationFirm).unwrap();
        assert_eq!(json, r#""valuation_firm""#);
        let json = serde_json::to_string(&ContactCategory::AccountingFirm).unwrap();
        assert_eq!(json, r#""accounting_firm""#);
    }

    #[test]
    fn cap_table_access_serialization() {
        let json = serde_json::to_string(&CapTableAccess::None).unwrap();
        assert_eq!(json, r#""none""#);
        let json = serde_json::to_string(&CapTableAccess::Detailed).unwrap();
        assert_eq!(json, r#""detailed""#);
    }

    #[test]
    fn json_roundtrip() {
        let c = make_contact();
        let json = serde_json::to_string(&c).unwrap();
        let de: Contact = serde_json::from_str(&json).unwrap();
        assert_eq!(c.contact_id, de.contact_id);
        assert_eq!(de.category, ContactCategory::Founder);
    }

    #[test]
    fn validate_name_method() {
        let c = make_contact();
        assert!(c.validate_name().is_ok());
    }

    // ── Additional ContactCategory serde roundtrips ───────────────────────────

    #[test]
    fn all_contact_categories_serde_roundtrip() {
        for cat in [
            ContactCategory::Employee,
            ContactCategory::Contractor,
            ContactCategory::BoardMember,
            ContactCategory::LawFirm,
            ContactCategory::ValuationFirm,
            ContactCategory::AccountingFirm,
            ContactCategory::Investor,
            ContactCategory::Officer,
            ContactCategory::Founder,
            ContactCategory::Member,
            ContactCategory::Other,
        ] {
            let s = serde_json::to_string(&cat).unwrap();
            let de: ContactCategory = serde_json::from_str(&s).unwrap();
            assert_eq!(de, cat, "roundtrip failed for {:?}", cat);
        }
    }

    #[test]
    fn all_contact_category_serde_values() {
        assert_eq!(
            serde_json::to_string(&ContactCategory::Employee).unwrap(),
            r#""employee""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::Contractor).unwrap(),
            r#""contractor""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::BoardMember).unwrap(),
            r#""board_member""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::LawFirm).unwrap(),
            r#""law_firm""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::ValuationFirm).unwrap(),
            r#""valuation_firm""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::AccountingFirm).unwrap(),
            r#""accounting_firm""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::Investor).unwrap(),
            r#""investor""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::Officer).unwrap(),
            r#""officer""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::Founder).unwrap(),
            r#""founder""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::Member).unwrap(),
            r#""member""#
        );
        assert_eq!(
            serde_json::to_string(&ContactCategory::Other).unwrap(),
            r#""other""#
        );
    }

    #[test]
    fn all_cap_table_access_serde_roundtrip() {
        for variant in [
            CapTableAccess::None,
            CapTableAccess::Summary,
            CapTableAccess::Detailed,
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: CapTableAccess = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
        assert_eq!(
            serde_json::to_string(&CapTableAccess::Summary).unwrap(),
            r#""summary""#
        );
    }

    #[test]
    fn contact_type_serde_roundtrip() {
        for variant in [ContactType::Individual, ContactType::Organization] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: ContactType = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
        assert_eq!(
            serde_json::to_string(&ContactType::Individual).unwrap(),
            r#""individual""#
        );
        assert_eq!(
            serde_json::to_string(&ContactType::Organization).unwrap(),
            r#""organization""#
        );
    }

    #[test]
    fn contact_status_serde_roundtrip() {
        for variant in [ContactStatus::Active, ContactStatus::Inactive] {
            let s = serde_json::to_string(&variant).unwrap();
            let de: ContactStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, variant);
        }
    }

    #[test]
    fn new_contact_with_organization_type() {
        let c = Contact::new(
            EntityId::new(),
            WorkspaceId::new(),
            ContactType::Organization,
            "Acme Legal LLP",
            ContactCategory::LawFirm,
        )
        .unwrap();
        assert_eq!(c.contact_type, ContactType::Organization);
        assert_eq!(c.category, ContactCategory::LawFirm);
    }

    #[test]
    fn new_contact_exact_256_char_name_ok() {
        let name = "A".repeat(256);
        let result = Contact::new(
            EntityId::new(),
            WorkspaceId::new(),
            ContactType::Individual,
            name,
            ContactCategory::Employee,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn new_contact_257_char_name_fails() {
        let name = "B".repeat(257);
        let result = Contact::new(
            EntityId::new(),
            WorkspaceId::new(),
            ContactType::Individual,
            name,
            ContactCategory::Employee,
        );
        assert!(matches!(result, Err(ContactError::InvalidName(257))));
    }

    #[test]
    fn set_name_to_valid_updates_field() {
        let mut c = make_contact();
        c.set_name("Updated Name").unwrap();
        assert_eq!(c.name, "Updated Name");
    }

    #[test]
    fn set_name_empty_fails() {
        let mut c = make_contact();
        assert!(matches!(c.set_name(""), Err(ContactError::InvalidName(0))));
    }

    #[test]
    fn set_category_updates_field() {
        let mut c = make_contact();
        c.set_category(ContactCategory::Investor);
        assert_eq!(c.category, ContactCategory::Investor);
    }

    #[test]
    fn set_contact_type_updates_field() {
        let mut c = make_contact();
        c.set_contact_type(ContactType::Organization);
        assert_eq!(c.contact_type, ContactType::Organization);
    }

    #[test]
    fn set_cap_table_access_updates_field() {
        let mut c = make_contact();
        c.set_cap_table_access(CapTableAccess::Summary);
        assert_eq!(c.cap_table_access, CapTableAccess::Summary);
    }

    #[test]
    fn set_notes_updates_field() {
        let mut c = make_contact();
        c.set_notes(Some("VIP investor".into()));
        assert_eq!(c.notes.as_deref(), Some("VIP investor"));
        c.set_notes(None);
        assert!(c.notes.is_none());
    }

    #[test]
    fn set_email_updates_field() {
        let mut c = make_contact();
        c.set_email(Some("test@example.com".into()));
        assert_eq!(c.email.as_deref(), Some("test@example.com"));
        c.set_email(None);
        assert!(c.email.is_none());
    }

    #[test]
    fn set_phone_updates_field() {
        let mut c = make_contact();
        c.set_phone(Some("+1-800-555-0100".into()));
        assert_eq!(c.phone.as_deref(), Some("+1-800-555-0100"));
    }

    #[test]
    fn set_mailing_address_updates_field() {
        let mut c = make_contact();
        c.set_mailing_address(Some("1 Corp Way, DE".into()));
        assert_eq!(c.mailing_address.as_deref(), Some("1 Corp Way, DE"));
    }

    #[test]
    fn reactivate_from_active_is_idempotent() {
        let mut c = make_contact();
        // Already active — reactivating should just keep it active
        c.reactivate();
        assert!(c.is_active());
    }

    #[test]
    fn contact_ids_are_unique() {
        let a = make_contact();
        let b = make_contact();
        assert_ne!(a.contact_id, b.contact_id);
    }
}
