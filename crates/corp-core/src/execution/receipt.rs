//! Execution receipts — idempotency and audit records for intent execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::types::ReceiptStatus;
use crate::ids::{IntentId, ReceiptId};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReceiptError {
    #[error("receipt must be Pending to be marked executed; current status: {0:?}")]
    NotPending(ReceiptStatus),
    #[error("receipt must be Pending to be marked failed; current status: {0:?}")]
    AlreadySettled(ReceiptStatus),
}

// ── Receipt ───────────────────────────────────────────────────────────────────

/// An idempotency and audit record for a single intent execution attempt.
///
/// The `request_hash` (SHA-256 of the serialised request) is stored so that
/// duplicate submissions can be detected. Once the intent has been processed
/// the `response_hash` is populated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_id: ReceiptId,
    pub intent_id: IntentId,
    /// A caller-supplied idempotency key (e.g. UUID or deterministic hash).
    pub idempotency_key: String,
    pub status: ReceiptStatus,
    /// SHA-256 hex digest of the serialised request payload.
    pub request_hash: String,
    /// SHA-256 hex digest of the serialised response payload, populated on
    /// success.
    pub response_hash: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Receipt {
    /// Create a new receipt in `Pending` status.
    pub fn new(
        intent_id: IntentId,
        idempotency_key: impl Into<String>,
        request_hash: impl Into<String>,
    ) -> Self {
        Self {
            receipt_id: ReceiptId::new(),
            intent_id,
            idempotency_key: idempotency_key.into(),
            status: ReceiptStatus::Pending,
            request_hash: request_hash.into(),
            response_hash: None,
            executed_at: None,
            created_at: Utc::now(),
        }
    }

    /// Transition `Pending → Executed`, recording the response hash.
    pub fn mark_executed(&mut self, response_hash: impl Into<String>) -> Result<(), ReceiptError> {
        match self.status {
            ReceiptStatus::Pending => {
                self.status = ReceiptStatus::Executed;
                self.response_hash = Some(response_hash.into());
                self.executed_at = Some(Utc::now());
                Ok(())
            }
            s => Err(ReceiptError::NotPending(s)),
        }
    }

    /// Transition `Pending → Failed`.
    pub fn mark_failed(&mut self) -> Result<(), ReceiptError> {
        match self.status {
            ReceiptStatus::Pending => {
                self.status = ReceiptStatus::Failed;
                Ok(())
            }
            s => Err(ReceiptError::AlreadySettled(s)),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_receipt() -> Receipt {
        Receipt::new(
            IntentId::new(),
            "idempotency-key-abc-123",
            "sha256-request-hash-abc",
        )
    }

    // ── new() ─────────────────────────────────────────────────────────────────

    #[test]
    fn new_receipt_is_pending() {
        let r = make_receipt();
        assert_eq!(r.status, ReceiptStatus::Pending);
    }

    #[test]
    fn new_receipt_stores_idempotency_key() {
        let r = make_receipt();
        assert_eq!(r.idempotency_key, "idempotency-key-abc-123");
    }

    #[test]
    fn new_receipt_stores_request_hash() {
        let r = make_receipt();
        assert_eq!(r.request_hash, "sha256-request-hash-abc");
    }

    #[test]
    fn new_receipt_response_hash_is_none() {
        let r = make_receipt();
        assert!(r.response_hash.is_none());
        assert!(r.executed_at.is_none());
    }

    // ── mark_executed() ───────────────────────────────────────────────────────

    #[test]
    fn mark_executed_from_pending() {
        let mut r = make_receipt();
        assert!(r.mark_executed("sha256-response-hash").is_ok());
        assert_eq!(r.status, ReceiptStatus::Executed);
        assert_eq!(r.response_hash.as_deref(), Some("sha256-response-hash"));
        assert!(r.executed_at.is_some());
    }

    #[test]
    fn mark_executed_twice_is_error() {
        let mut r = make_receipt();
        r.mark_executed("hash1").unwrap();
        assert!(matches!(
            r.mark_executed("hash2"),
            Err(ReceiptError::NotPending(_))
        ));
    }

    #[test]
    fn mark_executed_from_failed_is_error() {
        let mut r = make_receipt();
        r.mark_failed().unwrap();
        assert!(matches!(
            r.mark_executed("hash"),
            Err(ReceiptError::NotPending(_))
        ));
    }

    // ── mark_failed() ────────────────────────────────────────────────────────

    #[test]
    fn mark_failed_from_pending() {
        let mut r = make_receipt();
        assert!(r.mark_failed().is_ok());
        assert_eq!(r.status, ReceiptStatus::Failed);
    }

    #[test]
    fn mark_failed_twice_is_error() {
        let mut r = make_receipt();
        r.mark_failed().unwrap();
        assert!(matches!(
            r.mark_failed(),
            Err(ReceiptError::AlreadySettled(_))
        ));
    }

    #[test]
    fn mark_failed_after_executed_is_error() {
        let mut r = make_receipt();
        r.mark_executed("some-hash").unwrap();
        assert!(matches!(
            r.mark_failed(),
            Err(ReceiptError::AlreadySettled(_))
        ));
    }

    // ── ReceiptStatus serde roundtrips ────────────────────────────────────────

    #[test]
    fn receipt_status_serde_roundtrip() {
        for status in [
            ReceiptStatus::Pending,
            ReceiptStatus::Executed,
            ReceiptStatus::Failed,
        ] {
            let s = serde_json::to_string(&status).unwrap();
            let de: ReceiptStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, status);
        }
    }

    #[test]
    fn receipt_status_serde_values() {
        assert_eq!(
            serde_json::to_string(&ReceiptStatus::Pending).unwrap(),
            r#""pending""#
        );
        assert_eq!(
            serde_json::to_string(&ReceiptStatus::Executed).unwrap(),
            r#""executed""#
        );
        assert_eq!(
            serde_json::to_string(&ReceiptStatus::Failed).unwrap(),
            r#""failed""#
        );
    }

    #[test]
    fn receipt_ids_are_unique() {
        let a = make_receipt();
        let b = make_receipt();
        assert_ne!(a.receipt_id, b.receipt_id);
    }

    #[test]
    fn receipt_different_idempotency_keys() {
        let r1 = Receipt::new(IntentId::new(), "key-1", "hash-1");
        let r2 = Receipt::new(IntentId::new(), "key-2", "hash-2");
        assert_ne!(r1.idempotency_key, r2.idempotency_key);
    }
}
