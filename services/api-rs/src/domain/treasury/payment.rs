//! Payment record (stored as `treasury/payments/{payment_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{Cents, PaymentMethod};
use crate::domain::ids::{EntityId, PaymentId};

/// Lifecycle status of a payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Submitted,
    Processing,
    Completed,
    Failed,
}

/// A payment submitted for processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    payment_id: PaymentId,
    entity_id: EntityId,
    amount_cents: Cents,
    recipient: String,
    payment_method: PaymentMethod,
    description: String,
    status: PaymentStatus,
    created_at: DateTime<Utc>,
}

impl Payment {
    pub fn new(
        payment_id: PaymentId,
        entity_id: EntityId,
        amount_cents: Cents,
        recipient: String,
        payment_method: PaymentMethod,
        description: String,
    ) -> Self {
        Self {
            payment_id,
            entity_id,
            amount_cents,
            recipient,
            payment_method,
            description,
            status: PaymentStatus::Submitted,
            created_at: Utc::now(),
        }
    }

    pub fn mark_processing(&mut self) {
        self.status = PaymentStatus::Processing;
    }

    pub fn mark_completed(&mut self) {
        self.status = PaymentStatus::Completed;
    }

    pub fn mark_failed(&mut self) {
        self.status = PaymentStatus::Failed;
    }

    // Accessors
    pub fn payment_id(&self) -> PaymentId {
        self.payment_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn amount_cents(&self) -> Cents {
        self.amount_cents
    }
    pub fn recipient(&self) -> &str {
        &self.recipient
    }
    pub fn payment_method(&self) -> PaymentMethod {
        self.payment_method
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn status(&self) -> PaymentStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_payment_is_submitted() {
        let p = Payment::new(
            PaymentId::new(),
            EntityId::new(),
            Cents::new(50000),
            "Vendor Corp".to_owned(),
            PaymentMethod::Ach,
            "Monthly rent".to_owned(),
        );
        assert_eq!(p.status(), PaymentStatus::Submitted);
        assert_eq!(p.amount_cents().raw(), 50000);
    }

    #[test]
    fn status_transitions() {
        let mut p = Payment::new(
            PaymentId::new(),
            EntityId::new(),
            Cents::new(10000),
            "Bob".to_owned(),
            PaymentMethod::Wire,
            "Invoice payment".to_owned(),
        );
        p.mark_processing();
        assert_eq!(p.status(), PaymentStatus::Processing);
        p.mark_completed();
        assert_eq!(p.status(), PaymentStatus::Completed);
    }

    #[test]
    fn serde_roundtrip() {
        let p = Payment::new(
            PaymentId::new(),
            EntityId::new(),
            Cents::new(75000),
            "Alice".to_owned(),
            PaymentMethod::BankTransfer,
            "Contractor pay".to_owned(),
        );
        let json = serde_json::to_string(&p).unwrap();
        let parsed: Payment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.payment_id(), p.payment_id());
        assert_eq!(parsed.payment_method(), PaymentMethod::BankTransfer);
    }
}
