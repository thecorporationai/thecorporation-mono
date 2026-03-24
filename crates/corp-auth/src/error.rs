//! Auth error type and its HTTP response representation.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    /// The supplied JWT or API key could not be decoded / verified.
    #[error("invalid token")]
    InvalidToken,

    /// The JWT has passed its `exp` timestamp.
    #[error("token has expired")]
    ExpiredToken,

    /// No credential was supplied in the request.
    #[error("missing token")]
    MissingToken,

    /// The supplied API key was not found or did not match the stored hash.
    #[error("invalid API key")]
    InvalidApiKey,

    /// The principal's scope set does not include the required scope.
    #[error("insufficient scope: {0}")]
    InsufficientScope(String),

    /// The caller has exceeded the configured rate limit.
    #[error("rate limited")]
    RateLimited,

    /// An unexpected internal error occurred.
    #[error("internal error: {0}")]
    InternalError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InvalidApiKey => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InsufficientScope(_) => (StatusCode::FORBIDDEN, self.to_string()),
            AuthError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            AuthError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, body).into_response()
    }
}
