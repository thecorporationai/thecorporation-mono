//! Resolution record (stored as `governance/meetings/{meeting_id}/resolutions/{resolution_id}.json`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::types::ResolutionType;
use crate::domain::ids::{AgendaItemId, DocumentId, MeetingId, ResolutionId};

/// A resolution produced from a vote on an agenda item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    resolution_id: ResolutionId,
    meeting_id: MeetingId,
    agenda_item_id: AgendaItemId,
    resolution_type: ResolutionType,
    resolution_text: String,
    passed: bool,
    effective_date: Option<NaiveDate>,
    document_id: Option<DocumentId>,
    votes_for: u32,
    votes_against: u32,
    votes_abstain: u32,
    recused_count: u32,
    created_at: DateTime<Utc>,
}

impl Resolution {
    /// Create a new resolution. `document_id` starts as `None` and can be
    /// attached later via [`set_document_id`](Self::set_document_id).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resolution_id: ResolutionId,
        meeting_id: MeetingId,
        agenda_item_id: AgendaItemId,
        resolution_type: ResolutionType,
        resolution_text: String,
        passed: bool,
        effective_date: Option<NaiveDate>,
        votes_for: u32,
        votes_against: u32,
        votes_abstain: u32,
        recused_count: u32,
    ) -> Self {
        Self {
            resolution_id,
            meeting_id,
            agenda_item_id,
            resolution_type,
            resolution_text,
            passed,
            effective_date,
            document_id: None,
            votes_for,
            votes_against,
            votes_abstain,
            recused_count,
            created_at: Utc::now(),
        }
    }

    /// Attach a document (e.g., formal resolution PDF) to this resolution.
    pub fn set_document_id(&mut self, doc_id: DocumentId) {
        self.document_id = Some(doc_id);
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn resolution_id(&self) -> ResolutionId {
        self.resolution_id
    }

    pub fn meeting_id(&self) -> MeetingId {
        self.meeting_id
    }

    pub fn agenda_item_id(&self) -> AgendaItemId {
        self.agenda_item_id
    }

    pub fn resolution_type(&self) -> ResolutionType {
        self.resolution_type
    }

    pub fn resolution_text(&self) -> &str {
        &self.resolution_text
    }

    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn effective_date(&self) -> Option<NaiveDate> {
        self.effective_date
    }

    pub fn document_id(&self) -> Option<DocumentId> {
        self.document_id
    }

    pub fn votes_for(&self) -> u32 {
        self.votes_for
    }

    pub fn votes_against(&self) -> u32 {
        self.votes_against
    }

    pub fn votes_abstain(&self) -> u32 {
        self.votes_abstain
    }

    pub fn recused_count(&self) -> u32 {
        self.recused_count
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_resolution(passed: bool) -> Resolution {
        Resolution::new(
            ResolutionId::new(),
            MeetingId::new(),
            AgendaItemId::new(),
            ResolutionType::Ordinary,
            "RESOLVED that the quarterly financials are approved.".into(),
            passed,
            None,
            3,
            1,
            0,
            0,
        )
    }

    #[test]
    fn new_resolution_has_no_document() {
        let r = make_resolution(true);
        assert!(r.document_id().is_none());
        assert!(r.passed());
        assert_eq!(r.votes_for(), 3);
        assert_eq!(r.votes_against(), 1);
    }

    #[test]
    fn set_document_id() {
        let mut r = make_resolution(true);
        let doc_id = DocumentId::new();
        r.set_document_id(doc_id);
        assert_eq!(r.document_id(), Some(doc_id));
    }

    #[test]
    fn serde_roundtrip() {
        let mut r = make_resolution(false);
        r.set_document_id(DocumentId::new());
        let json = serde_json::to_string(&r).unwrap();
        let parsed: Resolution = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.resolution_id(), r.resolution_id());
        assert_eq!(parsed.passed(), r.passed());
        assert_eq!(parsed.resolution_text(), r.resolution_text());
        assert_eq!(parsed.document_id(), r.document_id());
        assert_eq!(parsed.votes_for(), r.votes_for());
    }
}
