//! Investor ledger: a per-entity record of investor capital events.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ContactId, EntityId, FundingRoundId, InvestorLedgerEntryId, SafeNoteId};
use super::types::InvestorLedgerEntryType;

// ── InvestorLedgerEntry ───────────────────────────────────────────────────────

/// A single line in the investor ledger — one capital event for one investor.
///
/// Examples: a SAFE investment, a priced-round close, a SAFE conversion
/// resulting in share issuance, or a pro-rata right exercised at a subsequent
/// round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorLedgerEntry {
    pub entry_id: InvestorLedgerEntryId,
    pub entity_id: EntityId,
    pub investor_id: ContactId,
    pub investor_name: String,
    /// Linked SAFE note, if this entry relates to a SAFE.
    pub safe_note_id: Option<SafeNoteId>,
    /// Linked funding round, if this entry belongs to a priced round.
    pub funding_round_id: Option<FundingRoundId>,
    pub entry_type: InvestorLedgerEntryType,
    /// Dollars invested (or notional amount) in whole cents.
    pub amount_cents: i64,
    /// Shares received (populated after conversion or issuance).
    pub shares_received: Option<i64>,
    /// Whether this investor has pro-rata rights for future rounds.
    pub pro_rata_eligible: bool,
    pub memo: Option<String>,
    pub effective_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

impl InvestorLedgerEntry {
    /// Create a new investor ledger entry.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        investor_id: ContactId,
        investor_name: impl Into<String>,
        safe_note_id: Option<SafeNoteId>,
        funding_round_id: Option<FundingRoundId>,
        entry_type: InvestorLedgerEntryType,
        amount_cents: i64,
        shares_received: Option<i64>,
        pro_rata_eligible: bool,
        memo: Option<String>,
        effective_date: NaiveDate,
    ) -> Self {
        Self {
            entry_id: InvestorLedgerEntryId::new(),
            entity_id,
            investor_id,
            investor_name: investor_name.into(),
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
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_safe_investment() -> InvestorLedgerEntry {
        InvestorLedgerEntry::new(
            EntityId::new(),
            ContactId::new(),
            "Acme Ventures",
            Some(SafeNoteId::new()),
            None,
            InvestorLedgerEntryType::SafeInvestment,
            500_000_00, // $500k
            None,
            true,
            Some("Seed SAFE".to_string()),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        )
    }

    #[test]
    fn new_entry_stores_investor_name() {
        let e = make_safe_investment();
        assert_eq!(e.investor_name, "Acme Ventures");
    }

    #[test]
    fn new_entry_stores_amount() {
        let e = make_safe_investment();
        assert_eq!(e.amount_cents, 500_000_00);
    }

    #[test]
    fn new_entry_safe_investment_type() {
        let e = make_safe_investment();
        assert_eq!(e.entry_type, InvestorLedgerEntryType::SafeInvestment);
    }

    #[test]
    fn new_entry_no_shares_received_initially() {
        let e = make_safe_investment();
        assert!(e.shares_received.is_none());
    }

    #[test]
    fn new_entry_pro_rata_eligible() {
        let e = make_safe_investment();
        assert!(e.pro_rata_eligible);
    }

    #[test]
    fn new_entry_stores_safe_note_id() {
        let e = make_safe_investment();
        assert!(e.safe_note_id.is_some());
    }

    #[test]
    fn new_entry_no_funding_round_for_safe() {
        let e = make_safe_investment();
        assert!(e.funding_round_id.is_none());
    }

    #[test]
    fn new_entry_has_unique_id() {
        let a = make_safe_investment();
        let b = make_safe_investment();
        assert_ne!(a.entry_id, b.entry_id);
    }

    #[test]
    fn priced_round_investment_entry() {
        let e = InvestorLedgerEntry::new(
            EntityId::new(),
            ContactId::new(),
            "Series A Fund",
            None,
            Some(FundingRoundId::new()),
            InvestorLedgerEntryType::PricedRoundInvestment,
            2_000_000_00,
            Some(1_000_000),
            true,
            None,
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        );
        assert_eq!(e.entry_type, InvestorLedgerEntryType::PricedRoundInvestment);
        assert_eq!(e.shares_received, Some(1_000_000));
        assert!(e.safe_note_id.is_none());
        assert!(e.funding_round_id.is_some());
    }

    #[test]
    fn safe_conversion_entry() {
        let e = InvestorLedgerEntry::new(
            EntityId::new(),
            ContactId::new(),
            "Early Investor",
            Some(SafeNoteId::new()),
            Some(FundingRoundId::new()),
            InvestorLedgerEntryType::SafeConversion,
            0,
            Some(250_000),
            false,
            None,
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
        );
        assert_eq!(e.entry_type, InvestorLedgerEntryType::SafeConversion);
        assert_eq!(e.shares_received, Some(250_000));
    }

    #[test]
    fn pro_rata_exercise_entry() {
        let e = InvestorLedgerEntry::new(
            EntityId::new(),
            ContactId::new(),
            "Pro Rata Investor",
            None,
            Some(FundingRoundId::new()),
            InvestorLedgerEntryType::ProRataExercise,
            100_000_00,
            Some(50_000),
            true,
            None,
            NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        );
        assert_eq!(e.entry_type, InvestorLedgerEntryType::ProRataExercise);
    }

    #[test]
    fn investor_ledger_entry_serde_roundtrip() {
        let e = make_safe_investment();
        let json = serde_json::to_string(&e).unwrap();
        let de: InvestorLedgerEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(de.entry_id, e.entry_id);
        assert_eq!(de.investor_name, "Acme Ventures");
        assert_eq!(de.entry_type, InvestorLedgerEntryType::SafeInvestment);
    }

    // ── InvestorLedgerEntryType serde ─────────────────────────────────────────

    #[test]
    fn entry_type_serde_safe_investment() {
        let json = serde_json::to_string(&InvestorLedgerEntryType::SafeInvestment).unwrap();
        assert_eq!(json, r#""safe_investment""#);
        let de: InvestorLedgerEntryType = serde_json::from_str(&json).unwrap();
        assert_eq!(de, InvestorLedgerEntryType::SafeInvestment);
    }

    #[test]
    fn entry_type_serde_priced_round_investment() {
        let json =
            serde_json::to_string(&InvestorLedgerEntryType::PricedRoundInvestment).unwrap();
        assert_eq!(json, r#""priced_round_investment""#);
    }

    #[test]
    fn entry_type_serde_safe_conversion() {
        let json = serde_json::to_string(&InvestorLedgerEntryType::SafeConversion).unwrap();
        assert_eq!(json, r#""safe_conversion""#);
    }

    #[test]
    fn entry_type_serde_pro_rata_exercise() {
        let json = serde_json::to_string(&InvestorLedgerEntryType::ProRataExercise).unwrap();
        assert_eq!(json, r#""pro_rata_exercise""#);
    }
}
