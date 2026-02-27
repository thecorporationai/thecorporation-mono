//! Formation domain errors.

use super::types::{EntityType, FormationStatus};
use crate::domain::ids::{DocumentId, EntityId};
use thiserror::Error;

/// Errors that can occur in the formation domain.
#[derive(Debug, Error)]
pub enum FormationError {
    /// A field value failed validation.
    #[error("validation error: {0}")]
    Validation(String),

    /// The requested entity does not exist.
    #[error("entity {0} not found")]
    EntityNotFound(EntityId),

    /// The requested document does not exist.
    #[error("document {0} not found")]
    DocumentNotFound(DocumentId),

    /// The formation cannot transition between the given states.
    #[error("invalid formation transition from {from} to {to}")]
    InvalidTransition {
        from: FormationStatus,
        to: FormationStatus,
    },

    /// The document has already been signed and cannot be signed again.
    #[error("document {0} has already been signed")]
    DocumentAlreadySigned(DocumentId),

    /// Not all required signatures have been collected.
    #[error("document {document_id} is missing signatures from: {missing:?}")]
    AllSignaturesRequired {
        document_id: DocumentId,
        missing: Vec<String>,
    },

    /// Document content has been tampered with since the hash was computed.
    #[error("content hash mismatch for document {0}")]
    ContentHashMismatch(DocumentId),

    /// An EIN has already been assigned to this entity.
    #[error("entity {0} already has an EIN assigned")]
    EinAlreadyAssigned(EntityId),

    /// The operation requires a different entity type.
    #[error("expected entity type {expected}, got {got}")]
    InvalidEntityType {
        expected: EntityType,
        got: EntityType,
    },

    /// An error from the git storage layer.
    #[error("storage error: {0}")]
    Storage(String),
}
