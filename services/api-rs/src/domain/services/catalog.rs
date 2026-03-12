//! Hard-coded service catalog.
//!
//! Products change rarely — no need for storage. Matches the billing plans
//! pattern in `admin.rs`.

use crate::domain::ids::ServiceItemId;
use crate::domain::services::types::PriceType;
use uuid::Uuid;

/// A purchasable service from the catalog.
#[derive(Debug, Clone)]
pub struct ServiceItem {
    pub item_id: ServiceItemId,
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub price_type: PriceType,
    pub amount_cents: i64,
    pub jurisdiction: Option<&'static str>,
}

// Deterministic UUIDs so item IDs are stable across restarts.
const EIN_REGISTRATION_ID: Uuid = Uuid::from_bytes([
    0x01, 0x5e, 0x10, 0x00, 0xca, 0x7a, 0x40, 0x01,
    0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
]);
const REGISTERED_AGENT_ID: Uuid = Uuid::from_bytes([
    0x01, 0x5e, 0x10, 0x00, 0xca, 0x7a, 0x40, 0x01,
    0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
]);
const REGISTERED_AGENT_RENEWAL_ID: Uuid = Uuid::from_bytes([
    0x01, 0x5e, 0x10, 0x00, 0xca, 0x7a, 0x40, 0x01,
    0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
]);
const STATE_FILING_INCORP_ID: Uuid = Uuid::from_bytes([
    0x01, 0x5e, 0x10, 0x00, 0xca, 0x7a, 0x40, 0x01,
    0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
]);
const STATE_FILING_INCORP_DE_ID: Uuid = Uuid::from_bytes([
    0x01, 0x5e, 0x10, 0x00, 0xca, 0x7a, 0x40, 0x01,
    0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05,
]);
const STATE_FILING_INCORP_WY_ID: Uuid = Uuid::from_bytes([
    0x01, 0x5e, 0x10, 0x00, 0xca, 0x7a, 0x40, 0x01,
    0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06,
]);

static CATALOG: &[ServiceItem] = &[
    ServiceItem {
        item_id: ServiceItemId::from_uuid_const(EIN_REGISTRATION_ID),
        slug: "ein_registration",
        name: "EIN Registration",
        description: "Apply for a Federal Employer Identification Number with the IRS",
        price_type: PriceType::OneTime,
        amount_cents: 29900,
        jurisdiction: None,
    },
    ServiceItem {
        item_id: ServiceItemId::from_uuid_const(REGISTERED_AGENT_ID),
        slug: "registered_agent",
        name: "Registered Agent",
        description: "Registered agent service for one year",
        price_type: PriceType::Annual,
        amount_cents: 14900,
        jurisdiction: None,
    },
    ServiceItem {
        item_id: ServiceItemId::from_uuid_const(REGISTERED_AGENT_RENEWAL_ID),
        slug: "registered_agent_renewal",
        name: "Registered Agent Renewal",
        description: "Renew registered agent service for one additional year",
        price_type: PriceType::Annual,
        amount_cents: 14900,
        jurisdiction: None,
    },
    ServiceItem {
        item_id: ServiceItemId::from_uuid_const(STATE_FILING_INCORP_ID),
        slug: "state_filing.incorporation",
        name: "State Filing — Incorporation",
        description: "File incorporation documents with the state",
        price_type: PriceType::OneTime,
        amount_cents: 29900,
        jurisdiction: None,
    },
    ServiceItem {
        item_id: ServiceItemId::from_uuid_const(STATE_FILING_INCORP_DE_ID),
        slug: "state_filing.incorporation.de",
        name: "State Filing — Delaware Incorporation",
        description: "File incorporation documents with the State of Delaware",
        price_type: PriceType::OneTime,
        amount_cents: 29900,
        jurisdiction: Some("US-DE"),
    },
    ServiceItem {
        item_id: ServiceItemId::from_uuid_const(STATE_FILING_INCORP_WY_ID),
        slug: "state_filing.incorporation.wy",
        name: "State Filing — Wyoming Incorporation",
        description: "File incorporation documents with the State of Wyoming",
        price_type: PriceType::OneTime,
        amount_cents: 19900,
        jurisdiction: Some("US-WY"),
    },
];

/// Returns the full service catalog.
pub fn catalog() -> &'static [ServiceItem] {
    CATALOG
}

/// Find a catalog item by its slug.
pub fn find_by_slug(slug: &str) -> Option<&'static ServiceItem> {
    CATALOG.iter().find(|item| item.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_non_empty() {
        assert!(!catalog().is_empty());
    }

    #[test]
    fn slugs_are_unique() {
        let slugs: Vec<&str> = catalog().iter().map(|i| i.slug).collect();
        let mut deduped = slugs.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(slugs.len(), deduped.len());
    }

    #[test]
    fn ids_are_unique() {
        let ids: Vec<_> = catalog().iter().map(|i| i.item_id).collect();
        let mut deduped = ids.clone();
        deduped.sort_by_key(|id| *id.as_uuid());
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len());
    }

    #[test]
    fn find_ein_registration() {
        let item = find_by_slug("ein_registration").expect("should find EIN registration");
        assert_eq!(item.amount_cents, 29900);
        assert_eq!(item.price_type, PriceType::OneTime);
    }

    #[test]
    fn find_missing_slug_returns_none() {
        assert!(find_by_slug("nonexistent").is_none());
    }

    #[test]
    fn delaware_has_jurisdiction() {
        let item = find_by_slug("state_filing.incorporation.de").expect("DE item");
        assert_eq!(item.jurisdiction, Some("US-DE"));
    }
}
