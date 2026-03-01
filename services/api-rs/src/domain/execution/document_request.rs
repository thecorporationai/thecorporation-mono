//! Document request record (stored as `execution/document-requests/{request_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::ExecutionError;
use super::types::DocumentRequestStatus;
use crate::domain::ids::{DocumentRequestId, EntityId, ObligationId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRequest {
    request_id: DocumentRequestId,
    entity_id: EntityId,
    obligation_id: ObligationId,
    description: String,
    document_type: String,
    status: DocumentRequestStatus,
    fulfilled_at: Option<DateTime<Utc>>,
    not_applicable_at: Option<DateTime<Utc>>,
    waived_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl DocumentRequest {
    pub fn new(
        request_id: DocumentRequestId,
        entity_id: EntityId,
        obligation_id: ObligationId,
        description: String,
        document_type: String,
    ) -> Self {
        Self {
            request_id,
            entity_id,
            obligation_id,
            description,
            document_type,
            status: DocumentRequestStatus::Requested,
            fulfilled_at: None,
            not_applicable_at: None,
            waived_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn fulfill(&mut self) -> Result<(), ExecutionError> {
        if self.status != DocumentRequestStatus::Requested {
            return Err(ExecutionError::InvalidDocumentRequestTransition {
                from: self.status,
                to: DocumentRequestStatus::Provided,
            });
        }
        self.status = DocumentRequestStatus::Provided;
        self.fulfilled_at = Some(Utc::now());
        Ok(())
    }

    pub fn mark_not_applicable(&mut self) -> Result<(), ExecutionError> {
        if self.status != DocumentRequestStatus::Requested {
            return Err(ExecutionError::InvalidDocumentRequestTransition {
                from: self.status,
                to: DocumentRequestStatus::NotApplicable,
            });
        }
        self.status = DocumentRequestStatus::NotApplicable;
        self.not_applicable_at = Some(Utc::now());
        Ok(())
    }

    pub fn waive(&mut self) -> Result<(), ExecutionError> {
        if self.status != DocumentRequestStatus::Requested {
            return Err(ExecutionError::InvalidDocumentRequestTransition {
                from: self.status,
                to: DocumentRequestStatus::Waived,
            });
        }
        self.status = DocumentRequestStatus::Waived;
        self.waived_at = Some(Utc::now());
        Ok(())
    }

    pub fn is_satisfied(&self) -> bool {
        matches!(
            self.status,
            DocumentRequestStatus::Provided
                | DocumentRequestStatus::NotApplicable
                | DocumentRequestStatus::Waived
        )
    }

    pub fn request_id(&self) -> DocumentRequestId {
        self.request_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn obligation_id(&self) -> ObligationId {
        self.obligation_id
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn document_type(&self) -> &str {
        &self.document_type
    }
    pub fn status(&self) -> DocumentRequestStatus {
        self.status
    }
    pub fn fulfilled_at(&self) -> Option<DateTime<Utc>> {
        self.fulfilled_at
    }
    pub fn not_applicable_at(&self) -> Option<DateTime<Utc>> {
        self.not_applicable_at
    }
    pub fn waived_at(&self) -> Option<DateTime<Utc>> {
        self.waived_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request() -> DocumentRequest {
        DocumentRequest::new(
            DocumentRequestId::new(),
            EntityId::new(),
            ObligationId::new(),
            "Provide certificate of incorporation".to_owned(),
            "certificate_of_incorporation".to_owned(),
        )
    }

    #[test]
    fn fulfill_from_requested() {
        let mut req = make_request();
        req.fulfill().unwrap();
        assert_eq!(req.status(), DocumentRequestStatus::Provided);
        assert!(req.fulfilled_at().is_some());
    }

    #[test]
    fn waive_from_requested() {
        let mut req = make_request();
        req.waive().unwrap();
        assert_eq!(req.status(), DocumentRequestStatus::Waived);
        assert!(req.waived_at().is_some());
    }

    #[test]
    fn mark_not_applicable_from_requested() {
        let mut req = make_request();
        req.mark_not_applicable().unwrap();
        assert_eq!(req.status(), DocumentRequestStatus::NotApplicable);
        assert!(req.not_applicable_at().is_some());
    }

    #[test]
    fn cannot_fulfill_from_provided() {
        let mut req = make_request();
        req.fulfill().unwrap();
        assert!(req.fulfill().is_err());
    }

    #[test]
    fn cannot_waive_from_provided() {
        let mut req = make_request();
        req.fulfill().unwrap();
        assert!(req.waive().is_err());
    }

    #[test]
    fn cannot_mark_na_from_waived() {
        let mut req = make_request();
        req.waive().unwrap();
        assert!(req.mark_not_applicable().is_err());
    }

    #[test]
    fn cannot_fulfill_from_not_applicable() {
        let mut req = make_request();
        req.mark_not_applicable().unwrap();
        assert!(req.fulfill().is_err());
    }
}
