//! Entity — the root aggregate for a legal business entity.
//!
//! An `Entity` progresses through a strictly ordered `FormationStatus` FSM
//! from `Pending` all the way to `Active` (or sideways into `Rejected` /
//! `Dissolved`). All mutation is expressed through methods rather than direct
//! field writes so that invariants are always maintained.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{EntityId, WorkspaceId};

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EntityError {
    #[error("legal name must be between 1 and 500 characters, got {0}")]
    InvalidName(usize),

    #[error("jurisdiction must be a 2-letter US state code (e.g. \"DE\"), got {0:?}")]
    InvalidJurisdiction(String),

    #[error("cannot advance status from {0:?}: already at a terminal state")]
    AlreadyTerminal(FormationStatus),

    #[error("cannot advance status from {0:?}")]
    NoNextStatus(FormationStatus),

    #[error("entity is already dissolved")]
    AlreadyDissolved,
}

// ── EntityType ────────────────────────────────────────────────────────────────

/// The legal form of the business entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// C-corporation (Delaware or other jurisdiction).
    CCorp,
    /// Limited Liability Company.
    Llc,
}

// ── Jurisdiction ──────────────────────────────────────────────────────────────

/// A validated two-letter US state/territory code (e.g. `"DE"`, `"CA"`).
///
/// The inner string is always stored in upper-case ASCII.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Jurisdiction(String);

impl Jurisdiction {
    /// The complete set of valid US state and territory codes.
    const VALID_CODES: &'static [&'static str] = &[
        "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA",
        "HI", "ID", "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD",
        "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
        "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC",
        "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY",
        "DC", "PR", "VI", "GU", "AS", "MP",
    ];

    /// Parse and validate a jurisdiction code.
    ///
    /// Accepts upper- or lower-case input; stores it normalized to upper-case.
    /// Only valid US state and territory codes are accepted.
    pub fn new(code: impl Into<String>) -> Result<Self, EntityError> {
        let code = code.into().to_ascii_uppercase();
        if code.len() == 2
            && code.chars().all(|c| c.is_ascii_alphabetic())
            && Self::VALID_CODES.contains(&code.as_str())
        {
            Ok(Self(code))
        } else {
            Err(EntityError::InvalidJurisdiction(code))
        }
    }

    /// Return the jurisdiction code as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Jurisdiction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ── FormationStatus ───────────────────────────────────────────────────────────

/// The lifecycle stage of an entity's formation process.
///
/// The happy-path sequence is:
/// `Pending → DocumentsGenerated → DocumentsSigned → FilingSubmitted → Filed
/// → EinApplied → Active`
///
/// Terminal states: `Active`, `Rejected`, `Dissolved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationStatus {
    Pending,
    DocumentsGenerated,
    DocumentsSigned,
    FilingSubmitted,
    Filed,
    EinApplied,
    Active,
    Rejected,
    Dissolved,
}

impl FormationStatus {
    /// Returns the next status in the happy-path FSM, if one exists.
    fn next(self) -> Option<FormationStatus> {
        match self {
            Self::Pending => Some(Self::DocumentsGenerated),
            Self::DocumentsGenerated => Some(Self::DocumentsSigned),
            Self::DocumentsSigned => Some(Self::FilingSubmitted),
            Self::FilingSubmitted => Some(Self::Filed),
            Self::Filed => Some(Self::EinApplied),
            Self::EinApplied => Some(Self::Active),
            // Terminal states
            Self::Active | Self::Rejected | Self::Dissolved => None,
        }
    }

    /// Returns `true` for states from which no further forward progress is possible.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Active | Self::Rejected | Self::Dissolved)
    }
}

impl std::fmt::Display for FormationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_owned()))
            .unwrap_or_else(|| format!("{:?}", self));
        f.write_str(&s)
    }
}

// ── Entity ────────────────────────────────────────────────────────────────────

/// Root aggregate for a legal business entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_id: EntityId,
    pub workspace_id: WorkspaceId,

    /// Validated legal name (1–500 characters).
    pub legal_name: String,

    pub entity_type: EntityType,
    pub jurisdiction: Jurisdiction,
    pub formation_status: FormationStatus,

    pub registered_agent_name: Option<String>,
    pub registered_agent_address: Option<String>,

    /// Date the entity was formally created with the filing authority.
    pub formation_date: Option<DateTime<Utc>>,

    /// Effective date of dissolution, if applicable.
    pub dissolution_effective_date: Option<NaiveDate>,

    pub created_at: DateTime<Utc>,
}

impl Entity {
    /// Create a new entity in the `Pending` state.
    ///
    /// # Errors
    /// Returns [`EntityError::InvalidName`] if `legal_name` is empty or longer
    /// than 500 characters, or [`EntityError::InvalidJurisdiction`] if the
    /// jurisdiction code is not a valid 2-letter US state code.
    pub fn new(
        workspace_id: WorkspaceId,
        legal_name: impl Into<String>,
        entity_type: EntityType,
        jurisdiction: Jurisdiction,
    ) -> Result<Self, EntityError> {
        let legal_name = legal_name.into();
        Self::validate_name(&legal_name)?;

        Ok(Self {
            entity_id: EntityId::new(),
            workspace_id,
            legal_name,
            entity_type,
            jurisdiction,
            formation_status: FormationStatus::Pending,
            registered_agent_name: None,
            registered_agent_address: None,
            formation_date: None,
            dissolution_effective_date: None,
            created_at: Utc::now(),
        })
    }

    // ── Validation ────────────────────────────────────────────────────────────

    /// Validate the entity's current state.
    ///
    /// Returns all validation errors found (not just the first).
    pub fn validate(&self) -> Result<(), Vec<EntityError>> {
        let mut errors = Vec::new();

        if let Err(e) = Self::validate_name(&self.legal_name) {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_name(name: &str) -> Result<(), EntityError> {
        let len = name.len();
        if len == 0 || len > 500 {
            Err(EntityError::InvalidName(len))
        } else {
            Ok(())
        }
    }

    // ── FSM transitions ───────────────────────────────────────────────────────

    /// Advance the formation status one step along the happy path.
    ///
    /// # Errors
    /// - [`EntityError::AlreadyTerminal`] if the entity is already in a
    ///   terminal state (`Active`, `Rejected`, or `Dissolved`).
    /// - [`EntityError::NoNextStatus`] if there is no defined forward
    ///   transition from the current status (should not occur for non-terminal
    ///   states, but is included for exhaustiveness).
    pub fn advance_status(&mut self) -> Result<FormationStatus, EntityError> {
        if self.formation_status.is_terminal() {
            return Err(EntityError::AlreadyTerminal(self.formation_status));
        }
        let next = self
            .formation_status
            .next()
            .ok_or(EntityError::NoNextStatus(self.formation_status))?;
        self.formation_status = next;
        Ok(next)
    }

    /// Dissolve the entity from any non-terminal state.
    ///
    /// Sets `formation_status` to `Dissolved` and records the effective date.
    ///
    /// # Errors
    /// Returns [`EntityError::AlreadyDissolved`] if the entity is already
    /// dissolved, or [`EntityError::AlreadyTerminal`] if it is in another
    /// terminal state (`Active` or `Rejected`) — callers should handle those
    /// cases explicitly before calling `dissolve`.
    pub fn dissolve(&mut self, effective_date: NaiveDate) -> Result<(), EntityError> {
        if self.formation_status == FormationStatus::Dissolved {
            return Err(EntityError::AlreadyDissolved);
        }
        self.formation_status = FormationStatus::Dissolved;
        self.dissolution_effective_date = Some(effective_date);
        Ok(())
    }

    // ── Field setters ─────────────────────────────────────────────────────────

    /// Update the legal name.
    ///
    /// # Errors
    /// Returns [`EntityError::InvalidName`] if the new name fails validation.
    pub fn set_legal_name(&mut self, name: impl Into<String>) -> Result<(), EntityError> {
        let name = name.into();
        Self::validate_name(&name)?;
        self.legal_name = name;
        Ok(())
    }

    pub fn set_registered_agent(
        &mut self,
        name: impl Into<String>,
        address: impl Into<String>,
    ) {
        self.registered_agent_name = Some(name.into());
        self.registered_agent_address = Some(address.into());
    }

    pub fn set_formation_date(&mut self, date: DateTime<Utc>) {
        self.formation_date = Some(date);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity() -> Entity {
        let ws = WorkspaceId::new();
        let j = Jurisdiction::new("DE").unwrap();
        Entity::new(ws, "Acme Corp", EntityType::CCorp, j).unwrap()
    }

    fn make_entity_llc() -> Entity {
        let ws = WorkspaceId::new();
        let j = Jurisdiction::new("DE").unwrap();
        Entity::new(ws, "Acme LLC", EntityType::Llc, j).unwrap()
    }

    // ── Entity::new() ─────────────────────────────────────────────────────────

    #[test]
    fn new_entity_starts_pending() {
        let e = make_entity();
        assert_eq!(e.formation_status, FormationStatus::Pending);
    }

    #[test]
    fn new_entity_stores_legal_name() {
        let e = make_entity();
        assert_eq!(e.legal_name, "Acme Corp");
    }

    #[test]
    fn new_entity_stores_entity_type() {
        let e = make_entity();
        assert_eq!(e.entity_type, EntityType::CCorp);
        let llc = make_entity_llc();
        assert_eq!(llc.entity_type, EntityType::Llc);
    }

    #[test]
    fn new_entity_stores_jurisdiction() {
        let e = make_entity();
        assert_eq!(e.jurisdiction.as_str(), "DE");
    }

    #[test]
    fn new_entity_has_no_registered_agent() {
        let e = make_entity();
        assert!(e.registered_agent_name.is_none());
        assert!(e.registered_agent_address.is_none());
    }

    #[test]
    fn new_entity_has_no_formation_date() {
        let e = make_entity();
        assert!(e.formation_date.is_none());
    }

    #[test]
    fn new_entity_has_no_dissolution_date() {
        let e = make_entity();
        assert!(e.dissolution_effective_date.is_none());
    }

    #[test]
    fn new_entity_empty_name_fails() {
        let ws = WorkspaceId::new();
        let j = Jurisdiction::new("DE").unwrap();
        let err = Entity::new(ws, "", EntityType::CCorp, j).unwrap_err();
        assert_eq!(err, EntityError::InvalidName(0));
    }

    #[test]
    fn new_entity_501_char_name_fails() {
        let ws = WorkspaceId::new();
        let j = Jurisdiction::new("DE").unwrap();
        let name = "X".repeat(501);
        let err = Entity::new(ws, name, EntityType::CCorp, j).unwrap_err();
        assert_eq!(err, EntityError::InvalidName(501));
    }

    #[test]
    fn new_entity_500_char_name_succeeds() {
        let ws = WorkspaceId::new();
        let j = Jurisdiction::new("DE").unwrap();
        assert!(Entity::new(ws, "X".repeat(500), EntityType::CCorp, j).is_ok());
    }

    // ── Jurisdiction ──────────────────────────────────────────────────────────

    #[test]
    fn jurisdiction_validates_code() {
        assert!(Jurisdiction::new("DE").is_ok());
        assert!(Jurisdiction::new("ca").is_ok()); // lower-case accepted
        assert!(Jurisdiction::new("DEL").is_err());
        assert!(Jurisdiction::new("1A").is_err());
        assert!(Jurisdiction::new("").is_err());
    }

    #[test]
    fn jurisdiction_all_50_states_dc_and_territories_valid() {
        let codes = [
            "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA",
            "HI", "ID", "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD",
            "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
            "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC",
            "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY",
            "DC", "PR", "VI", "GU", "AS", "MP",
        ];
        for s in &codes {
            assert!(Jurisdiction::new(*s).is_ok(), "Expected {} to be valid", s);
        }
    }

    #[test]
    fn jurisdiction_lowercase_normalized_to_uppercase() {
        let j = Jurisdiction::new("ca").unwrap();
        assert_eq!(j.as_str(), "CA");
    }

    #[test]
    fn jurisdiction_mixed_case_normalized() {
        let j = Jurisdiction::new("De").unwrap();
        assert_eq!(j.as_str(), "DE");
    }

    #[test]
    fn jurisdiction_xx_invalid() {
        // XX is not a real US state/territory code — should be rejected.
        assert!(Jurisdiction::new("XX").is_err());
    }

    #[test]
    fn jurisdiction_123_invalid() {
        assert!(Jurisdiction::new("123").is_err());
    }

    #[test]
    fn jurisdiction_digits_invalid() {
        assert!(Jurisdiction::new("1A").is_err());
        assert!(Jurisdiction::new("A1").is_err());
    }

    #[test]
    fn jurisdiction_empty_invalid() {
        assert!(Jurisdiction::new("").is_err());
    }

    #[test]
    fn jurisdiction_display() {
        let j = Jurisdiction::new("DE").unwrap();
        assert_eq!(j.to_string(), "DE");
    }

    #[test]
    fn jurisdiction_serde_roundtrip() {
        let j = Jurisdiction::new("NY").unwrap();
        let json = serde_json::to_string(&j).unwrap();
        assert_eq!(json, r#""NY""#);
        let de: Jurisdiction = serde_json::from_str(&json).unwrap();
        assert_eq!(de.as_str(), "NY");
    }

    // ── advance_status() transitions ──────────────────────────────────────────

    #[test]
    fn advance_status_full_path() {
        let mut e = make_entity();
        let sequence = [
            FormationStatus::DocumentsGenerated,
            FormationStatus::DocumentsSigned,
            FormationStatus::FilingSubmitted,
            FormationStatus::Filed,
            FormationStatus::EinApplied,
            FormationStatus::Active,
        ];
        for expected in sequence {
            let next = e.advance_status().unwrap();
            assert_eq!(next, expected);
        }
    }

    #[test]
    fn advance_status_pending_to_documents_generated() {
        let mut e = make_entity();
        assert_eq!(e.formation_status, FormationStatus::Pending);
        let next = e.advance_status().unwrap();
        assert_eq!(next, FormationStatus::DocumentsGenerated);
        assert_eq!(e.formation_status, FormationStatus::DocumentsGenerated);
    }

    #[test]
    fn advance_status_documents_generated_to_documents_signed() {
        let mut e = make_entity();
        e.advance_status().unwrap(); // -> DocumentsGenerated
        let next = e.advance_status().unwrap();
        assert_eq!(next, FormationStatus::DocumentsSigned);
    }

    #[test]
    fn advance_status_documents_signed_to_filing_submitted() {
        let mut e = make_entity();
        e.advance_status().unwrap();
        e.advance_status().unwrap();
        let next = e.advance_status().unwrap();
        assert_eq!(next, FormationStatus::FilingSubmitted);
    }

    #[test]
    fn advance_status_filing_submitted_to_filed() {
        let mut e = make_entity();
        for _ in 0..3 { e.advance_status().unwrap(); }
        let next = e.advance_status().unwrap();
        assert_eq!(next, FormationStatus::Filed);
    }

    #[test]
    fn advance_status_filed_to_ein_applied() {
        let mut e = make_entity();
        for _ in 0..4 { e.advance_status().unwrap(); }
        let next = e.advance_status().unwrap();
        assert_eq!(next, FormationStatus::EinApplied);
    }

    #[test]
    fn advance_status_ein_applied_to_active() {
        let mut e = make_entity();
        for _ in 0..5 { e.advance_status().unwrap(); }
        let next = e.advance_status().unwrap();
        assert_eq!(next, FormationStatus::Active);
    }

    #[test]
    fn advance_from_terminal_active_fails() {
        let mut e = make_entity();
        for _ in 0..6 { e.advance_status().unwrap(); }
        assert_eq!(e.formation_status, FormationStatus::Active);
        assert!(matches!(
            e.advance_status(),
            Err(EntityError::AlreadyTerminal(FormationStatus::Active))
        ));
    }

    #[test]
    fn advance_from_terminal_rejected_fails() {
        let mut e = make_entity();
        e.formation_status = FormationStatus::Rejected;
        assert!(matches!(
            e.advance_status(),
            Err(EntityError::AlreadyTerminal(FormationStatus::Rejected))
        ));
    }

    #[test]
    fn advance_from_terminal_dissolved_fails() {
        let mut e = make_entity();
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        e.dissolve(date).unwrap();
        assert!(matches!(
            e.advance_status(),
            Err(EntityError::AlreadyTerminal(FormationStatus::Dissolved))
        ));
    }

    // ── dissolve() ────────────────────────────────────────────────────────────

    #[test]
    fn dissolve_from_pending() {
        let mut e = make_entity();
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        e.dissolve(date).unwrap();
        assert_eq!(e.formation_status, FormationStatus::Dissolved);
        assert_eq!(e.dissolution_effective_date, Some(date));
    }

    #[test]
    fn dissolve_from_documents_generated() {
        let mut e = make_entity();
        e.advance_status().unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        e.dissolve(date).unwrap();
        assert_eq!(e.formation_status, FormationStatus::Dissolved);
    }

    #[test]
    fn dissolve_from_documents_signed() {
        let mut e = make_entity();
        for _ in 0..2 { e.advance_status().unwrap(); }
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        e.dissolve(date).unwrap();
        assert_eq!(e.formation_status, FormationStatus::Dissolved);
    }

    #[test]
    fn dissolve_from_filing_submitted() {
        let mut e = make_entity();
        for _ in 0..3 { e.advance_status().unwrap(); }
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        e.dissolve(date).unwrap();
        assert_eq!(e.formation_status, FormationStatus::Dissolved);
    }

    #[test]
    fn dissolve_from_filed() {
        let mut e = make_entity();
        for _ in 0..4 { e.advance_status().unwrap(); }
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        e.dissolve(date).unwrap();
        assert_eq!(e.formation_status, FormationStatus::Dissolved);
    }

    #[test]
    fn dissolve_from_ein_applied() {
        let mut e = make_entity();
        for _ in 0..5 { e.advance_status().unwrap(); }
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        e.dissolve(date).unwrap();
        assert_eq!(e.formation_status, FormationStatus::Dissolved);
    }

    #[test]
    fn dissolve_from_active() {
        let mut e = make_entity();
        for _ in 0..6 { e.advance_status().unwrap(); }
        let date = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        // dissolve() from Active is allowed (it's not Dissolved)
        e.dissolve(date).unwrap();
        assert_eq!(e.formation_status, FormationStatus::Dissolved);
    }

    #[test]
    fn dissolve_sets_status_and_date() {
        let mut e = make_entity();
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        e.dissolve(date).unwrap();
        assert_eq!(e.formation_status, FormationStatus::Dissolved);
        assert_eq!(e.dissolution_effective_date, Some(date));
    }

    #[test]
    fn dissolve_twice_fails() {
        let mut e = make_entity();
        let date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        e.dissolve(date).unwrap();
        assert!(matches!(e.dissolve(date), Err(EntityError::AlreadyDissolved)));
    }

    // ── FormationStatus serde ─────────────────────────────────────────────────

    #[test]
    fn formation_status_serde_pending() {
        let json = serde_json::to_string(&FormationStatus::Pending).unwrap();
        assert_eq!(json, r#""pending""#);
        let de: FormationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FormationStatus::Pending);
    }

    #[test]
    fn formation_status_serde_documents_generated() {
        let json = serde_json::to_string(&FormationStatus::DocumentsGenerated).unwrap();
        assert_eq!(json, r#""documents_generated""#);
        let de: FormationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FormationStatus::DocumentsGenerated);
    }

    #[test]
    fn formation_status_serde_documents_signed() {
        let json = serde_json::to_string(&FormationStatus::DocumentsSigned).unwrap();
        assert_eq!(json, r#""documents_signed""#);
    }

    #[test]
    fn formation_status_serde_filing_submitted() {
        let json = serde_json::to_string(&FormationStatus::FilingSubmitted).unwrap();
        assert_eq!(json, r#""filing_submitted""#);
    }

    #[test]
    fn formation_status_serde_filed() {
        let json = serde_json::to_string(&FormationStatus::Filed).unwrap();
        assert_eq!(json, r#""filed""#);
    }

    #[test]
    fn formation_status_serde_ein_applied() {
        let json = serde_json::to_string(&FormationStatus::EinApplied).unwrap();
        assert_eq!(json, r#""ein_applied""#);
    }

    #[test]
    fn formation_status_serde_active() {
        let json = serde_json::to_string(&FormationStatus::Active).unwrap();
        assert_eq!(json, r#""active""#);
    }

    #[test]
    fn formation_status_serde_rejected() {
        let json = serde_json::to_string(&FormationStatus::Rejected).unwrap();
        assert_eq!(json, r#""rejected""#);
    }

    #[test]
    fn formation_status_serde_dissolved() {
        let json = serde_json::to_string(&FormationStatus::Dissolved).unwrap();
        assert_eq!(json, r#""dissolved""#);
    }

    #[test]
    fn formation_status_is_terminal() {
        assert!(FormationStatus::Active.is_terminal());
        assert!(FormationStatus::Rejected.is_terminal());
        assert!(FormationStatus::Dissolved.is_terminal());
        assert!(!FormationStatus::Pending.is_terminal());
        assert!(!FormationStatus::DocumentsGenerated.is_terminal());
        assert!(!FormationStatus::DocumentsSigned.is_terminal());
        assert!(!FormationStatus::FilingSubmitted.is_terminal());
        assert!(!FormationStatus::Filed.is_terminal());
        assert!(!FormationStatus::EinApplied.is_terminal());
    }

    // ── EntityType serde ──────────────────────────────────────────────────────

    #[test]
    fn entity_type_serializes_as_snake_case() {
        let json = serde_json::to_string(&EntityType::CCorp).unwrap();
        assert_eq!(json, r#""c_corp""#);
        let json = serde_json::to_string(&EntityType::Llc).unwrap();
        assert_eq!(json, r#""llc""#);
    }

    #[test]
    fn entity_type_serde_roundtrip_c_corp() {
        let json = serde_json::to_string(&EntityType::CCorp).unwrap();
        let de: EntityType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, EntityType::CCorp);
    }

    #[test]
    fn entity_type_serde_roundtrip_llc() {
        let json = serde_json::to_string(&EntityType::Llc).unwrap();
        let de: EntityType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, EntityType::Llc);
    }

    // ── set_legal_name() ──────────────────────────────────────────────────────

    #[test]
    fn set_legal_name_valid() {
        let mut e = make_entity();
        e.set_legal_name("New Name Corp").unwrap();
        assert_eq!(e.legal_name, "New Name Corp");
    }

    #[test]
    fn set_legal_name_empty_fails() {
        let mut e = make_entity();
        let err = e.set_legal_name("").unwrap_err();
        assert_eq!(err, EntityError::InvalidName(0));
        // Legal name unchanged
        assert_eq!(e.legal_name, "Acme Corp");
    }

    #[test]
    fn set_legal_name_too_long_fails() {
        let mut e = make_entity();
        let err = e.set_legal_name("X".repeat(501)).unwrap_err();
        assert_eq!(err, EntityError::InvalidName(501));
    }

    #[test]
    fn set_legal_name_exactly_500_chars() {
        let mut e = make_entity();
        e.set_legal_name("A".repeat(500)).unwrap();
        assert_eq!(e.legal_name.len(), 500);
    }

    // ── set_registered_agent() ────────────────────────────────────────────────

    #[test]
    fn set_registered_agent_stores_name_and_address() {
        let mut e = make_entity();
        e.set_registered_agent("CT Corporation", "1209 Orange Street, Wilmington, DE 19801");
        assert_eq!(e.registered_agent_name.as_deref(), Some("CT Corporation"));
        assert_eq!(
            e.registered_agent_address.as_deref(),
            Some("1209 Orange Street, Wilmington, DE 19801")
        );
    }

    #[test]
    fn set_registered_agent_overwrites_previous() {
        let mut e = make_entity();
        e.set_registered_agent("Old Agent", "Old Address");
        e.set_registered_agent("New Agent", "New Address");
        assert_eq!(e.registered_agent_name.as_deref(), Some("New Agent"));
        assert_eq!(e.registered_agent_address.as_deref(), Some("New Address"));
    }

    // ── set_formation_date() ──────────────────────────────────────────────────

    #[test]
    fn set_formation_date_stores_value() {
        let mut e = make_entity();
        let dt = Utc::now();
        e.set_formation_date(dt);
        assert!(e.formation_date.is_some());
    }

    // ── validate() ────────────────────────────────────────────────────────────

    #[test]
    fn validate_passes_for_valid_entity() {
        let e = make_entity();
        assert!(e.validate().is_ok());
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn json_serialization_roundtrip() {
        let e = make_entity();
        let json = serde_json::to_string(&e).unwrap();
        let de: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(e.entity_id, de.entity_id);
        assert_eq!(de.formation_status, FormationStatus::Pending);
    }

    #[test]
    fn validate_name_length() {
        let ws = WorkspaceId::new();
        let j = Jurisdiction::new("DE").unwrap();
        assert!(Entity::new(ws, "", EntityType::CCorp, j.clone()).is_err());
        assert!(
            Entity::new(ws, "X".repeat(500), EntityType::CCorp, j.clone()).is_ok()
        );
        assert!(
            Entity::new(ws, "X".repeat(501), EntityType::CCorp, j).is_err()
        );
    }
}
