//! SAFE (Simple Agreement for Future Equity) note records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{SafeStatus, SafeType};
use crate::ids::{CapTableId, ContactId, EntityId, SafeNoteId};

/// A SAFE note issued to an investor.
///
/// State machine: `Issued` → `Converted` (via [`convert`]) or `Cancelled`
/// (via [`cancel`]). Both transitions are irreversible.
///
/// [`convert`]: SafeNote::convert
/// [`cancel`]: SafeNote::cancel
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeNote {
    pub safe_note_id: SafeNoteId,
    pub entity_id: EntityId,
    pub cap_table_id: CapTableId,
    /// Contact record for the investor.
    pub investor_contact_id: ContactId,
    pub investor_name: String,
    pub safe_type: SafeType,
    /// Principal investment amount in whole cents.
    pub investment_amount_cents: i64,
    /// Optional valuation cap in whole cents.
    pub valuation_cap_cents: Option<i64>,
    /// Optional discount percentage, expressed as an integer (e.g. `20` = 20%).
    pub discount_percent: Option<u32>,
    pub status: SafeStatus,
    pub converted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl SafeNote {
    /// Create a new SAFE note in the `Issued` state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        cap_table_id: CapTableId,
        investor_contact_id: ContactId,
        investor_name: impl Into<String>,
        safe_type: SafeType,
        investment_amount_cents: i64,
        valuation_cap_cents: Option<i64>,
        discount_percent: Option<u32>,
    ) -> Self {
        Self {
            safe_note_id: SafeNoteId::new(),
            entity_id,
            cap_table_id,
            investor_contact_id,
            investor_name: investor_name.into(),
            safe_type,
            investment_amount_cents,
            valuation_cap_cents,
            discount_percent,
            status: SafeStatus::Issued,
            converted_at: None,
            created_at: Utc::now(),
        }
    }

    /// Transition from `Issued` to `Converted`.
    ///
    /// Returns `Err` if the note is not currently in the `Issued` state.
    pub fn convert(&mut self) -> Result<(), SafeNoteError> {
        match self.status {
            SafeStatus::Issued => {
                self.status = SafeStatus::Converted;
                self.converted_at = Some(Utc::now());
                Ok(())
            }
            SafeStatus::Converted => Err(SafeNoteError::AlreadyConverted),
            SafeStatus::Cancelled => Err(SafeNoteError::AlreadyCancelled),
        }
    }

    /// Transition from `Issued` to `Cancelled`.
    ///
    /// Returns `Err` if the note is not currently in the `Issued` state.
    pub fn cancel(&mut self) -> Result<(), SafeNoteError> {
        match self.status {
            SafeStatus::Issued => {
                self.status = SafeStatus::Cancelled;
                Ok(())
            }
            SafeStatus::Converted => Err(SafeNoteError::AlreadyConverted),
            SafeStatus::Cancelled => Err(SafeNoteError::AlreadyCancelled),
        }
    }
}

/// Errors produced by [`SafeNote`] state transitions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SafeNoteError {
    #[error("safe note has already been converted")]
    AlreadyConverted,
    #[error("safe note has already been cancelled")]
    AlreadyCancelled,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_safe(safe_type: SafeType) -> SafeNote {
        SafeNote::new(
            EntityId::new(),
            CapTableId::new(),
            ContactId::new(),
            "Acme Ventures",
            safe_type,
            500_000_00,         // $500,000
            Some(5_000_000_00), // $5M cap
            Some(20),
        )
    }

    // ── SafeNote::new() ───────────────────────────────────────────────────────

    #[test]
    fn new_safe_status_is_issued() {
        let s = make_safe(SafeType::PostMoney);
        assert_eq!(s.status, SafeStatus::Issued);
    }

    #[test]
    fn new_safe_stores_investor_name() {
        let s = make_safe(SafeType::PostMoney);
        assert_eq!(s.investor_name, "Acme Ventures");
    }

    #[test]
    fn new_safe_stores_investment_amount() {
        let s = make_safe(SafeType::PostMoney);
        assert_eq!(s.investment_amount_cents, 500_000_00);
    }

    #[test]
    fn new_safe_stores_valuation_cap() {
        let s = make_safe(SafeType::PostMoney);
        assert_eq!(s.valuation_cap_cents, Some(5_000_000_00));
    }

    #[test]
    fn new_safe_stores_discount_percent() {
        let s = make_safe(SafeType::PostMoney);
        assert_eq!(s.discount_percent, Some(20));
    }

    #[test]
    fn new_safe_has_no_converted_at() {
        let s = make_safe(SafeType::PostMoney);
        assert!(s.converted_at.is_none());
    }

    #[test]
    fn new_safe_post_money() {
        let s = make_safe(SafeType::PostMoney);
        assert_eq!(s.safe_type, SafeType::PostMoney);
    }

    #[test]
    fn new_safe_pre_money() {
        let s = make_safe(SafeType::PreMoney);
        assert_eq!(s.safe_type, SafeType::PreMoney);
    }

    #[test]
    fn new_safe_mfn() {
        let s = SafeNote::new(
            EntityId::new(),
            CapTableId::new(),
            ContactId::new(),
            "MFN Investor",
            SafeType::Mfn,
            250_000_00,
            None,
            None,
        );
        assert_eq!(s.safe_type, SafeType::Mfn);
        assert!(s.valuation_cap_cents.is_none());
        assert!(s.discount_percent.is_none());
    }

    #[test]
    fn new_safe_has_unique_id() {
        let a = make_safe(SafeType::PostMoney);
        let b = make_safe(SafeType::PostMoney);
        assert_ne!(a.safe_note_id, b.safe_note_id);
    }

    // ── convert() ────────────────────────────────────────────────────────────

    #[test]
    fn convert_from_issued_to_converted() {
        let mut s = make_safe(SafeType::PostMoney);
        s.convert().unwrap();
        assert_eq!(s.status, SafeStatus::Converted);
    }

    #[test]
    fn convert_records_converted_at() {
        let mut s = make_safe(SafeType::PostMoney);
        s.convert().unwrap();
        assert!(s.converted_at.is_some());
    }

    #[test]
    fn convert_from_converted_fails() {
        let mut s = make_safe(SafeType::PostMoney);
        s.convert().unwrap();
        assert_eq!(s.convert().unwrap_err(), SafeNoteError::AlreadyConverted);
    }

    #[test]
    fn convert_from_cancelled_fails() {
        let mut s = make_safe(SafeType::PostMoney);
        s.cancel().unwrap();
        assert_eq!(s.convert().unwrap_err(), SafeNoteError::AlreadyCancelled);
    }

    // ── cancel() ─────────────────────────────────────────────────────────────

    #[test]
    fn cancel_from_issued_to_cancelled() {
        let mut s = make_safe(SafeType::PostMoney);
        s.cancel().unwrap();
        assert_eq!(s.status, SafeStatus::Cancelled);
    }

    #[test]
    fn cancel_from_converted_fails() {
        let mut s = make_safe(SafeType::PostMoney);
        s.convert().unwrap();
        assert_eq!(s.cancel().unwrap_err(), SafeNoteError::AlreadyConverted);
    }

    #[test]
    fn cancel_from_cancelled_fails() {
        let mut s = make_safe(SafeType::PostMoney);
        s.cancel().unwrap();
        assert_eq!(s.cancel().unwrap_err(), SafeNoteError::AlreadyCancelled);
    }

    // ── JSON roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn safe_note_serde_roundtrip_issued() {
        let s = make_safe(SafeType::PreMoney);
        let json = serde_json::to_string(&s).unwrap();
        let de: SafeNote = serde_json::from_str(&json).unwrap();
        assert_eq!(de.safe_note_id, s.safe_note_id);
        assert_eq!(de.status, SafeStatus::Issued);
        assert_eq!(de.safe_type, SafeType::PreMoney);
    }

    #[test]
    fn safe_note_serde_roundtrip_converted() {
        let mut s = make_safe(SafeType::PostMoney);
        s.convert().unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let de: SafeNote = serde_json::from_str(&json).unwrap();
        assert_eq!(de.status, SafeStatus::Converted);
        assert!(de.converted_at.is_some());
    }
}
