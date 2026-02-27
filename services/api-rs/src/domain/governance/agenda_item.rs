//! Agenda item record (stored as `governance/meetings/{meeting_id}/agenda/{item_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{AgendaItemStatus, AgendaItemType};
use crate::domain::ids::{AgendaItemId, MeetingId};

/// An item on a meeting agenda.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgendaItem {
    agenda_item_id: AgendaItemId,
    meeting_id: MeetingId,
    sequence_number: u32,
    title: String,
    description: Option<String>,
    item_type: AgendaItemType,
    status: AgendaItemStatus,
    created_at: DateTime<Utc>,
}

impl AgendaItem {
    /// Create a new agenda item. Defaults to `Pending` status.
    pub fn new(
        agenda_item_id: AgendaItemId,
        meeting_id: MeetingId,
        sequence_number: u32,
        title: String,
        description: Option<String>,
        item_type: AgendaItemType,
    ) -> Self {
        Self {
            agenda_item_id,
            meeting_id,
            sequence_number,
            title,
            description,
            item_type,
            status: AgendaItemStatus::Pending,
            created_at: Utc::now(),
        }
    }

    /// Mark as discussed.
    pub fn mark_discussed(&mut self) {
        self.status = AgendaItemStatus::Discussed;
    }

    /// Mark as voted.
    pub fn mark_voted(&mut self) {
        self.status = AgendaItemStatus::Voted;
    }

    /// Table the item (defer to a future meeting).
    pub fn table(&mut self) {
        self.status = AgendaItemStatus::Tabled;
    }

    /// Withdraw the item from the agenda.
    pub fn withdraw(&mut self) {
        self.status = AgendaItemStatus::Withdrawn;
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn agenda_item_id(&self) -> AgendaItemId {
        self.agenda_item_id
    }

    pub fn meeting_id(&self) -> MeetingId {
        self.meeting_id
    }

    pub fn sequence_number(&self) -> u32 {
        self.sequence_number
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn item_type(&self) -> AgendaItemType {
        self.item_type
    }

    pub fn status(&self) -> AgendaItemStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item() -> AgendaItem {
        AgendaItem::new(
            AgendaItemId::new(),
            MeetingId::new(),
            1,
            "Approve Q4 financials".into(),
            Some("Review and approve quarterly financial statements.".into()),
            AgendaItemType::Resolution,
        )
    }

    #[test]
    fn new_item_defaults_to_pending() {
        let item = make_item();
        assert_eq!(item.status(), AgendaItemStatus::Pending);
        assert_eq!(item.sequence_number(), 1);
        assert_eq!(item.title(), "Approve Q4 financials");
        assert!(item.description().is_some());
    }

    #[test]
    fn mark_discussed() {
        let mut item = make_item();
        item.mark_discussed();
        assert_eq!(item.status(), AgendaItemStatus::Discussed);
    }

    #[test]
    fn mark_voted() {
        let mut item = make_item();
        item.mark_voted();
        assert_eq!(item.status(), AgendaItemStatus::Voted);
    }

    #[test]
    fn table_item() {
        let mut item = make_item();
        item.table();
        assert_eq!(item.status(), AgendaItemStatus::Tabled);
    }

    #[test]
    fn withdraw_item() {
        let mut item = make_item();
        item.withdraw();
        assert_eq!(item.status(), AgendaItemStatus::Withdrawn);
    }

    #[test]
    fn serde_roundtrip() {
        let item = make_item();
        let json = serde_json::to_string(&item).unwrap();
        let parsed: AgendaItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.agenda_item_id(), item.agenda_item_id());
        assert_eq!(parsed.title(), item.title());
        assert_eq!(parsed.item_type(), item.item_type());
        assert_eq!(parsed.status(), item.status());
    }

    #[test]
    fn description_none() {
        let item = AgendaItem::new(
            AgendaItemId::new(),
            MeetingId::new(),
            2,
            "CEO report".into(),
            None,
            AgendaItemType::Report,
        );
        assert!(item.description().is_none());
    }
}
