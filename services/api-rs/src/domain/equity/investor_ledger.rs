//! Investor ledger entry (stored as `investor-ledger/{entry_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::{InvestorLedgerEntryType, ShareCount};
use crate::domain::ids::{
    ContactId, EntityId, FundingRoundId, InvestorLedgerEntryId, SafeNoteId,
};
use crate::domain::treasury::types::Cents;

/// A ledger entry tracking an investor's financial relationship with an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorLedgerEntry {
    entry_id: InvestorLedgerEntryId,
    entity_id: EntityId,
    investor_id: ContactId,
    investor_name: String,
    safe_note_id: Option<SafeNoteId>,
    funding_round_id: Option<FundingRoundId>,
    entry_type: InvestorLedgerEntryType,
    amount_cents: Cents,
    shares_received: Option<ShareCount>,
    pro_rata_eligible: bool,
    memo: Option<String>,
    effective_date: NaiveDate,
    created_at: DateTime<Utc>,
}

impl InvestorLedgerEntry {
    /// Create a new investor ledger entry.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entry_id: InvestorLedgerEntryId,
        entity_id: EntityId,
        investor_id: ContactId,
        investor_name: String,
        safe_note_id: Option<SafeNoteId>,
        funding_round_id: Option<FundingRoundId>,
        entry_type: InvestorLedgerEntryType,
        amount_cents: Cents,
        shares_received: Option<ShareCount>,
        pro_rata_eligible: bool,
        memo: Option<String>,
        effective_date: NaiveDate,
    ) -> Self {
        Self {
            entry_id,
            entity_id,
            investor_id,
            investor_name,
            safe_note_id,
            funding_round_id,
            entry_type,
            amount_cents,
            shares_received,
            pro_rata_eligible,
            memo,
            effective_date,
            created_at: Utc::now(),
        }
    }

    pub fn entry_id(&self) -> InvestorLedgerEntryId {
        self.entry_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn investor_id(&self) -> ContactId {
        self.investor_id
    }

    pub fn investor_name(&self) -> &str {
        &self.investor_name
    }

    pub fn safe_note_id(&self) -> Option<SafeNoteId> {
        self.safe_note_id
    }

    pub fn funding_round_id(&self) -> Option<FundingRoundId> {
        self.funding_round_id
    }

    pub fn entry_type(&self) -> InvestorLedgerEntryType {
        self.entry_type
    }

    pub fn amount_cents(&self) -> Cents {
        self.amount_cents
    }

    pub fn shares_received(&self) -> Option<ShareCount> {
        self.shares_received
    }

    pub fn pro_rata_eligible(&self) -> bool {
        self.pro_rata_eligible
    }

    pub fn memo(&self) -> Option<&str> {
        self.memo.as_deref()
    }

    pub fn effective_date(&self) -> NaiveDate {
        self.effective_date
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_entry() {
        let entry = InvestorLedgerEntry::new(
            InvestorLedgerEntryId::new(),
            EntityId::new(),
            ContactId::new(),
            "Investor A".to_string(),
            Some(SafeNoteId::new()),
            None,
            InvestorLedgerEntryType::SafeInvestment,
            Cents::new(100_000_00),
            None,
            true,
            Some("Seed SAFE".to_string()),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        );
        assert_eq!(entry.investor_name(), "Investor A");
        assert_eq!(entry.entry_type(), InvestorLedgerEntryType::SafeInvestment);
        assert!(entry.pro_rata_eligible());
        assert!(entry.safe_note_id().is_some());
    }

    #[test]
    fn serde_roundtrip() {
        let entry = InvestorLedgerEntry::new(
            InvestorLedgerEntryId::new(),
            EntityId::new(),
            ContactId::new(),
            "Investor B".to_string(),
            None,
            Some(FundingRoundId::new()),
            InvestorLedgerEntryType::PricedRoundInvestment,
            Cents::new(500_000_00),
            Some(ShareCount::new(250_000)),
            false,
            None,
            NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        );
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: InvestorLedgerEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.entry_id(), parsed.entry_id());
        assert_eq!(entry.investor_name(), parsed.investor_name());
        assert_eq!(entry.amount_cents(), parsed.amount_cents());
        assert_eq!(entry.shares_received(), parsed.shares_received());
    }
}
