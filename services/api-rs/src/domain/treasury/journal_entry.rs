//! Journal entry record (stored as `treasury/journal-entries/{entry_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::TreasuryError;
use super::types::{Cents, Currency, Side};
use crate::domain::ids::{AccountId, EntityId, JournalEntryId, LedgerLineId};

/// Status of a journal entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalEntryStatus {
    Draft,
    Posted,
    Voided,
}

/// A single line in a journal entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerLine {
    line_id: LedgerLineId,
    account_id: AccountId,
    side: Side,
    amount: Cents,
    memo: Option<String>,
}

impl LedgerLine {
    pub fn new(
        line_id: LedgerLineId,
        account_id: AccountId,
        side: Side,
        amount: Cents,
        memo: Option<String>,
    ) -> Self {
        Self {
            line_id,
            account_id,
            side,
            amount,
            memo,
        }
    }

    pub fn line_id(&self) -> LedgerLineId {
        self.line_id
    }

    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    pub fn side(&self) -> Side {
        self.side
    }

    pub fn amount(&self) -> Cents {
        self.amount
    }

    pub fn memo(&self) -> Option<&str> {
        self.memo.as_deref()
    }
}

/// A double-entry journal entry with balanced ledger lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    journal_entry_id: JournalEntryId,
    entity_id: EntityId,
    description: String,
    currency: Currency,
    effective_date: NaiveDate,
    lines: Vec<LedgerLine>,
    total_debits: Cents,
    total_credits: Cents,
    status: JournalEntryStatus,
    posted_at: Option<DateTime<Utc>>,
    voided_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl JournalEntry {
    /// Create a new journal entry. Validates that debits == credits.
    pub fn new(
        journal_entry_id: JournalEntryId,
        entity_id: EntityId,
        description: String,
        effective_date: NaiveDate,
        lines: Vec<LedgerLine>,
    ) -> Result<Self, TreasuryError> {
        let total_debits: i64 = lines
            .iter()
            .filter(|l| l.side == Side::Debit)
            .map(|l| l.amount.raw())
            .sum();
        let total_credits: i64 = lines
            .iter()
            .filter(|l| l.side == Side::Credit)
            .map(|l| l.amount.raw())
            .sum();

        if total_debits != total_credits {
            return Err(TreasuryError::UnbalancedEntry {
                debits: Cents::new(total_debits),
                credits: Cents::new(total_credits),
            });
        }

        Ok(Self {
            journal_entry_id,
            entity_id,
            description,
            currency: Currency::default(),
            effective_date,
            lines,
            total_debits: Cents::new(total_debits),
            total_credits: Cents::new(total_credits),
            status: JournalEntryStatus::Draft,
            posted_at: None,
            voided_at: None,
            created_at: Utc::now(),
        })
    }

    /// Post the entry. Draft -> Posted.
    pub fn post(&mut self) -> Result<(), TreasuryError> {
        if self.status != JournalEntryStatus::Draft {
            return Err(TreasuryError::AlreadyPosted(self.journal_entry_id));
        }
        self.status = JournalEntryStatus::Posted;
        self.posted_at = Some(Utc::now());
        Ok(())
    }

    /// Void a posted entry. Posted -> Voided.
    pub fn void(&mut self) -> Result<(), TreasuryError> {
        match self.status {
            JournalEntryStatus::Draft => Err(TreasuryError::CannotVoidDraft),
            JournalEntryStatus::Voided => {
                Err(TreasuryError::AlreadyVoided(self.journal_entry_id))
            }
            JournalEntryStatus::Posted => {
                self.status = JournalEntryStatus::Voided;
                self.voided_at = Some(Utc::now());
                Ok(())
            }
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn journal_entry_id(&self) -> JournalEntryId {
        self.journal_entry_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn effective_date(&self) -> NaiveDate {
        self.effective_date
    }

    pub fn lines(&self) -> &[LedgerLine] {
        &self.lines
    }

    pub fn total_debits(&self) -> Cents {
        self.total_debits
    }

    pub fn total_credits(&self) -> Cents {
        self.total_credits
    }

    pub fn status(&self) -> JournalEntryStatus {
        self.status
    }

    pub fn posted_at(&self) -> Option<DateTime<Utc>> {
        self.posted_at
    }

    pub fn voided_at(&self) -> Option<DateTime<Utc>> {
        self.voided_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_balanced_lines() -> Vec<LedgerLine> {
        vec![
            LedgerLine::new(
                LedgerLineId::new(),
                AccountId::new(),
                Side::Debit,
                Cents::new(10000),
                Some("Cash in".into()),
            ),
            LedgerLine::new(
                LedgerLineId::new(),
                AccountId::new(),
                Side::Credit,
                Cents::new(10000),
                None,
            ),
        ]
    }

    fn make_entry() -> JournalEntry {
        JournalEntry::new(
            JournalEntryId::new(),
            EntityId::new(),
            "Test entry".into(),
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            make_balanced_lines(),
        )
        .unwrap()
    }

    #[test]
    fn balanced_entry_succeeds() {
        let entry = make_entry();
        assert_eq!(entry.status(), JournalEntryStatus::Draft);
        assert_eq!(entry.total_debits(), Cents::new(10000));
        assert_eq!(entry.total_credits(), Cents::new(10000));
        assert_eq!(entry.description(), "Test entry");
    }

    #[test]
    fn unbalanced_entry_fails() {
        let lines = vec![
            LedgerLine::new(
                LedgerLineId::new(),
                AccountId::new(),
                Side::Debit,
                Cents::new(10000),
                None,
            ),
            LedgerLine::new(
                LedgerLineId::new(),
                AccountId::new(),
                Side::Credit,
                Cents::new(5000),
                None,
            ),
        ];
        let result = JournalEntry::new(
            JournalEntryId::new(),
            EntityId::new(),
            "Bad entry".into(),
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            lines,
        );
        assert!(result.is_err());
    }

    #[test]
    fn post_draft_succeeds() {
        let mut entry = make_entry();
        assert!(entry.post().is_ok());
        assert_eq!(entry.status(), JournalEntryStatus::Posted);
        assert!(entry.posted_at().is_some());
    }

    #[test]
    fn double_post_fails() {
        let mut entry = make_entry();
        entry.post().unwrap();
        assert!(entry.post().is_err());
    }

    #[test]
    fn void_posted_succeeds() {
        let mut entry = make_entry();
        entry.post().unwrap();
        assert!(entry.void().is_ok());
        assert_eq!(entry.status(), JournalEntryStatus::Voided);
        assert!(entry.voided_at().is_some());
    }

    #[test]
    fn void_draft_fails() {
        let mut entry = make_entry();
        assert!(entry.void().is_err());
    }

    #[test]
    fn double_void_fails() {
        let mut entry = make_entry();
        entry.post().unwrap();
        entry.void().unwrap();
        assert!(entry.void().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let entry = make_entry();
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: JournalEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.journal_entry_id(), entry.journal_entry_id());
        assert_eq!(parsed.description(), entry.description());
        assert_eq!(parsed.total_debits(), entry.total_debits());
        assert_eq!(parsed.status(), entry.status());
        assert_eq!(parsed.lines().len(), 2);
    }

    #[test]
    fn ledger_line_accessors() {
        let line = LedgerLine::new(
            LedgerLineId::new(),
            AccountId::new(),
            Side::Debit,
            Cents::new(500),
            Some("memo".into()),
        );
        assert_eq!(line.side(), Side::Debit);
        assert_eq!(line.amount(), Cents::new(500));
        assert_eq!(line.memo(), Some("memo"));
    }
}
