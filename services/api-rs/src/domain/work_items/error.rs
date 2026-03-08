use super::types::WorkItemStatus;
use crate::domain::ids::WorkItemId;

#[derive(Debug, thiserror::Error)]
pub enum WorkItemError {
    #[error("work item not found: {0}")]
    WorkItemNotFound(WorkItemId),

    #[error("invalid transition from {from} to {to}")]
    InvalidTransition {
        from: WorkItemStatus,
        to: WorkItemStatus,
    },

    #[error("work item {0} is not currently claimed")]
    NotClaimed(WorkItemId),
}
