//! Conversion execution audit records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{ConversionExecutionId, EntityId, EquityRoundId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionExecution {
    conversion_execution_id: ConversionExecutionId,
    entity_id: EntityId,
    equity_round_id: EquityRoundId,
    summary: serde_json::Value,
    source_reference: Option<String>,
    created_at: DateTime<Utc>,
}

impl ConversionExecution {
    pub fn new(
        conversion_execution_id: ConversionExecutionId,
        entity_id: EntityId,
        equity_round_id: EquityRoundId,
        summary: serde_json::Value,
        source_reference: Option<String>,
    ) -> Self {
        Self {
            conversion_execution_id,
            entity_id,
            equity_round_id,
            summary,
            source_reference,
            created_at: Utc::now(),
        }
    }

    pub fn conversion_execution_id(&self) -> ConversionExecutionId {
        self.conversion_execution_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn equity_round_id(&self) -> EquityRoundId {
        self.equity_round_id
    }

    pub fn summary(&self) -> &serde_json::Value {
        &self.summary
    }

    pub fn source_reference(&self) -> Option<&str> {
        self.source_reference.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
