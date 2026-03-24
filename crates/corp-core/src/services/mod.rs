//! Services domain — purchasable service requests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ids::{EntityId, ServiceRequestId};

// ── ServiceRequestStatus ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRequestStatus {
    Pending,
    Checkout,
    Paid,
    Fulfilling,
    Fulfilled,
    Failed,
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ServiceRequestError {
    #[error("service request must be Pending to begin checkout; current status: {0:?}")]
    NotPending(ServiceRequestStatus),
    #[error("service request must be Checkout to be marked paid; current status: {0:?}")]
    NotCheckout(ServiceRequestStatus),
    #[error("service request must be Paid to begin fulfillment; current status: {0:?}")]
    NotPaid(ServiceRequestStatus),
    #[error("service request must be Fulfilling to be fulfilled; current status: {0:?}")]
    NotFulfilling(ServiceRequestStatus),
    #[error("service request is already in a terminal state: {0:?}")]
    AlreadyTerminal(ServiceRequestStatus),
}

// ── ServiceRequest ────────────────────────────────────────────────────────────

/// A customer purchase of a platform service.
///
/// The FSM is:
/// ```text
/// Pending → Checkout → Paid → Fulfilling → Fulfilled
///                                 ↓
///                               Failed (from Fulfilling)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRequest {
    pub request_id: ServiceRequestId,
    pub entity_id: EntityId,
    /// Slug identifying the service product (e.g. `"registered-agent-de"`).
    pub service_slug: String,
    pub amount_cents: i64,
    pub status: ServiceRequestStatus,
    pub fulfillment_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub fulfilled_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
}

impl ServiceRequest {
    /// Create a new service request in `Pending` status.
    pub fn new(
        entity_id: EntityId,
        service_slug: impl Into<String>,
        amount_cents: i64,
    ) -> Self {
        Self {
            request_id: ServiceRequestId::new(),
            entity_id,
            service_slug: service_slug.into(),
            amount_cents,
            status: ServiceRequestStatus::Pending,
            fulfillment_note: None,
            created_at: Utc::now(),
            paid_at: None,
            fulfilled_at: None,
            failed_at: None,
        }
    }

    /// Transition `Pending → Checkout`.
    pub fn begin_checkout(&mut self) -> Result<(), ServiceRequestError> {
        match self.status {
            ServiceRequestStatus::Pending => {
                self.status = ServiceRequestStatus::Checkout;
                Ok(())
            }
            s => Err(ServiceRequestError::NotPending(s)),
        }
    }

    /// Transition `Checkout → Paid`.
    pub fn mark_paid(&mut self) -> Result<(), ServiceRequestError> {
        match self.status {
            ServiceRequestStatus::Checkout => {
                self.status = ServiceRequestStatus::Paid;
                self.paid_at = Some(Utc::now());
                Ok(())
            }
            s => Err(ServiceRequestError::NotCheckout(s)),
        }
    }

    /// Transition `Paid → Fulfilling`.
    pub fn begin_fulfillment(&mut self) -> Result<(), ServiceRequestError> {
        match self.status {
            ServiceRequestStatus::Paid => {
                self.status = ServiceRequestStatus::Fulfilling;
                Ok(())
            }
            s => Err(ServiceRequestError::NotPaid(s)),
        }
    }

    /// Transition `Fulfilling → Fulfilled`.
    pub fn fulfill(&mut self, note: Option<String>) -> Result<(), ServiceRequestError> {
        match self.status {
            ServiceRequestStatus::Fulfilling => {
                self.status = ServiceRequestStatus::Fulfilled;
                self.fulfillment_note = note;
                self.fulfilled_at = Some(Utc::now());
                Ok(())
            }
            s => Err(ServiceRequestError::NotFulfilling(s)),
        }
    }

    /// Transition `Fulfilling → Failed`.
    pub fn fail(&mut self) -> Result<(), ServiceRequestError> {
        match self.status {
            ServiceRequestStatus::Fulfilling => {
                self.status = ServiceRequestStatus::Failed;
                self.failed_at = Some(Utc::now());
                Ok(())
            }
            s => Err(ServiceRequestError::NotFulfilling(s)),
        }
    }

    /// Returns `true` if this request is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            ServiceRequestStatus::Fulfilled | ServiceRequestStatus::Failed
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request() -> ServiceRequest {
        ServiceRequest::new(EntityId::new(), "registered-agent-de", 10_000)
    }

    #[test]
    fn happy_path() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        r.fulfill(Some("Registered agent activated".into())).unwrap();
        assert_eq!(r.status, ServiceRequestStatus::Fulfilled);
        assert!(r.fulfilled_at.is_some());
    }

    #[test]
    fn fail_path() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        r.fail().unwrap();
        assert_eq!(r.status, ServiceRequestStatus::Failed);
    }

    #[test]
    fn wrong_state_transition_fails() {
        let mut r = make_request();
        assert!(matches!(r.mark_paid(), Err(ServiceRequestError::NotCheckout(_))));
    }

    // ── Additional coverage ───────────────────────────────────────────────────

    #[test]
    fn new_service_request_is_pending() {
        let r = make_request();
        assert_eq!(r.status, ServiceRequestStatus::Pending);
    }

    #[test]
    fn new_service_request_stores_fields() {
        let r = make_request();
        assert_eq!(r.service_slug, "registered-agent-de");
        assert_eq!(r.amount_cents, 10_000);
        assert!(r.paid_at.is_none());
        assert!(r.fulfilled_at.is_none());
        assert!(r.failed_at.is_none());
        assert!(r.fulfillment_note.is_none());
    }

    #[test]
    fn begin_checkout_from_pending() {
        let mut r = make_request();
        assert!(r.begin_checkout().is_ok());
        assert_eq!(r.status, ServiceRequestStatus::Checkout);
    }

    #[test]
    fn begin_checkout_from_checkout_is_error() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        assert!(matches!(r.begin_checkout(), Err(ServiceRequestError::NotPending(_))));
    }

    #[test]
    fn begin_checkout_from_paid_is_error() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        assert!(matches!(r.begin_checkout(), Err(ServiceRequestError::NotPending(_))));
    }

    #[test]
    fn mark_paid_from_checkout() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        assert!(r.mark_paid().is_ok());
        assert_eq!(r.status, ServiceRequestStatus::Paid);
        assert!(r.paid_at.is_some());
    }

    #[test]
    fn mark_paid_from_pending_is_error() {
        let mut r = make_request();
        assert!(matches!(r.mark_paid(), Err(ServiceRequestError::NotCheckout(_))));
    }

    #[test]
    fn mark_paid_from_paid_is_error() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        assert!(matches!(r.mark_paid(), Err(ServiceRequestError::NotCheckout(_))));
    }

    #[test]
    fn begin_fulfillment_from_paid() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        assert!(r.begin_fulfillment().is_ok());
        assert_eq!(r.status, ServiceRequestStatus::Fulfilling);
    }

    #[test]
    fn begin_fulfillment_from_pending_is_error() {
        let mut r = make_request();
        assert!(matches!(r.begin_fulfillment(), Err(ServiceRequestError::NotPaid(_))));
    }

    #[test]
    fn begin_fulfillment_from_checkout_is_error() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        assert!(matches!(r.begin_fulfillment(), Err(ServiceRequestError::NotPaid(_))));
    }

    #[test]
    fn fulfill_from_fulfilling_with_note() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        assert!(r.fulfill(Some("Service activated".into())).is_ok());
        assert_eq!(r.status, ServiceRequestStatus::Fulfilled);
        assert!(r.fulfilled_at.is_some());
        assert_eq!(r.fulfillment_note.as_deref(), Some("Service activated"));
    }

    #[test]
    fn fulfill_from_fulfilling_without_note() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        assert!(r.fulfill(None).is_ok());
        assert!(r.fulfillment_note.is_none());
    }

    #[test]
    fn fulfill_from_paid_is_error() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        assert!(matches!(r.fulfill(None), Err(ServiceRequestError::NotFulfilling(_))));
    }

    #[test]
    fn fail_from_fulfilling() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        assert!(r.fail().is_ok());
        assert_eq!(r.status, ServiceRequestStatus::Failed);
        assert!(r.failed_at.is_some());
    }

    #[test]
    fn fail_from_pending_is_error() {
        let mut r = make_request();
        assert!(matches!(r.fail(), Err(ServiceRequestError::NotFulfilling(_))));
    }

    #[test]
    fn fail_from_paid_is_error() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        assert!(matches!(r.fail(), Err(ServiceRequestError::NotFulfilling(_))));
    }

    #[test]
    fn fail_from_fulfilled_is_error() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        r.fulfill(None).unwrap();
        assert!(matches!(r.fail(), Err(ServiceRequestError::NotFulfilling(_))));
    }

    #[test]
    fn is_terminal_for_fulfilled() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        r.fulfill(None).unwrap();
        assert!(r.is_terminal());
    }

    #[test]
    fn is_terminal_for_failed() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        r.fail().unwrap();
        assert!(r.is_terminal());
    }

    #[test]
    fn is_not_terminal_for_pending() {
        assert!(!make_request().is_terminal());
    }

    #[test]
    fn is_not_terminal_for_checkout() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        assert!(!r.is_terminal());
    }

    #[test]
    fn is_not_terminal_for_paid() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        assert!(!r.is_terminal());
    }

    #[test]
    fn is_not_terminal_for_fulfilling() {
        let mut r = make_request();
        r.begin_checkout().unwrap();
        r.mark_paid().unwrap();
        r.begin_fulfillment().unwrap();
        assert!(!r.is_terminal());
    }

    #[test]
    fn service_request_status_serde_roundtrip() {
        for status in [
            ServiceRequestStatus::Pending,
            ServiceRequestStatus::Checkout,
            ServiceRequestStatus::Paid,
            ServiceRequestStatus::Fulfilling,
            ServiceRequestStatus::Fulfilled,
            ServiceRequestStatus::Failed,
        ] {
            let s = serde_json::to_string(&status).unwrap();
            let de: ServiceRequestStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(de, status);
        }
        assert_eq!(serde_json::to_string(&ServiceRequestStatus::Pending).unwrap(), r#""pending""#);
        assert_eq!(serde_json::to_string(&ServiceRequestStatus::Checkout).unwrap(), r#""checkout""#);
        assert_eq!(serde_json::to_string(&ServiceRequestStatus::Paid).unwrap(), r#""paid""#);
        assert_eq!(serde_json::to_string(&ServiceRequestStatus::Fulfilling).unwrap(), r#""fulfilling""#);
        assert_eq!(serde_json::to_string(&ServiceRequestStatus::Fulfilled).unwrap(), r#""fulfilled""#);
        assert_eq!(serde_json::to_string(&ServiceRequestStatus::Failed).unwrap(), r#""failed""#);
    }

    #[test]
    fn service_request_ids_are_unique() {
        let a = make_request();
        let b = make_request();
        assert_ne!(a.request_id, b.request_id);
    }
}
