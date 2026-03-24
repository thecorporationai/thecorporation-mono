//! Outbound payments.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::PaymentMethod;
use crate::ids::{EntityId, PaymentId};

// ── Payment ───────────────────────────────────────────────────────────────────

/// A recorded outbound payment made by the entity to a recipient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payment {
    pub payment_id: PaymentId,
    pub entity_id: EntityId,
    pub recipient_name: String,
    pub amount_cents: i64,
    pub method: PaymentMethod,
    /// External reference number (e.g. ACH trace ID, check number).
    pub reference: Option<String>,
    pub paid_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Payment {
    /// Record a new payment.
    pub fn new(
        entity_id: EntityId,
        recipient_name: impl Into<String>,
        amount_cents: i64,
        method: PaymentMethod,
        reference: Option<String>,
        paid_at: DateTime<Utc>,
    ) -> Self {
        Self {
            payment_id: PaymentId::new(),
            entity_id,
            recipient_name: recipient_name.into(),
            amount_cents,
            method,
            reference,
            paid_at,
            created_at: Utc::now(),
        }
    }
}
