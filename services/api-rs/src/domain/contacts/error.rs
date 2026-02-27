//! Contacts domain errors.

use crate::domain::ids::ContactId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContactError {
    #[error("contact {0} not found")]
    ContactNotFound(ContactId),
    #[error("contact validation error: {0}")]
    Validation(String),
}
