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
}
