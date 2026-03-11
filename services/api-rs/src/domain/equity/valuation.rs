//! Valuation record (stored as `valuations/{valuation_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::EquityError;
use super::types::{ValuationMethodology, ValuationStatus, ValuationType};
use crate::domain::ids::{
    AgendaItemId, ContactId, DocumentId, EntityId, MeetingId, ResolutionId, ValuationId,
    WorkspaceId,
};
use crate::domain::treasury::types::Cents;

/// A valuation record (409A, FMV, profits interest, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Valuation {
    valuation_id: ValuationId,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    valuation_type: ValuationType,
    effective_date: NaiveDate,
    expiration_date: Option<NaiveDate>,
    fmv_per_share_cents: Option<Cents>,
    enterprise_value_cents: Option<Cents>,
    hurdle_amount_cents: Option<Cents>,
    methodology: ValuationMethodology,
    provider_contact_id: Option<ContactId>,
    report_document_id: Option<DocumentId>,
    board_approval_meeting_id: Option<MeetingId>,
    board_approval_agenda_item_id: Option<AgendaItemId>,
    board_approval_resolution_id: Option<ResolutionId>,
    status: ValuationStatus,
    created_at: DateTime<Utc>,
}

impl Valuation {
    /// Create a new valuation.
    ///
    /// If the type is 409A or FMV, auto-sets expiration_date = effective_date + 365 days.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        valuation_id: ValuationId,
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        valuation_type: ValuationType,
        effective_date: NaiveDate,
        fmv_per_share_cents: Option<Cents>,
        enterprise_value_cents: Option<Cents>,
        hurdle_amount_cents: Option<Cents>,
        methodology: ValuationMethodology,
        provider_contact_id: Option<ContactId>,
        report_document_id: Option<DocumentId>,
    ) -> Self {
        let expiration_date = match valuation_type {
            ValuationType::FourOhNineA | ValuationType::FairMarketValue => {
                Some(effective_date + chrono::Duration::days(365))
            }
            _ => None,
        };

        Self {
            valuation_id,
            entity_id,
            workspace_id,
            valuation_type,
            effective_date,
            expiration_date,
            fmv_per_share_cents,
            enterprise_value_cents,
            hurdle_amount_cents,
            methodology,
            provider_contact_id,
            report_document_id,
            board_approval_meeting_id: None,
            board_approval_agenda_item_id: None,
            board_approval_resolution_id: None,
            status: ValuationStatus::Draft,
            created_at: Utc::now(),
        }
    }

    /// Submit for approval. Must be Draft.
    pub fn submit_for_approval(&mut self) -> Result<(), EquityError> {
        if self.status != ValuationStatus::Draft {
            return Err(EquityError::InvalidValuationTransition {
                from: self.status,
                to: ValuationStatus::PendingApproval,
            });
        }
        self.status = ValuationStatus::PendingApproval;
        Ok(())
    }

    /// Record the governance artifacts created when this valuation is submitted.
    pub fn record_submission_for_approval(
        &mut self,
        meeting_id: MeetingId,
        agenda_item_id: AgendaItemId,
    ) {
        self.board_approval_meeting_id = Some(meeting_id);
        self.board_approval_agenda_item_id = Some(agenda_item_id);
    }

    /// Approve the valuation. Must be PendingApproval.
    pub fn approve(&mut self, resolution_id: Option<ResolutionId>) -> Result<(), EquityError> {
        if self.status != ValuationStatus::PendingApproval {
            return Err(EquityError::InvalidValuationTransition {
                from: self.status,
                to: ValuationStatus::Approved,
            });
        }
        self.board_approval_resolution_id = resolution_id;
        self.status = ValuationStatus::Approved;
        Ok(())
    }

    /// Expire the valuation. Must be Approved.
    pub fn expire(&mut self) -> Result<(), EquityError> {
        if self.status != ValuationStatus::Approved {
            return Err(EquityError::InvalidValuationTransition {
                from: self.status,
                to: ValuationStatus::Expired,
            });
        }
        self.status = ValuationStatus::Expired;
        Ok(())
    }

    /// Supersede the valuation. Must be Approved.
    pub fn supersede(&mut self) -> Result<(), EquityError> {
        if self.status != ValuationStatus::Approved {
            return Err(EquityError::InvalidValuationTransition {
                from: self.status,
                to: ValuationStatus::Superseded,
            });
        }
        self.status = ValuationStatus::Superseded;
        Ok(())
    }

    /// Check whether this is a current, valid 409A valuation.
    pub fn is_current_409a(&self) -> bool {
        self.valuation_type == ValuationType::FourOhNineA
            && self.status == ValuationStatus::Approved
            && self
                .expiration_date
                .map(|exp| Utc::now().date_naive() <= exp)
                .unwrap_or(false)
    }

    pub fn valuation_id(&self) -> ValuationId {
        self.valuation_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn valuation_type(&self) -> ValuationType {
        self.valuation_type
    }

    pub fn effective_date(&self) -> NaiveDate {
        self.effective_date
    }

    pub fn expiration_date(&self) -> Option<NaiveDate> {
        self.expiration_date
    }

    pub fn fmv_per_share_cents(&self) -> Option<Cents> {
        self.fmv_per_share_cents
    }

    pub fn enterprise_value_cents(&self) -> Option<Cents> {
        self.enterprise_value_cents
    }

    pub fn hurdle_amount_cents(&self) -> Option<Cents> {
        self.hurdle_amount_cents
    }

    pub fn methodology(&self) -> ValuationMethodology {
        self.methodology
    }

    pub fn provider_contact_id(&self) -> Option<ContactId> {
        self.provider_contact_id
    }

    pub fn report_document_id(&self) -> Option<DocumentId> {
        self.report_document_id
    }

    pub fn board_approval_meeting_id(&self) -> Option<MeetingId> {
        self.board_approval_meeting_id
    }

    pub fn board_approval_agenda_item_id(&self) -> Option<AgendaItemId> {
        self.board_approval_agenda_item_id
    }

    pub fn board_approval_resolution_id(&self) -> Option<ResolutionId> {
        self.board_approval_resolution_id
    }

    pub fn status(&self) -> ValuationStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_409a() -> Valuation {
        Valuation::new(
            ValuationId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            ValuationType::FourOhNineA,
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            Some(Cents::new(1_00)),
            Some(Cents::new(5_000_000_00)),
            None,
            ValuationMethodology::Market,
            None,
            None,
        )
    }

    #[test]
    fn new_valuation() {
        let v = make_409a();
        assert_eq!(v.status(), ValuationStatus::Draft);
        assert_eq!(v.valuation_type(), ValuationType::FourOhNineA);
    }

    #[test]
    fn auto_expiration_409a() {
        let v = make_409a();
        let exp = v.expiration_date().expect("409A should have expiration");
        let expected = NaiveDate::from_ymd_opt(2027, 1, 15).unwrap();
        assert_eq!(exp, expected);
    }

    #[test]
    fn fmv_uses_one_year_expiration() {
        let v = Valuation::new(
            ValuationId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            ValuationType::FairMarketValue,
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            None,
            None,
            None,
            ValuationMethodology::Income,
            None,
            None,
        );
        let exp = v.expiration_date().expect("FMV should have expiration");
        let expected = NaiveDate::from_ymd_opt(2027, 6, 1).unwrap();
        assert_eq!(exp, expected);
    }

    #[test]
    fn approve_workflow() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        assert_eq!(v.status(), ValuationStatus::PendingApproval);

        let res_id = ResolutionId::new();
        v.approve(Some(res_id)).unwrap();
        assert_eq!(v.status(), ValuationStatus::Approved);
        assert_eq!(v.board_approval_resolution_id(), Some(res_id));
    }

    #[test]
    fn approve_requires_pending() {
        let mut v = make_409a();
        let result = v.approve(None);
        assert!(result.is_err());
    }

    #[test]
    fn is_current_409a() {
        let mut v = make_409a();
        assert!(!v.is_current_409a()); // Draft

        v.submit_for_approval().unwrap();
        v.approve(None).unwrap();
        // Approved with future expiration
        assert!(v.is_current_409a());
    }

    #[test]
    fn expire_and_supersede() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve(None).unwrap();
        v.expire().unwrap();
        assert_eq!(v.status(), ValuationStatus::Expired);

        // Can't supersede an expired one
        let result = v.supersede();
        assert!(result.is_err());
    }

    #[test]
    fn supersede() {
        let mut v = make_409a();
        v.submit_for_approval().unwrap();
        v.approve(None).unwrap();
        v.supersede().unwrap();
        assert_eq!(v.status(), ValuationStatus::Superseded);
    }
}
