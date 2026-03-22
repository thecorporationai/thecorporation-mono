//! Tax filing record (stored as `tax/filings/{filing_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ContactId, DocumentId, EntityId, TaxFilingId};

/// Status of a tax filing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaxFilingStatus {
    Pending,
    Filed,
    Accepted,
    Rejected,
}

/// A tax filing record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxFiling {
    filing_id: TaxFilingId,
    entity_id: EntityId,
    document_type: String,
    tax_year: i32,
    document_id: DocumentId,
    status: TaxFilingStatus,
    created_at: DateTime<Utc>,
    /// Optional contact scoping — for per-person filings like 83(b) elections.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    filer_contact_id: Option<ContactId>,
}

impl TaxFiling {
    pub fn new(
        filing_id: TaxFilingId,
        entity_id: EntityId,
        document_type: String,
        tax_year: i32,
        document_id: DocumentId,
    ) -> Self {
        Self {
            filing_id,
            entity_id,
            document_type,
            tax_year,
            document_id,
            status: TaxFilingStatus::Pending,
            created_at: Utc::now(),
            filer_contact_id: None,
        }
    }

    pub fn with_filer_contact(mut self, contact_id: Option<ContactId>) -> Self {
        self.filer_contact_id = contact_id;
        self
    }

    pub fn mark_filed(&mut self) {
        self.status = TaxFilingStatus::Filed;
    }

    // Accessors
    pub fn filing_id(&self) -> TaxFilingId {
        self.filing_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn document_type(&self) -> &str {
        &self.document_type
    }
    pub fn tax_year(&self) -> i32 {
        self.tax_year
    }
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }
    pub fn status(&self) -> TaxFilingStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn filer_contact_id(&self) -> Option<ContactId> {
        self.filer_contact_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_filing_is_pending() {
        let f = TaxFiling::new(
            TaxFilingId::new(),
            EntityId::new(),
            "1120".to_owned(),
            2025,
            DocumentId::new(),
        );
        assert_eq!(f.status(), TaxFilingStatus::Pending);
        assert_eq!(f.tax_year(), 2025);
    }

    #[test]
    fn serde_roundtrip() {
        let f = TaxFiling::new(
            TaxFilingId::new(),
            EntityId::new(),
            "1065".to_owned(),
            2024,
            DocumentId::new(),
        );
        let json = serde_json::to_string(&f).unwrap();
        let parsed: TaxFiling = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.filing_id(), f.filing_id());
        assert_eq!(parsed.document_type(), "1065");
    }
}
