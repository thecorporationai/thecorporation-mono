//! Agent domain errors.

use thiserror::Error;

use crate::domain::ids::AgentId;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("agent validation: {0}")]
    Validation(String),
}
