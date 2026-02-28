//! Services domain errors.

use super::types::ServiceRequestStatus;
use crate::domain::ids::ServiceRequestId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("service item '{0}' not found in catalog")]
    ItemNotFound(String),

    #[error("service request {0} not found")]
    RequestNotFound(ServiceRequestId),

    #[error("invalid service request transition from {from} to {to}")]
    InvalidTransition {
        from: ServiceRequestStatus,
        to: ServiceRequestStatus,
    },
}
