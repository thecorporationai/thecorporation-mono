//! Agenda item — a single topic or action item within a meeting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{AgendaItemId, MeetingId};

use super::types::AgendaItemType;

// ── AgendaItem ────────────────────────────────────────────────────────────────

/// An individual item on a meeting agenda (resolution, discussion, report, or election).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgendaItem {
    pub item_id: AgendaItemId,
    pub meeting_id: MeetingId,
    pub title: String,
    pub item_type: AgendaItemType,
    pub description: Option<String>,
    /// Proposed resolution text; populated for `Resolution` items.
    pub resolution_text: Option<String>,
    /// Whether the item has been resolved / acted upon.
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

impl AgendaItem {
    /// Create a new unresolved agenda item.
    pub fn new(
        meeting_id: MeetingId,
        title: String,
        item_type: AgendaItemType,
        description: Option<String>,
        resolution_text: Option<String>,
    ) -> Self {
        Self {
            item_id: AgendaItemId::new(),
            meeting_id,
            title,
            item_type,
            description,
            resolution_text,
            resolved: false,
            created_at: Utc::now(),
        }
    }

    /// Mark this item as resolved.
    pub fn resolve(&mut self) {
        self.resolved = true;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(item_type: AgendaItemType) -> AgendaItem {
        AgendaItem::new(MeetingId::new(), "Test Item".into(), item_type, None, None)
    }

    #[test]
    fn new_item_is_unresolved() {
        let item = AgendaItem::new(
            MeetingId::new(),
            "Approve Budget".into(),
            AgendaItemType::Resolution,
            None,
            Some("RESOLVED: the board approves the 2025 budget.".into()),
        );
        assert!(!item.resolved);
    }

    #[test]
    fn new_resolution_item_stores_text() {
        let item = AgendaItem::new(
            MeetingId::new(),
            "Approve Budget".into(),
            AgendaItemType::Resolution,
            Some("Discussion details".into()),
            Some("RESOLVED: approve.".into()),
        );
        assert_eq!(item.resolution_text.as_deref(), Some("RESOLVED: approve."));
        assert_eq!(item.description.as_deref(), Some("Discussion details"));
    }

    #[test]
    fn new_discussion_item_no_resolution_text() {
        let item = make_item(AgendaItemType::Discussion);
        assert_eq!(item.item_type, AgendaItemType::Discussion);
        assert!(item.resolution_text.is_none());
        assert!(!item.resolved);
    }

    #[test]
    fn new_report_item() {
        let item = make_item(AgendaItemType::Report);
        assert_eq!(item.item_type, AgendaItemType::Report);
        assert!(!item.resolved);
    }

    #[test]
    fn new_election_item() {
        let item = make_item(AgendaItemType::Election);
        assert_eq!(item.item_type, AgendaItemType::Election);
    }

    #[test]
    fn resolve_marks_item() {
        let mut item = AgendaItem::new(
            MeetingId::new(),
            "CEO Report".into(),
            AgendaItemType::Report,
            None,
            None,
        );
        item.resolve();
        assert!(item.resolved);
    }

    #[test]
    fn resolve_idempotent() {
        let mut item = make_item(AgendaItemType::Resolution);
        item.resolve();
        item.resolve(); // second call should not panic or change state
        assert!(item.resolved);
    }

    #[test]
    fn each_item_has_unique_id() {
        let i1 = make_item(AgendaItemType::Report);
        let i2 = make_item(AgendaItemType::Report);
        assert_ne!(i1.item_id, i2.item_id);
    }

    #[test]
    fn item_stores_meeting_id() {
        let meeting_id = MeetingId::new();
        let item = AgendaItem::new(
            meeting_id,
            "Test".into(),
            AgendaItemType::Discussion,
            None,
            None,
        );
        assert_eq!(item.meeting_id, meeting_id);
    }

    #[test]
    fn item_title_stored() {
        let item = AgendaItem::new(
            MeetingId::new(),
            "Approval of Minutes".into(),
            AgendaItemType::Resolution,
            None,
            None,
        );
        assert_eq!(item.title, "Approval of Minutes");
    }

    #[test]
    fn item_serde_roundtrip() {
        let item = AgendaItem::new(
            MeetingId::new(),
            "Budget Review".into(),
            AgendaItemType::Resolution,
            Some("Annual budget review".into()),
            Some("RESOLVED: approve budget.".into()),
        );
        let json = serde_json::to_string(&item).unwrap();
        let back: AgendaItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item.item_id, back.item_id);
        assert_eq!(item.resolved, back.resolved);
        assert_eq!(item.title, back.title);
    }
}
