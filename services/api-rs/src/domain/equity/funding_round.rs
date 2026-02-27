//! Funding round record (stored as `funding-rounds/{funding_round_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::EquityError;
use super::types::{FundingRoundStatus, ShareCount};
use crate::domain::ids::{ContactId, EntityId, FundingRoundId};
use crate::domain::treasury::types::Cents;

/// A funding round (seed, Series A, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRound {
    funding_round_id: FundingRoundId,
    entity_id: EntityId,
    round_name: String,
    pre_money_valuation_cents: Option<Cents>,
    post_money_valuation_cents: Option<Cents>,
    price_per_share_cents: Option<Cents>,
    shares_issued: Option<ShareCount>,
    lead_investor_id: Option<ContactId>,
    status: FundingRoundStatus,
    closing_date: Option<NaiveDate>,
    created_at: DateTime<Utc>,
}

impl FundingRound {
    /// Create a new funding round in TermSheet status.
    pub fn new(
        funding_round_id: FundingRoundId,
        entity_id: EntityId,
        round_name: String,
        pre_money_valuation_cents: Option<Cents>,
        lead_investor_id: Option<ContactId>,
    ) -> Self {
        Self {
            funding_round_id,
            entity_id,
            round_name,
            pre_money_valuation_cents,
            post_money_valuation_cents: None,
            price_per_share_cents: None,
            shares_issued: None,
            lead_investor_id,
            status: FundingRoundStatus::TermSheet,
            closing_date: None,
            created_at: Utc::now(),
        }
    }

    /// Advance round status through the FSM.
    ///
    /// Valid transitions: TermSheet -> Diligence -> Closing -> Closed.
    pub fn advance(&mut self, to: FundingRoundStatus) -> Result<(), EquityError> {
        let valid = matches!(
            (self.status, to),
            (FundingRoundStatus::TermSheet, FundingRoundStatus::Diligence)
                | (FundingRoundStatus::Diligence, FundingRoundStatus::Closing)
                | (FundingRoundStatus::Closing, FundingRoundStatus::Closed)
        );
        if !valid {
            return Err(EquityError::InvalidFundingRoundTransition {
                from: self.status,
                to,
            });
        }
        self.status = to;
        Ok(())
    }

    /// Close the round with final terms. Must be Closing -> Closed.
    pub fn close(
        &mut self,
        post_money: Cents,
        price: Cents,
        shares: ShareCount,
    ) -> Result<(), EquityError> {
        if self.status != FundingRoundStatus::Closing {
            return Err(EquityError::InvalidFundingRoundTransition {
                from: self.status,
                to: FundingRoundStatus::Closed,
            });
        }
        self.post_money_valuation_cents = Some(post_money);
        self.price_per_share_cents = Some(price);
        self.shares_issued = Some(shares);
        self.closing_date = Some(Utc::now().date_naive());
        self.status = FundingRoundStatus::Closed;
        Ok(())
    }

    pub fn funding_round_id(&self) -> FundingRoundId {
        self.funding_round_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn round_name(&self) -> &str {
        &self.round_name
    }

    pub fn pre_money_valuation_cents(&self) -> Option<Cents> {
        self.pre_money_valuation_cents
    }

    pub fn post_money_valuation_cents(&self) -> Option<Cents> {
        self.post_money_valuation_cents
    }

    pub fn price_per_share_cents(&self) -> Option<Cents> {
        self.price_per_share_cents
    }

    pub fn shares_issued(&self) -> Option<ShareCount> {
        self.shares_issued
    }

    pub fn lead_investor_id(&self) -> Option<ContactId> {
        self.lead_investor_id
    }

    pub fn status(&self) -> FundingRoundStatus {
        self.status
    }

    pub fn closing_date(&self) -> Option<NaiveDate> {
        self.closing_date
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_round() -> FundingRound {
        FundingRound::new(
            FundingRoundId::new(),
            EntityId::new(),
            "Seed".to_string(),
            Some(Cents::new(5_000_000_00)),
            None,
        )
    }

    #[test]
    fn new_round() {
        let r = make_round();
        assert_eq!(r.status(), FundingRoundStatus::TermSheet);
        assert_eq!(r.round_name(), "Seed");
    }

    #[test]
    fn advance_fsm() {
        let mut r = make_round();
        r.advance(FundingRoundStatus::Diligence).unwrap();
        assert_eq!(r.status(), FundingRoundStatus::Diligence);

        r.advance(FundingRoundStatus::Closing).unwrap();
        assert_eq!(r.status(), FundingRoundStatus::Closing);
    }

    #[test]
    fn advance_invalid() {
        let mut r = make_round();
        let result = r.advance(FundingRoundStatus::Closing);
        assert!(result.is_err());
    }

    #[test]
    fn close_round() {
        let mut r = make_round();
        r.advance(FundingRoundStatus::Diligence).unwrap();
        r.advance(FundingRoundStatus::Closing).unwrap();
        r.close(
            Cents::new(7_000_000_00),
            Cents::new(1_00),
            ShareCount::new(2_000_000),
        )
        .unwrap();
        assert_eq!(r.status(), FundingRoundStatus::Closed);
        assert_eq!(
            r.post_money_valuation_cents(),
            Some(Cents::new(7_000_000_00))
        );
        assert_eq!(r.shares_issued(), Some(ShareCount::new(2_000_000)));
        assert!(r.closing_date().is_some());
    }

    #[test]
    fn close_requires_closing_status() {
        let mut r = make_round();
        let result = r.close(
            Cents::new(7_000_000_00),
            Cents::new(1_00),
            ShareCount::new(2_000_000),
        );
        assert!(result.is_err());
    }
}
