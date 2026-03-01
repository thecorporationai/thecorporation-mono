//! Service catalog — the list of fulfillment services offered by TheCorporation.ai.
//!
//! For v1 the catalog is hardcoded. Each item maps an `obligation_type` slug
//! to a priced service that TheCorporation.ai can fulfill on behalf of an entity.

use serde::Serialize;

use super::types::PriceType;
use crate::domain::ids::ServiceItemId;

/// A service offered in the fulfillment catalog.
#[derive(Debug, Clone, Serialize)]
pub struct ServiceItem {
    pub item_id: ServiceItemId,
    /// URL-safe slug, also used as the obligation_type when auto-creating obligations.
    pub slug: String,
    pub name: String,
    pub description: String,
    /// Price in US cents.
    pub price_cents: i64,
    pub price_type: PriceType,
    /// The obligation_type this service fulfills.
    pub obligation_type: String,
    /// Whether this item is currently available for purchase.
    pub active: bool,
}

/// Return the full service catalog.
///
/// Item IDs are deterministic (derived from slug) so they remain stable across
/// restarts. In a future version this could be loaded from a config file or
/// database.
pub fn service_catalog() -> Vec<ServiceItem> {
    use sha2::{Digest, Sha256};
    use uuid::Uuid;

    /// Deterministic ID from a slug so catalog items have stable IDs.
    fn id_from_slug(slug: &str) -> ServiceItemId {
        let hash = Sha256::digest(slug.as_bytes());
        // Take the first 16 bytes of the SHA-256 hash as a UUID.
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&hash[..16]);
        ServiceItemId::from_uuid(Uuid::from_bytes(bytes))
    }

    vec![
        ServiceItem {
            item_id: id_from_slug("state_filing.incorporation"),
            slug: "state_filing.incorporation".to_owned(),
            name: "State Incorporation Filing".to_owned(),
            description: "File articles of incorporation with the state. \
                          Includes name availability check, document preparation, \
                          and state filing fee."
                .to_owned(),
            price_cents: 29900,
            price_type: PriceType::OneTime,
            obligation_type: "state_filing.incorporation".to_owned(),
            active: true,
        },
        ServiceItem {
            item_id: id_from_slug("registered_agent"),
            slug: "registered_agent".to_owned(),
            name: "Registered Agent Service".to_owned(),
            description: "Designated registered agent for service of process. \
                          Includes a registered address and mail forwarding."
                .to_owned(),
            price_cents: 14900,
            price_type: PriceType::Annual,
            obligation_type: "registered_agent".to_owned(),
            active: true,
        },
        ServiceItem {
            item_id: id_from_slug("ein_application"),
            slug: "ein_application".to_owned(),
            name: "EIN Application".to_owned(),
            description: "Apply for a Federal Employer Identification Number (EIN) \
                          with the IRS on behalf of the entity."
                .to_owned(),
            price_cents: 9900,
            price_type: PriceType::OneTime,
            obligation_type: "ein_application".to_owned(),
            active: true,
        },
        ServiceItem {
            item_id: id_from_slug("annual_report"),
            slug: "annual_report".to_owned(),
            name: "Annual Report Filing".to_owned(),
            description: "Prepare and file the entity's annual report with the \
                          Secretary of State."
                .to_owned(),
            price_cents: 9900,
            price_type: PriceType::Annual,
            obligation_type: "annual_report".to_owned(),
            active: true,
        },
        ServiceItem {
            item_id: id_from_slug("boi_report"),
            slug: "boi_report".to_owned(),
            name: "BOI Report Filing".to_owned(),
            description: "File Beneficial Ownership Information report with FinCEN.".to_owned(),
            price_cents: 4900,
            price_type: PriceType::OneTime,
            obligation_type: "boi_report".to_owned(),
            active: true,
        },
        ServiceItem {
            item_id: id_from_slug("transfer_agent"),
            slug: "transfer_agent".to_owned(),
            name: "Transfer Agent Service".to_owned(),
            description: "Maintain the official shareholder register, process \
                          share transfers, and issue certificates."
                .to_owned(),
            price_cents: 29900,
            price_type: PriceType::Annual,
            obligation_type: "transfer_agent".to_owned(),
            active: true,
        },
        ServiceItem {
            item_id: id_from_slug("franchise_tax"),
            slug: "franchise_tax".to_owned(),
            name: "Franchise Tax Filing".to_owned(),
            description: "Calculate and file the entity's annual franchise tax.".to_owned(),
            price_cents: 19900,
            price_type: PriceType::Annual,
            obligation_type: "franchise_tax".to_owned(),
            active: true,
        },
        ServiceItem {
            item_id: id_from_slug("qualification.foreign"),
            slug: "qualification.foreign".to_owned(),
            name: "Foreign Qualification".to_owned(),
            description: "Register the entity to do business in an additional state.".to_owned(),
            price_cents: 29900,
            price_type: PriceType::OneTime,
            obligation_type: "qualification.foreign".to_owned(),
            active: true,
        },
    ]
}

/// Look up a catalog item by its slug.
pub fn find_by_slug(slug: &str) -> Option<ServiceItem> {
    service_catalog().into_iter().find(|i| i.slug == slug)
}

/// Look up a catalog item by its ID.
pub fn find_by_id(id: ServiceItemId) -> Option<ServiceItem> {
    service_catalog().into_iter().find(|i| i.item_id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_items() {
        let catalog = service_catalog();
        assert!(!catalog.is_empty());
        assert!(catalog.len() >= 7);
    }

    #[test]
    fn all_items_have_positive_price() {
        for item in service_catalog() {
            assert!(item.price_cents > 0, "item {} has zero price", item.slug);
        }
    }

    #[test]
    fn slug_lookup_works() {
        let item = find_by_slug("transfer_agent").expect("transfer_agent should exist");
        assert_eq!(item.name, "Transfer Agent Service");
        assert_eq!(item.price_cents, 29900);
    }

    #[test]
    fn id_lookup_works() {
        let catalog = service_catalog();
        let first = &catalog[0];
        let found = find_by_id(first.item_id).expect("should find by id");
        assert_eq!(found.slug, first.slug);
    }

    #[test]
    fn ids_are_deterministic() {
        let a = service_catalog();
        let b = service_catalog();
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(
                x.item_id, y.item_id,
                "IDs should be stable for slug {}",
                x.slug
            );
        }
    }

    #[test]
    fn slugs_are_unique() {
        let catalog = service_catalog();
        let mut slugs: Vec<&str> = catalog.iter().map(|i| i.slug.as_str()).collect();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), catalog.len(), "duplicate slugs in catalog");
    }

    #[test]
    fn missing_slug_returns_none() {
        assert!(find_by_slug("nonexistent_service").is_none());
    }
}
