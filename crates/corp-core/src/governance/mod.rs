//! Corporate governance domain — bodies, seats, meetings, votes, resolutions,
//! capabilities, policy decisions, proof obligations, and entity profiles.

pub mod agenda_item;
pub mod body;
pub mod capability;
pub mod meeting;
pub mod policy;
pub mod profile;
pub mod proof;
pub mod resolution;
pub mod seat;
pub mod types;
pub mod vote;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use agenda_item::AgendaItem;
pub use body::GovernanceBody;
pub use capability::{AuthorityTier, GovernanceCapability, default_tier};
pub use meeting::{Meeting, QuorumStatus};
pub use policy::PolicyDecision;
pub use profile::{
    CompanyAddress, DirectorInfo, FiscalYearEnd, FounderInfo, GovernanceProfile, OfficerInfo,
    StockDetails,
};
pub use proof::{ProofReport, ProofViolation, verify_decision};
pub use resolution::{Resolution, compute_resolution};
pub use seat::GovernanceSeat;
pub use types::{
    AgendaItemType, BodyStatus, BodyType, MinutesStatus, MeetingStatus, MeetingType,
    QuorumThreshold, ResolutionType, SeatRole, SeatStatus, VoteValue, VotingMethod, VotingPower,
    check_quorum,
};
pub use vote::Vote;
