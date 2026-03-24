//! Customer invoices.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{EntityId, InvoiceId};
use super::types::{Currency, InvoiceStatus};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvoiceError {
    #[error("invoice must be in Draft status to be sent; current status: {0:?}")]
    NotDraft(InvoiceStatus),
    #[error("invoice must be in Sent status to be marked paid; current status: {0:?}")]
    NotSent(InvoiceStatus),
    #[error("invoice cannot be voided from status {0:?}")]
    CannotVoid(InvoiceStatus),
}

// ── Invoice ───────────────────────────────────────────────────────────────────

/// A customer-facing invoice. Follows the FSM:
/// ```text
/// Draft → Sent → Paid
///           ↓
///         Voided  (also from Draft)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_id: InvoiceId,
    pub entity_id: EntityId,
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub amount_cents: i64,
    pub currency: Currency,
    pub description: String,
    pub due_date: NaiveDate,
    pub status: InvoiceStatus,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Invoice {
    /// Create a new invoice in `Draft` status.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        customer_name: impl Into<String>,
        customer_email: Option<String>,
        amount_cents: i64,
        currency: Currency,
        description: impl Into<String>,
        due_date: NaiveDate,
    ) -> Self {
        Self {
            invoice_id: InvoiceId::new(),
            entity_id,
            customer_name: customer_name.into(),
            customer_email,
            amount_cents,
            currency,
            description: description.into(),
            due_date,
            status: InvoiceStatus::Draft,
            paid_at: None,
            created_at: Utc::now(),
        }
    }

    /// Transition `Draft → Sent`.
    pub fn send(&mut self) -> Result<(), InvoiceError> {
        match self.status {
            InvoiceStatus::Draft => {
                self.status = InvoiceStatus::Sent;
                Ok(())
            }
            s => Err(InvoiceError::NotDraft(s)),
        }
    }

    /// Transition `Sent → Paid`.
    pub fn mark_paid(&mut self) -> Result<(), InvoiceError> {
        match self.status {
            InvoiceStatus::Sent => {
                self.status = InvoiceStatus::Paid;
                self.paid_at = Some(Utc::now());
                Ok(())
            }
            s => Err(InvoiceError::NotSent(s)),
        }
    }

    /// Void this invoice. Allowed from `Draft` or `Sent`.
    pub fn void(&mut self) -> Result<(), InvoiceError> {
        match self.status {
            InvoiceStatus::Draft | InvoiceStatus::Sent => {
                self.status = InvoiceStatus::Voided;
                Ok(())
            }
            s => Err(InvoiceError::CannotVoid(s)),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntityId;

    fn make_invoice() -> Invoice {
        Invoice::new(
            EntityId::new(),
            "Acme Corp",
            Some("billing@acme.com".into()),
            10_000,
            Currency::Usd,
            "Legal services Q1",
            NaiveDate::from_ymd_opt(2026, 4, 30).unwrap(),
        )
    }

    #[test]
    fn new_invoice_is_draft() {
        let inv = make_invoice();
        assert_eq!(inv.status, InvoiceStatus::Draft);
        assert!(inv.paid_at.is_none());
    }

    #[test]
    fn new_invoice_stores_fields() {
        let inv = make_invoice();
        assert_eq!(inv.customer_name, "Acme Corp");
        assert_eq!(inv.customer_email.as_deref(), Some("billing@acme.com"));
        assert_eq!(inv.amount_cents, 10_000);
        assert_eq!(inv.currency, Currency::Usd);
        assert_eq!(inv.description, "Legal services Q1");
    }

    #[test]
    fn draft_to_sent() {
        let mut inv = make_invoice();
        assert!(inv.send().is_ok());
        assert_eq!(inv.status, InvoiceStatus::Sent);
    }

    #[test]
    fn sent_to_paid() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        assert!(inv.mark_paid().is_ok());
        assert_eq!(inv.status, InvoiceStatus::Paid);
        assert!(inv.paid_at.is_some());
    }

    #[test]
    fn full_lifecycle_draft_sent_paid() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        inv.mark_paid().unwrap();
        assert_eq!(inv.status, InvoiceStatus::Paid);
    }

    #[test]
    fn void_from_draft() {
        let mut inv = make_invoice();
        assert!(inv.void().is_ok());
        assert_eq!(inv.status, InvoiceStatus::Voided);
    }

    #[test]
    fn void_from_sent() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        assert!(inv.void().is_ok());
        assert_eq!(inv.status, InvoiceStatus::Voided);
    }

    #[test]
    fn cannot_skip_draft_to_paid() {
        let mut inv = make_invoice();
        // Must be Sent before Paid
        let err = inv.mark_paid();
        assert!(matches!(err, Err(InvoiceError::NotSent(_))));
    }

    #[test]
    fn cannot_send_already_sent() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        assert!(matches!(inv.send(), Err(InvoiceError::NotDraft(_))));
    }

    #[test]
    fn cannot_send_paid_invoice() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        inv.mark_paid().unwrap();
        assert!(matches!(inv.send(), Err(InvoiceError::NotDraft(_))));
    }

    #[test]
    fn cannot_send_voided_invoice() {
        let mut inv = make_invoice();
        inv.void().unwrap();
        assert!(matches!(inv.send(), Err(InvoiceError::NotDraft(_))));
    }

    #[test]
    fn cannot_void_paid_invoice() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        inv.mark_paid().unwrap();
        assert!(matches!(inv.void(), Err(InvoiceError::CannotVoid(InvoiceStatus::Paid))));
    }

    #[test]
    fn cannot_void_already_voided_invoice() {
        let mut inv = make_invoice();
        inv.void().unwrap();
        assert!(matches!(inv.void(), Err(InvoiceError::CannotVoid(InvoiceStatus::Voided))));
    }

    #[test]
    fn cannot_mark_paid_from_draft() {
        let mut inv = make_invoice();
        let err = inv.mark_paid();
        assert!(matches!(err, Err(InvoiceError::NotSent(InvoiceStatus::Draft))));
    }

    #[test]
    fn cannot_mark_paid_from_voided() {
        let mut inv = make_invoice();
        inv.void().unwrap();
        let err = inv.mark_paid();
        assert!(matches!(err, Err(InvoiceError::NotSent(InvoiceStatus::Voided))));
    }

    #[test]
    fn invoice_no_email() {
        let inv = Invoice::new(
            EntityId::new(),
            "Solo Client",
            None,
            500,
            Currency::Usd,
            "Consulting",
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        );
        assert!(inv.customer_email.is_none());
    }

    #[test]
    fn invoice_ids_are_unique() {
        let a = make_invoice();
        let b = make_invoice();
        assert_ne!(a.invoice_id, b.invoice_id);
    }
}
