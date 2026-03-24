//! Tax profile — IRS registration details for a legal entity.
//!
//! Tracks EIN (Employer Identification Number) assignment and the entity's
//! IRS tax classification, which drives how federal income taxes are reported.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{EntityId, TaxProfileId, WorkspaceId};

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TaxProfileError {
    #[error("EIN must be exactly 9 digits (XX-XXXXXXX), got {0:?}")]
    InvalidEin(String),

    #[error("EIN is already active; it cannot be overwritten")]
    EinAlreadyActive,
}

// ── EinStatus ─────────────────────────────────────────────────────────────────

/// Application/assignment status of the Employer Identification Number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EinStatus {
    /// SS-4 has been submitted; EIN not yet assigned.
    Pending,
    /// IRS has assigned an EIN.
    Active,
}

// ── IrsTaxClassification ──────────────────────────────────────────────────────

/// How the entity is classified for U.S. federal income tax purposes.
///
/// This determines the correct federal income tax return form to file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IrsTaxClassification {
    /// Single-member LLC treated as a sole proprietorship (Schedule C).
    DisregardedEntity,
    /// Multi-member LLC or other entity taxed as a partnership (Form 1065).
    Partnership,
    /// Subchapter C corporation (Form 1120).
    CCorporation,
}

// ── TaxProfile ────────────────────────────────────────────────────────────────

/// IRS registration details associated with a legal entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxProfile {
    pub tax_profile_id: TaxProfileId,
    pub entity_id: EntityId,
    pub workspace_id: WorkspaceId,

    /// Employer Identification Number in `XX-XXXXXXX` format, once assigned.
    pub ein: Option<String>,

    pub ein_status: EinStatus,

    /// IRS federal income tax classification for this entity.
    pub classification: IrsTaxClassification,

    /// Timestamp when the SS-4 application was submitted.
    pub application_submitted_at: Option<DateTime<Utc>>,

    /// Timestamp when the IRS assigned the EIN.
    pub ein_assigned_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
}

impl TaxProfile {
    /// Create a new `TaxProfile` in `EinStatus::Pending` with no EIN yet.
    pub fn new(
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        classification: IrsTaxClassification,
    ) -> Self {
        Self {
            tax_profile_id: TaxProfileId::new(),
            entity_id,
            workspace_id,
            ein: None,
            ein_status: EinStatus::Pending,
            classification,
            application_submitted_at: None,
            ein_assigned_at: None,
            created_at: Utc::now(),
        }
    }

    // ── EIN management ────────────────────────────────────────────────────────

    /// Record that the SS-4 application has been submitted to the IRS.
    pub fn record_application_submitted(&mut self) {
        self.application_submitted_at = Some(Utc::now());
    }

    /// Assign the EIN once the IRS confirms it.
    ///
    /// Accepts either the raw 9-digit string (`"123456789"`) or the formatted
    /// version with a hyphen (`"12-3456789"`).  The value is stored normalized
    /// to the hyphen-formatted form.
    ///
    /// # Errors
    /// - [`TaxProfileError::InvalidEin`] if the value does not contain exactly
    ///   9 digits.
    /// - [`TaxProfileError::EinAlreadyActive`] if an EIN has already been
    ///   assigned and activated.
    pub fn assign_ein(&mut self, ein: impl Into<String>) -> Result<(), TaxProfileError> {
        if self.ein_status == EinStatus::Active {
            return Err(TaxProfileError::EinAlreadyActive);
        }

        let ein = ein.into();
        let normalized = Self::normalize_ein(&ein)?;
        self.ein = Some(normalized);
        self.ein_status = EinStatus::Active;
        self.ein_assigned_at = Some(Utc::now());
        Ok(())
    }

    /// Update the tax classification.
    pub fn set_classification(&mut self, classification: IrsTaxClassification) {
        self.classification = classification;
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Normalize an EIN string to the canonical `XX-XXXXXXX` format.
    ///
    /// Accepts `"123456789"` or `"12-3456789"`.
    fn normalize_ein(ein: &str) -> Result<String, TaxProfileError> {
        // Strip any existing hyphens so we're working with raw digits.
        let digits: String = ein.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() != 9 {
            return Err(TaxProfileError::InvalidEin(ein.to_owned()));
        }

        Ok(format!("{}-{}", &digits[..2], &digits[2..]))
    }

    /// Returns `true` if an EIN has been assigned and activated.
    pub fn has_active_ein(&self) -> bool {
        self.ein_status == EinStatus::Active && self.ein.is_some()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile() -> TaxProfile {
        TaxProfile::new(
            EntityId::new(),
            WorkspaceId::new(),
            IrsTaxClassification::CCorporation,
        )
    }

    // ── TaxProfile::new() ─────────────────────────────────────────────────────

    #[test]
    fn new_profile_is_pending() {
        let p = make_profile();
        assert_eq!(p.ein_status, EinStatus::Pending);
        assert!(p.ein.is_none());
        assert!(!p.has_active_ein());
    }

    #[test]
    fn new_profile_has_no_application_submitted_at() {
        let p = make_profile();
        assert!(p.application_submitted_at.is_none());
    }

    #[test]
    fn new_profile_has_no_ein_assigned_at() {
        let p = make_profile();
        assert!(p.ein_assigned_at.is_none());
    }

    #[test]
    fn new_profile_c_corporation_classification() {
        let p = TaxProfile::new(
            EntityId::new(),
            WorkspaceId::new(),
            IrsTaxClassification::CCorporation,
        );
        assert_eq!(p.classification, IrsTaxClassification::CCorporation);
    }

    #[test]
    fn new_profile_disregarded_entity_classification() {
        let p = TaxProfile::new(
            EntityId::new(),
            WorkspaceId::new(),
            IrsTaxClassification::DisregardedEntity,
        );
        assert_eq!(p.classification, IrsTaxClassification::DisregardedEntity);
    }

    #[test]
    fn new_profile_partnership_classification() {
        let p = TaxProfile::new(
            EntityId::new(),
            WorkspaceId::new(),
            IrsTaxClassification::Partnership,
        );
        assert_eq!(p.classification, IrsTaxClassification::Partnership);
    }

    // ── assign_ein() ──────────────────────────────────────────────────────────

    #[test]
    fn assign_ein_with_raw_digits() {
        let mut p = make_profile();
        p.assign_ein("123456789").unwrap();
        assert_eq!(p.ein.as_deref(), Some("12-3456789"));
        assert_eq!(p.ein_status, EinStatus::Active);
        assert!(p.has_active_ein());
    }

    #[test]
    fn assign_ein_with_hyphen_format() {
        let mut p = make_profile();
        p.assign_ein("12-3456789").unwrap();
        assert_eq!(p.ein.as_deref(), Some("12-3456789"));
    }

    #[test]
    fn assign_ein_records_assigned_at() {
        let mut p = make_profile();
        p.assign_ein("123456789").unwrap();
        assert!(p.ein_assigned_at.is_some());
    }

    #[test]
    fn assign_ein_sets_status_active() {
        let mut p = make_profile();
        p.assign_ein("123456789").unwrap();
        assert_eq!(p.ein_status, EinStatus::Active);
    }

    #[test]
    fn assign_ein_normalizes_hyphen_format() {
        let mut p = make_profile();
        // Input with hyphen is normalized to XX-XXXXXXX
        p.assign_ein("98-7654321").unwrap();
        assert_eq!(p.ein.as_deref(), Some("98-7654321"));
    }

    #[test]
    fn assign_ein_invalid_too_short_fails() {
        let mut p = make_profile();
        let err = p.assign_ein("12345").unwrap_err();
        assert!(matches!(err, TaxProfileError::InvalidEin(_)));
    }

    #[test]
    fn assign_ein_invalid_too_long_fails() {
        let mut p = make_profile();
        let err = p.assign_ein("1234567890").unwrap_err();
        assert!(matches!(err, TaxProfileError::InvalidEin(_)));
    }

    #[test]
    fn assign_ein_empty_fails() {
        let mut p = make_profile();
        let err = p.assign_ein("").unwrap_err();
        assert!(matches!(err, TaxProfileError::InvalidEin(_)));
    }

    #[test]
    fn assign_ein_letters_fail() {
        let mut p = make_profile();
        // Non-digit characters (other than hyphen) cause digit count to be wrong
        let err = p.assign_ein("abcdefghi").unwrap_err();
        assert!(matches!(err, TaxProfileError::InvalidEin(_)));
    }

    #[test]
    fn assign_ein_twice_fails() {
        let mut p = make_profile();
        p.assign_ein("123456789").unwrap();
        assert!(matches!(
            p.assign_ein("987654321"),
            Err(TaxProfileError::EinAlreadyActive)
        ));
    }

    #[test]
    fn assign_ein_when_already_active_does_not_change_ein() {
        let mut p = make_profile();
        p.assign_ein("123456789").unwrap();
        let _ = p.assign_ein("987654321");
        assert_eq!(p.ein.as_deref(), Some("12-3456789"));
    }

    // ── record_application_submitted() ────────────────────────────────────────

    #[test]
    fn record_application_submitted_sets_timestamp() {
        let mut p = make_profile();
        p.record_application_submitted();
        assert!(p.application_submitted_at.is_some());
    }

    // ── set_classification() ──────────────────────────────────────────────────

    #[test]
    fn set_classification_updates_field() {
        let mut p = make_profile();
        p.set_classification(IrsTaxClassification::Partnership);
        assert_eq!(p.classification, IrsTaxClassification::Partnership);
    }

    // ── IrsTaxClassification serde ────────────────────────────────────────────

    #[test]
    fn classification_serializes_as_snake_case() {
        let json =
            serde_json::to_string(&IrsTaxClassification::DisregardedEntity).unwrap();
        assert_eq!(json, r#""disregarded_entity""#);
    }

    #[test]
    fn classification_serde_c_corporation() {
        let json = serde_json::to_string(&IrsTaxClassification::CCorporation).unwrap();
        assert_eq!(json, r#""c_corporation""#);
        let de: IrsTaxClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(de, IrsTaxClassification::CCorporation);
    }

    #[test]
    fn classification_serde_partnership() {
        let json = serde_json::to_string(&IrsTaxClassification::Partnership).unwrap();
        assert_eq!(json, r#""partnership""#);
        let de: IrsTaxClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(de, IrsTaxClassification::Partnership);
    }

    #[test]
    fn classification_serde_disregarded_entity() {
        let json = serde_json::to_string(&IrsTaxClassification::DisregardedEntity).unwrap();
        let de: IrsTaxClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(de, IrsTaxClassification::DisregardedEntity);
    }

    // ── EinStatus serde ───────────────────────────────────────────────────────

    #[test]
    fn ein_status_serializes_as_snake_case() {
        let json = serde_json::to_string(&EinStatus::Active).unwrap();
        assert_eq!(json, r#""active""#);
    }

    #[test]
    fn ein_status_serde_pending() {
        let json = serde_json::to_string(&EinStatus::Pending).unwrap();
        assert_eq!(json, r#""pending""#);
        let de: EinStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, EinStatus::Pending);
    }

    #[test]
    fn ein_status_serde_active() {
        let json = serde_json::to_string(&EinStatus::Active).unwrap();
        let de: EinStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, EinStatus::Active);
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn json_roundtrip() {
        let p = make_profile();
        let json = serde_json::to_string(&p).unwrap();
        let de: TaxProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(p.tax_profile_id, de.tax_profile_id);
    }

    #[test]
    fn json_roundtrip_after_ein_assignment() {
        let mut p = make_profile();
        p.assign_ein("12-3456789").unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let de: TaxProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(de.ein.as_deref(), Some("12-3456789"));
        assert_eq!(de.ein_status, EinStatus::Active);
    }
}
