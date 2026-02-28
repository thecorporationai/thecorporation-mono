//! Transfer workflow orchestration state (stored as `cap-table/transfer-workflows/{id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::transfer::ShareTransfer;
use super::types::TransferStatus;
use crate::domain::ids::{
    EntityId, IntentId, MeetingId, PacketId, ResolutionId, TransferId, TransferWorkflowId,
    WorkspaceId,
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

/// Workflow record for preparing/routing/recording a share transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferWorkflow {
    transfer_workflow_id: TransferWorkflowId,
    entity_id: EntityId,
    workspace_id: WorkspaceId,
    transfer_id: TransferId,
    prepare_intent_id: IntentId,
    execute_intent_id: Option<IntentId>,
    transfer_status: TransferStatus,
    board_approval_meeting_id: Option<MeetingId>,
    board_approval_resolution_id: Option<ResolutionId>,
    generated_documents: Vec<String>,
    execution_status: WorkflowExecutionStatus,
    active_packet_id: Option<PacketId>,
    last_packet_hash: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TransferWorkflow {
    pub fn new(
        transfer_workflow_id: TransferWorkflowId,
        entity_id: EntityId,
        workspace_id: WorkspaceId,
        transfer_id: TransferId,
        prepare_intent_id: IntentId,
    ) -> Self {
        let now = Utc::now();
        Self {
            transfer_workflow_id,
            entity_id,
            workspace_id,
            transfer_id,
            prepare_intent_id,
            execute_intent_id: None,
            transfer_status: TransferStatus::Draft,
            board_approval_meeting_id: None,
            board_approval_resolution_id: None,
            generated_documents: Vec::new(),
            execution_status: WorkflowExecutionStatus::Draft,
            active_packet_id: None,
            last_packet_hash: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn sync_from_transfer(&mut self, transfer: &ShareTransfer) {
        self.transfer_status = transfer.status();
        self.board_approval_resolution_id = transfer.board_approval_resolution_id();
        self.updated_at = Utc::now();
    }

    pub fn record_board_approval(&mut self, meeting_id: MeetingId, resolution_id: ResolutionId) {
        self.board_approval_meeting_id = Some(meeting_id);
        self.board_approval_resolution_id = Some(resolution_id);
        self.updated_at = Utc::now();
    }

    pub fn set_execute_intent_id(&mut self, intent_id: IntentId) {
        self.execute_intent_id = Some(intent_id);
        self.updated_at = Utc::now();
    }

    pub fn add_generated_documents<I>(&mut self, documents: I)
    where
        I: IntoIterator<Item = String>,
    {
        for doc in documents {
            if doc.trim().is_empty() {
                continue;
            }
            if !self.generated_documents.iter().any(|d| d == &doc) {
                self.generated_documents.push(doc);
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

    pub fn transfer_workflow_id(&self) -> TransferWorkflowId {
        self.transfer_workflow_id
    }

    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn transfer_id(&self) -> TransferId {
        self.transfer_id
    }

    pub fn prepare_intent_id(&self) -> IntentId {
        self.prepare_intent_id
    }

    pub fn execute_intent_id(&self) -> Option<IntentId> {
        self.execute_intent_id
    }

    pub fn transfer_status(&self) -> TransferStatus {
        self.transfer_status
    }

    pub fn board_approval_meeting_id(&self) -> Option<MeetingId> {
        self.board_approval_meeting_id
    }

    pub fn board_approval_resolution_id(&self) -> Option<ResolutionId> {
        self.board_approval_resolution_id
    }

    pub fn generated_documents(&self) -> &[String] {
        &self.generated_documents
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
    use crate::domain::equity::types::{GoverningDocType, TransferType, TransfereeRights};
    use crate::domain::ids::{ContactId, ShareClassId};

    #[test]
    fn syncs_status_from_transfer() {
        let entity_id = EntityId::new();
        let workspace_id = WorkspaceId::new();
        let mut transfer = ShareTransfer::new(
            TransferId::new(),
            entity_id,
            workspace_id,
            ShareClassId::new(),
            ContactId::new(),
            ContactId::new(),
            TransferType::SecondarySale,
            crate::domain::equity::types::ShareCount::new(10),
            None,
            None,
            GoverningDocType::Bylaws,
            TransfereeRights::FullMember,
        )
        .unwrap();
        transfer.submit_for_review().unwrap();

        let mut workflow = TransferWorkflow::new(
            TransferWorkflowId::new(),
            entity_id,
            workspace_id,
            transfer.transfer_id(),
            IntentId::new(),
        );
        workflow.sync_from_transfer(&transfer);

        assert_eq!(
            workflow.transfer_status(),
            TransferStatus::PendingBylawsReview
        );
    }

    #[test]
    fn deduplicates_generated_documents() {
        let mut workflow = TransferWorkflow::new(
            TransferWorkflowId::new(),
            EntityId::new(),
            WorkspaceId::new(),
            TransferId::new(),
            IntentId::new(),
        );
        workflow.add_generated_documents(vec![
            "a.md".to_owned(),
            "a.md".to_owned(),
            "b.md".to_owned(),
        ]);
        assert_eq!(workflow.generated_documents().len(), 2);
    }
}
