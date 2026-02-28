//! Formation filing record (stored as `formation/filing.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{FilingStatus, FilingType, Jurisdiction};
use crate::domain::ids::{EntityId, FilingId};

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
    created_at: DateTime<Utc>,
}

impl Filing {
    /// Create a new pending filing record.
    pub fn new(
        filing_id: FilingId,
        entity_id: EntityId,
        filing_type: FilingType,
        jurisdiction: Jurisdiction,
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
    fn serde_roundtrip() {
        let mut f = make_filing();
        f.confirm("EXT-789".into(), None);
        let json = serde_json::to_string(&f).unwrap();
        let parsed: Filing = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.filing_id(), f.filing_id());
        assert_eq!(parsed.status(), FilingStatus::Filed);
    }
}
