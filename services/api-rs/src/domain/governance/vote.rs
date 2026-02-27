//! Vote record (stored as `governance/meetings/{meeting_id}/votes/{vote_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::error::GovernanceError;
use super::types::{VoteValue, VotingPower};
use crate::domain::ids::{AgendaItemId, ContactId, GovernanceSeatId, MeetingId, VoteId};

/// A vote cast by a seat holder on an agenda item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    vote_id: VoteId,
    meeting_id: MeetingId,
    agenda_item_id: AgendaItemId,
    seat_id: GovernanceSeatId,
    voter_id: ContactId,
    vote_value: VoteValue,
    voting_power_applied: VotingPower,
    signature_hash: String,
    cast_at: DateTime<Utc>,
}

impl Vote {
    /// Create a new vote with an auto-computed signature hash.
    ///
    /// Returns `Err` if `voting_power_applied` is zero.
    pub fn new(
        vote_id: VoteId,
        meeting_id: MeetingId,
        agenda_item_id: AgendaItemId,
        seat_id: GovernanceSeatId,
        voter_id: ContactId,
        vote_value: VoteValue,
        voting_power_applied: VotingPower,
    ) -> Result<Self, GovernanceError> {
        if voting_power_applied.raw() == 0 {
            return Err(GovernanceError::Validation(
                "voting power must be greater than zero".into(),
            ));
        }
        let cast_at = Utc::now();
        let signature_hash =
            Self::compute_hash(vote_id, meeting_id, agenda_item_id, voter_id, vote_value, cast_at);
        Ok(Self {
            vote_id,
            meeting_id,
            agenda_item_id,
            seat_id,
            voter_id,
            vote_value,
            voting_power_applied,
            signature_hash,
            cast_at,
        })
    }

    fn compute_hash(
        vote_id: VoteId,
        meeting_id: MeetingId,
        agenda_item_id: AgendaItemId,
        voter_id: ContactId,
        vote_value: VoteValue,
        cast_at: DateTime<Utc>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(vote_id.to_string().as_bytes());
        hasher.update(meeting_id.to_string().as_bytes());
        hasher.update(agenda_item_id.to_string().as_bytes());
        hasher.update(voter_id.to_string().as_bytes());
        hasher.update(format!("{:?}", vote_value).as_bytes());
        hasher.update(cast_at.to_rfc3339().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn vote_id(&self) -> VoteId {
        self.vote_id
    }

    pub fn meeting_id(&self) -> MeetingId {
        self.meeting_id
    }

    pub fn agenda_item_id(&self) -> AgendaItemId {
        self.agenda_item_id
    }

    pub fn seat_id(&self) -> GovernanceSeatId {
        self.seat_id
    }

    pub fn voter_id(&self) -> ContactId {
        self.voter_id
    }

    pub fn vote_value(&self) -> VoteValue {
        self.vote_value
    }

    pub fn voting_power_applied(&self) -> VotingPower {
        self.voting_power_applied
    }

    pub fn signature_hash(&self) -> &str {
        &self.signature_hash
    }

    pub fn cast_at(&self) -> DateTime<Utc> {
        self.cast_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vote() -> Vote {
        Vote::new(
            VoteId::new(),
            MeetingId::new(),
            AgendaItemId::new(),
            GovernanceSeatId::new(),
            ContactId::new(),
            VoteValue::For,
            VotingPower::new(1),
        )
        .unwrap()
    }

    #[test]
    fn signature_hash_is_non_empty() {
        let v = make_vote();
        assert!(!v.signature_hash().is_empty());
        // SHA-256 hex is 64 characters
        assert_eq!(v.signature_hash().len(), 64);
    }

    #[test]
    fn different_votes_have_different_hashes() {
        let v1 = make_vote();
        let v2 = make_vote();
        // Different vote IDs => different hashes (with overwhelming probability)
        assert_ne!(v1.signature_hash(), v2.signature_hash());
    }

    #[test]
    fn serde_roundtrip() {
        let v = make_vote();
        let json = serde_json::to_string(&v).unwrap();
        let parsed: Vote = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.vote_id(), v.vote_id());
        assert_eq!(parsed.vote_value(), v.vote_value());
        assert_eq!(parsed.signature_hash(), v.signature_hash());
    }
}
