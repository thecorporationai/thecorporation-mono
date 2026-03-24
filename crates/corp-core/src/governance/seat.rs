//! Governance seat — an individual's membership in a governance body.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ContactId, GovernanceBodyId, GovernanceSeatId};

use super::types::{SeatRole, SeatStatus, VotingPower};

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors that can arise when constructing or mutating a [`GovernanceSeat`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GovernanceSeatError {
    #[error("seat is not in Active status")]
    NotActive,
}

// ── GovernanceSeat ────────────────────────────────────────────────────────────

/// A seat assigned to a contact within a [`GovernanceBody`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceSeat {
    pub seat_id: GovernanceSeatId,
    pub body_id: GovernanceBodyId,
    /// The contact who holds this seat.
    pub holder_id: ContactId,
    pub role: SeatRole,
    pub appointed_date: NaiveDate,
    /// `None` means the seat has no fixed term.
    pub term_expiration: Option<NaiveDate>,
    pub voting_power: VotingPower,
    pub status: SeatStatus,
    pub created_at: DateTime<Utc>,
}

impl GovernanceSeat {
    /// Create a new active governance seat.
    pub fn new(
        body_id: GovernanceBodyId,
        holder_id: ContactId,
        role: SeatRole,
        appointed_date: NaiveDate,
        term_expiration: Option<NaiveDate>,
        voting_power: VotingPower,
    ) -> Self {
        Self {
            seat_id: GovernanceSeatId::new(),
            body_id,
            holder_id,
            role,
            appointed_date,
            term_expiration,
            voting_power,
            status: SeatStatus::Active,
            created_at: Utc::now(),
        }
    }

    /// Mark the seat as resigned. Returns `Err` if the seat is not currently active.
    pub fn resign(&mut self) -> Result<(), GovernanceSeatError> {
        if self.status != SeatStatus::Active {
            return Err(GovernanceSeatError::NotActive);
        }
        self.status = SeatStatus::Resigned;
        Ok(())
    }

    /// Mark the seat as expired (term lapsed). Returns `Err` if the seat is not currently active.
    pub fn expire(&mut self) -> Result<(), GovernanceSeatError> {
        if self.status != SeatStatus::Active {
            return Err(GovernanceSeatError::NotActive);
        }
        self.status = SeatStatus::Expired;
        Ok(())
    }

    /// Returns `true` if the seat holder may cast a vote.
    ///
    /// Observers never vote regardless of status.
    pub fn can_vote(&self) -> bool {
        self.status == SeatStatus::Active && self.role != SeatRole::Observer
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::types::VotingPower;

    fn make_seat(role: SeatRole) -> GovernanceSeat {
        GovernanceSeat::new(
            GovernanceBodyId::new(),
            ContactId::new(),
            role,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            VotingPower::new(1).unwrap(),
        )
    }

    fn make_seat_with_expiry(role: SeatRole, expiry: Option<NaiveDate>) -> GovernanceSeat {
        GovernanceSeat::new(
            GovernanceBodyId::new(),
            ContactId::new(),
            role,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            expiry,
            VotingPower::new(1).unwrap(),
        )
    }

    // ── new() creates Active seats ────────────────────────────────────────────

    #[test]
    fn new_seat_is_active() {
        let seat = make_seat(SeatRole::Member);
        assert_eq!(seat.status, SeatStatus::Active);
    }

    #[test]
    fn new_seat_chair_role() {
        let seat = make_seat(SeatRole::Chair);
        assert_eq!(seat.role, SeatRole::Chair);
        assert_eq!(seat.status, SeatStatus::Active);
    }

    #[test]
    fn new_seat_officer_role() {
        let seat = make_seat(SeatRole::Officer);
        assert_eq!(seat.role, SeatRole::Officer);
    }

    #[test]
    fn new_seat_observer_role() {
        let seat = make_seat(SeatRole::Observer);
        assert_eq!(seat.role, SeatRole::Observer);
    }

    #[test]
    fn new_seat_has_unique_id() {
        let s1 = make_seat(SeatRole::Member);
        let s2 = make_seat(SeatRole::Member);
        assert_ne!(s1.seat_id, s2.seat_id);
    }

    // ── term_expiration ───────────────────────────────────────────────────────

    #[test]
    fn seat_without_term_expiration() {
        let seat = make_seat_with_expiry(SeatRole::Member, None);
        assert!(seat.term_expiration.is_none());
    }

    #[test]
    fn seat_with_term_expiration() {
        let expiry = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let seat = make_seat_with_expiry(SeatRole::Member, Some(expiry));
        assert_eq!(seat.term_expiration, Some(expiry));
    }

    #[test]
    fn seat_same_day_appointment_and_expiry() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let seat = GovernanceSeat::new(
            GovernanceBodyId::new(),
            ContactId::new(),
            SeatRole::Member,
            date,
            Some(date),
            VotingPower::new(1).unwrap(),
        );
        assert_eq!(seat.appointed_date, seat.term_expiration.unwrap());
    }

    // ── can_vote(): Active seats ──────────────────────────────────────────────

    #[test]
    fn active_member_can_vote() {
        assert!(make_seat(SeatRole::Member).can_vote());
        assert!(make_seat(SeatRole::Chair).can_vote());
        assert!(make_seat(SeatRole::Officer).can_vote());
    }

    #[test]
    fn active_chair_can_vote() {
        assert!(make_seat(SeatRole::Chair).can_vote());
    }

    #[test]
    fn active_officer_can_vote() {
        assert!(make_seat(SeatRole::Officer).can_vote());
    }

    #[test]
    fn observer_cannot_vote() {
        assert!(!make_seat(SeatRole::Observer).can_vote());
    }

    // ── can_vote(): Resigned seats ────────────────────────────────────────────

    #[test]
    fn resigned_member_cannot_vote() {
        let mut seat = make_seat(SeatRole::Member);
        seat.resign().unwrap();
        assert!(!seat.can_vote());
    }

    #[test]
    fn resigned_chair_cannot_vote() {
        let mut seat = make_seat(SeatRole::Chair);
        seat.resign().unwrap();
        assert!(!seat.can_vote());
    }

    #[test]
    fn resigned_officer_cannot_vote() {
        let mut seat = make_seat(SeatRole::Officer);
        seat.resign().unwrap();
        assert!(!seat.can_vote());
    }

    #[test]
    fn resigned_observer_cannot_vote() {
        let mut seat = make_seat(SeatRole::Observer);
        seat.resign().unwrap();
        assert!(!seat.can_vote());
    }

    // ── can_vote(): Expired seats ─────────────────────────────────────────────

    #[test]
    fn expired_member_cannot_vote() {
        let mut seat = make_seat(SeatRole::Member);
        seat.expire().unwrap();
        assert!(!seat.can_vote());
    }

    #[test]
    fn expired_chair_cannot_vote() {
        let mut seat = make_seat(SeatRole::Chair);
        seat.expire().unwrap();
        assert!(!seat.can_vote());
    }

    #[test]
    fn expired_officer_cannot_vote() {
        let mut seat = make_seat(SeatRole::Officer);
        seat.expire().unwrap();
        assert!(!seat.can_vote());
    }

    // ── resign() ─────────────────────────────────────────────────────────────

    #[test]
    fn resign_active_seat_succeeds() {
        let mut seat = make_seat(SeatRole::Member);
        assert!(seat.resign().is_ok());
        assert_eq!(seat.status, SeatStatus::Resigned);
    }

    #[test]
    fn resign_already_resigned_errors() {
        let mut seat = make_seat(SeatRole::Member);
        seat.resign().unwrap();
        assert_eq!(seat.resign().unwrap_err(), GovernanceSeatError::NotActive);
    }

    #[test]
    fn resign_expired_seat_errors() {
        let mut seat = make_seat(SeatRole::Member);
        seat.expire().unwrap();
        assert_eq!(seat.resign().unwrap_err(), GovernanceSeatError::NotActive);
    }

    // ── expire() ─────────────────────────────────────────────────────────────

    #[test]
    fn expire_active_seat() {
        let mut seat = make_seat(SeatRole::Member);
        seat.expire().unwrap();
        assert_eq!(seat.status, SeatStatus::Expired);
    }

    #[test]
    fn expire_already_expired_errors() {
        let mut seat = make_seat(SeatRole::Member);
        seat.expire().unwrap();
        assert_eq!(seat.expire().unwrap_err(), GovernanceSeatError::NotActive);
    }

    #[test]
    fn expire_resigned_seat_errors() {
        let mut seat = make_seat(SeatRole::Member);
        seat.resign().unwrap();
        assert_eq!(seat.expire().unwrap_err(), GovernanceSeatError::NotActive);
    }

    // ── voting_power ──────────────────────────────────────────────────────────

    #[test]
    fn seat_voting_power_stored() {
        let seat = GovernanceSeat::new(
            GovernanceBodyId::new(),
            ContactId::new(),
            SeatRole::Member,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            VotingPower::new(10).unwrap(),
        );
        assert_eq!(seat.voting_power.value(), 10);
    }

    #[test]
    fn seat_serde_roundtrip() {
        let seat = make_seat(SeatRole::Chair);
        let json = serde_json::to_string(&seat).unwrap();
        let back: GovernanceSeat = serde_json::from_str(&json).unwrap();
        assert_eq!(seat.seat_id, back.seat_id);
        assert_eq!(seat.role, back.role);
        assert_eq!(seat.status, back.status);
    }

    #[test]
    fn seat_not_active_error_display() {
        let err = GovernanceSeatError::NotActive;
        let msg = format!("{err}");
        assert!(msg.contains("Active"));
    }
}
