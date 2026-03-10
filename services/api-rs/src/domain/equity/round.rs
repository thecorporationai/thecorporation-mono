//! Equity round terms and state (stored as `cap-table/rounds/{equity_round_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::error::EquityError;
use crate::domain::ids::{
    ContactId, EquityRoundId, EquityRuleSetId, InstrumentId, LegalEntityId, MeetingId, ResolutionId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EquityRoundStatus {
    Draft,
    Open,
    BoardApproved,
    Accepted,
    Closed,
    Cancelled,
}

impl fmt::Display for EquityRoundStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Open => write!(f, "open"),
            Self::BoardApproved => write!(f, "board_approved"),
            Self::Accepted => write!(f, "accepted"),
            Self::Closed => write!(f, "closed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityRound {
    equity_round_id: EquityRoundId,
    issuer_legal_entity_id: LegalEntityId,
    name: String,
    pre_money_cents: Option<i64>,
    round_price_cents: Option<i64>,
    target_raise_cents: Option<i64>,
    /// Instrument that receives converted/new shares in this round.
    conversion_target_instrument_id: Option<InstrumentId>,
    rule_set_id: Option<EquityRuleSetId>,
    board_approval_meeting_id: Option<MeetingId>,
    board_approval_resolution_id: Option<ResolutionId>,
    board_approved_at: Option<DateTime<Utc>>,
    accepted_by_contact_id: Option<ContactId>,
    accepted_at: Option<DateTime<Utc>>,
    metadata: serde_json::Value,
    status: EquityRoundStatus,
    created_at: DateTime<Utc>,
}

impl EquityRound {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        equity_round_id: EquityRoundId,
        issuer_legal_entity_id: LegalEntityId,
        name: String,
        pre_money_cents: Option<i64>,
        round_price_cents: Option<i64>,
        target_raise_cents: Option<i64>,
        conversion_target_instrument_id: Option<InstrumentId>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            equity_round_id,
            issuer_legal_entity_id,
            name,
            pre_money_cents,
            round_price_cents,
            target_raise_cents,
            conversion_target_instrument_id,
            rule_set_id: None,
            board_approval_meeting_id: None,
            board_approval_resolution_id: None,
            board_approved_at: None,
            accepted_by_contact_id: None,
            accepted_at: None,
            metadata,
            status: EquityRoundStatus::Draft,
            created_at: Utc::now(),
        }
    }

    pub fn equity_round_id(&self) -> EquityRoundId {
        self.equity_round_id
    }

    pub fn issuer_legal_entity_id(&self) -> LegalEntityId {
        self.issuer_legal_entity_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn pre_money_cents(&self) -> Option<i64> {
        self.pre_money_cents
    }

    pub fn round_price_cents(&self) -> Option<i64> {
        self.round_price_cents
    }

    pub fn target_raise_cents(&self) -> Option<i64> {
        self.target_raise_cents
    }

    pub fn conversion_target_instrument_id(&self) -> Option<InstrumentId> {
        self.conversion_target_instrument_id
    }

    pub fn rule_set_id(&self) -> Option<EquityRuleSetId> {
        self.rule_set_id
    }

    pub fn board_approval_meeting_id(&self) -> Option<MeetingId> {
        self.board_approval_meeting_id
    }

    pub fn board_approval_resolution_id(&self) -> Option<ResolutionId> {
        self.board_approval_resolution_id
    }

    pub fn board_approved_at(&self) -> Option<DateTime<Utc>> {
        self.board_approved_at
    }

    pub fn accepted_by_contact_id(&self) -> Option<ContactId> {
        self.accepted_by_contact_id
    }

    pub fn accepted_at(&self) -> Option<DateTime<Utc>> {
        self.accepted_at
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    pub fn status(&self) -> EquityRoundStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn apply_terms(&mut self, rule_set_id: EquityRuleSetId) -> Result<(), EquityError> {
        if self.status != EquityRoundStatus::Draft {
            return Err(EquityError::InvalidRoundTransition {
                from: self.status,
                to: EquityRoundStatus::Open,
            });
        }
        self.rule_set_id = Some(rule_set_id);
        self.status = EquityRoundStatus::Open;
        Ok(())
    }

    pub fn record_board_approval(
        &mut self,
        meeting_id: MeetingId,
        resolution_id: ResolutionId,
    ) -> Result<(), EquityError> {
        if !matches!(self.status, EquityRoundStatus::Draft | EquityRoundStatus::Open) {
            return Err(EquityError::InvalidRoundTransition {
                from: self.status,
                to: EquityRoundStatus::BoardApproved,
            });
        }
        self.board_approval_meeting_id = Some(meeting_id);
        self.board_approval_resolution_id = Some(resolution_id);
        self.board_approved_at = Some(Utc::now());
        self.status = EquityRoundStatus::BoardApproved;
        Ok(())
    }

    pub fn accept(&mut self, accepted_by_contact_id: Option<ContactId>) -> Result<(), EquityError> {
        if self.status != EquityRoundStatus::BoardApproved {
            return Err(EquityError::InvalidRoundTransition {
                from: self.status,
                to: EquityRoundStatus::Accepted,
            });
        }
        self.accepted_by_contact_id = accepted_by_contact_id;
        self.accepted_at = Some(Utc::now());
        self.status = EquityRoundStatus::Accepted;
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), EquityError> {
        if self.status != EquityRoundStatus::Accepted {
            return Err(EquityError::InvalidRoundTransition {
                from: self.status,
                to: EquityRoundStatus::Closed,
            });
        }
        self.status = EquityRoundStatus::Closed;
        Ok(())
    }

    /// Close a round directly from Draft status, skipping the full governance
    /// lifecycle. Used by the staged equity round flow.
    pub fn close_from_draft(&mut self) -> Result<(), EquityError> {
        if self.status != EquityRoundStatus::Draft {
            return Err(EquityError::InvalidRoundTransition {
                from: self.status,
                to: EquityRoundStatus::Closed,
            });
        }
        self.status = EquityRoundStatus::Closed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_round() -> EquityRound {
        EquityRound::new(
            EquityRoundId::new(),
            LegalEntityId::new(),
            "Series A".to_owned(),
            Some(100_000_000),
            Some(100),
            Some(20_000_000),
            Some(InstrumentId::new()),
            serde_json::json!({}),
        )
    }

    #[test]
    fn round_happy_path_lifecycle() {
        let mut round = make_round();
        round.apply_terms(EquityRuleSetId::new()).unwrap();
        assert_eq!(round.status(), EquityRoundStatus::Open);

        round
            .record_board_approval(MeetingId::new(), ResolutionId::new())
            .unwrap();
        assert_eq!(round.status(), EquityRoundStatus::BoardApproved);

        round.accept(None).unwrap();
        assert_eq!(round.status(), EquityRoundStatus::Accepted);

        round.close().unwrap();
        assert_eq!(round.status(), EquityRoundStatus::Closed);
    }

    #[test]
    fn draft_round_can_be_board_approved_for_simple_issuance() {
        let mut round = make_round();
        round
            .record_board_approval(MeetingId::new(), ResolutionId::new())
            .unwrap();
        assert_eq!(round.status(), EquityRoundStatus::BoardApproved);
    }

    #[test]
    fn round_cannot_accept_before_board_approval() {
        let mut round = make_round();
        round.apply_terms(EquityRuleSetId::new()).unwrap();
        let err = round.accept(None).unwrap_err();
        assert!(matches!(
            err,
            EquityError::InvalidRoundTransition {
                from: EquityRoundStatus::Open,
                to: EquityRoundStatus::Accepted
            }
        ));
    }

    #[test]
    fn round_close_from_draft() {
        let mut round = make_round();
        assert_eq!(round.status(), EquityRoundStatus::Draft);
        round.close_from_draft().unwrap();
        assert_eq!(round.status(), EquityRoundStatus::Closed);
    }

    #[test]
    fn round_close_from_draft_rejects_non_draft() {
        let mut round = make_round();
        round.apply_terms(EquityRuleSetId::new()).unwrap();
        let err = round.close_from_draft().unwrap_err();
        assert!(matches!(
            err,
            EquityError::InvalidRoundTransition {
                from: EquityRoundStatus::Open,
                to: EquityRoundStatus::Closed
            }
        ));
    }

    #[test]
    fn round_cannot_close_before_accept() {
        let mut round = make_round();
        round.apply_terms(EquityRuleSetId::new()).unwrap();
        round
            .record_board_approval(MeetingId::new(), ResolutionId::new())
            .unwrap();
        let err = round.close().unwrap_err();
        assert!(matches!(
            err,
            EquityError::InvalidRoundTransition {
                from: EquityRoundStatus::BoardApproved,
                to: EquityRoundStatus::Closed
            }
        ));
    }
}
