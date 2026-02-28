//! Services domain types — catalog items and fulfillment requests.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── PriceType ──────────────────────────────────────────────────────────

/// How a service item is priced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceType {
    /// One-time payment (e.g., incorporation filing).
    OneTime,
    /// Recurring annual fee (e.g., registered agent).
    Annual,
    /// Per-use pricing (e.g., per filing).
    PerUse,
}

impl fmt::Display for PriceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OneTime => write!(f, "one_time"),
            Self::Annual => write!(f, "annual"),
            Self::PerUse => write!(f, "per_use"),
        }
    }
}

// ── ServiceRequestStatus ───────────────────────────────────────────────

/// Lifecycle status of a fulfillment service request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRequestStatus {
    /// Request created, awaiting payment.
    Pending,
    /// Stripe checkout session created, awaiting completion.
    Checkout,
    /// Payment confirmed; awaiting fulfillment.
    Paid,
    /// Fulfillment is in progress.
    Fulfilling,
    /// Service has been fulfilled.
    Fulfilled,
    /// Request failed or was cancelled.
    Failed,
}

impl fmt::Display for ServiceRequestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Checkout => write!(f, "checkout"),
            Self::Paid => write!(f, "paid"),
            Self::Fulfilling => write!(f, "fulfilling"),
            Self::Fulfilled => write!(f, "fulfilled"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_type_serde() {
        let pt = PriceType::Annual;
        let json = serde_json::to_string(&pt).expect("serialize PriceType");
        assert_eq!(json, "\"annual\"");
        let parsed: PriceType = serde_json::from_str(&json).expect("deserialize PriceType");
        assert_eq!(pt, parsed);
    }

    #[test]
    fn service_request_status_serde() {
        let status = ServiceRequestStatus::Fulfilling;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, "\"fulfilling\"");
        let parsed: ServiceRequestStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status, parsed);
    }

    #[test]
    fn price_type_display() {
        assert_eq!(PriceType::OneTime.to_string(), "one_time");
        assert_eq!(PriceType::Annual.to_string(), "annual");
        assert_eq!(PriceType::PerUse.to_string(), "per_use");
    }

    #[test]
    fn service_request_status_display() {
        assert_eq!(ServiceRequestStatus::Pending.to_string(), "pending");
        assert_eq!(ServiceRequestStatus::Checkout.to_string(), "checkout");
        assert_eq!(ServiceRequestStatus::Paid.to_string(), "paid");
        assert_eq!(ServiceRequestStatus::Fulfilling.to_string(), "fulfilling");
        assert_eq!(ServiceRequestStatus::Fulfilled.to_string(), "fulfilled");
        assert_eq!(ServiceRequestStatus::Failed.to_string(), "failed");
    }
}
