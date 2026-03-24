//! Core governance enumerations and value types.

use serde::{Deserialize, Serialize};

// ── QuorumThreshold ───────────────────────────────────────────────────────────

/// The minimum fraction of eligible votes required to pass a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuorumThreshold {
    /// More than 50% of eligible votes.
    Majority,
    /// At least two-thirds (≥ 2/3) of eligible votes.
    Supermajority,
    /// All eligible votes must be cast in favour.
    Unanimous,
}

// ── BodyType ─────────────────────────────────────────────────────────────────

/// The legal character of a governance body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    BoardOfDirectors,
    LlcMemberVote,
}

// ── BodyStatus ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyStatus {
    Active,
    Inactive,
}

// ── SeatRole ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatRole {
    Chair,
    Member,
    Officer,
    /// Observer seats do not carry voting rights.
    Observer,
}

// ── SeatStatus ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatStatus {
    Active,
    Resigned,
    Expired,
}

// ── MeetingType ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingType {
    BoardMeeting,
    ShareholderMeeting,
    WrittenConsent,
    MemberMeeting,
}

// ── MeetingStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingStatus {
    Draft,
    Noticed,
    Convened,
    Adjourned,
    Cancelled,
}

// ── AgendaItemType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgendaItemType {
    Resolution,
    Discussion,
    Report,
    Election,
}

// ── VoteValue ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteValue {
    For,
    Against,
    Abstain,
    Recusal,
}

// ── ResolutionType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionType {
    Ordinary,
    Special,
    UnanimousWrittenConsent,
}

// ── VotingMethod ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VotingMethod {
    /// One vote per seat regardless of share count.
    PerCapita,
    /// Votes weighted by the `voting_power` carried by each seat.
    PerUnit,
}

// ── MinutesStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinutesStatus {
    Draft,
    Approved,
    Signed,
}

// ── VotingPower ───────────────────────────────────────────────────────────────

/// A strictly-positive unit of voting weight carried by a governance seat.
///
/// ```
/// use corp_core::governance::types::VotingPower;
///
/// let vp = VotingPower::new(100).unwrap();
/// assert_eq!(vp.value(), 100);
/// assert!(VotingPower::new(0).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VotingPower(u32);

/// Error returned when constructing a [`VotingPower`] from a zero value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VotingPowerError {
    #[error("voting power must be greater than zero")]
    Zero,
}

impl VotingPower {
    /// Construct a `VotingPower`. Returns `Err` if `n == 0`.
    pub fn new(n: u32) -> Result<Self, VotingPowerError> {
        if n == 0 {
            Err(VotingPowerError::Zero)
        } else {
            Ok(Self(n))
        }
    }

    /// Return the underlying value.
    #[inline]
    pub fn value(self) -> u32 {
        self.0
    }
}

// ── Quorum check ─────────────────────────────────────────────────────────────

/// Returns `true` if `votes_for` satisfies `threshold` out of `total_eligible`.
///
/// # Panics
///
/// Does not panic; all arithmetic uses overflow-safe comparisons.
pub fn check_quorum(threshold: QuorumThreshold, votes_for: u32, total_eligible: u32) -> bool {
    match threshold {
        QuorumThreshold::Majority => {
            // votes_for > total_eligible / 2  ⟺  votes_for * 2 > total_eligible
            votes_for.saturating_mul(2) > total_eligible
        }
        QuorumThreshold::Supermajority => {
            // votes_for >= 2/3 * total_eligible  ⟺  votes_for * 3 >= total_eligible * 2
            votes_for.saturating_mul(3) >= total_eligible.saturating_mul(2)
        }
        QuorumThreshold::Unanimous => votes_for == total_eligible,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── VotingPower ───────────────────────────────────────────────────────────

    #[test]
    fn voting_power_nonzero() {
        assert!(VotingPower::new(1).is_ok());
        assert!(VotingPower::new(u32::MAX).is_ok());
        assert_eq!(VotingPower::new(0).unwrap_err(), VotingPowerError::Zero);
    }

    #[test]
    fn voting_power_zero_is_err() {
        let err = VotingPower::new(0).unwrap_err();
        assert_eq!(err, VotingPowerError::Zero);
    }

    #[test]
    fn voting_power_value_accessor() {
        assert_eq!(VotingPower::new(1).unwrap().value(), 1);
        assert_eq!(VotingPower::new(42).unwrap().value(), 42);
        assert_eq!(VotingPower::new(100).unwrap().value(), 100);
        assert_eq!(VotingPower::new(u32::MAX).unwrap().value(), u32::MAX);
    }

    #[test]
    fn voting_power_ordering() {
        let low = VotingPower::new(1).unwrap();
        let high = VotingPower::new(1000).unwrap();
        assert!(low < high);
        assert!(high > low);
        assert_eq!(low, VotingPower::new(1).unwrap());
    }

    #[test]
    fn voting_power_serde_roundtrip() {
        let vp = VotingPower::new(99).unwrap();
        let json = serde_json::to_string(&vp).unwrap();
        let back: VotingPower = serde_json::from_str(&json).unwrap();
        assert_eq!(vp, back);
    }

    #[test]
    fn voting_power_error_display() {
        let err = VotingPowerError::Zero;
        let msg = format!("{err}");
        assert!(msg.contains("zero"));
    }

    // ── QuorumThreshold serde ─────────────────────────────────────────────────

    #[test]
    fn quorum_threshold_serde_majority() {
        let json = serde_json::to_string(&QuorumThreshold::Majority).unwrap();
        assert_eq!(json, r#""majority""#);
        let back: QuorumThreshold = serde_json::from_str(&json).unwrap();
        assert_eq!(back, QuorumThreshold::Majority);
    }

    #[test]
    fn quorum_threshold_serde_supermajority() {
        let json = serde_json::to_string(&QuorumThreshold::Supermajority).unwrap();
        assert_eq!(json, r#""supermajority""#);
        let back: QuorumThreshold = serde_json::from_str(&json).unwrap();
        assert_eq!(back, QuorumThreshold::Supermajority);
    }

    #[test]
    fn quorum_threshold_serde_unanimous() {
        let json = serde_json::to_string(&QuorumThreshold::Unanimous).unwrap();
        assert_eq!(json, r#""unanimous""#);
        let back: QuorumThreshold = serde_json::from_str(&json).unwrap();
        assert_eq!(back, QuorumThreshold::Unanimous);
    }

    // ── check_quorum: Majority ────────────────────────────────────────────────

    #[test]
    fn quorum_majority() {
        // 3 out of 5 → strictly more than half
        assert!(check_quorum(QuorumThreshold::Majority, 3, 5));
        // 2 out of 4 → exactly half, not majority
        assert!(!check_quorum(QuorumThreshold::Majority, 2, 4));
        // 3 out of 4 → majority
        assert!(check_quorum(QuorumThreshold::Majority, 3, 4));
    }

    #[test]
    fn quorum_majority_1_of_2_fails() {
        // 1/2 = exactly 50%, not majority
        assert!(!check_quorum(QuorumThreshold::Majority, 1, 2));
    }

    #[test]
    fn quorum_majority_2_of_3_passes() {
        assert!(check_quorum(QuorumThreshold::Majority, 2, 3));
    }

    #[test]
    fn quorum_majority_51_of_100_passes() {
        assert!(check_quorum(QuorumThreshold::Majority, 51, 100));
    }

    #[test]
    fn quorum_majority_50_of_100_fails() {
        assert!(!check_quorum(QuorumThreshold::Majority, 50, 100));
    }

    #[test]
    fn quorum_majority_zero_of_zero() {
        // 0 > 0/2 => 0 > 0 is false
        assert!(!check_quorum(QuorumThreshold::Majority, 0, 0));
    }

    #[test]
    fn quorum_majority_3_of_4() {
        assert!(check_quorum(QuorumThreshold::Majority, 3, 4));
    }

    // ── check_quorum: Supermajority ───────────────────────────────────────────

    #[test]
    fn quorum_supermajority() {
        // 2 out of 3 → exactly 2/3
        assert!(check_quorum(QuorumThreshold::Supermajority, 2, 3));
        // 1 out of 3 → below 2/3
        assert!(!check_quorum(QuorumThreshold::Supermajority, 1, 3));
        // 4 out of 6 → exactly 2/3
        assert!(check_quorum(QuorumThreshold::Supermajority, 4, 6));
    }

    #[test]
    fn quorum_supermajority_67_of_100_passes() {
        // 67 * 3 = 201 >= 100 * 2 = 200
        assert!(check_quorum(QuorumThreshold::Supermajority, 67, 100));
    }

    #[test]
    fn quorum_supermajority_66_of_100_fails() {
        // 66 * 3 = 198 < 100 * 2 = 200
        assert!(!check_quorum(QuorumThreshold::Supermajority, 66, 100));
    }

    #[test]
    fn quorum_supermajority_zero_of_zero() {
        // 0 * 3 >= 0 * 2 => 0 >= 0 is true
        assert!(check_quorum(QuorumThreshold::Supermajority, 0, 0));
    }

    #[test]
    fn quorum_supermajority_all_passes() {
        assert!(check_quorum(QuorumThreshold::Supermajority, 10, 10));
    }

    // ── check_quorum: Unanimous ───────────────────────────────────────────────

    #[test]
    fn quorum_unanimous() {
        assert!(check_quorum(QuorumThreshold::Unanimous, 5, 5));
        assert!(!check_quorum(QuorumThreshold::Unanimous, 4, 5));
    }

    #[test]
    fn quorum_unanimous_missing_one_fails() {
        assert!(!check_quorum(QuorumThreshold::Unanimous, 99, 100));
    }

    #[test]
    fn quorum_unanimous_zero_of_zero() {
        // 0 == 0 is true (edge case: empty body)
        assert!(check_quorum(QuorumThreshold::Unanimous, 0, 0));
    }

    #[test]
    fn quorum_unanimous_one_of_one() {
        assert!(check_quorum(QuorumThreshold::Unanimous, 1, 1));
    }

    // ── Enum serde roundtrips ─────────────────────────────────────────────────

    #[test]
    fn body_type_serde_roundtrip() {
        for variant in [BodyType::BoardOfDirectors, BodyType::LlcMemberVote] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: BodyType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn body_status_serde_roundtrip() {
        for variant in [BodyStatus::Active, BodyStatus::Inactive] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: BodyStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn seat_role_serde_roundtrip() {
        for variant in [
            SeatRole::Chair,
            SeatRole::Member,
            SeatRole::Officer,
            SeatRole::Observer,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: SeatRole = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn seat_status_serde_roundtrip() {
        for variant in [SeatStatus::Active, SeatStatus::Resigned, SeatStatus::Expired] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: SeatStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn meeting_type_serde_roundtrip() {
        for variant in [
            MeetingType::BoardMeeting,
            MeetingType::ShareholderMeeting,
            MeetingType::WrittenConsent,
            MeetingType::MemberMeeting,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: MeetingType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn meeting_status_serde_roundtrip() {
        for variant in [
            MeetingStatus::Draft,
            MeetingStatus::Noticed,
            MeetingStatus::Convened,
            MeetingStatus::Adjourned,
            MeetingStatus::Cancelled,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: MeetingStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn vote_value_serde_roundtrip() {
        for variant in [
            VoteValue::For,
            VoteValue::Against,
            VoteValue::Abstain,
            VoteValue::Recusal,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: VoteValue = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn resolution_type_serde_roundtrip() {
        for variant in [
            ResolutionType::Ordinary,
            ResolutionType::Special,
            ResolutionType::UnanimousWrittenConsent,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: ResolutionType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn voting_method_serde_roundtrip() {
        for variant in [VotingMethod::PerCapita, VotingMethod::PerUnit] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: VotingMethod = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn minutes_status_serde_roundtrip() {
        for variant in [
            MinutesStatus::Draft,
            MinutesStatus::Approved,
            MinutesStatus::Signed,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: MinutesStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn agenda_item_type_serde_roundtrip() {
        for variant in [
            AgendaItemType::Resolution,
            AgendaItemType::Discussion,
            AgendaItemType::Report,
            AgendaItemType::Election,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: AgendaItemType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn body_type_snake_case_serialization() {
        let json = serde_json::to_string(&BodyType::BoardOfDirectors).unwrap();
        assert_eq!(json, r#""board_of_directors""#);
        let json2 = serde_json::to_string(&BodyType::LlcMemberVote).unwrap();
        assert_eq!(json2, r#""llc_member_vote""#);
    }

    #[test]
    fn seat_role_snake_case_serialization() {
        let json = serde_json::to_string(&SeatRole::Observer).unwrap();
        assert_eq!(json, r#""observer""#);
    }

    #[test]
    fn vote_value_snake_case_serialization() {
        let json = serde_json::to_string(&VoteValue::For).unwrap();
        assert_eq!(json, r#""for""#);
        let json2 = serde_json::to_string(&VoteValue::Against).unwrap();
        assert_eq!(json2, r#""against""#);
        let json3 = serde_json::to_string(&VoteValue::Abstain).unwrap();
        assert_eq!(json3, r#""abstain""#);
        let json4 = serde_json::to_string(&VoteValue::Recusal).unwrap();
        assert_eq!(json4, r#""recusal""#);
    }
}
