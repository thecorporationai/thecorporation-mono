//! Resolution — the formal outcome of a vote on an agenda item.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{AgendaItemId, MeetingId, ResolutionId};

use super::types::{QuorumThreshold, ResolutionType, check_quorum};

// ── compute_resolution ────────────────────────────────────────────────────────

/// Determine whether a resolution passes.
///
/// `UnanimousWrittenConsent` requires all eligible votes to be in favour
/// (no abstentions, no against). `Ordinary` and `Special` use the standard
/// [`check_quorum`] rules.
pub fn compute_resolution(
    resolution_type: ResolutionType,
    votes_for: u32,
    votes_against: u32,
    votes_abstain: u32,
    quorum_threshold: QuorumThreshold,
) -> bool {
    match resolution_type {
        ResolutionType::UnanimousWrittenConsent => {
            votes_against == 0 && votes_abstain == 0 && votes_for > 0
        }
        ResolutionType::Ordinary | ResolutionType::Special => {
            let total_cast = votes_for + votes_against + votes_abstain;
            check_quorum(quorum_threshold, votes_for, total_cast)
        }
    }
}

// ── Resolution ────────────────────────────────────────────────────────────────

/// The recorded outcome of a formal vote on a specific agenda item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub resolution_id: ResolutionId,
    pub meeting_id: MeetingId,
    pub agenda_item_id: AgendaItemId,
    pub resolution_type: ResolutionType,
    pub resolution_text: String,
    pub votes_for: u32,
    pub votes_against: u32,
    pub votes_abstain: u32,
    /// `true` if the resolution passed the applicable threshold.
    pub passed: bool,
    pub resolved_at: DateTime<Utc>,
}

impl Resolution {
    /// Record a new resolution, computing `passed` from the vote tallies.
    pub fn new(
        meeting_id: MeetingId,
        agenda_item_id: AgendaItemId,
        resolution_type: ResolutionType,
        resolution_text: String,
        votes_for: u32,
        votes_against: u32,
        votes_abstain: u32,
        quorum_threshold: QuorumThreshold,
    ) -> Self {
        let passed = compute_resolution(
            resolution_type,
            votes_for,
            votes_against,
            votes_abstain,
            quorum_threshold,
        );
        Self {
            resolution_id: ResolutionId::new(),
            meeting_id,
            agenda_item_id,
            resolution_type,
            resolution_text,
            votes_for,
            votes_against,
            votes_abstain,
            passed,
            resolved_at: Utc::now(),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── compute_resolution: Ordinary ──────────────────────────────────────────

    #[test]
    fn ordinary_majority_passes() {
        assert!(compute_resolution(
            ResolutionType::Ordinary,
            3,
            1,
            1,
            QuorumThreshold::Majority
        ));
    }

    #[test]
    fn ordinary_majority_fails_tie() {
        // 2 for, 2 against — not a majority
        assert!(!compute_resolution(
            ResolutionType::Ordinary,
            2,
            2,
            0,
            QuorumThreshold::Majority
        ));
    }

    #[test]
    fn ordinary_majority_1_of_2_fails() {
        // 1 out of 2 total cast — exactly half, not majority
        assert!(!compute_resolution(
            ResolutionType::Ordinary,
            1,
            1,
            0,
            QuorumThreshold::Majority
        ));
    }

    #[test]
    fn ordinary_majority_2_of_3_passes() {
        assert!(compute_resolution(
            ResolutionType::Ordinary,
            2,
            1,
            0,
            QuorumThreshold::Majority
        ));
    }

    #[test]
    fn ordinary_with_abstentions_counted_in_total() {
        // 3 for, 1 against, 1 abstain → total_cast = 5, 3/5 > 50% → passes
        assert!(compute_resolution(
            ResolutionType::Ordinary,
            3,
            1,
            1,
            QuorumThreshold::Majority
        ));
    }

    #[test]
    fn ordinary_all_abstain_fails() {
        // 0 for, 0 against, 5 abstain → 0 out of 5 → fails majority
        assert!(!compute_resolution(
            ResolutionType::Ordinary,
            0,
            0,
            5,
            QuorumThreshold::Majority
        ));
    }

    #[test]
    fn ordinary_all_recusal_zero_votes_fails() {
        // zero total cast — 0 > 0/2 is false
        assert!(!compute_resolution(
            ResolutionType::Ordinary,
            0,
            0,
            0,
            QuorumThreshold::Majority
        ));
    }

    #[test]
    fn ordinary_all_for_passes() {
        assert!(compute_resolution(
            ResolutionType::Ordinary,
            5,
            0,
            0,
            QuorumThreshold::Majority
        ));
    }

    // ── compute_resolution: Special (supermajority) ───────────────────────────

    #[test]
    fn special_supermajority_exactly_two_thirds_passes() {
        // 4 for, 2 against → 4/6 = exactly 2/3 → passes supermajority
        assert!(compute_resolution(
            ResolutionType::Special,
            4,
            2,
            0,
            QuorumThreshold::Supermajority
        ));
    }

    #[test]
    fn special_supermajority_below_two_thirds_fails() {
        // 3 for, 3 against → 3/6 = 50% → fails supermajority
        assert!(!compute_resolution(
            ResolutionType::Special,
            3,
            3,
            0,
            QuorumThreshold::Supermajority
        ));
    }

    #[test]
    fn special_supermajority_67_of_100_passes() {
        assert!(compute_resolution(
            ResolutionType::Special,
            67,
            33,
            0,
            QuorumThreshold::Supermajority
        ));
    }

    #[test]
    fn special_supermajority_66_of_100_fails() {
        assert!(!compute_resolution(
            ResolutionType::Special,
            66,
            34,
            0,
            QuorumThreshold::Supermajority
        ));
    }

    #[test]
    fn special_supermajority_with_abstentions() {
        // 2 for, 1 against, 1 abstain → total = 4; 2/4 = 50% → fails supermajority
        assert!(!compute_resolution(
            ResolutionType::Special,
            2,
            1,
            1,
            QuorumThreshold::Supermajority
        ));
    }

    #[test]
    fn special_all_for_passes_supermajority() {
        assert!(compute_resolution(
            ResolutionType::Special,
            5,
            0,
            0,
            QuorumThreshold::Supermajority
        ));
    }

    #[test]
    fn special_all_abstain_passes_supermajority_edge() {
        // 0/0 supermajority: 0*3 >= 0*2 → true (vacuously passes)
        assert!(compute_resolution(
            ResolutionType::Special,
            0,
            0,
            0,
            QuorumThreshold::Supermajority
        ));
    }

    // ── compute_resolution: UnanimousWrittenConsent ───────────────────────────

    #[test]
    fn unanimous_written_consent_all_for_passes() {
        assert!(compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            5,
            0,
            0,
            QuorumThreshold::Unanimous
        ));
    }

    #[test]
    fn unanimous_written_consent_requires_no_dissent() {
        assert!(compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            5,
            0,
            0,
            QuorumThreshold::Unanimous
        ));
        assert!(!compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            4,
            1,
            0,
            QuorumThreshold::Unanimous
        ));
        // Abstention also defeats UWC.
        assert!(!compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            4,
            0,
            1,
            QuorumThreshold::Unanimous
        ));
    }

    #[test]
    fn unanimous_written_consent_one_against_fails() {
        assert!(!compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            4,
            1,
            0,
            QuorumThreshold::Unanimous
        ));
    }

    #[test]
    fn unanimous_written_consent_one_abstain_fails() {
        assert!(!compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            4,
            0,
            1,
            QuorumThreshold::Unanimous
        ));
    }

    #[test]
    fn unanimous_written_consent_zero_for_fails() {
        // No votes for → fails even if nothing against
        assert!(!compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            0,
            0,
            0,
            QuorumThreshold::Unanimous
        ));
    }

    #[test]
    fn unanimous_written_consent_single_vote_for_passes() {
        assert!(compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            1,
            0,
            0,
            QuorumThreshold::Unanimous
        ));
    }

    #[test]
    fn unanimous_written_consent_all_against_fails() {
        assert!(!compute_resolution(
            ResolutionType::UnanimousWrittenConsent,
            0,
            5,
            0,
            QuorumThreshold::Unanimous
        ));
    }

    // ── Resolution::new() stores computed `passed` ────────────────────────────

    #[test]
    fn resolution_new_computes_passed_true() {
        let r = Resolution::new(
            MeetingId::new(),
            AgendaItemId::new(),
            ResolutionType::Ordinary,
            "RESOLVED: approve budget".into(),
            3,
            1,
            0,
            QuorumThreshold::Majority,
        );
        assert!(r.passed);
        assert_eq!(r.votes_for, 3);
        assert_eq!(r.votes_against, 1);
        assert_eq!(r.votes_abstain, 0);
    }

    #[test]
    fn resolution_new_computes_passed_false() {
        let r = Resolution::new(
            MeetingId::new(),
            AgendaItemId::new(),
            ResolutionType::Ordinary,
            "RESOLVED: approve budget".into(),
            2,
            2,
            0,
            QuorumThreshold::Majority,
        );
        assert!(!r.passed);
    }

    #[test]
    fn resolution_new_unanimous_written_consent_passed() {
        let r = Resolution::new(
            MeetingId::new(),
            AgendaItemId::new(),
            ResolutionType::UnanimousWrittenConsent,
            "RESOLVED: issue equity".into(),
            3,
            0,
            0,
            QuorumThreshold::Unanimous,
        );
        assert!(r.passed);
    }

    #[test]
    fn resolution_serde_roundtrip() {
        let r = Resolution::new(
            MeetingId::new(),
            AgendaItemId::new(),
            ResolutionType::Special,
            "RESOLVED: amend charter".into(),
            4,
            2,
            0,
            QuorumThreshold::Supermajority,
        );
        let json = serde_json::to_string(&r).unwrap();
        let back: Resolution = serde_json::from_str(&json).unwrap();
        assert_eq!(r.resolution_id, back.resolution_id);
        assert_eq!(r.passed, back.passed);
        assert_eq!(r.votes_for, back.votes_for);
    }

    #[test]
    fn special_supermajority() {
        // 4 for, 2 against → 4/6 = exactly 2/3 → passes supermajority
        assert!(compute_resolution(
            ResolutionType::Special,
            4,
            2,
            0,
            QuorumThreshold::Supermajority
        ));
        // 3 for, 3 against → 3/6 = 50% → fails supermajority
        assert!(!compute_resolution(
            ResolutionType::Special,
            3,
            3,
            0,
            QuorumThreshold::Supermajority
        ));
    }
}
