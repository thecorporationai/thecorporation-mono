//! Top-level application error type.
//!
//! Domain errors are converted into `AppError`, which implements `IntoResponse`
//! to produce appropriate HTTP status codes. `anyhow::Error` is only used here
//! at the boundary — never in domain code.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::domain::agents::error::AgentError;
use crate::domain::auth::error::AuthError;
use crate::domain::contacts::error::ContactError;
use crate::domain::equity::error::EquityError;
use crate::domain::execution::error::ExecutionError;
use crate::domain::formation::error::FormationError;
use crate::domain::governance::error::GovernanceError;
use crate::domain::services::error::ServiceError;
use crate::domain::treasury::error::TreasuryError;
use crate::domain::work_items::error::WorkItemError;
use crate::git::error::GitStorageError;

#[derive(Debug)]
pub enum AppError {
    /// 400 — client sent invalid input
    BadRequest(String),
    /// 401 — missing or invalid auth
    Unauthorized(String),
    /// 403 — authenticated but lacks permission
    Forbidden(String),
    /// 404 — resource not found
    NotFound(String),
    /// 409 — conflict (e.g., merge conflict, duplicate)
    Conflict(String),
    /// 422 — domain validation failure
    UnprocessableEntity(String),
    /// 429 — rate limited
    RateLimited { limit: u32, window_seconds: u32 },
    /// 501 — not implemented
    NotImplemented(String),
    /// 503 — service temporarily unavailable (e.g., queue full)
    ServiceUnavailable(String),
    /// 500 — unexpected internal error
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, detail) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            Self::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            Self::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg),
            Self::UnprocessableEntity(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation_error", msg)
            }
            Self::RateLimited {
                limit,
                window_seconds,
            } => {
                let body = json!({
                    "error": {
                        "code": "rate_limit_exceeded",
                        "limit": limit,
                        "window_seconds": window_seconds,
                    }
                });
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [("Retry-After", window_seconds.to_string())],
                    Json(body),
                )
                    .into_response();
            }
            Self::NotImplemented(msg) => {
                (StatusCode::NOT_IMPLEMENTED, "not_implemented", msg)
            }
            Self::ServiceUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable", msg)
            }
            Self::Internal(msg) => {
                tracing::error!("internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal server error".to_owned(),
                )
            }
        };

        let body = json!({ "error": { "code": code, "detail": detail } });
        (status, Json(body)).into_response()
    }
}

// ── Domain error conversions ───────────────────────────────────────────

impl From<GitStorageError> for AppError {
    fn from(e: GitStorageError) -> Self {
        match e {
            GitStorageError::NotFound(msg) => Self::NotFound(msg),
            GitStorageError::BranchNotFound(name) => {
                Self::NotFound(format!("branch not found: {name}"))
            }
            GitStorageError::BranchAlreadyExists(name) => {
                Self::Conflict(format!("branch already exists: {name}"))
            }
            GitStorageError::MergeConflict(msg) => Self::Conflict(msg),
            _ => Self::Internal(e.to_string()),
        }
    }
}

impl From<AuthError> for AppError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::InvalidApiKey | AuthError::Unauthorized => Self::Unauthorized(e.to_string()),
            AuthError::ExpiredApiKey | AuthError::TokenExpired => Self::Unauthorized(e.to_string()),
            AuthError::InvalidToken(_) => Self::Unauthorized(e.to_string()),
            AuthError::InsufficientScopes(_) => Self::Forbidden(e.to_string()),
        }
    }
}

impl From<FormationError> for AppError {
    fn from(e: FormationError) -> Self {
        match e {
            FormationError::EntityNotFound(_) => Self::NotFound(e.to_string()),
            FormationError::DocumentNotFound(_) => Self::NotFound(e.to_string()),
            FormationError::InvalidTransition { .. } => Self::UnprocessableEntity(e.to_string()),
            FormationError::Storage(_) => Self::Internal(e.to_string()),
            _ => Self::UnprocessableEntity(e.to_string()),
        }
    }
}

impl From<EquityError> for AppError {
    fn from(e: EquityError) -> Self {
        match e {
            EquityError::GrantNotFound(_)
            | EquityError::ShareClassNotFound(_)
            | EquityError::CapTableNotFound(_)
            | EquityError::SafeNotFound(_)
            | EquityError::ValuationNotFound(_)
            | EquityError::TransferNotFound(_)
            | EquityError::FundingRoundNotFound(_) => Self::NotFound(e.to_string()),
            _ => Self::UnprocessableEntity(e.to_string()),
        }
    }
}

impl From<GovernanceError> for AppError {
    fn from(e: GovernanceError) -> Self {
        match e {
            GovernanceError::BodyNotFound(_)
            | GovernanceError::SeatNotFound(_)
            | GovernanceError::MeetingNotFound(_) => Self::NotFound(e.to_string()),
            _ => Self::UnprocessableEntity(e.to_string()),
        }
    }
}

impl From<TreasuryError> for AppError {
    fn from(e: TreasuryError) -> Self {
        match e {
            TreasuryError::AccountNotFound(_)
            | TreasuryError::InvoiceNotFound(_)
            | TreasuryError::BankAccountNotFound(_) => Self::NotFound(e.to_string()),
            _ => Self::UnprocessableEntity(e.to_string()),
        }
    }
}

impl From<ExecutionError> for AppError {
    fn from(e: ExecutionError) -> Self {
        match e {
            ExecutionError::IntentNotFound(_)
            | ExecutionError::ReceiptNotFound(_)
            | ExecutionError::ObligationNotFound(_) => Self::NotFound(e.to_string()),
            _ => Self::UnprocessableEntity(e.to_string()),
        }
    }
}

impl From<ContactError> for AppError {
    fn from(e: ContactError) -> Self {
        match e {
            ContactError::ContactNotFound(_) => Self::NotFound(e.to_string()),
            ContactError::Validation(msg) => Self::UnprocessableEntity(msg),
        }
    }
}

impl From<AgentError> for AppError {
    fn from(e: AgentError) -> Self {
        match e {
            AgentError::AgentNotFound(_) => Self::NotFound(e.to_string()),
            AgentError::Validation(msg) => Self::UnprocessableEntity(msg),
        }
    }
}

impl From<WorkItemError> for AppError {
    fn from(e: WorkItemError) -> Self {
        match e {
            WorkItemError::WorkItemNotFound(_) => Self::NotFound(e.to_string()),
            WorkItemError::InvalidTransition { .. } => Self::UnprocessableEntity(e.to_string()),
            WorkItemError::NotClaimed(_) => Self::UnprocessableEntity(e.to_string()),
        }
    }
}

impl From<ServiceError> for AppError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::ItemNotFound(_) => Self::NotFound(e.to_string()),
            ServiceError::RequestNotFound(_) => Self::NotFound(e.to_string()),
            ServiceError::InvalidTransition { .. } => Self::UnprocessableEntity(e.to_string()),
        }
    }
}
