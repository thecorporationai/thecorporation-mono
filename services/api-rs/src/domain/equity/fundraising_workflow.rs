//! Fundraising workflow orchestration state (stored as `cap-table/fundraising-workflows/{id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::round::{EquityRound, EquityRoundStatus};
use crate::domain::ids::{
    EntityId, EquityRoundId, EquityRuleSetId, FundraisingWorkflowId, IntentId, MeetingId, PacketId,
    ResolutionId, WorkspaceId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExecutionStatus {
    Draft,
    PrereqsBlocked,
    PrereqsReady,
    PacketCompiled,
    SigningInProgress,
    SigningComplete,
    Executable,
    Executed,
    Failed,
    Cancelled,
}

/// Workflow record for preparing/routing/recording a fundraising round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundraisingWorkflow {
    fundraising_workflow_id: FundraisingWorkflowId,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    round_id: EquityRoundId,
    prepare_intent_id: IntentId,
    accept_intent_id: Option<IntentId>,
    close_intent_id: Option<IntentId>,
    rule_set_id: Option<EquityRuleSetId>,
    round_status: EquityRoundStatus,
    board_approval_meeting_id: Option<MeetingId>,
    board_approval_resolution_id: Option<ResolutionId>,
    board_packet_documents: Vec<String>,
    closing_packet_documents: Vec<String>,
    execution_status: WorkflowExecutionStatus,
    active_packet_id: Option<PacketId>,
    last_packet_hash: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl FundraisingWorkflow {
    pub fn new(
        fundraising_workflow_id: FundraisingWorkflowId,
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        round_id: EquityRoundId,
        prepare_intent_id: IntentId,
    ) -> Self {
        let now = Utc::now();
        Self {
            fundraising_workflow_id,
            entity_id,
            workspace_id,
            round_id,
            prepare_intent_id,
            accept_intent_id: None,
            close_intent_id: None,
            rule_set_id: None,
            round_status: EquityRoundStatus::Draft,
            board_approval_meeting_id: None,
            board_approval_resolution_id: None,
            board_packet_documents: Vec::new(),
            closing_packet_documents: Vec::new(),
            execution_status: WorkflowExecutionStatus::Draft,
            active_packet_id: None,
            last_packet_hash: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn sync_from_round(&mut self, round: &EquityRound) {
        self.round_status = round.status();
        self.rule_set_id = round.rule_set_id();
        self.board_approval_meeting_id = round.board_approval_meeting_id();
        self.board_approval_resolution_id = round.board_approval_resolution_id();
        self.updated_at = Utc::now();
    }

    pub fn set_accept_intent_id(&mut self, intent_id: IntentId) {
        self.accept_intent_id = Some(intent_id);
        self.updated_at = Utc::now();
    }

    pub fn set_close_intent_id(&mut self, intent_id: IntentId) {
        self.close_intent_id = Some(intent_id);
        self.updated_at = Utc::now();
    }

    pub fn add_board_packet_documents<I>(&mut self, documents: I)
    where
        I: IntoIterator<Item = String>,
    {
        for doc in documents {
            if doc.trim().is_empty() {
                continue;
            }
            if !self.board_packet_documents.iter().any(|d| d == &doc) {
                self.board_packet_documents.push(doc);
            }
        }
        self.updated_at = Utc::now();
    }

    pub fn add_closing_packet_documents<I>(&mut self, documents: I)
    where
        I: IntoIterator<Item = String>,
    {
        for doc in documents {
            if doc.trim().is_empty() {
                continue;
            }
            if !self.closing_packet_documents.iter().any(|d| d == &doc) {
                self.closing_packet_documents.push(doc);
            }
        }
        self.updated_at = Utc::now();
    }

    pub fn mark_prereqs_ready(&mut self) {
        self.execution_status = WorkflowExecutionStatus::PrereqsReady;
        self.updated_at = Utc::now();
    }

    pub fn mark_prereqs_blocked(&mut self) {
        self.execution_status = WorkflowExecutionStatus::PrereqsBlocked;
        self.updated_at = Utc::now();
    }

    pub fn mark_packet_compiled(&mut self, packet_id: PacketId, packet_hash: String) {
        self.execution_status = WorkflowExecutionStatus::PacketCompiled;
        self.active_packet_id = Some(packet_id);
        self.last_packet_hash = Some(packet_hash);
        self.updated_at = Utc::now();
    }

    pub fn mark_signing_in_progress(&mut self) {
        self.execution_status = WorkflowExecutionStatus::SigningInProgress;
        self.updated_at = Utc::now();
    }

    pub fn mark_signing_complete(&mut self) {
        self.execution_status = WorkflowExecutionStatus::SigningComplete;
        self.updated_at = Utc::now();
    }

    pub fn mark_executable(&mut self) {
        self.execution_status = WorkflowExecutionStatus::Executable;
        self.updated_at = Utc::now();
    }

    pub fn mark_executed(&mut self) {
        self.execution_status = WorkflowExecutionStatus::Executed;
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self) {
        self.execution_status = WorkflowExecutionStatus::Failed;
        self.updated_at = Utc::now();
    }

    pub fn fundraising_workflow_id(&self) -> FundraisingWorkflowId {
        self.fundraising_workflow_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn round_id(&self) -> EquityRoundId {
        self.round_id
    }

    pub fn prepare_intent_id(&self) -> IntentId {
        self.prepare_intent_id
    }

    pub fn accept_intent_id(&self) -> Option<IntentId> {
        self.accept_intent_id
    }

    pub fn close_intent_id(&self) -> Option<IntentId> {
        self.close_intent_id
    }

    pub fn rule_set_id(&self) -> Option<EquityRuleSetId> {
        self.rule_set_id
    }

    pub fn round_status(&self) -> EquityRoundStatus {
        self.round_status
    }

    pub fn board_approval_meeting_id(&self) -> Option<MeetingId> {
        self.board_approval_meeting_id
    }

    pub fn board_approval_resolution_id(&self) -> Option<ResolutionId> {
        self.board_approval_resolution_id
    }

    pub fn board_packet_documents(&self) -> &[String] {
        &self.board_packet_documents
    }

    pub fn closing_packet_documents(&self) -> &[String] {
        &self.closing_packet_documents
    }

    pub fn execution_status(&self) -> WorkflowExecutionStatus {
        self.execution_status
    }

    pub fn active_packet_id(&self) -> Option<PacketId> {
        self.active_packet_id
    }

    pub fn last_packet_hash(&self) -> Option<&str> {
        self.last_packet_hash.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::equity::round::EquityRound;
    use crate::domain::ids::{InstrumentId, LegalEntityId};

    #[test]
    fn syncs_from_round() {
        let round = EquityRound::new(
            EquityRoundId::new(),
            LegalEntityId::new(),
            "Seed".to_owned(),
            Some(10_000),
            Some(100),
            Some(5_000),
            Some(InstrumentId::new()),
            serde_json::json!({}),
        );
        let mut workflow = FundraisingWorkflow::new(
            FundraisingWorkflowId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            round.equity_round_id(),
            IntentId::new(),
        );
        workflow.sync_from_round(&round);

        assert_eq!(workflow.round_status(), EquityRoundStatus::Draft);
    }

    #[test]
    fn deduplicates_packet_documents() {
        let mut workflow = FundraisingWorkflow::new(
            FundraisingWorkflowId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            EquityRoundId::new(),
            IntentId::new(),
        );
        workflow
            .add_board_packet_documents(vec!["packet-a.md".to_owned(), "packet-a.md".to_owned()]);
        workflow.add_closing_packet_documents(vec![
            "closing-a.md".to_owned(),
            "closing-a.md".to_owned(),
        ]);

        assert_eq!(workflow.board_packet_documents().len(), 1);
        assert_eq!(workflow.closing_packet_documents().len(), 1);
    }
}
