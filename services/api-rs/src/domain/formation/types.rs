//! Formation domain types — entity types, formation lifecycle, and documents.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── Enums ──────────────────────────────────────────────────────────────

/// The legal structure of a business entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// C-Corporation (or S-Corporation).
    Corporation,
    /// Limited Liability Company.
    Llc,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corporation => write!(f, "corporation"),
            Self::Llc => write!(f, "llc"),
        }
    }
}

/// High-level state of a forming entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationState {
    /// Entity is in the process of being formed.
    Forming,
    /// Entity is fully formed and operational.
    Active,
}

/// Detailed formation workflow status with valid state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationStatus {
    /// Initial state — formation request received.
    Pending,
    /// Governing documents have been generated.
    DocumentsGenerated,
    /// All required documents have been signed.
    DocumentsSigned,
    /// Filing has been submitted to the state.
    FilingSubmitted,
    /// State has accepted the filing.
    Filed,
    /// EIN application has been submitted to the IRS.
    EinApplied,
    /// Entity is fully formed and active.
    Active,
    /// Formation was rejected by the state.
    Rejected,
    /// Entity has been dissolved.
    Dissolved,
}

impl FormationStatus {
    /// Return the valid next states from this status.
    ///
    /// The formation FSM:
    /// ```text
    /// Pending -> DocumentsGenerated -> DocumentsSigned -> FilingSubmitted
    ///   -> Filed -> EinApplied -> Active
    /// FilingSubmitted -> Rejected
    /// ```
    pub fn allowed_transitions(&self) -> &[FormationStatus] {
        match self {
            Self::Pending => &[Self::DocumentsGenerated, Self::Rejected],
            Self::DocumentsGenerated => &[Self::DocumentsSigned, Self::Rejected],
            Self::DocumentsSigned => &[Self::FilingSubmitted, Self::Rejected],
            Self::FilingSubmitted => &[Self::Filed, Self::Rejected],
            Self::Filed => &[Self::EinApplied, Self::Rejected],
            Self::EinApplied => &[Self::Active, Self::Rejected],
            Self::Active => &[Self::Dissolved],
            Self::Rejected => &[],
            Self::Dissolved => &[],
        }
    }
}

impl fmt::Display for FormationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::DocumentsGenerated => write!(f, "documents_generated"),
            Self::DocumentsSigned => write!(f, "documents_signed"),
            Self::FilingSubmitted => write!(f, "filing_submitted"),
            Self::Filed => write!(f, "filed"),
            Self::EinApplied => write!(f, "ein_applied"),
            Self::Active => write!(f, "active"),
            Self::Rejected => write!(f, "rejected"),
            Self::Dissolved => write!(f, "dissolved"),
        }
    }
}

/// Type of legal document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// Articles of Incorporation (C-Corp).
    ArticlesOfIncorporation,
    /// Articles of Organization (LLC).
    ArticlesOfOrganization,
    /// Corporate bylaws (C-Corp).
    Bylaws,
    /// Operating agreement (LLC).
    OperatingAgreement,
    /// IRS Form SS-4 (EIN application).
    Ss4Application,
    /// Meeting notice.
    MeetingNotice,
    /// Board or member resolution.
    Resolution,
    /// SAFE agreement.
    SafeAgreement,
}

/// Status of a document in the signing workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// Document has been drafted but not signed.
    Draft,
    /// Document has been signed by all required parties.
    Signed,
    /// Document has been amended.
    Amended,
    /// Document has been filed with a government agency.
    Filed,
}

/// Status of an EIN (Employer Identification Number) application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EinStatus {
    /// Application has been submitted.
    Pending,
    /// EIN has been assigned and is active.
    Active,
}

/// IRS tax classification election for the entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IrsTaxClassification {
    /// Single-member LLC treated as disregarded entity.
    DisregardedEntity,
    /// Multi-member LLC or entity taxed as partnership.
    Partnership,
    /// C-Corporation tax treatment.
    CCorporation,
}

/// Type of state filing for entity formation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilingType {
    /// Certificate of Formation (LLC).
    CertificateOfFormation,
    /// Certificate of Incorporation (Corporation).
    CertificateOfIncorporation,
}

impl fmt::Display for FilingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CertificateOfFormation => write!(f, "certificate_of_formation"),
            Self::CertificateOfIncorporation => write!(f, "certificate_of_incorporation"),
        }
    }
}

/// Status of a formation filing with the state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilingStatus {
    /// Filing has been prepared but not yet submitted.
    Pending,
    /// Filing has been accepted by the state.
    Filed,
}

impl fmt::Display for FilingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Filed => write!(f, "filed"),
        }
    }
}

// ── String newtypes ────────────────────────────────────────────────────

/// Maximum length for a jurisdiction string.
const MAX_JURISDICTION_LEN: usize = 200;

/// A validated jurisdiction (e.g., "Delaware", "California").
///
/// Guarantees: non-empty, at most 200 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Jurisdiction(String);

impl Jurisdiction {
    /// Create a validated jurisdiction.
    pub fn new(s: impl Into<String>) -> Result<Self, String> {
        let s = s.into();
        if s.is_empty() {
            return Err("jurisdiction must not be empty".into());
        }
        if s.len() > MAX_JURISDICTION_LEN {
            return Err(format!(
                "jurisdiction must be at most {} characters",
                MAX_JURISDICTION_LEN
            ));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Jurisdiction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

impl std::ops::Deref for Jurisdiction {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for Jurisdiction {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl AsRef<str> for Jurisdiction {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Jurisdiction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formation_status_transitions() {
        let transitions = FormationStatus::Pending.allowed_transitions();
        assert!(transitions.contains(&FormationStatus::DocumentsGenerated));

        let transitions = FormationStatus::FilingSubmitted.allowed_transitions();
        assert!(transitions.contains(&FormationStatus::Filed));
        assert!(transitions.contains(&FormationStatus::Rejected));

        // Rejected is reachable from any non-terminal state
        let non_terminal = [
            FormationStatus::Pending,
            FormationStatus::DocumentsGenerated,
            FormationStatus::DocumentsSigned,
            FormationStatus::FilingSubmitted,
            FormationStatus::Filed,
            FormationStatus::EinApplied,
        ];
        for status in &non_terminal {
            assert!(
                status
                    .allowed_transitions()
                    .contains(&FormationStatus::Rejected),
                "{status} should allow transition to Rejected"
            );
        }

        // Active can only transition to Dissolved
        assert!(
            FormationStatus::Active
                .allowed_transitions()
                .contains(&FormationStatus::Dissolved)
        );
        // Terminal states have no transitions
        assert!(FormationStatus::Rejected.allowed_transitions().is_empty());
        assert!(FormationStatus::Dissolved.allowed_transitions().is_empty());
    }

    #[test]
    fn formation_status_serde() {
        let status = FormationStatus::DocumentsGenerated;
        let json = serde_json::to_string(&status).expect("serialize FormationStatus");
        assert_eq!(json, "\"documents_generated\"");
        let parsed: FormationStatus =
            serde_json::from_str(&json).expect("deserialize FormationStatus");
        assert_eq!(status, parsed);
    }

    #[test]
    fn entity_type_display() {
        assert_eq!(EntityType::Corporation.to_string(), "corporation");
        assert_eq!(EntityType::Llc.to_string(), "llc");
    }

    #[test]
    fn entity_type_serde() {
        let et = EntityType::Llc;
        let json = serde_json::to_string(&et).expect("serialize EntityType");
        assert_eq!(json, "\"llc\"");
        let parsed: EntityType = serde_json::from_str(&json).expect("deserialize EntityType");
        assert_eq!(et, parsed);
    }

    #[test]
    fn document_type_serde() {
        let dt = DocumentType::Ss4Application;
        let json = serde_json::to_string(&dt).expect("serialize DocumentType");
        assert_eq!(json, "\"ss4_application\"");
        let parsed: DocumentType = serde_json::from_str(&json).expect("deserialize DocumentType");
        assert_eq!(dt, parsed);
    }

    #[test]
    fn irs_classification_serde() {
        let cls = IrsTaxClassification::CCorporation;
        let json = serde_json::to_string(&cls).expect("serialize IrsTaxClassification");
        // serde rename_all snake_case: CCorporation -> "c_corporation"
        assert_eq!(json, "\"c_corporation\"");
    }

    #[test]
    fn jurisdiction_valid() {
        assert!(Jurisdiction::new("Delaware").is_ok());
        assert!(Jurisdiction::new("California").is_ok());
    }

    #[test]
    fn jurisdiction_rejects_empty() {
        assert!(Jurisdiction::new("").is_err());
    }

    #[test]
    fn jurisdiction_rejects_oversized() {
        let long = "x".repeat(MAX_JURISDICTION_LEN + 1);
        assert!(Jurisdiction::new(long).is_err());
    }

    #[test]
    fn jurisdiction_serde_roundtrip() {
        let j = Jurisdiction::new("Delaware").unwrap();
        let json = serde_json::to_string(&j).unwrap();
        let parsed: Jurisdiction = serde_json::from_str(&json).unwrap();
        assert_eq!(j, parsed);
    }

    #[test]
    fn jurisdiction_deserialize_rejects_empty() {
        let json = serde_json::json!("");
        let result: Result<Jurisdiction, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
