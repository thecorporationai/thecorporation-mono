//! Formation filings — records of documents submitted to a government authority.
//!
//! A `Filing` tracks the submission of a certificate (incorporation,
//! organization, etc.) to a state filing authority, from `Pending` through
//! to the `Filed` confirmation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{EntityId, FilingId, WorkspaceId};

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FilingError {
    #[error("filing is already in a terminal state ({0:?})")]
    AlreadyFiled(FilingStatus),
}

// ── FilingType ────────────────────────────────────────────────────────────────

/// The type of certificate being filed with the state authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilingType {
    /// Certificate of Formation — filed for LLCs.
    CertificateOfFormation,
    /// Certificate of Incorporation — filed for corporations.
    CertificateOfIncorporation,
}

// ── FilingStatus ──────────────────────────────────────────────────────────────

/// Lifecycle status of a filing submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilingStatus {
    /// Submission has been prepared but not yet transmitted.
    Pending,
    /// State authority has confirmed receipt and acceptance.
    Filed,
}

// ── Filing ────────────────────────────────────────────────────────────────────

/// A record of a formation document submitted to a state filing authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filing {
    pub filing_id: FilingId,
    pub entity_id: EntityId,
    pub workspace_id: WorkspaceId,

    pub filing_type: FilingType,

    /// Two-letter US state code for the jurisdiction of filing.
    pub jurisdiction: String,

    pub status: FilingStatus,

    /// Name of the person or agent submitting the filing (attestation).
    pub attested_by: Option<String>,

    /// Freeform attestation statement included in the filing.
    pub attestation_statement: Option<String>,

    /// Timestamp when the filing was submitted to the authority.
    pub submitted_at: Option<DateTime<Utc>>,

    /// Timestamp when the state confirmed the filing.
    pub filed_at: Option<DateTime<Utc>>,

    /// State-issued filing confirmation number, if available.
    pub confirmation_number: Option<String>,

    pub created_at: DateTime<Utc>,
}

impl Filing {
    /// Create a new filing in `Pending` status.
    pub fn new(
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        filing_type: FilingType,
        jurisdiction: impl Into<String>,
    ) -> Self {
        Self {
            filing_id: FilingId::new(),
            entity_id,
            workspace_id,
            filing_type,
            jurisdiction: jurisdiction.into(),
            status: FilingStatus::Pending,
            attested_by: None,
            attestation_statement: None,
            submitted_at: None,
            filed_at: None,
            confirmation_number: None,
            created_at: Utc::now(),
        }
    }

    // ── Submission ────────────────────────────────────────────────────────────

    /// Record the submission of this filing to the authority.
    ///
    /// Sets `submitted_at` and the optional attestation fields.
    pub fn record_submission(
        &mut self,
        attested_by: Option<String>,
        attestation_statement: Option<String>,
    ) {
        self.attested_by = attested_by;
        self.attestation_statement = attestation_statement;
        self.submitted_at = Some(Utc::now());
    }

    // ── Confirmation ──────────────────────────────────────────────────────────

    /// Mark the filing as confirmed by the state authority.
    ///
    /// Transitions status to `Filed` and records the confirmation number and
    /// timestamp.
    ///
    /// # Errors
    /// Returns [`FilingError::AlreadyFiled`] if the filing is already in
    /// `Filed` status.
    pub fn confirm(
        &mut self,
        confirmation_number: impl Into<String>,
        filed_at: DateTime<Utc>,
    ) -> Result<(), FilingError> {
        if self.status == FilingStatus::Filed {
            return Err(FilingError::AlreadyFiled(self.status));
        }
        self.status = FilingStatus::Filed;
        self.confirmation_number = Some(confirmation_number.into());
        self.filed_at = Some(filed_at);
        Ok(())
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn is_filed(&self) -> bool {
        self.status == FilingStatus::Filed
    }

    pub fn is_pending(&self) -> bool {
        self.status == FilingStatus::Pending
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_filing() -> Filing {
        Filing::new(
            EntityId::new(),
            WorkspaceId::new(),
            FilingType::CertificateOfIncorporation,
            "DE",
        )
    }

    fn make_formation_filing() -> Filing {
        Filing::new(
            EntityId::new(),
            WorkspaceId::new(),
            FilingType::CertificateOfFormation,
            "DE",
        )
    }

    // ── Filing::new() ─────────────────────────────────────────────────────────

    #[test]
    fn new_filing_is_pending() {
        let f = make_filing();
        assert_eq!(f.status, FilingStatus::Pending);
        assert!(f.is_pending());
        assert!(!f.is_filed());
        assert!(f.confirmation_number.is_none());
    }

    #[test]
    fn new_filing_has_no_attested_by() {
        let f = make_filing();
        assert!(f.attested_by.is_none());
    }

    #[test]
    fn new_filing_has_no_attestation_statement() {
        let f = make_filing();
        assert!(f.attestation_statement.is_none());
    }

    #[test]
    fn new_filing_has_no_submitted_at() {
        let f = make_filing();
        assert!(f.submitted_at.is_none());
    }

    #[test]
    fn new_filing_has_no_filed_at() {
        let f = make_filing();
        assert!(f.filed_at.is_none());
    }

    #[test]
    fn new_filing_certificate_of_incorporation() {
        let f = make_filing();
        assert_eq!(f.filing_type, FilingType::CertificateOfIncorporation);
    }

    #[test]
    fn new_filing_certificate_of_formation() {
        let f = make_formation_filing();
        assert_eq!(f.filing_type, FilingType::CertificateOfFormation);
    }

    #[test]
    fn new_filing_stores_jurisdiction() {
        let f = make_filing();
        assert_eq!(f.jurisdiction, "DE");
    }

    // ── confirm() ────────────────────────────────────────────────────────────

    #[test]
    fn confirm_transitions_to_filed() {
        let mut f = make_filing();
        let ts = Utc::now();
        f.confirm("DE-2026-123456", ts).unwrap();
        assert_eq!(f.status, FilingStatus::Filed);
        assert_eq!(f.confirmation_number.as_deref(), Some("DE-2026-123456"));
        assert!(f.is_filed());
    }

    #[test]
    fn confirm_records_filed_at_timestamp() {
        let mut f = make_filing();
        let ts = Utc::now();
        f.confirm("CONF-001", ts).unwrap();
        assert_eq!(f.filed_at, Some(ts));
    }

    #[test]
    fn confirm_sets_is_filed_true() {
        let mut f = make_filing();
        f.confirm("CONF-001", Utc::now()).unwrap();
        assert!(f.is_filed());
        assert!(!f.is_pending());
    }

    #[test]
    fn confirm_twice_fails() {
        let mut f = make_filing();
        f.confirm("NUM-1", Utc::now()).unwrap();
        assert!(matches!(
            f.confirm("NUM-2", Utc::now()),
            Err(FilingError::AlreadyFiled(_))
        ));
    }

    #[test]
    fn confirm_already_filed_error_contains_status() {
        let mut f = make_filing();
        f.confirm("NUM-1", Utc::now()).unwrap();
        match f.confirm("NUM-2", Utc::now()) {
            Err(FilingError::AlreadyFiled(status)) => {
                assert_eq!(status, FilingStatus::Filed);
            }
            _ => panic!("expected AlreadyFiled"),
        }
    }

    // ── record_submission() ───────────────────────────────────────────────────

    #[test]
    fn record_submission_sets_fields() {
        let mut f = make_filing();
        f.record_submission(
            Some("Jane Founder".into()),
            Some("I attest the above is true.".into()),
        );
        assert_eq!(f.attested_by.as_deref(), Some("Jane Founder"));
        assert!(f.submitted_at.is_some());
    }

    #[test]
    fn record_submission_sets_attestation_statement() {
        let mut f = make_filing();
        f.record_submission(None, Some("All information is accurate.".into()));
        assert_eq!(
            f.attestation_statement.as_deref(),
            Some("All information is accurate.")
        );
    }

    #[test]
    fn record_submission_with_no_attested_by() {
        let mut f = make_filing();
        f.record_submission(None, None);
        assert!(f.attested_by.is_none());
        assert!(f.submitted_at.is_some());
    }

    #[test]
    fn record_submission_then_confirm_full_lifecycle() {
        let mut f = make_filing();
        f.record_submission(Some("Jane Founder".into()), Some("I attest.".into()));
        f.confirm("DE-2026-999", Utc::now()).unwrap();
        assert!(f.is_filed());
        assert!(f.submitted_at.is_some());
        assert!(f.filed_at.is_some());
        assert_eq!(f.attested_by.as_deref(), Some("Jane Founder"));
    }

    // ── FilingType serde ──────────────────────────────────────────────────────

    #[test]
    fn filing_type_serialization_certificate_of_formation() {
        let json = serde_json::to_string(&FilingType::CertificateOfFormation).unwrap();
        assert_eq!(json, r#""certificate_of_formation""#);
    }

    #[test]
    fn filing_type_serialization_certificate_of_incorporation() {
        let json = serde_json::to_string(&FilingType::CertificateOfIncorporation).unwrap();
        assert_eq!(json, r#""certificate_of_incorporation""#);
    }

    #[test]
    fn filing_type_serde_roundtrip_formation() {
        let json = serde_json::to_string(&FilingType::CertificateOfFormation).unwrap();
        let de: FilingType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FilingType::CertificateOfFormation);
    }

    #[test]
    fn filing_type_serde_roundtrip_incorporation() {
        let json = serde_json::to_string(&FilingType::CertificateOfIncorporation).unwrap();
        let de: FilingType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FilingType::CertificateOfIncorporation);
    }

    // ── FilingStatus serde ────────────────────────────────────────────────────

    #[test]
    fn filing_status_serde_pending() {
        let json = serde_json::to_string(&FilingStatus::Pending).unwrap();
        assert_eq!(json, r#""pending""#);
        let de: FilingStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FilingStatus::Pending);
    }

    #[test]
    fn filing_status_serde_filed() {
        let json = serde_json::to_string(&FilingStatus::Filed).unwrap();
        assert_eq!(json, r#""filed""#);
        let de: FilingStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, FilingStatus::Filed);
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn json_roundtrip() {
        let f = make_filing();
        let json = serde_json::to_string(&f).unwrap();
        let de: Filing = serde_json::from_str(&json).unwrap();
        assert_eq!(f.filing_id, de.filing_id);
        assert_eq!(de.filing_type, FilingType::CertificateOfIncorporation);
    }

    #[test]
    fn json_roundtrip_after_confirm() {
        let mut f = make_filing();
        f.confirm("DE-99", Utc::now()).unwrap();
        let json = serde_json::to_string(&f).unwrap();
        let de: Filing = serde_json::from_str(&json).unwrap();
        assert_eq!(de.status, FilingStatus::Filed);
        assert_eq!(de.confirmation_number.as_deref(), Some("DE-99"));
    }

    #[test]
    fn filing_type_serialization() {
        let json = serde_json::to_string(&FilingType::CertificateOfFormation).unwrap();
        assert_eq!(json, r#""certificate_of_formation""#);
    }
}
