//! Formation filing record (stored as `formation/filing.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::FormationError;
use super::types::{FilingStatus, FilingType, Jurisdiction};
use crate::domain::ids::{EntityId, FilingId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilingAttestation {
    signer_name: String,
    signer_role: String,
    signer_email: String,
    consent_text: String,
    notes: Option<String>,
    attested_at: DateTime<Utc>,
}

impl FilingAttestation {
    pub fn signer_name(&self) -> &str {
        &self.signer_name
    }
    pub fn signer_role(&self) -> &str {
        &self.signer_role
    }
    pub fn signer_email(&self) -> &str {
        &self.signer_email
    }
    pub fn consent_text(&self) -> &str {
        &self.consent_text
    }
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
    pub fn attested_at(&self) -> DateTime<Utc> {
        self.attested_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredAgentConsentEvidence {
    evidence_uri: String,
    evidence_type: String,
    notes: Option<String>,
    recorded_at: DateTime<Utc>,
}

impl RegisteredAgentConsentEvidence {
    pub fn evidence_uri(&self) -> &str {
        &self.evidence_uri
    }
    pub fn evidence_type(&self) -> &str {
        &self.evidence_type
    }
    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }
}

/// A formation filing submitted to a state government.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filing {
    filing_id: FilingId,
    entity_id: EntityId,
    filing_type: FilingType,
    jurisdiction: Jurisdiction,
    status: FilingStatus,
    external_filing_id: Option<String>,
    receipt_reference: Option<String>,
    filed_at: Option<DateTime<Utc>>,
    #[serde(default = "default_true")]
    requires_natural_person_attestation: bool,
    designated_attestor_name: String,
    designated_attestor_email: Option<String>,
    designated_attestor_role: String,
    #[serde(default)]
    attestation: Option<FilingAttestation>,
    #[serde(default = "default_true")]
    requires_registered_agent_consent_evidence: bool,
    #[serde(default)]
    registered_agent_consent_evidence: Vec<RegisteredAgentConsentEvidence>,
    created_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

impl Filing {
    /// Create a new pending filing record.
    pub fn new(
        filing_id: FilingId,
        entity_id: EntityId,
        filing_type: FilingType,
        jurisdiction: Jurisdiction,
        designated_attestor_name: String,
        designated_attestor_email: Option<String>,
        designated_attestor_role: String,
    ) -> Self {
        Self {
            filing_id,
            entity_id,
            filing_type,
            jurisdiction,
            status: FilingStatus::Pending,
            external_filing_id: None,
            receipt_reference: None,
            filed_at: None,
            requires_natural_person_attestation: true,
            designated_attestor_name,
            designated_attestor_email,
            designated_attestor_role,
            attestation: None,
            requires_registered_agent_consent_evidence: true,
            registered_agent_consent_evidence: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Confirm that the filing has been accepted by the state.
    pub fn confirm(&mut self, external_id: String, receipt: Option<String>) {
        self.status = FilingStatus::Filed;
        self.external_filing_id = Some(external_id);
        self.receipt_reference = receipt;
        self.filed_at = Some(Utc::now());
    }

    pub fn record_attestation(
        &mut self,
        signer_name: String,
        signer_role: String,
        signer_email: String,
        consent_text: String,
        notes: Option<String>,
    ) -> Result<(), FormationError> {
        if signer_name.trim().is_empty() {
            return Err(FormationError::Validation(
                "signer_name is required for filing attestation".to_owned(),
            ));
        }
        if signer_role.trim().is_empty() {
            return Err(FormationError::Validation(
                "signer_role is required for filing attestation".to_owned(),
            ));
        }
        if signer_email.trim().is_empty() {
            return Err(FormationError::Validation(
                "signer_email is required for filing attestation".to_owned(),
            ));
        }
        if consent_text.trim().is_empty() {
            return Err(FormationError::Validation(
                "consent_text is required for filing attestation".to_owned(),
            ));
        }

        let signer_name_norm = signer_name.trim().to_lowercase();
        let designated_name_norm = self.designated_attestor_name.trim().to_lowercase();
        if signer_name_norm != designated_name_norm {
            return Err(FormationError::Validation(format!(
                "filing attestation signer must match designated attestor {}",
                self.designated_attestor_name
            )));
        }

        if let Some(designated_email) = self.designated_attestor_email.as_ref()
            && !designated_email.eq_ignore_ascii_case(signer_email.trim())
        {
            return Err(FormationError::Validation(format!(
                "filing attestation signer_email must match designated attestor email {}",
                designated_email
            )));
        }

        if !self
            .designated_attestor_role
            .eq_ignore_ascii_case(signer_role.trim())
        {
            return Err(FormationError::Validation(format!(
                "filing attestation signer_role must match designated role {}",
                self.designated_attestor_role
            )));
        }

        self.attestation = Some(FilingAttestation {
            signer_name: signer_name.trim().to_owned(),
            signer_role: signer_role.trim().to_owned(),
            signer_email: signer_email.trim().to_owned(),
            consent_text: consent_text.trim().to_owned(),
            notes,
            attested_at: Utc::now(),
        });
        Ok(())
    }

    pub fn add_registered_agent_evidence(
        &mut self,
        evidence_uri: String,
        evidence_type: Option<String>,
        notes: Option<String>,
    ) -> Result<(), FormationError> {
        let uri = evidence_uri.trim();
        if uri.is_empty() {
            return Err(FormationError::Validation(
                "evidence_uri is required for registered agent consent evidence".to_owned(),
            ));
        }
        if self
            .registered_agent_consent_evidence
            .iter()
            .any(|e| e.evidence_uri.eq_ignore_ascii_case(uri))
        {
            return Ok(());
        }
        self.registered_agent_consent_evidence
            .push(RegisteredAgentConsentEvidence {
                evidence_uri: uri.to_owned(),
                evidence_type: evidence_type
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "registered_agent_consent".to_owned()),
                notes,
                recorded_at: Utc::now(),
            });
        Ok(())
    }

    pub fn submission_blockers(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.requires_natural_person_attestation && self.attestation.is_none() {
            blockers.push("missing natural-person filing attestation".to_owned());
        }
        if self.requires_registered_agent_consent_evidence
            && self.registered_agent_consent_evidence.is_empty()
        {
            blockers.push("missing registered agent consent evidence".to_owned());
        }
        blockers
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn filing_id(&self) -> FilingId {
        self.filing_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn filing_type(&self) -> FilingType {
        self.filing_type
    }

    pub fn jurisdiction(&self) -> &str {
        &self.jurisdiction
    }

    pub fn status(&self) -> FilingStatus {
        self.status
    }

    pub fn external_filing_id(&self) -> Option<&str> {
        self.external_filing_id.as_deref()
    }

    pub fn receipt_reference(&self) -> Option<&str> {
        self.receipt_reference.as_deref()
    }

    pub fn filed_at(&self) -> Option<DateTime<Utc>> {
        self.filed_at
    }

    pub fn requires_natural_person_attestation(&self) -> bool {
        self.requires_natural_person_attestation
    }

    pub fn designated_attestor_name(&self) -> &str {
        &self.designated_attestor_name
    }

    pub fn designated_attestor_email(&self) -> Option<&str> {
        self.designated_attestor_email.as_deref()
    }

    pub fn designated_attestor_role(&self) -> &str {
        &self.designated_attestor_role
    }

    pub fn attestation(&self) -> Option<&FilingAttestation> {
        self.attestation.as_ref()
    }

    pub fn requires_registered_agent_consent_evidence(&self) -> bool {
        self.requires_registered_agent_consent_evidence
    }

    pub fn registered_agent_consent_evidence(&self) -> &[RegisteredAgentConsentEvidence] {
        &self.registered_agent_consent_evidence
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_filing() -> Filing {
        Filing::new(
            FilingId::new(),
            EntityId::new(),
            FilingType::CertificateOfIncorporation,
            Jurisdiction::new("US-DE").unwrap(),
            "Alice Founder".to_owned(),
            Some("alice@example.com".to_owned()),
            "director".to_owned(),
        )
    }

    #[test]
    fn new_filing_is_pending() {
        let f = make_filing();
        assert_eq!(f.status(), FilingStatus::Pending);
        assert!(f.external_filing_id().is_none());
        assert!(f.filed_at().is_none());
    }

    #[test]
    fn confirm_sets_filed() {
        let mut f = make_filing();
        f.confirm("EXT-123".into(), Some("REC-456".into()));
        assert_eq!(f.status(), FilingStatus::Filed);
        assert_eq!(f.external_filing_id(), Some("EXT-123"));
        assert_eq!(f.receipt_reference(), Some("REC-456"));
        assert!(f.filed_at().is_some());
    }

    #[test]
    fn submission_blockers_require_attestation_and_ra_evidence() {
        let f = make_filing();
        let blockers = f.submission_blockers();
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("natural-person filing attestation"))
        );
        assert!(
            blockers
                .iter()
                .any(|b| b.contains("registered agent consent evidence"))
        );
    }

    #[test]
    fn record_attestation_and_evidence_clears_blockers() {
        let mut f = make_filing();
        f.record_attestation(
            "Alice Founder".to_owned(),
            "director".to_owned(),
            "alice@example.com".to_owned(),
            "I attest this filing submission is accurate.".to_owned(),
            None,
        )
        .unwrap();
        f.add_registered_agent_evidence(
            "s3://evidence/ra-consent.pdf".to_owned(),
            None,
            Some("RA engagement letter".to_owned()),
        )
        .unwrap();
        assert!(f.submission_blockers().is_empty());
        assert!(f.attestation().is_some());
        assert_eq!(f.registered_agent_consent_evidence().len(), 1);
    }

    #[test]
    fn attestation_rejects_non_designated_signer() {
        let mut f = make_filing();
        let err = f
            .record_attestation(
                "Bob Founder".to_owned(),
                "director".to_owned(),
                "alice@example.com".to_owned(),
                "I attest".to_owned(),
                None,
            )
            .unwrap_err();
        assert!(err.to_string().contains("designated attestor"));
    }

    #[test]
    fn serde_roundtrip() {
        let mut f = make_filing();
        f.confirm("EXT-789".into(), None);
        let json = serde_json::to_string(&f).unwrap();
        let parsed: Filing = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.filing_id(), f.filing_id());
        assert_eq!(parsed.status(), FilingStatus::Filed);
    }
}
