//! Meeting record (stored as `governance/meetings/{meeting_id}/meeting.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::GovernanceError;
use super::types::{MeetingStatus, MeetingType, QuorumStatus};
use crate::domain::ids::{GovernanceBodyId, GovernanceSeatId, MeetingId};

/// A meeting of a governance body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meeting {
    meeting_id: MeetingId,
    body_id: GovernanceBodyId,
    meeting_type: MeetingType,
    title: String,
    scheduled_date: Option<NaiveDate>,
    location: String,
    notice_days: u32,
    status: MeetingStatus,
    quorum_met: QuorumStatus,
    present_seat_ids: Vec<GovernanceSeatId>,
    convened_at: Option<DateTime<Utc>>,
    adjourned_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl Meeting {
    /// Create a new meeting.
    ///
    /// `WrittenConsent` meetings start in `Convened` status (no physical meeting
    /// is required). All other types start in `Draft`.
    pub fn new(
        meeting_id: MeetingId,
        body_id: GovernanceBodyId,
        meeting_type: MeetingType,
        title: String,
        scheduled_date: Option<NaiveDate>,
        location: String,
        notice_days: u32,
    ) -> Self {
        let initial_status = if meeting_type == MeetingType::WrittenConsent {
            MeetingStatus::Convened
        } else {
            MeetingStatus::Draft
        };
        Self {
            meeting_id,
            body_id,
            meeting_type,
            title,
            scheduled_date,
            location,
            notice_days,
            status: initial_status,
            quorum_met: QuorumStatus::Unknown,
            present_seat_ids: Vec::new(),
            convened_at: if meeting_type == MeetingType::WrittenConsent {
                Some(Utc::now())
            } else {
                None
            },
            adjourned_at: None,
            created_at: Utc::now(),
        }
    }

    /// Send notice. `Draft` -> `Noticed`.
    pub fn send_notice(&mut self) -> Result<(), GovernanceError> {
        if self.status != MeetingStatus::Draft {
            return Err(GovernanceError::InvalidMeetingTransition {
                from: self.status,
                to: MeetingStatus::Noticed,
            });
        }
        self.status = MeetingStatus::Noticed;
        Ok(())
    }

    /// Convene the meeting with present seat IDs. `Noticed` -> `Convened`.
    ///
    /// Also records whether quorum was met.
    pub fn convene(
        &mut self,
        present_seat_ids: Vec<GovernanceSeatId>,
        quorum_met: bool,
    ) -> Result<(), GovernanceError> {
        if self.status != MeetingStatus::Noticed {
            return Err(GovernanceError::InvalidMeetingTransition {
                from: self.status,
                to: MeetingStatus::Convened,
            });
        }
        self.present_seat_ids = present_seat_ids;
        self.quorum_met = if quorum_met {
            QuorumStatus::Met
        } else {
            QuorumStatus::NotMet
        };
        self.convened_at = Some(Utc::now());
        self.status = MeetingStatus::Convened;
        Ok(())
    }

    /// Adjourn the meeting. `Convened` -> `Adjourned`.
    pub fn adjourn(&mut self) -> Result<(), GovernanceError> {
        if self.status != MeetingStatus::Convened {
            return Err(GovernanceError::InvalidMeetingTransition {
                from: self.status,
                to: MeetingStatus::Adjourned,
            });
        }
        self.adjourned_at = Some(Utc::now());
        self.status = MeetingStatus::Adjourned;
        Ok(())
    }

    /// Re-open an adjourned meeting so discussion and voting can continue.
    pub fn reopen(&mut self) -> Result<(), GovernanceError> {
        if self.status != MeetingStatus::Adjourned {
            return Err(GovernanceError::InvalidMeetingTransition {
                from: self.status,
                to: MeetingStatus::Convened,
            });
        }
        self.adjourned_at = None;
        self.status = MeetingStatus::Convened;
        Ok(())
    }

    /// Update quorum status (used by adjournment handler to recompute from votes).
    pub fn set_quorum_status(&mut self, status: QuorumStatus) {
        self.quorum_met = status;
    }

    /// Cancel the meeting. Only from `Draft` or `Noticed`.
    pub fn cancel(&mut self) -> Result<(), GovernanceError> {
        match self.status {
            MeetingStatus::Draft | MeetingStatus::Noticed => {
                self.status = MeetingStatus::Cancelled;
                Ok(())
            }
            _ => Err(GovernanceError::InvalidMeetingTransition {
                from: self.status,
                to: MeetingStatus::Cancelled,
            }),
        }
    }

    /// Whether voting is allowed (convened and quorum met).
    pub fn can_vote(&self) -> bool {
        self.status == MeetingStatus::Convened
            && (self.meeting_type == MeetingType::WrittenConsent || self.quorum_met.is_met())
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn meeting_id(&self) -> MeetingId {
        self.meeting_id
    }

    pub fn body_id(&self) -> GovernanceBodyId {
        self.body_id
    }

    pub fn meeting_type(&self) -> MeetingType {
        self.meeting_type
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn scheduled_date(&self) -> Option<NaiveDate> {
        self.scheduled_date
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    pub fn notice_days(&self) -> u32 {
        self.notice_days
    }

    pub fn status(&self) -> MeetingStatus {
        self.status
    }

    pub fn quorum_met(&self) -> QuorumStatus {
        self.quorum_met
    }

    pub fn present_seat_ids(&self) -> &[GovernanceSeatId] {
        &self.present_seat_ids
    }

    pub fn convened_at(&self) -> Option<DateTime<Utc>> {
        self.convened_at
    }

    pub fn adjourned_at(&self) -> Option<DateTime<Utc>> {
        self.adjourned_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meeting(meeting_type: MeetingType) -> Meeting {
        Meeting::new(
            MeetingId::new(),
            GovernanceBodyId::new(),
            meeting_type,
            "Annual Board Meeting".into(),
            None,
            "Registered Office".into(),
            10,
        )
    }

    #[test]
    fn draft_to_noticed_to_convened_to_adjourned() {
        let mut m = make_meeting(MeetingType::BoardMeeting);
        assert_eq!(m.status(), MeetingStatus::Draft);

        m.send_notice().unwrap();
        assert_eq!(m.status(), MeetingStatus::Noticed);

        let seats = vec![GovernanceSeatId::new(), GovernanceSeatId::new()];
        m.convene(seats.clone(), true).unwrap();
        assert_eq!(m.status(), MeetingStatus::Convened);
        assert_eq!(m.present_seat_ids().len(), 2);
        assert_eq!(m.quorum_met(), QuorumStatus::Met);
        assert!(m.convened_at().is_some());

        m.adjourn().unwrap();
        assert_eq!(m.status(), MeetingStatus::Adjourned);
        assert!(m.adjourned_at().is_some());
    }

    #[test]
    fn cancel_from_draft() {
        let mut m = make_meeting(MeetingType::BoardMeeting);
        m.cancel().unwrap();
        assert_eq!(m.status(), MeetingStatus::Cancelled);
    }

    #[test]
    fn cancel_from_noticed() {
        let mut m = make_meeting(MeetingType::BoardMeeting);
        m.send_notice().unwrap();
        m.cancel().unwrap();
        assert_eq!(m.status(), MeetingStatus::Cancelled);
    }

    #[test]
    fn adjourned_meeting_can_reopen() {
        let mut m = make_meeting(MeetingType::BoardMeeting);
        m.send_notice().unwrap();
        m.convene(vec![GovernanceSeatId::new()], true).unwrap();
        m.adjourn().unwrap();
        m.reopen().unwrap();
        assert_eq!(m.status(), MeetingStatus::Convened);
        assert!(m.adjourned_at().is_none());
    }

    #[test]
    fn cannot_cancel_convened() {
        let mut m = make_meeting(MeetingType::BoardMeeting);
        m.send_notice().unwrap();
        m.convene(vec![], true).unwrap();
        assert!(m.cancel().is_err());
    }

    #[test]
    fn written_consent_starts_convened() {
        let m = make_meeting(MeetingType::WrittenConsent);
        assert_eq!(m.status(), MeetingStatus::Convened);
        assert_eq!(m.quorum_met(), QuorumStatus::Unknown);
        assert!(m.convened_at().is_some());
    }

    #[test]
    fn can_vote_requires_convened_and_quorum() {
        let mut m = make_meeting(MeetingType::BoardMeeting);
        assert!(!m.can_vote());

        m.send_notice().unwrap();
        assert!(!m.can_vote());

        // Convened without quorum
        m.convene(vec![], false).unwrap();
        assert!(!m.can_vote());
    }

    #[test]
    fn can_vote_with_quorum() {
        let mut m = make_meeting(MeetingType::BoardMeeting);
        m.send_notice().unwrap();
        m.convene(vec![GovernanceSeatId::new()], true).unwrap();
        assert!(m.can_vote());
    }

    #[test]
    fn written_consent_can_vote_before_quorum_is_computed() {
        let m = make_meeting(MeetingType::WrittenConsent);
        assert!(m.can_vote());
    }

    #[test]
    fn invalid_transition_noticed_from_convened() {
        let mut m = make_meeting(MeetingType::BoardMeeting);
        m.send_notice().unwrap();
        m.convene(vec![], true).unwrap();
        assert!(m.send_notice().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let mut m = make_meeting(MeetingType::ShareholderMeeting);
        m.send_notice().unwrap();
        let json = serde_json::to_string(&m).unwrap();
        let parsed: Meeting = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.meeting_id(), m.meeting_id());
        assert_eq!(parsed.status(), MeetingStatus::Noticed);
        assert_eq!(parsed.title(), m.title());
    }

    // ── Property-based-style FSM tests ───────────────────────────────────

    /// `send_notice` must only succeed from Draft; every other status must
    /// return an error.
    #[test]
    fn send_notice_only_allowed_from_draft() {
        // (label, setup)
        type SetupFn = fn(&mut Meeting);
        let non_draft: &[(&str, SetupFn)] = &[
            ("Noticed", |m| {
                m.send_notice().unwrap();
            }),
            ("Convened", |m| {
                m.send_notice().unwrap();
                m.convene(vec![GovernanceSeatId::new()], true).unwrap();
            }),
            ("Adjourned", |m| {
                m.send_notice().unwrap();
                m.convene(vec![GovernanceSeatId::new()], true).unwrap();
                m.adjourn().unwrap();
            }),
            ("Cancelled", |m| {
                m.cancel().unwrap();
            }),
        ];
        for (label, setup) in non_draft {
            let mut m = make_meeting(MeetingType::BoardMeeting);
            setup(&mut m);
            let result = m.send_notice();
            assert!(
                result.is_err(),
                "send_notice() should fail from status '{label}' but succeeded"
            );
        }

        // Confirm it works exactly once from Draft.
        let mut m = make_meeting(MeetingType::BoardMeeting);
        assert!(m.send_notice().is_ok());
    }

    /// `convene` must only succeed from Noticed; every other status must
    /// return an error (including calling it twice).
    #[test]
    fn convene_only_allowed_from_noticed() {
        type SetupFn = fn(&mut Meeting);
        let non_noticed: &[(&str, SetupFn)] = &[
            ("Draft",     |_m| {}),
            ("Convened",  |m| {
                m.send_notice().unwrap();
                m.convene(vec![GovernanceSeatId::new()], true).unwrap();
            }),
            ("Adjourned", |m| {
                m.send_notice().unwrap();
                m.convene(vec![GovernanceSeatId::new()], true).unwrap();
                m.adjourn().unwrap();
            }),
            ("Cancelled", |m| {
                m.cancel().unwrap();
            }),
        ];
        for (label, setup) in non_noticed {
            let mut m = make_meeting(MeetingType::BoardMeeting);
            setup(&mut m);
            let result = m.convene(vec![GovernanceSeatId::new()], true);
            assert!(
                result.is_err(),
                "convene() should fail from status '{label}' but succeeded"
            );
        }
    }

    /// Convening with an empty seat list records QuorumStatus::NotMet when
    /// quorum=false is passed — the meeting is still Convened but can_vote()
    /// returns false for non-WrittenConsent types.
    #[test]
    fn convene_with_no_seats_records_not_met() {
        let meeting_types = [
            MeetingType::BoardMeeting,
            MeetingType::ShareholderMeeting,
            MeetingType::MemberMeeting,
        ];
        for meeting_type in meeting_types {
            let mut m = make_meeting(meeting_type);
            m.send_notice().unwrap();
            m.convene(vec![], false).unwrap();
            assert_eq!(m.status(), MeetingStatus::Convened);
            assert_eq!(m.quorum_met(), QuorumStatus::NotMet);
            assert_eq!(m.present_seat_ids().len(), 0);
            assert!(
                !m.can_vote(),
                "can_vote() must be false when quorum is NotMet for {meeting_type:?}"
            );
        }
    }

    /// `adjourn` must only succeed from Convened.
    #[test]
    fn adjourn_only_allowed_from_convened() {
        type SetupFn = fn(&mut Meeting);
        let non_convened: &[(&str, SetupFn)] = &[
            ("Draft",     |_m| {}),
            ("Noticed",   |m| { m.send_notice().unwrap(); }),
            ("Cancelled", |m| { m.cancel().unwrap(); }),
        ];
        for (label, setup) in non_convened {
            let mut m = make_meeting(MeetingType::BoardMeeting);
            setup(&mut m);
            let result = m.adjourn();
            assert!(
                result.is_err(),
                "adjourn() should fail from status '{label}' but succeeded"
            );
        }

        // Also fails from Adjourned itself.
        let mut m = make_meeting(MeetingType::BoardMeeting);
        m.send_notice().unwrap();
        m.convene(vec![GovernanceSeatId::new()], true).unwrap();
        m.adjourn().unwrap();
        assert!(m.adjourn().is_err(), "adjourn() should fail when already Adjourned");
    }

    /// `cancel` must fail from Convened and Adjourned.
    #[test]
    fn cancel_blocked_once_convened_or_adjourned() {
        type SetupFn = fn(&mut Meeting);
        let blocked: &[(&str, SetupFn)] = &[
            ("Convened", |m| {
                m.send_notice().unwrap();
                m.convene(vec![GovernanceSeatId::new()], true).unwrap();
            }),
            ("Adjourned", |m| {
                m.send_notice().unwrap();
                m.convene(vec![GovernanceSeatId::new()], true).unwrap();
                m.adjourn().unwrap();
            }),
        ];
        for (label, setup) in blocked {
            let mut m = make_meeting(MeetingType::BoardMeeting);
            setup(&mut m);
            let result = m.cancel();
            assert!(
                result.is_err(),
                "cancel() should fail from status '{label}' but succeeded"
            );
        }
    }

    /// Full Noticed -> Convened -> Adjourned -> Reopen -> Adjourn cycle
    /// works correctly for all non-WrittenConsent meeting types.
    #[test]
    fn full_lifecycle_for_all_meeting_types() {
        let meeting_types = [
            MeetingType::BoardMeeting,
            MeetingType::ShareholderMeeting,
            MeetingType::MemberMeeting,
        ];
        for meeting_type in meeting_types {
            let mut m = make_meeting(meeting_type);
            assert_eq!(m.status(), MeetingStatus::Draft);

            m.send_notice().unwrap();
            assert_eq!(m.status(), MeetingStatus::Noticed);

            let seats: Vec<_> = (0..3).map(|_| GovernanceSeatId::new()).collect();
            m.convene(seats.clone(), true).unwrap();
            assert_eq!(m.status(), MeetingStatus::Convened);
            assert_eq!(m.present_seat_ids().len(), 3);
            assert!(m.convened_at().is_some());

            m.adjourn().unwrap();
            assert_eq!(m.status(), MeetingStatus::Adjourned);
            assert!(m.adjourned_at().is_some());

            // Reopen and adjourn again.
            m.reopen().unwrap();
            assert_eq!(m.status(), MeetingStatus::Convened);
            assert!(m.adjourned_at().is_none());

            m.adjourn().unwrap();
            assert_eq!(m.status(), MeetingStatus::Adjourned);
        }
    }

    /// `can_vote` is true when Convened + quorum met, false in all other
    /// status/quorum combinations for non-WrittenConsent meetings.
    #[test]
    fn can_vote_is_false_outside_of_convened_with_quorum() {
        // Cases where can_vote must be false.
        type SetupFn = fn(&mut Meeting);
        let false_cases: &[(&str, SetupFn)] = &[
            ("Draft",            |_m| {}),
            ("Noticed",          |m| { m.send_notice().unwrap(); }),
            ("Convened-NoQuorum",|m| {
                m.send_notice().unwrap();
                m.convene(vec![], false).unwrap();
            }),
            ("Adjourned",        |m| {
                m.send_notice().unwrap();
                m.convene(vec![GovernanceSeatId::new()], true).unwrap();
                m.adjourn().unwrap();
            }),
            ("Cancelled",        |m| { m.cancel().unwrap(); }),
        ];
        for (label, setup) in false_cases {
            let mut m = make_meeting(MeetingType::BoardMeeting);
            setup(&mut m);
            assert!(
                !m.can_vote(),
                "can_vote() should be false in case '{label}'"
            );
        }
    }
}
