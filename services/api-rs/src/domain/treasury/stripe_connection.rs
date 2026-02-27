//! Stripe connection record (stored as `treasury/stripe-connection.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{EntityId, StripeConnectionId};

/// A Stripe account connection for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConnection {
    connection_id: StripeConnectionId,
    entity_id: EntityId,
    stripe_account_id: String,
    status: String,
    created_at: DateTime<Utc>,
}

impl StripeConnection {
    pub fn new(
        connection_id: StripeConnectionId,
        entity_id: EntityId,
        stripe_account_id: String,
    ) -> Self {
        Self {
            connection_id,
            entity_id,
            stripe_account_id,
            status: "active".to_owned(),
            created_at: Utc::now(),
        }
    }

    pub fn connection_id(&self) -> StripeConnectionId { self.connection_id }
    pub fn entity_id(&self) -> EntityId { self.entity_id }
    pub fn stripe_account_id(&self) -> &str { &self.stripe_account_id }
    pub fn status(&self) -> &str { &self.status }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
}
