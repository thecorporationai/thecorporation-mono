//! Equity distributions to stockholders or members.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{DistributionId, EntityId};

// ── Distribution ──────────────────────────────────────────────────────────────

/// A declared distribution of proceeds to equity holders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Distribution {
    pub distribution_id: DistributionId,
    pub entity_id: EntityId,
    pub total_amount_cents: i64,
    /// Amount per share in cents, if applicable (may be `None` for pro-rata
    /// distributions where the per-share amount is computed separately).
    pub per_share_cents: Option<i64>,
    /// Date used to determine which shareholders are eligible to receive the
    /// distribution.
    pub record_date: NaiveDate,
    /// Date on which the distribution is scheduled to be paid.
    pub payment_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

impl Distribution {
    /// Record a new distribution declaration.
    pub fn new(
        entity_id: EntityId,
        total_amount_cents: i64,
        per_share_cents: Option<i64>,
        record_date: NaiveDate,
        payment_date: NaiveDate,
    ) -> Self {
        Self {
            distribution_id: DistributionId::new(),
            entity_id,
            total_amount_cents,
            per_share_cents,
            record_date,
            payment_date,
            created_at: Utc::now(),
        }
    }
}
