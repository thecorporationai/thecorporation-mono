//! Double-entry journal entries and lines.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::types::Side;
use crate::ids::{AccountId, EntityId, JournalEntryId};

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JournalEntryError {
    #[error("journal entry is already posted")]
    AlreadyPosted,
    #[error("journal entry is already voided")]
    AlreadyVoided,
    #[error("journal entry is voided and cannot be posted")]
    PostingVoidedEntry,
    #[error("debits ({debits}) do not equal credits ({credits}): entry does not balance")]
    Unbalanced { debits: i64, credits: i64 },
}

// ── JournalLine ───────────────────────────────────────────────────────────────

/// A single debit or credit line within a [`JournalEntry`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalLine {
    pub account_id: AccountId,
    /// Absolute amount in cents. The `side` field determines whether this is a
    /// debit or credit; `amount_cents` is always stored as a non-negative value.
    pub amount_cents: i64,
    pub side: Side,
    pub memo: Option<String>,
}

// ── JournalEntry ──────────────────────────────────────────────────────────────

/// A balanced double-entry accounting record.
///
/// A `JournalEntry` must satisfy the accounting equation before it can be
/// posted: the sum of all debit lines must equal the sum of all credit lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub entry_id: JournalEntryId,
    pub entity_id: EntityId,
    pub date: NaiveDate,
    pub description: String,
    pub lines: Vec<JournalLine>,
    pub posted: bool,
    pub voided: bool,
    pub created_at: DateTime<Utc>,
}

impl JournalEntry {
    /// Create a new, unposted journal entry. Lines can be added directly to
    /// the `lines` field before calling [`post`](JournalEntry::post).
    pub fn new(
        entity_id: EntityId,
        date: NaiveDate,
        description: impl Into<String>,
        lines: Vec<JournalLine>,
    ) -> Self {
        Self {
            entry_id: JournalEntryId::new(),
            entity_id,
            date,
            description: description.into(),
            lines,
            posted: false,
            voided: false,
            created_at: Utc::now(),
        }
    }

    /// Validate that debits == credits and mark the entry as posted.
    ///
    /// Returns `Err` if the entry is already posted, already voided, or does
    /// not balance.
    pub fn post(&mut self) -> Result<(), JournalEntryError> {
        if self.voided {
            return Err(JournalEntryError::PostingVoidedEntry);
        }
        if self.posted {
            return Err(JournalEntryError::AlreadyPosted);
        }

        let debits: i64 = self
            .lines
            .iter()
            .filter(|l| l.side == Side::Debit)
            .map(|l| l.amount_cents)
            .sum();
        let credits: i64 = self
            .lines
            .iter()
            .filter(|l| l.side == Side::Credit)
            .map(|l| l.amount_cents)
            .sum();

        if debits != credits {
            return Err(JournalEntryError::Unbalanced { debits, credits });
        }

        self.posted = true;
        Ok(())
    }

    /// Void a posted (or unposted) entry. A voided entry cannot be re-posted.
    pub fn void(&mut self) -> Result<(), JournalEntryError> {
        if self.voided {
            return Err(JournalEntryError::AlreadyVoided);
        }
        self.voided = true;
        self.posted = false;
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntityId;

    fn cash_id() -> AccountId {
        AccountId::new()
    }

    fn revenue_id() -> AccountId {
        AccountId::new()
    }

    fn make_balanced_entry(cash: AccountId, rev: AccountId) -> JournalEntry {
        JournalEntry::new(
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            "Test entry",
            vec![
                JournalLine {
                    account_id: cash,
                    amount_cents: 100,
                    side: Side::Debit,
                    memo: None,
                },
                JournalLine {
                    account_id: rev,
                    amount_cents: 100,
                    side: Side::Credit,
                    memo: None,
                },
            ],
        )
    }

    #[test]
    fn post_balanced_entry() {
        let mut je = make_balanced_entry(cash_id(), revenue_id());
        assert!(je.post().is_ok());
        assert!(je.posted);
    }

    #[test]
    fn post_unbalanced_entry() {
        let mut je = JournalEntry::new(
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            "Unbalanced",
            vec![JournalLine {
                account_id: cash_id(),
                amount_cents: 100,
                side: Side::Debit,
                memo: None,
            }],
        );
        assert!(matches!(
            je.post(),
            Err(JournalEntryError::Unbalanced { .. })
        ));
    }

    #[test]
    fn double_post_is_error() {
        let mut je = make_balanced_entry(cash_id(), revenue_id());
        je.post().unwrap();
        assert_eq!(je.post(), Err(JournalEntryError::AlreadyPosted));
    }

    #[test]
    fn void_entry() {
        let mut je = make_balanced_entry(cash_id(), revenue_id());
        je.post().unwrap();
        assert!(je.void().is_ok());
        assert!(je.voided);
        assert!(!je.posted);
    }

    #[test]
    fn void_unposted_entry() {
        // void() is also allowed from unposted state
        let mut je = make_balanced_entry(cash_id(), revenue_id());
        assert!(!je.posted);
        assert!(je.void().is_ok());
        assert!(je.voided);
    }

    #[test]
    fn double_void_is_error() {
        let mut je = make_balanced_entry(cash_id(), revenue_id());
        je.void().unwrap();
        assert_eq!(je.void(), Err(JournalEntryError::AlreadyVoided));
    }

    #[test]
    fn post_voided_entry_is_error() {
        let mut je = make_balanced_entry(cash_id(), revenue_id());
        je.void().unwrap();
        assert_eq!(je.post(), Err(JournalEntryError::PostingVoidedEntry));
    }

    #[test]
    fn multi_line_balanced_entry_posts() {
        let cash = cash_id();
        let rev = revenue_id();
        let ap = AccountId::new();
        // Two debits totalling 300, two credits totalling 300
        let mut je = JournalEntry::new(
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            "Multi-line",
            vec![
                JournalLine {
                    account_id: cash,
                    amount_cents: 200,
                    side: Side::Debit,
                    memo: None,
                },
                JournalLine {
                    account_id: ap,
                    amount_cents: 100,
                    side: Side::Debit,
                    memo: None,
                },
                JournalLine {
                    account_id: rev,
                    amount_cents: 200,
                    side: Side::Credit,
                    memo: Some("Service".into()),
                },
                JournalLine {
                    account_id: rev,
                    amount_cents: 100,
                    side: Side::Credit,
                    memo: None,
                },
            ],
        );
        assert!(je.post().is_ok());
    }

    #[test]
    fn multi_line_unbalanced_entry_fails() {
        let cash = cash_id();
        let rev = revenue_id();
        let mut je = JournalEntry::new(
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            "Unbalanced multi",
            vec![
                JournalLine {
                    account_id: cash,
                    amount_cents: 500,
                    side: Side::Debit,
                    memo: None,
                },
                JournalLine {
                    account_id: rev,
                    amount_cents: 300,
                    side: Side::Credit,
                    memo: None,
                },
            ],
        );
        assert!(matches!(
            je.post(),
            Err(JournalEntryError::Unbalanced {
                debits: 500,
                credits: 300
            })
        ));
    }

    #[test]
    fn zero_amount_lines_balance() {
        let cash = cash_id();
        let rev = revenue_id();
        let mut je = JournalEntry::new(
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            "Zero amounts",
            vec![
                JournalLine {
                    account_id: cash,
                    amount_cents: 0,
                    side: Side::Debit,
                    memo: None,
                },
                JournalLine {
                    account_id: rev,
                    amount_cents: 0,
                    side: Side::Credit,
                    memo: None,
                },
            ],
        );
        assert!(je.post().is_ok());
    }

    #[test]
    fn entry_description_stored() {
        let je = make_balanced_entry(cash_id(), revenue_id());
        assert_eq!(je.description, "Test entry");
    }

    #[test]
    fn entry_starts_unposted_and_unvoided() {
        let je = make_balanced_entry(cash_id(), revenue_id());
        assert!(!je.posted);
        assert!(!je.voided);
    }

    #[test]
    fn new_with_memo_line() {
        let cash = cash_id();
        let rev = revenue_id();
        let mut je = JournalEntry::new(
            EntityId::new(),
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            "With memo",
            vec![
                JournalLine {
                    account_id: cash,
                    amount_cents: 100,
                    side: Side::Debit,
                    memo: Some("Deposit".into()),
                },
                JournalLine {
                    account_id: rev,
                    amount_cents: 100,
                    side: Side::Credit,
                    memo: Some("Sale".into()),
                },
            ],
        );
        assert!(je.post().is_ok());
        assert_eq!(je.lines[0].memo.as_deref(), Some("Deposit"));
    }

    #[test]
    fn void_then_cannot_post() {
        let mut je = make_balanced_entry(cash_id(), revenue_id());
        je.post().unwrap();
        je.void().unwrap();
        // After void, posting should fail
        assert_eq!(je.post(), Err(JournalEntryError::PostingVoidedEntry));
    }
}
