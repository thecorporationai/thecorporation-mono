//! Service request record (stored as `services/requests/{request_id}.json`).
//!
//! A service request links an entity's obligation to a paid fulfillment
//! service from the catalog. It tracks the payment lifecycle (Stripe checkout)
//! and the fulfillment lifecycle.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::ServiceError;
use super::types::ServiceRequestStatus;
use crate::domain::ids::{EntityId, ObligationId, ServiceItemId, ServiceRequestId};

/// A request for a fulfillment service, linked to an obligation and a catalog item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRequest {
    request_id: ServiceRequestId,
    entity_id: EntityId,
    obligation_id: ObligationId,
    service_item_id: ServiceItemId,
    /// The catalog slug at time of purchase (denormalized for auditability).
    service_slug: String,
    /// Price in US cents at time of purchase.
    amount_cents: i64,
    /// Stripe checkout session ID (set when checkout is initiated).
    stripe_checkout_session_id: Option<String>,
    /// Stripe payment intent ID (set when payment completes).
    stripe_payment_intent_id: Option<String>,
    status: ServiceRequestStatus,
    created_at: DateTime<Utc>,
    paid_at: Option<DateTime<Utc>>,
    fulfilled_at: Option<DateTime<Utc>>,
    failed_at: Option<DateTime<Utc>>,
    /// Free-text note from the fulfillment operator.
    fulfillment_note: Option<String>,
}

impl ServiceRequest {
    pub fn new(
        request_id: ServiceRequestId,
        entity_id: EntityId,
        obligation_id: ObligationId,
        service_item_id: ServiceItemId,
        service_slug: String,
        amount_cents: i64,
    ) -> Self {
        Self {
            request_id,
            entity_id,
            obligation_id,
            service_item_id,
            service_slug,
            amount_cents,
            stripe_checkout_session_id: None,
            stripe_payment_intent_id: None,
            status: ServiceRequestStatus::Pending,
            created_at: Utc::now(),
            paid_at: None,
            fulfilled_at: None,
            failed_at: None,
            fulfillment_note: None,
        }
    }

    // ── State transitions ─────────────────────────────────────────────

    /// Pending -> Checkout (Stripe session created).
    pub fn begin_checkout(
        &mut self,
        session_id: String,
    ) -> Result<(), ServiceError> {
        if self.status != ServiceRequestStatus::Pending {
            return Err(ServiceError::InvalidTransition {
                from: self.status,
                to: ServiceRequestStatus::Checkout,
            });
        }
        self.stripe_checkout_session_id = Some(session_id);
        self.status = ServiceRequestStatus::Checkout;
        Ok(())
    }

    /// Checkout -> Paid (Stripe payment confirmed).
    pub fn mark_paid(
        &mut self,
        payment_intent_id: Option<String>,
    ) -> Result<(), ServiceError> {
        if self.status != ServiceRequestStatus::Checkout {
            return Err(ServiceError::InvalidTransition {
                from: self.status,
                to: ServiceRequestStatus::Paid,
            });
        }
        self.stripe_payment_intent_id = payment_intent_id;
        self.paid_at = Some(Utc::now());
        self.status = ServiceRequestStatus::Paid;
        Ok(())
    }

    /// Paid -> Fulfilling (operator picked up the request).
    pub fn begin_fulfillment(&mut self) -> Result<(), ServiceError> {
        if self.status != ServiceRequestStatus::Paid {
            return Err(ServiceError::InvalidTransition {
                from: self.status,
                to: ServiceRequestStatus::Fulfilling,
            });
        }
        self.status = ServiceRequestStatus::Fulfilling;
        Ok(())
    }

    /// Fulfilling -> Fulfilled (service completed).
    pub fn fulfill(&mut self, note: Option<String>) -> Result<(), ServiceError> {
        if self.status != ServiceRequestStatus::Fulfilling {
            return Err(ServiceError::InvalidTransition {
                from: self.status,
                to: ServiceRequestStatus::Fulfilled,
            });
        }
        self.fulfilled_at = Some(Utc::now());
        self.fulfillment_note = note;
        self.status = ServiceRequestStatus::Fulfilled;
        Ok(())
    }

    /// Any non-terminal state -> Failed.
    pub fn fail(&mut self) -> Result<(), ServiceError> {
        match self.status {
            ServiceRequestStatus::Fulfilled | ServiceRequestStatus::Failed => {
                Err(ServiceError::InvalidTransition {
                    from: self.status,
                    to: ServiceRequestStatus::Failed,
                })
            }
            _ => {
                self.failed_at = Some(Utc::now());
                self.status = ServiceRequestStatus::Failed;
                Ok(())
            }
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────

    pub fn request_id(&self) -> ServiceRequestId {
        self.request_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn obligation_id(&self) -> ObligationId {
        self.obligation_id
    }
    pub fn service_item_id(&self) -> ServiceItemId {
        self.service_item_id
    }
    pub fn service_slug(&self) -> &str {
        &self.service_slug
    }
    pub fn amount_cents(&self) -> i64 {
        self.amount_cents
    }
    pub fn stripe_checkout_session_id(&self) -> Option<&str> {
        self.stripe_checkout_session_id.as_deref()
    }
    pub fn stripe_payment_intent_id(&self) -> Option<&str> {
        self.stripe_payment_intent_id.as_deref()
    }
    pub fn status(&self) -> ServiceRequestStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn paid_at(&self) -> Option<DateTime<Utc>> {
        self.paid_at
    }
    pub fn fulfilled_at(&self) -> Option<DateTime<Utc>> {
        self.fulfilled_at
    }
    pub fn failed_at(&self) -> Option<DateTime<Utc>> {
        self.failed_at
    }
    pub fn fulfillment_note(&self) -> Option<&str> {
        self.fulfillment_note.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request() -> ServiceRequest {
        ServiceRequest::new(
            ServiceRequestId::new(),
            EntityId::new(),
            ObligationId::new(),
            ServiceItemId::new(),
            "state_filing.incorporation".to_owned(),
            29900,
        )
    }

    #[test]
    fn happy_path_lifecycle() {
        let mut req = make_request();
        assert_eq!(req.status(), ServiceRequestStatus::Pending);

        req.begin_checkout("cs_test_123".to_owned()).unwrap();
        assert_eq!(req.status(), ServiceRequestStatus::Checkout);
        assert_eq!(req.stripe_checkout_session_id(), Some("cs_test_123"));

        req.mark_paid(Some("pi_test_456".to_owned())).unwrap();
        assert_eq!(req.status(), ServiceRequestStatus::Paid);
        assert!(req.paid_at().is_some());
        assert_eq!(req.stripe_payment_intent_id(), Some("pi_test_456"));

        req.begin_fulfillment().unwrap();
        assert_eq!(req.status(), ServiceRequestStatus::Fulfilling);

        req.fulfill(Some("Filed with DE SOS".to_owned())).unwrap();
        assert_eq!(req.status(), ServiceRequestStatus::Fulfilled);
        assert!(req.fulfilled_at().is_some());
        assert_eq!(req.fulfillment_note(), Some("Filed with DE SOS"));
    }

    #[test]
    fn cannot_skip_checkout() {
        let mut req = make_request();
        assert!(req.mark_paid(None).is_err());
    }

    #[test]
    fn cannot_skip_payment() {
        let mut req = make_request();
        req.begin_checkout("cs_1".to_owned()).unwrap();
        assert!(req.begin_fulfillment().is_err());
    }

    #[test]
    fn cannot_fulfill_before_fulfilling() {
        let mut req = make_request();
        req.begin_checkout("cs_1".to_owned()).unwrap();
        req.mark_paid(None).unwrap();
        assert!(req.fulfill(None).is_err());
    }

    #[test]
    fn can_fail_from_any_non_terminal_state() {
        for start_fn in [
            |_r: &mut ServiceRequest| {},
            |r: &mut ServiceRequest| { r.begin_checkout("cs".to_owned()).unwrap(); },
            |r: &mut ServiceRequest| {
                r.begin_checkout("cs".to_owned()).unwrap();
                r.mark_paid(None).unwrap();
            },
            |r: &mut ServiceRequest| {
                r.begin_checkout("cs".to_owned()).unwrap();
                r.mark_paid(None).unwrap();
                r.begin_fulfillment().unwrap();
            },
        ] {
            let mut req = make_request();
            start_fn(&mut req);
            assert!(req.fail().is_ok(), "should be able to fail from {:?}", req.status());
        }
    }

    #[test]
    fn cannot_fail_from_fulfilled() {
        let mut req = make_request();
        req.begin_checkout("cs".to_owned()).unwrap();
        req.mark_paid(None).unwrap();
        req.begin_fulfillment().unwrap();
        req.fulfill(None).unwrap();
        assert!(req.fail().is_err());
    }

    #[test]
    fn cannot_fail_twice() {
        let mut req = make_request();
        req.fail().unwrap();
        assert!(req.fail().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let mut req = make_request();
        req.begin_checkout("cs_rt".to_owned()).unwrap();
        req.mark_paid(Some("pi_rt".to_owned())).unwrap();

        let json = serde_json::to_string_pretty(&req).expect("serialize");
        let parsed: ServiceRequest = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed.request_id(), req.request_id());
        assert_eq!(parsed.status(), ServiceRequestStatus::Paid);
        assert_eq!(parsed.service_slug(), "state_filing.incorporation");
        assert_eq!(parsed.amount_cents(), 29900);
        assert_eq!(parsed.stripe_checkout_session_id(), Some("cs_rt"));
        assert_eq!(parsed.stripe_payment_intent_id(), Some("pi_rt"));
    }
}
