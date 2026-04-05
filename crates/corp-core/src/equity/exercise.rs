//! Option exercise records: audit trail for exercising stock options.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::ShareCount;
use crate::ids::{EntityId, EquityGrantId, HolderId, OptionExerciseId};

/// How the exercise was classified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExerciseType {
    /// All vested shares exercised.
    Full,
    /// A subset of vested shares exercised.
    Partial,
    /// Exercise before full vesting (requires `early_exercise_allowed`).
    Early,
}

/// An immutable record of an option exercise event.
///
/// Created by the `exercise_option` handler. Links a grant to a position via
/// the holder, recording the shares exercised and the total cost at the
/// strike price.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionExercise {
    pub exercise_id: OptionExerciseId,
    pub entity_id: EntityId,
    pub grant_id: EquityGrantId,
    pub holder_id: HolderId,
    pub shares_exercised: ShareCount,
    /// Strike price copied from the grant at exercise time (cents).
    pub strike_price_cents: i64,
    /// `shares_exercised × strike_price_cents`.
    pub total_cost_cents: i64,
    pub exercise_date: NaiveDate,
    pub exercise_type: ExerciseType,
    pub created_at: DateTime<Utc>,
}

impl OptionExercise {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        grant_id: EquityGrantId,
        holder_id: HolderId,
        shares_exercised: ShareCount,
        strike_price_cents: i64,
        exercise_date: NaiveDate,
        exercise_type: ExerciseType,
    ) -> Self {
        let total_cost_cents = shares_exercised.raw() * strike_price_cents;
        Self {
            exercise_id: OptionExerciseId::new(),
            entity_id,
            grant_id,
            holder_id,
            shares_exercised,
            strike_price_cents,
            total_cost_cents,
            exercise_date,
            exercise_type,
            created_at: Utc::now(),
        }
    }
}

// ── Tests ───────────���─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_exercise() -> OptionExercise {
        OptionExercise::new(
            EntityId::new(),
            EquityGrantId::new(),
            HolderId::new(),
            ShareCount::new(50_000),
            100, // $1.00 per share
            NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
            ExerciseType::Full,
        )
    }

    #[test]
    fn total_cost_calculated() {
        let ex = make_exercise();
        assert_eq!(ex.total_cost_cents, 50_000 * 100);
    }

    #[test]
    fn exercise_has_unique_id() {
        let a = make_exercise();
        let b = make_exercise();
        assert_ne!(a.exercise_id, b.exercise_id);
    }

    #[test]
    fn serde_roundtrip() {
        let ex = make_exercise();
        let json = serde_json::to_string(&ex).unwrap();
        let de: OptionExercise = serde_json::from_str(&json).unwrap();
        assert_eq!(de.exercise_id, ex.exercise_id);
        assert_eq!(de.shares_exercised, ex.shares_exercised);
        assert_eq!(de.exercise_type, ExerciseType::Full);
    }

    #[test]
    fn exercise_type_serde() {
        let json = serde_json::to_string(&ExerciseType::Early).unwrap();
        assert_eq!(json, r#""early""#);
        let de: ExerciseType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, ExerciseType::Early);
    }
}
