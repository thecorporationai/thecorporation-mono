//! Governance seat record (stored as `governance/seats/{seat_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::GovernanceError;
use super::types::{SeatRole, SeatStatus, VotingPower};
use crate::domain::ids::{ContactId, GovernanceBodyId, GovernanceSeatId};

/// A seat in a governance body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceSeat {
    seat_id: GovernanceSeatId,
    body_id: GovernanceBodyId,
    holder_id: ContactId,
    role: SeatRole,
    appointed_date: Option<NaiveDate>,
    term_expiration: Option<NaiveDate>,
    voting_power: VotingPower,
    status: SeatStatus,
    created_at: DateTime<Utc>,
}

impl GovernanceSeat {
    /// Create a new governance seat. Defaults to `Active` status and voting power of 1.
    ///
    /// Returns `Err` if `appointed_date` is after `term_expiration` (when both are set).
    pub fn new(
        seat_id: GovernanceSeatId,
        body_id: GovernanceBodyId,
        holder_id: ContactId,
        role: SeatRole,
        appointed_date: Option<NaiveDate>,
        term_expiration: Option<NaiveDate>,
        voting_power: Option<VotingPower>,
    ) -> Result<Self, GovernanceError> {
        if let (Some(appointed), Some(expires)) = (appointed_date, term_expiration) {
            if appointed >= expires {
                return Err(GovernanceError::Validation(
                    "appointed_date must be before term_expiration".into(),
                ));
            }
        }
        Ok(Self {
            seat_id,
            body_id,
            holder_id,
            role,
            appointed_date,
            term_expiration,
            voting_power: voting_power
                .unwrap_or(VotingPower::new(1).expect("1 is valid voting power")),
            status: SeatStatus::Active,
            created_at: Utc::now(),
        })
    }

    /// Resign from this seat. Only valid if currently `Active`.
    pub fn resign(&mut self) -> Result<(), GovernanceError> {
        if self.status != SeatStatus::Active {
            return Err(GovernanceError::SeatNotActive(self.seat_id));
        }
        self.status = SeatStatus::Resigned;
        Ok(())
    }

    /// Expire this seat (term ended).
    pub fn expire(&mut self) {
        self.status = SeatStatus::Expired;
    }

    /// Whether this seat can vote (active and not an observer).
    pub fn can_vote(&self) -> bool {
        self.status == SeatStatus::Active && self.role != SeatRole::Observer
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn seat_id(&self) -> GovernanceSeatId {
        self.seat_id
    }

    pub fn body_id(&self) -> GovernanceBodyId {
        self.body_id
    }

    pub fn holder_id(&self) -> ContactId {
        self.holder_id
    }

    pub fn role(&self) -> SeatRole {
        self.role
    }

    pub fn appointed_date(&self) -> Option<NaiveDate> {
        self.appointed_date
    }

    pub fn term_expiration(&self) -> Option<NaiveDate> {
        self.term_expiration
    }

    pub fn voting_power(&self) -> VotingPower {
        self.voting_power
    }

    pub fn status(&self) -> SeatStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_seat(role: SeatRole) -> GovernanceSeat {
        GovernanceSeat::new(
            GovernanceSeatId::new(),
            GovernanceBodyId::new(),
            ContactId::new(),
            role,
            None,
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn new_seat_defaults_to_active() {
        let s = make_seat(SeatRole::Member);
        assert_eq!(s.status(), SeatStatus::Active);
        assert_eq!(s.voting_power(), VotingPower::new(1).unwrap());
    }

    #[test]
    fn active_member_can_vote() {
        let s = make_seat(SeatRole::Member);
        assert!(s.can_vote());
    }

    #[test]
    fn active_chair_can_vote() {
        let s = make_seat(SeatRole::Chair);
        assert!(s.can_vote());
    }

    #[test]
    fn observer_cannot_vote() {
        let s = make_seat(SeatRole::Observer);
        assert!(!s.can_vote());
    }

    #[test]
    fn resigned_cannot_vote() {
        let mut s = make_seat(SeatRole::Member);
        s.resign().unwrap();
        assert!(!s.can_vote());
        assert_eq!(s.status(), SeatStatus::Resigned);
    }

    #[test]
    fn expired_cannot_vote() {
        let mut s = make_seat(SeatRole::Member);
        s.expire();
        assert!(!s.can_vote());
        assert_eq!(s.status(), SeatStatus::Expired);
    }

    #[test]
    fn resign_only_from_active() {
        let mut s = make_seat(SeatRole::Member);
        s.expire();
        assert!(s.resign().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let s = make_seat(SeatRole::Officer);
        let json = serde_json::to_string(&s).unwrap();
        let parsed: GovernanceSeat = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.seat_id(), s.seat_id());
        assert_eq!(parsed.role(), s.role());
        assert_eq!(parsed.status(), s.status());
    }
}
