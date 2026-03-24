//! Application-level error type with axum `IntoResponse` mapping.
//!
//! All route handlers return `Result<T, AppError>`.  The `IntoResponse` impl
//! turns every variant into a JSON body `{ "error": "<message>" }` with the
//! appropriate HTTP status code.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// All errors that can be returned from a route handler.
#[derive(Debug, Error)]
pub enum AppError {
    /// The requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// The request was malformed or contained invalid data.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// A resource already exists or a state conflict was detected.
    #[error("conflict: {0}")]
    Conflict(String),

    /// Authentication / authorisation failure — delegates status mapping to
    /// [`corp_auth::AuthError`].
    #[error("auth: {0}")]
    Auth(#[from] corp_auth::AuthError),

    /// A storage layer error.
    #[error("storage: {0}")]
    Storage(#[from] corp_storage::error::StorageError),

    /// An unexpected internal server error.
    #[error("internal: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        use corp_auth::AuthError;
        use corp_storage::error::StorageError;

        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),

            // Delegate status selection to the inner auth error.
            AppError::Auth(e) => {
                let status = match e {
                    AuthError::InvalidToken => StatusCode::UNAUTHORIZED,
                    AuthError::ExpiredToken => StatusCode::UNAUTHORIZED,
                    AuthError::MissingToken => StatusCode::UNAUTHORIZED,
                    AuthError::InvalidApiKey => StatusCode::UNAUTHORIZED,
                    AuthError::InsufficientScope(_) => StatusCode::FORBIDDEN,
                    AuthError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
                    AuthError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                };
                (status, e.to_string())
            }

            // Storage not-found maps to 404; everything else is a 500.
            AppError::Storage(e) => {
                let status = match e {
                    StorageError::NotFound(_) => StatusCode::NOT_FOUND,
                    StorageError::AlreadyExists(_) => StatusCode::CONFLICT,
                    StorageError::ConcurrencyConflict(_) => StatusCode::CONFLICT,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                (status, e.to_string())
            }

            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
