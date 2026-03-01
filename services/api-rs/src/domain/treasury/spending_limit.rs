//! Spending limit record (stored as `treasury/spending-limits/{limit_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{EntityId, SpendingLimitId};

/// A spending limit for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingLimit {
    spending_limit_id: SpendingLimitId,
    entity_id: EntityId,
    amount_cents: i64,
    period: String,
    category: String,
    created_at: DateTime<Utc>,
}

impl SpendingLimit {
    pub fn new(
        spending_limit_id: SpendingLimitId,
        entity_id: EntityId,
        amount_cents: i64,
        period: String,
        category: String,
    ) -> Self {
        Self {
            spending_limit_id,
            entity_id,
            amount_cents,
            period,
            category,
            created_at: Utc::now(),
        }
    }

    pub fn spending_limit_id(&self) -> SpendingLimitId {
        self.spending_limit_id
    }
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
    pub fn amount_cents(&self) -> i64 {
        self.amount_cents
    }
    pub fn period(&self) -> &str {
        &self.period
    }
    pub fn category(&self) -> &str {
        &self.category
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
