//! Governance domain errors.

use super::types::{MeetingStatus, QuorumThreshold};
use crate::domain::ids::{GovernanceBodyId, GovernanceSeatId, MeetingId, ResolutionId};
use thiserror::Error;

/// Errors that can occur in the governance domain.
#[derive(Debug, Error)]
pub enum GovernanceError {
    /// The requested governance body does not exist.
    #[error("governance body {0} not found")]
    BodyNotFound(GovernanceBodyId),

    /// The requested governance seat does not exist.
    #[error("governance seat {0} not found")]
    SeatNotFound(GovernanceSeatId),

    /// The requested meeting does not exist.
    #[error("meeting {0} not found")]
    MeetingNotFound(MeetingId),

    /// The meeting cannot transition between the given states.
    #[error("invalid meeting transition from {from} to {to}")]
    InvalidMeetingTransition {
        from: MeetingStatus,
        to: MeetingStatus,
    },

    /// A vote did not meet the required quorum.
    #[error("quorum not met: required={required}, present={present}, total={total}")]
    QuorumNotMet {
        required: QuorumThreshold,
        present: u32,
        total: u32,
    },

    /// A voter has already cast a vote on this item.
    #[error("duplicate vote from voter {voter_id}")]
    DuplicateVote { voter_id: String },

    /// Attempted to vote when the voting session is not open.
    #[error("voting session is not open")]
    VotingSessionNotOpen,

    /// Attempted to open voting when a session is already closed.
    #[error("voting session is already closed")]
    VotingSessionAlreadyClosed,

    /// A resolution with this ID already exists.
    #[error("resolution {0} already exists")]
    ResolutionAlreadyExists(ResolutionId),

    /// The seat is already occupied.
    #[error("governance seat {0} is already filled")]
    SeatAlreadyFilled(GovernanceSeatId),

    /// Observers do not have voting rights.
    #[error("observers cannot vote")]
    CannotVoteAsObserver,

    /// General validation error.
    #[error("governance validation error: {0}")]
    Validation(String),
}
