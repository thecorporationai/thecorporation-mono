//! Invoice record (stored as `treasury/invoices/{invoice_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::TreasuryError;
use super::types::{Cents, Currency, InvoiceStatus};
use crate::domain::ids::{EntityId, InvoiceId};

/// An invoice issued by an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    invoice_id: InvoiceId,
    entity_id: EntityId,
    customer_name: String,
    amount_cents: Cents,
    currency: Currency,
    description: String,
    due_date: NaiveDate,
    status: InvoiceStatus,
    paid_at: Option<DateTime<Utc>>,
    voided_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl Invoice {
    /// Create a new invoice in Draft status.
    pub fn new(
        invoice_id: InvoiceId,
        entity_id: EntityId,
        customer_name: String,
        amount_cents: Cents,
        description: String,
        due_date: NaiveDate,
    ) -> Self {
        Self {
            invoice_id,
            entity_id,
            customer_name,
            amount_cents,
            currency: Currency::default(),
            description,
            due_date,
            status: InvoiceStatus::Draft,
            paid_at: None,
            voided_at: None,
            created_at: Utc::now(),
        }
    }

    /// Send the invoice. Draft -> Sent.
    pub fn send(&mut self) -> Result<(), TreasuryError> {
        if self.status != InvoiceStatus::Draft {
            return Err(TreasuryError::InvalidInvoiceTransition {
                from: self.status,
                to: InvoiceStatus::Sent,
            });
        }
        self.status = InvoiceStatus::Sent;
        Ok(())
    }

    /// Mark as paid. Sent -> Paid.
    pub fn mark_paid(&mut self) -> Result<(), TreasuryError> {
        if self.status != InvoiceStatus::Sent {
            return Err(TreasuryError::InvalidInvoiceTransition {
                from: self.status,
                to: InvoiceStatus::Paid,
            });
        }
        self.status = InvoiceStatus::Paid;
        self.paid_at = Some(Utc::now());
        Ok(())
    }

    /// Void the invoice. Draft or Sent -> Voided.
    pub fn void(&mut self) -> Result<(), TreasuryError> {
        match self.status {
            InvoiceStatus::Draft | InvoiceStatus::Sent => {
                self.status = InvoiceStatus::Voided;
                self.voided_at = Some(Utc::now());
                Ok(())
            }
            _ => Err(TreasuryError::InvalidInvoiceTransition {
                from: self.status,
                to: InvoiceStatus::Voided,
            }),
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn invoice_id(&self) -> InvoiceId {
        self.invoice_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn customer_name(&self) -> &str {
        &self.customer_name
    }

    pub fn amount_cents(&self) -> Cents {
        self.amount_cents
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn due_date(&self) -> NaiveDate {
        self.due_date
    }

    pub fn status(&self) -> InvoiceStatus {
        self.status
    }

    pub fn paid_at(&self) -> Option<DateTime<Utc>> {
        self.paid_at
    }

    pub fn voided_at(&self) -> Option<DateTime<Utc>> {
        self.voided_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_invoice() -> Invoice {
        Invoice::new(
            InvoiceId::new(),
            EntityId::new(),
            "Acme Client".into(),
            Cents::new(50000),
            "Consulting services".into(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        )
    }

    #[test]
    fn new_invoice_is_draft() {
        let inv = make_invoice();
        assert_eq!(inv.status(), InvoiceStatus::Draft);
        assert_eq!(inv.customer_name(), "Acme Client");
        assert_eq!(inv.amount_cents(), Cents::new(50000));
    }

    #[test]
    fn full_lifecycle_draft_sent_paid() {
        let mut inv = make_invoice();
        assert!(inv.send().is_ok());
        assert_eq!(inv.status(), InvoiceStatus::Sent);
        assert!(inv.mark_paid().is_ok());
        assert_eq!(inv.status(), InvoiceStatus::Paid);
        assert!(inv.paid_at().is_some());
    }

    #[test]
    fn void_from_draft() {
        let mut inv = make_invoice();
        assert!(inv.void().is_ok());
        assert_eq!(inv.status(), InvoiceStatus::Voided);
        assert!(inv.voided_at().is_some());
    }

    #[test]
    fn void_from_sent() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        assert!(inv.void().is_ok());
        assert_eq!(inv.status(), InvoiceStatus::Voided);
    }

    #[test]
    fn cannot_void_paid() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        inv.mark_paid().unwrap();
        assert!(inv.void().is_err());
    }

    #[test]
    fn cannot_pay_draft() {
        let mut inv = make_invoice();
        assert!(inv.mark_paid().is_err());
    }

    #[test]
    fn cannot_send_twice() {
        let mut inv = make_invoice();
        inv.send().unwrap();
        assert!(inv.send().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let inv = make_invoice();
        let json = serde_json::to_string(&inv).unwrap();
        let parsed: Invoice = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.invoice_id(), inv.invoice_id());
        assert_eq!(parsed.customer_name(), inv.customer_name());
        assert_eq!(parsed.amount_cents(), inv.amount_cents());
        assert_eq!(parsed.status(), inv.status());
    }
}
