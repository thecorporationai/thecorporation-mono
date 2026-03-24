//! Share transfer records between holders.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{CapTableId, EntityId, HolderId, ShareClassId, TransferId};
use super::types::{ShareCount, TransferStatus, TransferType};

/// A transfer of shares from one holder to another.
///
/// State machine:
/// ```text
/// Draft → PendingBoardApproval (approve)
///       → Approved              (approve, if board approval not needed)
///       → Executed              (execute)
///       → Denied                (deny)
///       → Cancelled             (cancel)
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareTransfer {
    pub transfer_id: TransferId,
    pub entity_id: EntityId,
    pub cap_table_id: CapTableId,
    pub from_holder_id: HolderId,
    pub to_holder_id: HolderId,
    pub share_class_id: ShareClassId,
    pub shares: ShareCount,
    pub transfer_type: TransferType,
    /// Agreed price per share in whole cents, if applicable.
    pub price_per_share_cents: Option<i64>,
    pub status: TransferStatus,
    pub created_at: DateTime<Utc>,
}

impl ShareTransfer {
    /// Create a new share transfer in `Draft` state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        cap_table_id: CapTableId,
        from_holder_id: HolderId,
        to_holder_id: HolderId,
        share_class_id: ShareClassId,
        shares: ShareCount,
        transfer_type: TransferType,
        price_per_share_cents: Option<i64>,
    ) -> Self {
        Self {
            transfer_id: TransferId::new(),
            entity_id,
            cap_table_id,
            from_holder_id,
            to_holder_id,
            share_class_id,
            shares,
            transfer_type,
            price_per_share_cents,
            status: TransferStatus::Draft,
            created_at: Utc::now(),
        }
    }

    /// Submit for board approval (`Draft` → `PendingBoardApproval`).
    pub fn approve(&mut self) -> Result<(), TransferError> {
        match self.status {
            TransferStatus::Draft => {
                self.status = TransferStatus::PendingBoardApproval;
                Ok(())
            }
            TransferStatus::PendingBoardApproval => {
                self.status = TransferStatus::Approved;
                Ok(())
            }
            _ => Err(TransferError::InvalidTransition {
                from: self.status.clone(),
                to: TransferStatus::Approved,
            }),
        }
    }

    /// Execute an approved transfer (`Approved` → `Executed`).
    pub fn execute(&mut self) -> Result<(), TransferError> {
        if self.status == TransferStatus::Approved {
            self.status = TransferStatus::Executed;
            Ok(())
        } else {
            Err(TransferError::InvalidTransition {
                from: self.status.clone(),
                to: TransferStatus::Executed,
            })
        }
    }

    /// Deny the transfer (`PendingBoardApproval` → `Denied`).
    pub fn deny(&mut self) -> Result<(), TransferError> {
        if self.status == TransferStatus::PendingBoardApproval {
            self.status = TransferStatus::Denied;
            Ok(())
        } else {
            Err(TransferError::InvalidTransition {
                from: self.status.clone(),
                to: TransferStatus::Denied,
            })
        }
    }

    /// Cancel a draft or pending transfer (`Draft` | `PendingBoardApproval` → `Cancelled`).
    pub fn cancel(&mut self) -> Result<(), TransferError> {
        match self.status {
            TransferStatus::Draft | TransferStatus::PendingBoardApproval => {
                self.status = TransferStatus::Cancelled;
                Ok(())
            }
            _ => Err(TransferError::InvalidTransition {
                from: self.status.clone(),
                to: TransferStatus::Cancelled,
            }),
        }
    }
}

/// Errors produced by [`ShareTransfer`] state transitions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransferError {
    #[error("cannot transition transfer from {from:?} to {to:?}")]
    InvalidTransition {
        from: TransferStatus,
        to: TransferStatus,
    },
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_transfer() -> ShareTransfer {
        ShareTransfer::new(
            EntityId::new(),
            CapTableId::new(),
            HolderId::new(),
            HolderId::new(),
            ShareClassId::new(),
            ShareCount::new(10_000),
            TransferType::SecondarySale,
            Some(5_00), // $5.00 per share
        )
    }

    fn make_transfer_of_type(transfer_type: TransferType) -> ShareTransfer {
        ShareTransfer::new(
            EntityId::new(),
            CapTableId::new(),
            HolderId::new(),
            HolderId::new(),
            ShareClassId::new(),
            ShareCount::new(1_000),
            transfer_type,
            None,
        )
    }

    // ── ShareTransfer::new() ──────────────────────────────────────────────────

    #[test]
    fn new_transfer_status_is_draft() {
        let t = make_transfer();
        assert_eq!(t.status, TransferStatus::Draft);
    }

    #[test]
    fn new_transfer_stores_shares() {
        let t = make_transfer();
        assert_eq!(t.shares, ShareCount::new(10_000));
    }

    #[test]
    fn new_transfer_stores_price_per_share() {
        let t = make_transfer();
        assert_eq!(t.price_per_share_cents, Some(5_00));
    }

    #[test]
    fn new_transfer_no_price_when_none() {
        let t = make_transfer_of_type(TransferType::Gift);
        assert!(t.price_per_share_cents.is_none());
    }

    #[test]
    fn new_transfer_has_unique_id() {
        let a = make_transfer();
        let b = make_transfer();
        assert_ne!(a.transfer_id, b.transfer_id);
    }

    // ── approve() ─────────────────────────────────────────────────────────────

    #[test]
    fn approve_from_draft_goes_to_pending_board_approval() {
        let mut t = make_transfer();
        t.approve().unwrap();
        assert_eq!(t.status, TransferStatus::PendingBoardApproval);
    }

    #[test]
    fn approve_from_pending_board_approval_goes_to_approved() {
        let mut t = make_transfer();
        t.approve().unwrap(); // Draft -> PendingBoardApproval
        t.approve().unwrap(); // PendingBoardApproval -> Approved
        assert_eq!(t.status, TransferStatus::Approved);
    }

    #[test]
    fn approve_from_approved_fails() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.approve().unwrap();
        assert!(matches!(
            t.approve(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn approve_from_executed_fails() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.approve().unwrap();
        t.execute().unwrap();
        assert!(matches!(
            t.approve(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn approve_from_denied_fails() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.deny().unwrap();
        assert!(matches!(
            t.approve(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    // ── execute() ─────────────────────────────────────────────────────────────

    #[test]
    fn execute_from_approved_to_executed() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.approve().unwrap();
        t.execute().unwrap();
        assert_eq!(t.status, TransferStatus::Executed);
    }

    #[test]
    fn execute_from_draft_fails() {
        let mut t = make_transfer();
        assert!(matches!(
            t.execute(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn execute_from_pending_board_approval_fails() {
        let mut t = make_transfer();
        t.approve().unwrap();
        assert!(matches!(
            t.execute(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    // ── deny() ────────────────────────────────────────────────────────────────

    #[test]
    fn deny_from_pending_board_approval_to_denied() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.deny().unwrap();
        assert_eq!(t.status, TransferStatus::Denied);
    }

    #[test]
    fn deny_from_draft_fails() {
        let mut t = make_transfer();
        assert!(matches!(
            t.deny(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn deny_from_approved_fails() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.approve().unwrap();
        assert!(matches!(
            t.deny(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    // ── cancel() ──────────────────────────────────────────────────────────────

    #[test]
    fn cancel_from_draft_to_cancelled() {
        let mut t = make_transfer();
        t.cancel().unwrap();
        assert_eq!(t.status, TransferStatus::Cancelled);
    }

    #[test]
    fn cancel_from_pending_board_approval_to_cancelled() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.cancel().unwrap();
        assert_eq!(t.status, TransferStatus::Cancelled);
    }

    #[test]
    fn cancel_from_executed_fails() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.approve().unwrap();
        t.execute().unwrap();
        assert!(matches!(
            t.cancel(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn cancel_from_approved_fails() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.approve().unwrap();
        assert!(matches!(
            t.cancel(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn cancel_from_denied_fails() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.deny().unwrap();
        assert!(matches!(
            t.cancel(),
            Err(TransferError::InvalidTransition { .. })
        ));
    }

    // ── Full lifecycle ─────────────────────────────────────────────────────────

    #[test]
    fn full_lifecycle_draft_to_executed() {
        let mut t = make_transfer();
        assert_eq!(t.status, TransferStatus::Draft);
        t.approve().unwrap();
        assert_eq!(t.status, TransferStatus::PendingBoardApproval);
        t.approve().unwrap();
        assert_eq!(t.status, TransferStatus::Approved);
        t.execute().unwrap();
        assert_eq!(t.status, TransferStatus::Executed);
    }

    #[test]
    fn full_lifecycle_draft_to_denied() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.deny().unwrap();
        assert_eq!(t.status, TransferStatus::Denied);
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn transfer_serde_roundtrip_draft() {
        let t = make_transfer();
        let json = serde_json::to_string(&t).unwrap();
        let de: ShareTransfer = serde_json::from_str(&json).unwrap();
        assert_eq!(de.transfer_id, t.transfer_id);
        assert_eq!(de.status, TransferStatus::Draft);
    }

    #[test]
    fn transfer_serde_roundtrip_executed() {
        let mut t = make_transfer();
        t.approve().unwrap();
        t.approve().unwrap();
        t.execute().unwrap();
        let json = serde_json::to_string(&t).unwrap();
        let de: ShareTransfer = serde_json::from_str(&json).unwrap();
        assert_eq!(de.status, TransferStatus::Executed);
    }

    // ── TransferType variants ─────────────────────────────────────────────────

    #[test]
    fn transfer_type_gift() {
        let t = make_transfer_of_type(TransferType::Gift);
        assert_eq!(t.transfer_type, TransferType::Gift);
    }

    #[test]
    fn transfer_type_trust_transfer() {
        let t = make_transfer_of_type(TransferType::TrustTransfer);
        assert_eq!(t.transfer_type, TransferType::TrustTransfer);
    }

    #[test]
    fn transfer_type_estate() {
        let t = make_transfer_of_type(TransferType::Estate);
        assert_eq!(t.transfer_type, TransferType::Estate);
    }

    #[test]
    fn transfer_type_other() {
        let t = make_transfer_of_type(TransferType::Other);
        assert_eq!(t.transfer_type, TransferType::Other);
    }
}
