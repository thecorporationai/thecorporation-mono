//! Funding round records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::FundingRoundStatus;
use crate::ids::{CapTableId, EntityId, FundingRoundId};

/// A fundraising round (e.g. Seed, Series A).
///
/// Status advances through the pipeline via [`advance_status`] and is
/// finalised with [`close`].
///
/// [`advance_status`]: FundingRound::advance_status
/// [`close`]: FundingRound::close
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundingRound {
    pub round_id: FundingRoundId,
    pub entity_id: EntityId,
    pub cap_table_id: CapTableId,
    /// Human-readable name, e.g. `"Series Seed"`.
    pub name: String,
    /// Fundraising target in whole cents.
    pub target_amount_cents: i64,
    /// Amount raised to date in whole cents.
    pub raised_amount_cents: i64,
    /// Price per share agreed for this round in whole cents.
    pub price_per_share_cents: Option<i64>,
    pub status: FundingRoundStatus,
    pub created_at: DateTime<Utc>,
}

impl FundingRound {
    /// Create a new funding round in `TermSheet` status.
    pub fn new(
        entity_id: EntityId,
        cap_table_id: CapTableId,
        name: impl Into<String>,
        target_amount_cents: i64,
        price_per_share_cents: Option<i64>,
    ) -> Self {
        Self {
            round_id: FundingRoundId::new(),
            entity_id,
            cap_table_id,
            name: name.into(),
            target_amount_cents,
            raised_amount_cents: 0,
            price_per_share_cents,
            status: FundingRoundStatus::TermSheet,
            created_at: Utc::now(),
        }
    }

    /// Advance through the pipeline in order:
    /// `TermSheet` → `Diligence` → `Closing`.
    ///
    /// Returns `Err` if the round is already `Closed` or if `advance_status`
    /// is called on `Closing` (use [`close`] instead).
    ///
    /// [`close`]: FundingRound::close
    pub fn advance_status(&mut self) -> Result<(), FundingRoundError> {
        match self.status {
            FundingRoundStatus::TermSheet => {
                self.status = FundingRoundStatus::Diligence;
                Ok(())
            }
            FundingRoundStatus::Diligence => {
                self.status = FundingRoundStatus::Closing;
                Ok(())
            }
            FundingRoundStatus::Closing => Err(FundingRoundError::UseCloseMethod),
            FundingRoundStatus::Closed => Err(FundingRoundError::AlreadyClosed),
        }
    }

    /// Finalise the round (`Closing` → `Closed`).
    pub fn close(&mut self) -> Result<(), FundingRoundError> {
        if self.status == FundingRoundStatus::Closing {
            self.status = FundingRoundStatus::Closed;
            Ok(())
        } else {
            Err(FundingRoundError::NotInClosing {
                current: self.status.clone(),
            })
        }
    }
}

/// Errors produced by [`FundingRound`] state transitions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FundingRoundError {
    #[error("round is already closed")]
    AlreadyClosed,
    #[error("round is in Closing status — call close() to finalise")]
    UseCloseMethod,
    #[error("round must be in Closing status to close, currently {current:?}")]
    NotInClosing { current: FundingRoundStatus },
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_round() -> FundingRound {
        FundingRound::new(
            EntityId::new(),
            CapTableId::new(),
            "Series Seed",
            2_000_000_00, // $2M target
            Some(1_00),   // $1.00 per share
        )
    }

    // ── FundingRound::new() ───────────────────────────────────────────────────

    #[test]
    fn new_round_status_is_term_sheet() {
        let r = make_round();
        assert_eq!(r.status, FundingRoundStatus::TermSheet);
    }

    #[test]
    fn new_round_stores_name() {
        let r = make_round();
        assert_eq!(r.name, "Series Seed");
    }

    #[test]
    fn new_round_stores_target_amount() {
        let r = make_round();
        assert_eq!(r.target_amount_cents, 2_000_000_00);
    }

    #[test]
    fn new_round_raised_amount_is_zero() {
        let r = make_round();
        assert_eq!(r.raised_amount_cents, 0);
    }

    #[test]
    fn new_round_stores_price_per_share() {
        let r = make_round();
        assert_eq!(r.price_per_share_cents, Some(1_00));
    }

    #[test]
    fn new_round_no_price_per_share() {
        let r = FundingRound::new(
            EntityId::new(),
            CapTableId::new(),
            "Seed",
            1_000_000_00,
            None,
        );
        assert!(r.price_per_share_cents.is_none());
    }

    #[test]
    fn new_round_has_unique_id() {
        let a = make_round();
        let b = make_round();
        assert_ne!(a.round_id, b.round_id);
    }

    // ── advance_status() ──────────────────────────────────────────────────────

    #[test]
    fn advance_status_term_sheet_to_diligence() {
        let mut r = make_round();
        r.advance_status().unwrap();
        assert_eq!(r.status, FundingRoundStatus::Diligence);
    }

    #[test]
    fn advance_status_diligence_to_closing() {
        let mut r = make_round();
        r.advance_status().unwrap();
        r.advance_status().unwrap();
        assert_eq!(r.status, FundingRoundStatus::Closing);
    }

    #[test]
    fn advance_status_from_closing_fails_use_close() {
        let mut r = make_round();
        r.advance_status().unwrap();
        r.advance_status().unwrap();
        assert_eq!(
            r.advance_status().unwrap_err(),
            FundingRoundError::UseCloseMethod
        );
    }

    #[test]
    fn advance_status_from_closed_fails() {
        let mut r = make_round();
        r.advance_status().unwrap();
        r.advance_status().unwrap();
        r.close().unwrap();
        assert_eq!(
            r.advance_status().unwrap_err(),
            FundingRoundError::AlreadyClosed
        );
    }

    // ── close() ───────────────────────────────────────────────────────────────

    #[test]
    fn close_from_closing_to_closed() {
        let mut r = make_round();
        r.advance_status().unwrap(); // -> Diligence
        r.advance_status().unwrap(); // -> Closing
        r.close().unwrap();
        assert_eq!(r.status, FundingRoundStatus::Closed);
    }

    #[test]
    fn close_from_term_sheet_fails() {
        let mut r = make_round();
        assert!(matches!(
            r.close(),
            Err(FundingRoundError::NotInClosing { .. })
        ));
    }

    #[test]
    fn close_from_diligence_fails() {
        let mut r = make_round();
        r.advance_status().unwrap();
        assert!(matches!(
            r.close(),
            Err(FundingRoundError::NotInClosing { .. })
        ));
    }

    #[test]
    fn close_from_closed_fails() {
        let mut r = make_round();
        r.advance_status().unwrap();
        r.advance_status().unwrap();
        r.close().unwrap();
        assert!(matches!(
            r.close(),
            Err(FundingRoundError::NotInClosing { .. })
        ));
    }

    // ── Full lifecycle ─────────────────────────────────────────────────────────

    #[test]
    fn full_lifecycle_term_sheet_to_closed() {
        let mut r = make_round();
        assert_eq!(r.status, FundingRoundStatus::TermSheet);
        r.advance_status().unwrap();
        assert_eq!(r.status, FundingRoundStatus::Diligence);
        r.advance_status().unwrap();
        assert_eq!(r.status, FundingRoundStatus::Closing);
        r.close().unwrap();
        assert_eq!(r.status, FundingRoundStatus::Closed);
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn round_serde_roundtrip() {
        let r = make_round();
        let json = serde_json::to_string(&r).unwrap();
        let de: FundingRound = serde_json::from_str(&json).unwrap();
        assert_eq!(de.round_id, r.round_id);
        assert_eq!(de.status, FundingRoundStatus::TermSheet);
    }

    #[test]
    fn round_serde_roundtrip_closed() {
        let mut r = make_round();
        r.advance_status().unwrap();
        r.advance_status().unwrap();
        r.close().unwrap();
        let json = serde_json::to_string(&r).unwrap();
        let de: FundingRound = serde_json::from_str(&json).unwrap();
        assert_eq!(de.status, FundingRoundStatus::Closed);
    }
}
