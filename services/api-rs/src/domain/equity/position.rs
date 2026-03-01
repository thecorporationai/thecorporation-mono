//! Canonical ownership positions (stored as `cap-table/positions/{position_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::EquityError;
use crate::domain::ids::{HolderId, InstrumentId, LegalEntityId, PositionId};

/// Lifecycle status for a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionStatus {
    Active,
    Closed,
}

/// Canonical current state of ownership for a holder and instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    position_id: PositionId,
    issuer_legal_entity_id: LegalEntityId,
    holder_id: HolderId,
    instrument_id: InstrumentId,
    quantity_units: i64,
    principal_cents: i64,
    source_reference: Option<String>,
    as_of_commit: Option<String>,
    formula_inputs_hash: Option<String>,
    status: PositionStatus,
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl Position {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        position_id: PositionId,
        issuer_legal_entity_id: LegalEntityId,
        holder_id: HolderId,
        instrument_id: InstrumentId,
        quantity_units: i64,
        principal_cents: i64,
        source_reference: Option<String>,
        as_of_commit: Option<String>,
        formula_inputs_hash: Option<String>,
    ) -> Result<Self, EquityError> {
        if quantity_units < 0 {
            return Err(EquityError::Validation(
                "position quantity cannot be negative".to_owned(),
            ));
        }
        if principal_cents < 0 {
            return Err(EquityError::Validation(
                "position principal cannot be negative".to_owned(),
            ));
        }
        let now = Utc::now();
        Ok(Self {
            position_id,
            issuer_legal_entity_id,
            holder_id,
            instrument_id,
            quantity_units,
            principal_cents,
            source_reference,
            as_of_commit,
            formula_inputs_hash,
            status: PositionStatus::Active,
            updated_at: now,
            created_at: now,
        })
    }

    pub fn apply_delta(
        &mut self,
        quantity_delta: i64,
        principal_delta: i64,
        source_reference: Option<String>,
        as_of_commit: Option<String>,
        formula_inputs_hash: Option<String>,
    ) -> Result<(), EquityError> {
        let new_qty = self
            .quantity_units
            .checked_add(quantity_delta)
            .ok_or_else(|| EquityError::Validation("position quantity overflow".to_owned()))?;
        let new_principal = self
            .principal_cents
            .checked_add(principal_delta)
            .ok_or_else(|| EquityError::Validation("position principal overflow".to_owned()))?;

        if new_qty < 0 || new_principal < 0 {
            return Err(EquityError::Validation(
                "position update would produce negative values".to_owned(),
            ));
        }

        self.quantity_units = new_qty;
        self.principal_cents = new_principal;
        self.source_reference = source_reference;
        self.as_of_commit = as_of_commit;
        self.formula_inputs_hash = formula_inputs_hash;
        self.status = if self.quantity_units == 0 && self.principal_cents == 0 {
            PositionStatus::Closed
        } else {
            PositionStatus::Active
        };
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn position_id(&self) -> PositionId {
        self.position_id
    }

    pub fn issuer_legal_entity_id(&self) -> LegalEntityId {
        self.issuer_legal_entity_id
    }

    pub fn holder_id(&self) -> HolderId {
        self.holder_id
    }

    pub fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    pub fn quantity_units(&self) -> i64 {
        self.quantity_units
    }

    pub fn principal_cents(&self) -> i64 {
        self.principal_cents
    }

    pub fn source_reference(&self) -> Option<&str> {
        self.source_reference.as_deref()
    }

    pub fn as_of_commit(&self) -> Option<&str> {
        self.as_of_commit.as_deref()
    }

    pub fn formula_inputs_hash(&self) -> Option<&str> {
        self.formula_inputs_hash.as_deref()
    }

    pub fn status(&self) -> PositionStatus {
        self.status
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
