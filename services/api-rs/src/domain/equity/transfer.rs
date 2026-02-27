//! Share transfer record (stored as `cap-table/transfers/{transfer_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::EquityError;
use super::types::{BylawsReviewStatus, GoverningDocType, ShareCount, TransfereeRights, TransferStatus, TransferType};
use crate::domain::ids::{
    ContactId, EntityId, ResolutionId, ShareClassId, TransferId, ValuationId, WorkspaceId,
};
use crate::domain::treasury::types::Cents;

/// A share transfer between parties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareTransfer {
    transfer_id: TransferId,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    share_class_id: ShareClassId,
    from_contact_id: ContactId,
    to_contact_id: ContactId,
    transfer_type: TransferType,
    share_count: ShareCount,
    price_per_share_cents: Option<Cents>,
    relationship_to_holder: Option<String>,
    governing_doc_type: GoverningDocType,
    bylaws_review_status: Option<BylawsReviewStatus>,
    bylaws_review_notes: Option<String>,
    reviewed_by: Option<String>,
    transferee_rights: TransfereeRights,
    rofr_offered: bool,
    rofr_waived: bool,
    board_approval_resolution_id: Option<ResolutionId>,
    valuation_id: Option<ValuationId>,
    status: TransferStatus,
    created_at: DateTime<Utc>,
}

impl ShareTransfer {
    /// Create a new share transfer in Draft status.
    ///
    /// Returns `Err` if `share_count` is not positive or `from_contact_id == to_contact_id`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transfer_id: TransferId,
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        share_class_id: ShareClassId,
        from_contact_id: ContactId,
        to_contact_id: ContactId,
        transfer_type: TransferType,
        share_count: ShareCount,
        price_per_share_cents: Option<Cents>,
        relationship_to_holder: Option<String>,
        governing_doc_type: GoverningDocType,
        transferee_rights: TransfereeRights,
    ) -> Result<Self, EquityError> {
        if share_count.raw() <= 0 {
            return Err(EquityError::Validation("share_count must be positive".into()));
        }
        if from_contact_id == to_contact_id {
            return Err(EquityError::Validation("from_holder and to_holder must be different".into()));
        }
        Ok(Self {
            transfer_id,
            entity_id,
            workspace_id,
            share_class_id,
            from_contact_id,
            to_contact_id,
            transfer_type,
            share_count,
            price_per_share_cents,
            relationship_to_holder,
            governing_doc_type,
            bylaws_review_status: None,
            bylaws_review_notes: None,
            reviewed_by: None,
            transferee_rights,
            rofr_offered: false,
            rofr_waived: false,
            board_approval_resolution_id: None,
            valuation_id: None,
            status: TransferStatus::Draft,
            created_at: Utc::now(),
        })
    }

    /// Submit for bylaws review. Must be Draft.
    pub fn submit_for_review(&mut self) -> Result<(), EquityError> {
        if self.status != TransferStatus::Draft {
            return Err(EquityError::InvalidTransferTransition {
                from: self.status,
                to: TransferStatus::PendingBylawsReview,
            });
        }
        self.status = TransferStatus::PendingBylawsReview;
        Ok(())
    }

    /// Record bylaws review result. Must be PendingBylawsReview.
    ///
    /// If denied -> Denied.
    /// If approved and notes contains "rofr_not_required" -> PendingBoardApproval.
    /// If approved -> PendingRofr.
    pub fn record_bylaws_review(
        &mut self,
        approved: bool,
        notes: String,
        reviewer: String,
    ) -> Result<(), EquityError> {
        if self.status != TransferStatus::PendingBylawsReview {
            return Err(EquityError::InvalidTransferTransition {
                from: self.status,
                to: TransferStatus::PendingRofr,
            });
        }
        self.bylaws_review_notes = Some(notes.clone());
        self.reviewed_by = Some(reviewer);

        if !approved {
            self.bylaws_review_status = Some(BylawsReviewStatus::Denied);
            self.status = TransferStatus::Denied;
        } else if notes.contains("rofr_not_required") {
            self.bylaws_review_status = Some(BylawsReviewStatus::Approved);
            self.status = TransferStatus::PendingBoardApproval;
        } else {
            self.bylaws_review_status = Some(BylawsReviewStatus::Approved);
            self.status = TransferStatus::PendingRofr;
        }
        Ok(())
    }

    /// Record ROFR decision. Must be PendingRofr -> PendingBoardApproval.
    pub fn record_rofr_decision(
        &mut self,
        offered: bool,
        waived: bool,
    ) -> Result<(), EquityError> {
        if self.status != TransferStatus::PendingRofr {
            return Err(EquityError::InvalidTransferTransition {
                from: self.status,
                to: TransferStatus::PendingBoardApproval,
            });
        }
        self.rofr_offered = offered;
        self.rofr_waived = waived;
        self.status = TransferStatus::PendingBoardApproval;
        Ok(())
    }

    /// Approve the transfer. Must be PendingBoardApproval.
    pub fn approve(
        &mut self,
        resolution_id: Option<ResolutionId>,
    ) -> Result<(), EquityError> {
        if self.status != TransferStatus::PendingBoardApproval {
            return Err(EquityError::InvalidTransferTransition {
                from: self.status,
                to: TransferStatus::Approved,
            });
        }
        self.board_approval_resolution_id = resolution_id;
        self.status = TransferStatus::Approved;
        Ok(())
    }

    /// Execute the transfer. Must be Approved -> Executed.
    pub fn execute(&mut self) -> Result<(), EquityError> {
        if self.status != TransferStatus::Approved {
            return Err(EquityError::InvalidTransferTransition {
                from: self.status,
                to: TransferStatus::Executed,
            });
        }
        self.status = TransferStatus::Executed;
        Ok(())
    }

    /// Cancel the transfer. Must not be Executed/Cancelled/Denied.
    pub fn cancel(&mut self) -> Result<(), EquityError> {
        match self.status {
            TransferStatus::Executed | TransferStatus::Cancelled | TransferStatus::Denied => {
                Err(EquityError::InvalidTransferTransition {
                    from: self.status,
                    to: TransferStatus::Cancelled,
                })
            }
            _ => {
                self.status = TransferStatus::Cancelled;
                Ok(())
            }
        }
    }

    /// Set the valuation used for this transfer (gift/estate).
    pub fn set_valuation_id(&mut self, valuation_id: ValuationId) {
        self.valuation_id = Some(valuation_id);
    }

    pub fn transfer_id(&self) -> TransferId {
        self.transfer_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn share_class_id(&self) -> ShareClassId {
        self.share_class_id
    }

    pub fn sender_contact_id(&self) -> ContactId {
        self.from_contact_id
    }

    pub fn to_contact_id(&self) -> ContactId {
        self.to_contact_id
    }

    pub fn transfer_type(&self) -> TransferType {
        self.transfer_type
    }

    pub fn share_count(&self) -> ShareCount {
        self.share_count
    }

    pub fn price_per_share_cents(&self) -> Option<Cents> {
        self.price_per_share_cents
    }

    pub fn relationship_to_holder(&self) -> Option<&str> {
        self.relationship_to_holder.as_deref()
    }

    pub fn governing_doc_type(&self) -> GoverningDocType {
        self.governing_doc_type
    }

    pub fn bylaws_review_status(&self) -> Option<BylawsReviewStatus> {
        self.bylaws_review_status
    }

    pub fn bylaws_review_notes(&self) -> Option<&str> {
        self.bylaws_review_notes.as_deref()
    }

    pub fn reviewed_by(&self) -> Option<&str> {
        self.reviewed_by.as_deref()
    }

    pub fn transferee_rights(&self) -> TransfereeRights {
        self.transferee_rights
    }

    pub fn rofr_offered(&self) -> bool {
        self.rofr_offered
    }

    pub fn rofr_waived(&self) -> bool {
        self.rofr_waived
    }

    pub fn board_approval_resolution_id(&self) -> Option<ResolutionId> {
        self.board_approval_resolution_id
    }

    pub fn valuation_id(&self) -> Option<ValuationId> {
        self.valuation_id
    }

    pub fn status(&self) -> TransferStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_transfer() -> ShareTransfer {
        let from = ContactId::new();
        let to = ContactId::new();
        ShareTransfer::new(
            TransferId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            ShareClassId::new(),
            from,
            to,
            TransferType::SecondarySale,
            ShareCount::new(1000),
            Some(Cents::new(5_00)),
            None,
            GoverningDocType::Bylaws,
            TransfereeRights::FullMember,
        )
        .unwrap()
    }

    #[test]
    fn new_transfer() {
        let t = make_transfer();
        assert_eq!(t.status(), TransferStatus::Draft);
        assert_eq!(t.share_count().raw(), 1000);
    }

    #[test]
    fn full_fsm_workflow() {
        let mut t = make_transfer();

        // Draft -> PendingBylawsReview
        t.submit_for_review().unwrap();
        assert_eq!(t.status(), TransferStatus::PendingBylawsReview);

        // PendingBylawsReview -> PendingRofr
        t.record_bylaws_review(true, "approved".to_string(), "counsel".to_string())
            .unwrap();
        assert_eq!(t.status(), TransferStatus::PendingRofr);

        // PendingRofr -> PendingBoardApproval
        t.record_rofr_decision(true, true).unwrap();
        assert_eq!(t.status(), TransferStatus::PendingBoardApproval);

        // PendingBoardApproval -> Approved
        t.approve(Some(ResolutionId::new())).unwrap();
        assert_eq!(t.status(), TransferStatus::Approved);

        // Approved -> Executed
        t.execute().unwrap();
        assert_eq!(t.status(), TransferStatus::Executed);
    }

    #[test]
    fn bylaws_review_skip_rofr() {
        let mut t = make_transfer();
        t.submit_for_review().unwrap();
        t.record_bylaws_review(
            true,
            "approved, rofr_not_required".to_string(),
            "counsel".to_string(),
        )
        .unwrap();
        assert_eq!(t.status(), TransferStatus::PendingBoardApproval);
    }

    #[test]
    fn bylaws_review_denied() {
        let mut t = make_transfer();
        t.submit_for_review().unwrap();
        t.record_bylaws_review(false, "violates restrictions".to_string(), "counsel".to_string())
            .unwrap();
        assert_eq!(t.status(), TransferStatus::Denied);
    }

    #[test]
    fn cancel_from_draft() {
        let mut t = make_transfer();
        t.cancel().unwrap();
        assert_eq!(t.status(), TransferStatus::Cancelled);
    }

    #[test]
    fn cancel_from_pending() {
        let mut t = make_transfer();
        t.submit_for_review().unwrap();
        t.cancel().unwrap();
        assert_eq!(t.status(), TransferStatus::Cancelled);
    }

    #[test]
    fn cannot_cancel_executed() {
        let mut t = make_transfer();
        t.submit_for_review().unwrap();
        t.record_bylaws_review(true, "rofr_not_required".to_string(), "counsel".to_string())
            .unwrap();
        t.approve(None).unwrap();
        t.execute().unwrap();
        let result = t.cancel();
        assert!(result.is_err());
    }

    #[test]
    fn cannot_cancel_denied() {
        let mut t = make_transfer();
        t.submit_for_review().unwrap();
        t.record_bylaws_review(false, "denied".to_string(), "counsel".to_string())
            .unwrap();
        let result = t.cancel();
        assert!(result.is_err());
    }
}
