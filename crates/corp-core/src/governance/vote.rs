//! Vote cast by a seat holder on an agenda item.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{AgendaItemId, GovernanceSeatId, MeetingId, VoteId};

use super::types::VoteValue;

// ── Vote ──────────────────────────────────────────────────────────────────────

/// A single vote cast by a seat holder against a specific agenda item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub vote_id: VoteId,
    pub meeting_id: MeetingId,
    pub agenda_item_id: AgendaItemId,
    /// The governance seat that cast this vote.
    pub seat_id: GovernanceSeatId,
    pub value: VoteValue,
    pub cast_at: DateTime<Utc>,
}

impl Vote {
    /// Record a new vote, stamping the current UTC time.
    pub fn new(
        meeting_id: MeetingId,
        agenda_item_id: AgendaItemId,
        seat_id: GovernanceSeatId,
        value: VoteValue,
    ) -> Self {
        Self {
            vote_id: VoteId::new(),
            meeting_id,
            agenda_item_id,
            seat_id,
            value,
            cast_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vote(value: VoteValue) -> Vote {
        Vote::new(
            MeetingId::new(),
            AgendaItemId::new(),
            GovernanceSeatId::new(),
            value,
        )
    }

    #[test]
    fn new_vote_roundtrip() {
        let seat = GovernanceSeatId::new();
        let meeting = MeetingId::new();
        let item = AgendaItemId::new();
        let vote = Vote::new(meeting, item, seat, VoteValue::For);
        assert_eq!(vote.value, VoteValue::For);
        assert_eq!(vote.seat_id, seat);
    }

    #[test]
    fn new_vote_stores_meeting_id() {
        let meeting = MeetingId::new();
        let item = AgendaItemId::new();
        let vote = Vote::new(meeting, item, GovernanceSeatId::new(), VoteValue::Against);
        assert_eq!(vote.meeting_id, meeting);
    }

    #[test]
    fn new_vote_stores_agenda_item_id() {
        let meeting = MeetingId::new();
        let item = AgendaItemId::new();
        let vote = Vote::new(meeting, item, GovernanceSeatId::new(), VoteValue::Abstain);
        assert_eq!(vote.agenda_item_id, item);
    }

    #[test]
    fn vote_for_value() {
        let vote = make_vote(VoteValue::For);
        assert_eq!(vote.value, VoteValue::For);
    }

    #[test]
    fn vote_against_value() {
        let vote = make_vote(VoteValue::Against);
        assert_eq!(vote.value, VoteValue::Against);
    }

    #[test]
    fn vote_abstain_value() {
        let vote = make_vote(VoteValue::Abstain);
        assert_eq!(vote.value, VoteValue::Abstain);
    }

    #[test]
    fn vote_recusal_value() {
        let vote = make_vote(VoteValue::Recusal);
        assert_eq!(vote.value, VoteValue::Recusal);
    }

    #[test]
    fn each_vote_has_unique_id() {
        let v1 = make_vote(VoteValue::For);
        let v2 = make_vote(VoteValue::For);
        assert_ne!(v1.vote_id, v2.vote_id);
    }

    #[test]
    fn vote_cast_at_is_recent() {
        let before = chrono::Utc::now();
        let vote = make_vote(VoteValue::For);
        let after = chrono::Utc::now();
        assert!(vote.cast_at >= before);
        assert!(vote.cast_at <= after);
    }

    #[test]
    fn vote_serde_roundtrip() {
        let vote = make_vote(VoteValue::Against);
        let json = serde_json::to_string(&vote).unwrap();
        let back: Vote = serde_json::from_str(&json).unwrap();
        assert_eq!(vote.vote_id, back.vote_id);
        assert_eq!(vote.value, back.value);
    }
}
