//! Distribution record (stored as `treasury/distributions/{distribution_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::Cents;
use crate::domain::ids::{DistributionId, EntityId};

/// Type of distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionType {
    Dividend,
    Return,
    Liquidation,
}

/// Status of a distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionStatus {
    Pending,
    Approved,
    Distributed,
}

/// A distribution of funds to stakeholders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distribution {
    distribution_id: DistributionId,
    entity_id: EntityId,
    distribution_type: DistributionType,
    total_amount_cents: Cents,
    description: String,
    status: DistributionStatus,
    created_at: DateTime<Utc>,
}

impl Distribution {
    pub fn new(
        distribution_id: DistributionId,
        entity_id: EntityId,
        distribution_type: DistributionType,
        total_amount_cents: Cents,
        description: String,
    ) -> Self {
        Self {
            distribution_id,
            entity_id,
            distribution_type,
            total_amount_cents,
            description,
            status: DistributionStatus::Pending,
            created_at: Utc::now(),
        }
    }

    pub fn approve(&mut self) {
        self.status = DistributionStatus::Approved;
    }

    pub fn mark_distributed(&mut self) {
        self.status = DistributionStatus::Distributed;
    }

    // Accessors
    pub fn distribution_id(&self) -> DistributionId { self.distribution_id }
    pub fn entity_id(&self) -> EntityId { self.entity_id }
    pub fn distribution_type(&self) -> DistributionType { self.distribution_type }
    pub fn total_amount_cents(&self) -> Cents { self.total_amount_cents }
    pub fn description(&self) -> &str { &self.description }
    pub fn status(&self) -> DistributionStatus { self.status }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let d = Distribution::new(
            DistributionId::new(),
            EntityId::new(),
            DistributionType::Dividend,
            Cents::new(1000000),
            "Q4 dividend".to_owned(),
        );
        let json = serde_json::to_string(&d).unwrap();
        let parsed: Distribution = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.distribution_id(), d.distribution_id());
        assert_eq!(parsed.distribution_type(), DistributionType::Dividend);
    }
}
