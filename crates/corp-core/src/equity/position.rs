//! Equity positions: a holder's stake in a specific instrument.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{HolderId, InstrumentId, LegalEntityId, PositionId};
use super::types::PositionStatus;
use super::vesting::EquityError;

// ── Position ──────────────────────────────────────────────────────────────────

/// A holder's current stake in a single [`Instrument`].
///
/// `quantity_units` is the number of units (shares, notes, etc.) held.
/// `principal_cents` is the total cost basis in whole cents.
///
/// Both start non-negative. `apply_delta` enforces this invariant and
/// auto-closes the position when `quantity_units` reaches zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub position_id: PositionId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub quantity_units: i64,
    pub principal_cents: i64,
    /// Optional reference to the originating transaction or document.
    pub source_reference: Option<String>,
    pub status: PositionStatus,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Position {
    /// Create a new position. Returns `Err` if `quantity_units` or
    /// `principal_cents` is negative.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        issuer_legal_entity_id: LegalEntityId,
        holder_id: HolderId,
        instrument_id: InstrumentId,
        quantity_units: i64,
        principal_cents: i64,
        source_reference: Option<String>,
    ) -> Result<Self, EquityError> {
        if quantity_units < 0 {
            return Err(EquityError::NegativeQuantity);
        }
        if principal_cents < 0 {
            return Err(EquityError::NegativePrincipal);
        }
        let now = Utc::now();
        Ok(Self {
            position_id: PositionId::new(),
            issuer_legal_entity_id,
            holder_id,
            instrument_id,
            quantity_units,
            principal_cents,
            source_reference,
            status: PositionStatus::Active,
            updated_at: now,
            created_at: now,
        })
    }

    /// Apply a signed delta to both `quantity_units` and `principal_cents`.
    ///
    /// Rules:
    /// - Checked arithmetic — returns [`EquityError::QuantityOverflow`] on
    ///   overflow.
    /// - The resulting `quantity_units` must not be negative.
    /// - If `quantity_units` reaches zero, the position is automatically
    ///   closed.
    pub fn apply_delta(
        &mut self,
        qty_delta: i64,
        principal_delta: i64,
        source: Option<String>,
    ) -> Result<(), EquityError> {
        let new_qty = self
            .quantity_units
            .checked_add(qty_delta)
            .ok_or(EquityError::QuantityOverflow)?;

        if new_qty < 0 {
            return Err(EquityError::NegativeQuantity);
        }

        let new_principal = self
            .principal_cents
            .checked_add(principal_delta)
            .ok_or(EquityError::QuantityOverflow)?;

        self.quantity_units = new_qty;
        self.principal_cents = new_principal;
        self.source_reference = source;
        self.updated_at = Utc::now();

        if self.quantity_units == 0 {
            self.status = PositionStatus::Closed;
        }

        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_position(qty: i64, principal: i64) -> Position {
        Position::new(
            LegalEntityId::new(),
            HolderId::new(),
            InstrumentId::new(),
            qty,
            principal,
            None,
        )
        .unwrap()
    }

    // ── Position::new() ───────────────────────────────────────────────────────

    #[test]
    fn new_position_status_is_active() {
        let p = make_position(1000, 100_000);
        assert_eq!(p.status, PositionStatus::Active);
    }

    #[test]
    fn new_position_stores_quantity() {
        let p = make_position(500, 50_000);
        assert_eq!(p.quantity_units, 500);
    }

    #[test]
    fn new_position_stores_principal() {
        let p = make_position(500, 50_000);
        assert_eq!(p.principal_cents, 50_000);
    }

    #[test]
    fn new_position_zero_quantity_is_valid() {
        let p = make_position(0, 0);
        assert_eq!(p.quantity_units, 0);
    }

    #[test]
    fn new_position_negative_quantity_fails() {
        let err = Position::new(
            LegalEntityId::new(),
            HolderId::new(),
            InstrumentId::new(),
            -1,
            0,
            None,
        )
        .unwrap_err();
        assert_eq!(err, EquityError::NegativeQuantity);
    }

    #[test]
    fn new_position_negative_principal_fails() {
        let err = Position::new(
            LegalEntityId::new(),
            HolderId::new(),
            InstrumentId::new(),
            100,
            -1,
            None,
        )
        .unwrap_err();
        assert_eq!(err, EquityError::NegativePrincipal);
    }

    // ── apply_delta() ─────────────────────────────────────────────────────────

    #[test]
    fn apply_delta_increases_quantity() {
        let mut p = make_position(100, 10_000);
        p.apply_delta(50, 5_000, None).unwrap();
        assert_eq!(p.quantity_units, 150);
        assert_eq!(p.principal_cents, 15_000);
    }

    #[test]
    fn apply_delta_decreases_quantity() {
        let mut p = make_position(100, 10_000);
        p.apply_delta(-30, -3_000, None).unwrap();
        assert_eq!(p.quantity_units, 70);
        assert_eq!(p.principal_cents, 7_000);
    }

    #[test]
    fn apply_delta_to_zero_closes_position() {
        let mut p = make_position(100, 10_000);
        p.apply_delta(-100, -10_000, None).unwrap();
        assert_eq!(p.quantity_units, 0);
        assert_eq!(p.status, PositionStatus::Closed);
    }

    #[test]
    fn apply_delta_below_zero_fails() {
        let mut p = make_position(50, 5_000);
        let err = p.apply_delta(-100, 0, None).unwrap_err();
        assert_eq!(err, EquityError::NegativeQuantity);
    }

    #[test]
    fn apply_delta_qty_overflow_fails() {
        let mut p = make_position(i64::MAX, 0);
        let err = p.apply_delta(1, 0, None).unwrap_err();
        assert_eq!(err, EquityError::QuantityOverflow);
    }

    #[test]
    fn apply_delta_principal_overflow_fails() {
        let mut p = make_position(100, i64::MAX);
        let err = p.apply_delta(0, 1, None).unwrap_err();
        assert_eq!(err, EquityError::QuantityOverflow);
    }

    #[test]
    fn apply_delta_updates_source_reference() {
        let mut p = make_position(100, 0);
        p.apply_delta(0, 0, Some("tx-001".to_string())).unwrap();
        assert_eq!(p.source_reference, Some("tx-001".to_string()));
    }

    #[test]
    fn position_has_unique_id() {
        let a = make_position(100, 0);
        let b = make_position(100, 0);
        assert_ne!(a.position_id, b.position_id);
    }

    // ── serde roundtrip ───────────────────────────────────────────────────────

    #[test]
    fn position_serde_roundtrip() {
        let p = make_position(1_000, 100_000);
        let json = serde_json::to_string(&p).unwrap();
        let de: Position = serde_json::from_str(&json).unwrap();
        assert_eq!(de.position_id, p.position_id);
        assert_eq!(de.quantity_units, 1_000);
        assert_eq!(de.status, PositionStatus::Active);
    }
}
