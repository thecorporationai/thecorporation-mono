//! Receipt record (stored as `execution/receipts/{receipt_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::types::ReceiptStatus;
use crate::domain::ids::{IntentId, ReceiptId};

/// An execution receipt proving that an action was carried out.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    receipt_id: ReceiptId,
    intent_id: IntentId,
    idempotency_key: String,
    status: ReceiptStatus,
    request_hash: String,
    response_hash: Option<String>,
    executed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl Receipt {
    pub fn new(
        receipt_id: ReceiptId,
        intent_id: IntentId,
        idempotency_key: String,
        request_data: &[u8],
    ) -> Self {
        let request_hash = {
            let mut hasher = Sha256::new();
            hasher.update(request_data);
            format!("{:x}", hasher.finalize())
        };
        Self {
            receipt_id,
            intent_id,
            idempotency_key,
            status: ReceiptStatus::Pending,
            request_hash,
            response_hash: None,
            executed_at: None,
            created_at: Utc::now(),
        }
    }

    /// Mark receipt as executed with response hash.
    pub fn mark_executed(&mut self, response_data: &[u8]) {
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(response_data);
            format!("{:x}", hasher.finalize())
        };
        self.response_hash = Some(hash);
        self.status = ReceiptStatus::Executed;
        self.executed_at = Some(Utc::now());
    }

    /// Mark receipt as failed.
    pub fn mark_failed(&mut self) {
        self.status = ReceiptStatus::Failed;
    }

    // ── Accessors ─────────────────────────────────────────────────────

    pub fn receipt_id(&self) -> ReceiptId {
        self.receipt_id
    }
    pub fn intent_id(&self) -> IntentId {
        self.intent_id
    }
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }
    pub fn status(&self) -> ReceiptStatus {
        self.status
    }
    pub fn request_hash(&self) -> &str {
        &self.request_hash
    }
    pub fn response_hash(&self) -> Option<&str> {
        self.response_hash.as_deref()
    }
    pub fn executed_at(&self) -> Option<DateTime<Utc>> {
        self.executed_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_receipt() -> Receipt {
        Receipt::new(
            ReceiptId::new(),
            IntentId::new(),
            "idem-key-001".to_owned(),
            b"request body bytes",
        )
    }

    #[test]
    fn request_hash_computed_on_creation() {
        let receipt = make_receipt();
        assert!(!receipt.request_hash().is_empty());
        assert_eq!(receipt.request_hash().len(), 64); // SHA-256 hex = 64 chars
        assert!(receipt.response_hash().is_none());
    }

    #[test]
    fn mark_executed_sets_response_hash_and_status() {
        let mut receipt = make_receipt();
        receipt.mark_executed(b"response body bytes");

        assert_eq!(receipt.status(), ReceiptStatus::Executed);
        assert!(receipt.response_hash().is_some());
        assert_eq!(receipt.response_hash().unwrap().len(), 64);
        assert!(receipt.executed_at().is_some());
    }

    #[test]
    fn mark_failed_sets_status() {
        let mut receipt = make_receipt();
        receipt.mark_failed();

        assert_eq!(receipt.status(), ReceiptStatus::Failed);
        assert!(receipt.executed_at().is_none());
        assert!(receipt.response_hash().is_none());
    }

    #[test]
    fn serde_roundtrip() {
        let mut receipt = make_receipt();
        receipt.mark_executed(b"some response");

        let json = serde_json::to_string_pretty(&receipt).expect("serialize");
        let parsed: Receipt = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.receipt_id(), receipt.receipt_id());
        assert_eq!(parsed.intent_id(), receipt.intent_id());
        assert_eq!(parsed.idempotency_key(), "idem-key-001");
        assert_eq!(parsed.status(), ReceiptStatus::Executed);
        assert_eq!(parsed.request_hash(), receipt.request_hash());
        assert_eq!(parsed.response_hash(), receipt.response_hash());
    }

    #[test]
    fn deterministic_hash() {
        let r1 = Receipt::new(
            ReceiptId::new(),
            IntentId::new(),
            "key".to_owned(),
            b"same data",
        );
        let r2 = Receipt::new(
            ReceiptId::new(),
            IntentId::new(),
            "key".to_owned(),
            b"same data",
        );
        assert_eq!(r1.request_hash(), r2.request_hash());
    }
}
