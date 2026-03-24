//! Meeting FSM — the lifecycle of a governance meeting from draft to adjournment.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{GovernanceBodyId, GovernanceSeatId, MeetingId};

use super::types::{MeetingStatus, MeetingType, QuorumThreshold, check_quorum};

// ── QuorumStatus ──────────────────────────────────────────────────────────────

/// Whether the required quorum has been established for this meeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuorumStatus {
    Unknown,
    Met,
    NotMet,
}

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors produced by [`Meeting`] FSM transitions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MeetingError {
    #[error("invalid transition: meeting is in status '{current:?}', cannot {action}")]
    InvalidTransition {
        current: MeetingStatus,
        action: &'static str,
    },
}

// ── Meeting ───────────────────────────────────────────────────────────────────

/// A scheduled (or written-consent) governance meeting with a full FSM lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meeting {
    pub meeting_id: MeetingId,
    pub body_id: GovernanceBodyId,
    pub meeting_type: MeetingType,
    pub title: String,
    pub scheduled_date: Option<DateTime<Utc>>,
    pub location: Option<String>,
    /// Advance-notice requirement in calendar days.
    pub notice_days: Option<u32>,
    pub status: MeetingStatus,
    /// Whether the required quorum was achieved.
    pub quorum_met: QuorumStatus,
    /// Seat IDs recorded as present at this meeting.
    pub present_seat_ids: Vec<GovernanceSeatId>,
    pub convened_at: Option<DateTime<Utc>>,
    pub adjourned_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Meeting {
    /// Create a new meeting.
    ///
    /// `WrittenConsent` meetings are immediately placed in `Convened` status.
    /// All other types start as `Draft`.
    pub fn new(
        body_id: GovernanceBodyId,
        meeting_type: MeetingType,
        title: String,
        scheduled_date: Option<DateTime<Utc>>,
        location: Option<String>,
        notice_days: Option<u32>,
    ) -> Self {
        let now = Utc::now();
        let (status, convened_at) = if meeting_type == MeetingType::WrittenConsent {
            (MeetingStatus::Convened, Some(now))
        } else {
            (MeetingStatus::Draft, None)
        };

        Self {
            meeting_id: MeetingId::new(),
            body_id,
            meeting_type,
            title,
            scheduled_date,
            location,
            notice_days,
            status,
            quorum_met: QuorumStatus::Unknown,
            present_seat_ids: Vec::new(),
            convened_at,
            adjourned_at: None,
            created_at: now,
        }
    }

    // ── FSM transitions ───────────────────────────────────────────────────────

    /// `Draft` → `Noticed`.
    pub fn send_notice(&mut self) -> Result<(), MeetingError> {
        match self.status {
            MeetingStatus::Draft => {
                self.status = MeetingStatus::Noticed;
                Ok(())
            }
            s => Err(MeetingError::InvalidTransition {
                current: s,
                action: "send notice",
            }),
        }
    }

    /// `Draft | Noticed` → `Convened`. Records `convened_at`.
    pub fn convene(&mut self) -> Result<(), MeetingError> {
        match self.status {
            MeetingStatus::Draft | MeetingStatus::Noticed => {
                self.status = MeetingStatus::Convened;
                self.convened_at = Some(Utc::now());
                Ok(())
            }
            s => Err(MeetingError::InvalidTransition {
                current: s,
                action: "convene",
            }),
        }
    }

    /// `Convened` → `Adjourned`. Records `adjourned_at`.
    pub fn adjourn(&mut self) -> Result<(), MeetingError> {
        match self.status {
            MeetingStatus::Convened => {
                self.status = MeetingStatus::Adjourned;
                self.adjourned_at = Some(Utc::now());
                Ok(())
            }
            s => Err(MeetingError::InvalidTransition {
                current: s,
                action: "adjourn",
            }),
        }
    }

    /// `Draft | Noticed` → `Cancelled`.
    pub fn cancel(&mut self) -> Result<(), MeetingError> {
        match self.status {
            MeetingStatus::Draft | MeetingStatus::Noticed => {
                self.status = MeetingStatus::Cancelled;
                Ok(())
            }
            s => Err(MeetingError::InvalidTransition {
                current: s,
                action: "cancel",
            }),
        }
    }

    /// `Adjourned` → `Convened`. Clears `adjourned_at`.
    pub fn reopen(&mut self) -> Result<(), MeetingError> {
        match self.status {
            MeetingStatus::Adjourned => {
                self.status = MeetingStatus::Convened;
                self.adjourned_at = None;
                Ok(())
            }
            s => Err(MeetingError::InvalidTransition {
                current: s,
                action: "reopen",
            }),
        }
    }

    // ── Business rules ────────────────────────────────────────────────────────

    /// Returns `true` if votes may currently be cast.
    ///
    /// Voting is allowed when the meeting is `Convened` AND either:
    /// - the meeting type is `WrittenConsent` (quorum not required), or
    /// - quorum has been recorded as `Met`.
    pub fn can_vote(&self) -> bool {
        if self.status != MeetingStatus::Convened {
            return false;
        }
        self.meeting_type == MeetingType::WrittenConsent
            || self.quorum_met == QuorumStatus::Met
    }

    /// Record the attending seats and evaluate quorum against `threshold`.
    ///
    /// `total_eligible` is the count of seats (or total voting units, depending
    /// on the body's `VotingMethod`) that are eligible to participate.
    pub fn record_attendance(
        &mut self,
        seat_ids: Vec<GovernanceSeatId>,
        present_count: u32,
        total_eligible: u32,
        threshold: QuorumThreshold,
    ) {
        self.present_seat_ids = seat_ids;
        self.quorum_met = if check_quorum(threshold, present_count, total_eligible) {
            QuorumStatus::Met
        } else {
            QuorumStatus::NotMet
        };
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn board_meeting() -> Meeting {
        Meeting::new(
            GovernanceBodyId::new(),
            MeetingType::BoardMeeting,
            "Q1 Board Meeting".into(),
            None,
            None,
            None,
        )
    }

    fn written_consent() -> Meeting {
        Meeting::new(
            GovernanceBodyId::new(),
            MeetingType::WrittenConsent,
            "Written Consent".into(),
            None,
            None,
            None,
        )
    }

    fn shareholder_meeting() -> Meeting {
        Meeting::new(
            GovernanceBodyId::new(),
            MeetingType::ShareholderMeeting,
            "Annual Shareholder Meeting".into(),
            None,
            None,
            None,
        )
    }

    fn member_meeting() -> Meeting {
        Meeting::new(
            GovernanceBodyId::new(),
            MeetingType::MemberMeeting,
            "LLC Member Meeting".into(),
            None,
            None,
            None,
        )
    }

    // ── new() initial state ───────────────────────────────────────────────────

    #[test]
    fn new_board_meeting_is_draft() {
        let m = board_meeting();
        assert_eq!(m.status, MeetingStatus::Draft);
        assert!(m.convened_at.is_none());
        assert!(m.adjourned_at.is_none());
    }

    #[test]
    fn new_shareholder_meeting_is_draft() {
        let m = shareholder_meeting();
        assert_eq!(m.status, MeetingStatus::Draft);
        assert!(m.convened_at.is_none());
    }

    #[test]
    fn new_member_meeting_is_draft() {
        let m = member_meeting();
        assert_eq!(m.status, MeetingStatus::Draft);
    }

    #[test]
    fn written_consent_auto_convened() {
        let m = written_consent();
        assert_eq!(m.status, MeetingStatus::Convened);
        assert!(m.convened_at.is_some());
    }

    #[test]
    fn written_consent_quorum_starts_unknown() {
        let m = written_consent();
        assert_eq!(m.quorum_met, QuorumStatus::Unknown);
    }

    #[test]
    fn new_meeting_has_no_present_seats() {
        let m = board_meeting();
        assert!(m.present_seat_ids.is_empty());
    }

    #[test]
    fn new_meeting_has_unique_ids() {
        let m1 = board_meeting();
        let m2 = board_meeting();
        assert_ne!(m1.meeting_id, m2.meeting_id);
    }

    #[test]
    fn meeting_with_scheduled_date_stores_it() {
        use chrono::TimeZone;
        let dt = chrono::Utc.with_ymd_and_hms(2026, 6, 15, 9, 0, 0).unwrap();
        let m = Meeting::new(
            GovernanceBodyId::new(),
            MeetingType::BoardMeeting,
            "Summer Board Meeting".into(),
            Some(dt),
            Some("123 Main St".into()),
            Some(7),
        );
        assert_eq!(m.scheduled_date, Some(dt));
        assert_eq!(m.location.as_deref(), Some("123 Main St"));
        assert_eq!(m.notice_days, Some(7));
    }

    // ── send_notice() ─────────────────────────────────────────────────────────

    #[test]
    fn send_notice_from_draft_succeeds() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        assert_eq!(m.status, MeetingStatus::Noticed);
    }

    #[test]
    fn send_notice_from_noticed_errors() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        let err = m.send_notice().unwrap_err();
        match err {
            MeetingError::InvalidTransition { current, action } => {
                assert_eq!(current, MeetingStatus::Noticed);
                assert_eq!(action, "send notice");
            }
        }
    }

    #[test]
    fn send_notice_from_convened_errors() {
        let mut m = board_meeting();
        m.convene().unwrap();
        assert!(m.send_notice().is_err());
    }

    #[test]
    fn send_notice_from_adjourned_errors() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.adjourn().unwrap();
        assert!(m.send_notice().is_err());
    }

    #[test]
    fn send_notice_from_cancelled_errors() {
        let mut m = board_meeting();
        m.cancel().unwrap();
        assert!(m.send_notice().is_err());
    }

    // ── convene() ─────────────────────────────────────────────────────────────

    #[test]
    fn convene_from_draft_succeeds() {
        let mut m = board_meeting();
        m.convene().unwrap();
        assert_eq!(m.status, MeetingStatus::Convened);
        assert!(m.convened_at.is_some());
    }

    #[test]
    fn convene_from_noticed_succeeds() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        m.convene().unwrap();
        assert_eq!(m.status, MeetingStatus::Convened);
        assert!(m.convened_at.is_some());
    }

    #[test]
    fn convene_from_adjourned_errors() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.adjourn().unwrap();
        assert!(m.convene().is_err());
    }

    #[test]
    fn convene_from_cancelled_errors() {
        let mut m = board_meeting();
        m.cancel().unwrap();
        assert!(m.convene().is_err());
    }

    #[test]
    fn convene_from_already_convened_errors() {
        let mut m = board_meeting();
        m.convene().unwrap();
        assert!(m.convene().is_err());
    }

    // ── adjourn() ─────────────────────────────────────────────────────────────

    #[test]
    fn adjourn_from_convened_succeeds() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.adjourn().unwrap();
        assert_eq!(m.status, MeetingStatus::Adjourned);
        assert!(m.adjourned_at.is_some());
    }

    #[test]
    fn adjourn_from_draft_errors() {
        let mut m = board_meeting();
        assert!(m.adjourn().is_err());
    }

    #[test]
    fn adjourn_from_noticed_errors() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        assert!(m.adjourn().is_err());
    }

    #[test]
    fn adjourn_from_adjourned_errors() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.adjourn().unwrap();
        assert!(m.adjourn().is_err());
    }

    #[test]
    fn adjourn_written_consent_succeeds() {
        let mut m = written_consent();
        // WrittenConsent starts as Convened
        m.adjourn().unwrap();
        assert_eq!(m.status, MeetingStatus::Adjourned);
    }

    // ── cancel() ─────────────────────────────────────────────────────────────

    #[test]
    fn cancel_from_draft_succeeds() {
        let mut m = board_meeting();
        m.cancel().unwrap();
        assert_eq!(m.status, MeetingStatus::Cancelled);
    }

    #[test]
    fn cancel_from_noticed_succeeds() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        m.cancel().unwrap();
        assert_eq!(m.status, MeetingStatus::Cancelled);
    }

    #[test]
    fn cancel_from_convened_errors() {
        let mut m = board_meeting();
        m.convene().unwrap();
        assert!(m.cancel().is_err());
    }

    #[test]
    fn cancel_from_adjourned_errors() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.adjourn().unwrap();
        assert!(m.cancel().is_err());
    }

    #[test]
    fn cancel_from_cancelled_errors() {
        let mut m = board_meeting();
        m.cancel().unwrap();
        assert!(m.cancel().is_err());
    }

    // ── reopen() ─────────────────────────────────────────────────────────────

    #[test]
    fn reopen_adjourned_succeeds() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.adjourn().unwrap();
        m.reopen().unwrap();
        assert_eq!(m.status, MeetingStatus::Convened);
        assert!(m.adjourned_at.is_none());
    }

    #[test]
    fn reopen_from_draft_errors() {
        let mut m = board_meeting();
        assert!(m.reopen().is_err());
    }

    #[test]
    fn reopen_from_noticed_errors() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        assert!(m.reopen().is_err());
    }

    #[test]
    fn reopen_from_convened_errors() {
        let mut m = board_meeting();
        m.convene().unwrap();
        assert!(m.reopen().is_err());
    }

    #[test]
    fn reopen_from_cancelled_errors() {
        let mut m = board_meeting();
        m.cancel().unwrap();
        assert!(m.reopen().is_err());
    }

    // ── can_vote() ────────────────────────────────────────────────────────────

    #[test]
    fn can_vote_draft_is_false() {
        let m = board_meeting();
        assert!(!m.can_vote());
    }

    #[test]
    fn can_vote_noticed_is_false() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        assert!(!m.can_vote());
    }

    #[test]
    fn can_vote_convened_without_quorum_is_false() {
        let mut m = board_meeting();
        m.convene().unwrap();
        assert!(!m.can_vote());
    }

    #[test]
    fn can_vote_convened_with_quorum_met() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.record_attendance(vec![], 3, 4, QuorumThreshold::Majority);
        assert!(m.can_vote());
    }

    #[test]
    fn can_vote_convened_quorum_not_met() {
        let mut m = board_meeting();
        m.convene().unwrap();
        // 1 out of 4 does not meet majority
        m.record_attendance(vec![], 1, 4, QuorumThreshold::Majority);
        assert!(!m.can_vote());
    }

    #[test]
    fn can_vote_adjourned_is_false() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.record_attendance(vec![], 3, 4, QuorumThreshold::Majority);
        m.adjourn().unwrap();
        assert!(!m.can_vote());
    }

    #[test]
    fn can_vote_cancelled_is_false() {
        let mut m = board_meeting();
        m.cancel().unwrap();
        assert!(!m.can_vote());
    }

    #[test]
    fn written_consent_can_vote_without_quorum_record() {
        let m = written_consent();
        assert!(m.can_vote());
    }

    #[test]
    fn written_consent_can_vote_after_attendance_recorded() {
        let mut m = written_consent();
        m.record_attendance(vec![], 0, 5, QuorumThreshold::Majority);
        // WrittenConsent always allows voting when Convened
        assert!(m.can_vote());
    }

    // ── record_attendance() ───────────────────────────────────────────────────

    #[test]
    fn record_attendance_sets_quorum_met() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.record_attendance(vec![], 3, 4, QuorumThreshold::Majority);
        assert_eq!(m.quorum_met, QuorumStatus::Met);
    }

    #[test]
    fn record_attendance_sets_quorum_not_met() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.record_attendance(vec![], 1, 4, QuorumThreshold::Majority);
        assert_eq!(m.quorum_met, QuorumStatus::NotMet);
    }

    #[test]
    fn record_attendance_stores_seat_ids() {
        let mut m = board_meeting();
        m.convene().unwrap();
        let seat1 = GovernanceSeatId::new();
        let seat2 = GovernanceSeatId::new();
        m.record_attendance(vec![seat1, seat2], 2, 3, QuorumThreshold::Majority);
        assert_eq!(m.present_seat_ids.len(), 2);
        assert!(m.present_seat_ids.contains(&seat1));
        assert!(m.present_seat_ids.contains(&seat2));
    }

    #[test]
    fn record_attendance_supermajority_met() {
        let mut m = board_meeting();
        m.convene().unwrap();
        // 2/3 exactly meets supermajority
        m.record_attendance(vec![], 2, 3, QuorumThreshold::Supermajority);
        assert_eq!(m.quorum_met, QuorumStatus::Met);
    }

    #[test]
    fn record_attendance_supermajority_not_met() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.record_attendance(vec![], 1, 3, QuorumThreshold::Supermajority);
        assert_eq!(m.quorum_met, QuorumStatus::NotMet);
    }

    #[test]
    fn record_attendance_unanimous_met() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.record_attendance(vec![], 5, 5, QuorumThreshold::Unanimous);
        assert_eq!(m.quorum_met, QuorumStatus::Met);
    }

    #[test]
    fn record_attendance_unanimous_not_met() {
        let mut m = board_meeting();
        m.convene().unwrap();
        m.record_attendance(vec![], 4, 5, QuorumThreshold::Unanimous);
        assert_eq!(m.quorum_met, QuorumStatus::NotMet);
    }

    // ── Full lifecycles ───────────────────────────────────────────────────────

    #[test]
    fn full_happy_path_draft_to_adjourned() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        assert_eq!(m.status, MeetingStatus::Noticed);
        m.convene().unwrap();
        assert_eq!(m.status, MeetingStatus::Convened);
        assert!(m.convened_at.is_some());
        m.adjourn().unwrap();
        assert_eq!(m.status, MeetingStatus::Adjourned);
        assert!(m.adjourned_at.is_some());
    }

    #[test]
    fn written_consent_lifecycle_convened_to_adjourned() {
        let mut m = written_consent();
        assert_eq!(m.status, MeetingStatus::Convened);
        assert!(m.can_vote());
        m.adjourn().unwrap();
        assert_eq!(m.status, MeetingStatus::Adjourned);
        assert!(!m.can_vote());
    }

    #[test]
    fn full_lifecycle_draft_noticed_convened_adjourned_reopened() {
        let mut m = board_meeting();
        m.send_notice().unwrap();
        m.convene().unwrap();
        m.record_attendance(vec![], 3, 5, QuorumThreshold::Majority);
        m.adjourn().unwrap();
        assert_eq!(m.status, MeetingStatus::Adjourned);
        m.reopen().unwrap();
        assert_eq!(m.status, MeetingStatus::Convened);
        // adjourned_at cleared after reopen
        assert!(m.adjourned_at.is_none());
    }

    #[test]
    fn invalid_transition_returns_err() {
        let mut m = board_meeting();
        assert!(m.adjourn().is_err());
        assert!(m.cancel().is_ok());
        assert!(m.cancel().is_err());
    }

    #[test]
    fn can_vote_requires_quorum_except_written_consent() {
        let mut m = board_meeting();
        m.convene().unwrap();
        // No quorum recorded yet.
        assert!(!m.can_vote());
        m.record_attendance(vec![], 3, 4, QuorumThreshold::Majority);
        assert!(m.can_vote());
    }

    // ── serde roundtrip ───────────────────────────────────────────────────────

    #[test]
    fn meeting_serde_roundtrip() {
        let m = board_meeting();
        let json = serde_json::to_string(&m).unwrap();
        let back: Meeting = serde_json::from_str(&json).unwrap();
        assert_eq!(m.meeting_id, back.meeting_id);
        assert_eq!(m.status, back.status);
        assert_eq!(m.meeting_type, back.meeting_type);
    }

    #[test]
    fn quorum_status_serde_roundtrip() {
        for variant in [QuorumStatus::Unknown, QuorumStatus::Met, QuorumStatus::NotMet] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: QuorumStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn meeting_error_display() {
        let err = MeetingError::InvalidTransition {
            current: MeetingStatus::Draft,
            action: "adjourn",
        };
        let msg = format!("{err}");
        assert!(msg.contains("Draft"));
        assert!(msg.contains("adjourn"));
    }
}
