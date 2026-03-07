//! Tax profile (stored as `tax/profile.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{EinStatus, EntityType, IrsTaxClassification};
use crate::domain::ids::{EntityId, TaxProfileId};

/// Tax profile for an entity, including EIN and IRS classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxProfile {
    tax_profile_id: TaxProfileId,
    entity_id: EntityId,
    ein: Option<String>,
    ein_status: EinStatus,
    irs_tax_classification: IrsTaxClassification,
    created_at: DateTime<Utc>,
}

impl TaxProfile {
    /// Create a new tax profile with the appropriate IRS classification.
    pub fn new(
        tax_profile_id: TaxProfileId,
        entity_id: EntityId,
        irs_tax_classification: IrsTaxClassification,
    ) -> Self {
        Self {
            tax_profile_id,
            entity_id,
            ein: None,
            ein_status: EinStatus::Pending,
            irs_tax_classification,
            created_at: Utc::now(),
        }
    }

    /// Confirm that an EIN has been assigned.
    pub fn confirm_ein(&mut self, ein: String) {
        self.ein = Some(ein);
        self.ein_status = EinStatus::Active;
    }

    /// Determine the correct IRS tax classification based on entity type and
    /// member count.
    ///
    /// - Corporation -> C-Corporation
    /// - LLC with 1 member -> Disregarded Entity
    /// - LLC with 2+ members -> Partnership
    pub fn classify(entity_type: EntityType, member_count: usize) -> IrsTaxClassification {
        match entity_type {
            EntityType::CCorp => IrsTaxClassification::CCorporation,
            EntityType::Llc => {
                if member_count <= 1 {
                    IrsTaxClassification::DisregardedEntity
                } else {
                    IrsTaxClassification::Partnership
                }
            }
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn tax_profile_id(&self) -> TaxProfileId {
        self.tax_profile_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn ein(&self) -> Option<&str> {
        self.ein.as_deref()
    }

    pub fn ein_status(&self) -> EinStatus {
        self.ein_status
    }

    pub fn irs_tax_classification(&self) -> IrsTaxClassification {
        self.irs_tax_classification
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile() -> TaxProfile {
        TaxProfile::new(
            TaxProfileId::new(),
            EntityId::new(),
            IrsTaxClassification::CCorporation,
        )
    }

    #[test]
    fn new_profile_has_pending_ein() {
        let p = make_profile();
        assert_eq!(p.ein_status(), EinStatus::Pending);
        assert!(p.ein().is_none());
    }

    #[test]
    fn confirm_ein_activates() {
        let mut p = make_profile();
        p.confirm_ein("12-3456789".into());
        assert_eq!(p.ein_status(), EinStatus::Active);
        assert_eq!(p.ein(), Some("12-3456789"));
    }

    #[test]
    fn classify_corporation() {
        assert_eq!(
            TaxProfile::classify(EntityType::CCorp, 5),
            IrsTaxClassification::CCorporation,
        );
    }

    #[test]
    fn classify_single_member_llc() {
        assert_eq!(
            TaxProfile::classify(EntityType::Llc, 1),
            IrsTaxClassification::DisregardedEntity,
        );
    }

    #[test]
    fn classify_multi_member_llc() {
        assert_eq!(
            TaxProfile::classify(EntityType::Llc, 3),
            IrsTaxClassification::Partnership,
        );
    }

    #[test]
    fn serde_roundtrip() {
        let mut p = make_profile();
        p.confirm_ein("98-7654321".into());
        let json = serde_json::to_string(&p).unwrap();
        let parsed: TaxProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tax_profile_id(), p.tax_profile_id());
        assert_eq!(parsed.ein(), Some("98-7654321"));
    }
}
