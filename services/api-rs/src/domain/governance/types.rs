//! Governance domain types — voting, meetings, and board structures.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── VotingPower ────────────────────────────────────────────────────────

/// The voting weight of a governance seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VotingPower(u32);

impl VotingPower {
    /// Create a new voting power value.
    #[inline]
    pub const fn new(power: u32) -> Self {
        Self(power)
    }

    /// Return the raw integer value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Create a validated voting power (rejects 0).
    #[inline]
    pub fn new_validated(power: u32) -> Result<Self, &'static str> {
        if power == 0 {
            Err("voting power must be greater than zero")
        } else {
            Ok(Self(power))
        }
    }
}

impl fmt::Display for VotingPower {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── QuorumThreshold ────────────────────────────────────────────────────

/// The threshold required for a vote to pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuorumThreshold {
    /// Simple majority (> 50%).
    Majority,
    /// Supermajority (>= 2/3).
    Supermajority,
    /// All eligible voters must vote in favor.
    Unanimous,
}

impl QuorumThreshold {
    /// Return the required fraction as (numerator, denominator).
    ///
    /// - `Majority` -> (1, 2) meaning more than 1/2
    /// - `Supermajority` -> (2, 3) meaning at least 2/3
    /// - `Unanimous` -> (1, 1) meaning all must agree
    pub const fn required_fraction(self) -> (u32, u32) {
        match self {
            Self::Majority => (1, 2),
            Self::Supermajority => (2, 3),
            Self::Unanimous => (1, 1),
        }
    }

    /// Check whether the quorum threshold is met given votes for and total eligible.
    ///
    /// For `Majority`: votes_for must be strictly greater than half of total.
    /// For `Supermajority`: votes_for * 3 >= total * 2.
    /// For `Unanimous`: votes_for must equal total.
    pub fn is_met(self, votes_for: u32, total_eligible: u32) -> bool {
        if total_eligible == 0 {
            return false;
        }
        match self {
            Self::Majority => {
                // Strictly more than half: votes_for * 2 > total_eligible
                votes_for * 2 > total_eligible
            }
            Self::Supermajority => {
                // At least 2/3: votes_for * 3 >= total_eligible * 2
                votes_for * 3 >= total_eligible * 2
            }
            Self::Unanimous => votes_for == total_eligible,
        }
    }
}

impl fmt::Display for QuorumThreshold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Majority => write!(f, "majority"),
            Self::Supermajority => write!(f, "supermajority"),
            Self::Unanimous => write!(f, "unanimous"),
        }
    }
}

// ── Enums ──────────────────────────────────────────────────────────────

/// The type of governance body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    /// Board of directors (C-Corp).
    BoardOfDirectors,
    /// LLC member vote.
    LlcMemberVote,
}

/// Whether a governance body is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyStatus {
    /// Body is active and can conduct business.
    Active,
    /// Body is inactive (dissolved or suspended).
    Inactive,
}

/// Role of a seat in a governance body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatRole {
    /// Chairperson of the body.
    Chair,
    /// Regular voting member.
    Member,
    /// Officer (e.g. secretary, treasurer).
    Officer,
    /// Non-voting observer.
    Observer,
}

/// Status of a governance seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatStatus {
    /// Seat is currently occupied.
    Active,
    /// Occupant has resigned.
    Resigned,
    /// Term has expired.
    Expired,
}

/// Type of meeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingType {
    /// Regular or special board meeting.
    BoardMeeting,
    /// Shareholder meeting (annual or special).
    ShareholderMeeting,
    /// Action by written consent (no physical meeting).
    WrittenConsent,
    /// LLC member meeting.
    MemberMeeting,
}

/// Lifecycle status of a meeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingStatus {
    /// Meeting is being planned.
    Draft,
    /// Notice has been sent to participants.
    Noticed,
    /// Meeting is in session.
    Convened,
    /// Meeting has been adjourned.
    Adjourned,
    /// Meeting was cancelled.
    Cancelled,
}

impl fmt::Display for MeetingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Noticed => write!(f, "noticed"),
            Self::Convened => write!(f, "convened"),
            Self::Adjourned => write!(f, "adjourned"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Type of item on a meeting agenda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgendaItemType {
    /// A formal resolution requiring a vote.
    Resolution,
    /// A discussion item (no vote needed).
    Discussion,
    /// An informational report.
    Report,
    /// An election of officers or directors.
    Election,
}

/// Status of an agenda item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgendaItemStatus {
    /// Not yet discussed.
    Pending,
    /// Has been discussed.
    Discussed,
    /// Vote has been taken.
    Voted,
    /// Deferred to a future meeting.
    Tabled,
    /// Withdrawn from the agenda.
    Withdrawn,
}

/// How a participant voted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteValue {
    /// In favor.
    For,
    /// Opposed.
    Against,
    /// Neither for nor against.
    Abstain,
    /// Recused due to conflict of interest.
    Recusal,
}

/// Type of resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionType {
    /// Ordinary resolution (simple majority).
    Ordinary,
    /// Special resolution (supermajority).
    Special,
    /// Unanimous written consent.
    UnanimousWrittenConsent,
}

/// How votes are counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VotingMethod {
    /// One vote per person/seat.
    PerCapita,
    /// Votes weighted by ownership units.
    PerUnit,
}

/// Status of meeting minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinutesStatus {
    /// Minutes are being drafted.
    Draft,
    /// Minutes have been approved.
    Approved,
    /// Minutes have been signed by the secretary.
    Signed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quorum_majority() {
        let q = QuorumThreshold::Majority;
        assert_eq!(q.required_fraction(), (1, 2));
        // 3 of 5 is majority
        assert!(q.is_met(3, 5));
        // 2 of 5 is not
        assert!(!q.is_met(2, 5));
        // 2 of 4 is not strictly more than half
        assert!(!q.is_met(2, 4));
        // 3 of 4 is
        assert!(q.is_met(3, 4));
    }

    #[test]
    fn quorum_supermajority() {
        let q = QuorumThreshold::Supermajority;
        assert_eq!(q.required_fraction(), (2, 3));
        // 2 of 3 meets 2/3
        assert!(q.is_met(2, 3));
        // 1 of 3 does not
        assert!(!q.is_met(1, 3));
        // 6 of 9 meets 2/3
        assert!(q.is_met(6, 9));
        // 5 of 9 does not
        assert!(!q.is_met(5, 9));
    }

    #[test]
    fn quorum_unanimous() {
        let q = QuorumThreshold::Unanimous;
        assert_eq!(q.required_fraction(), (1, 1));
        assert!(q.is_met(5, 5));
        assert!(!q.is_met(4, 5));
    }

    #[test]
    fn quorum_zero_total() {
        // Zero total eligible should never be met
        assert!(!QuorumThreshold::Majority.is_met(0, 0));
        assert!(!QuorumThreshold::Supermajority.is_met(0, 0));
        assert!(!QuorumThreshold::Unanimous.is_met(0, 0));
    }

    #[test]
    fn voting_power_display() {
        assert_eq!(VotingPower::new(42).to_string(), "42");
    }

    #[test]
    fn meeting_status_serde() {
        let status = MeetingStatus::Convened;
        let json = serde_json::to_string(&status).expect("serialize MeetingStatus");
        assert_eq!(json, "\"convened\"");
        let parsed: MeetingStatus =
            serde_json::from_str(&json).expect("deserialize MeetingStatus");
        assert_eq!(status, parsed);
    }

    #[test]
    fn vote_value_serde() {
        let v = VoteValue::Recusal;
        let json = serde_json::to_string(&v).expect("serialize VoteValue");
        assert_eq!(json, "\"recusal\"");
    }

    #[test]
    fn body_type_serde() {
        let bt = BodyType::BoardOfDirectors;
        let json = serde_json::to_string(&bt).expect("serialize BodyType");
        assert_eq!(json, "\"board_of_directors\"");
        let parsed: BodyType = serde_json::from_str(&json).expect("deserialize BodyType");
        assert_eq!(bt, parsed);
    }
}
