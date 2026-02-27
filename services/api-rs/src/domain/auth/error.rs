//! Auth domain errors.

use thiserror::Error;

/// Errors that can occur in the auth domain.
#[derive(Debug, Error)]
pub enum AuthError {
    /// The API key is not recognized.
    #[error("invalid api key")]
    InvalidApiKey,

    /// The API key has passed its expiration date.
    #[error("expired api key")]
    ExpiredApiKey,

    /// The JWT token is malformed or has an invalid signature.
    #[error("invalid token: {0}")]
    InvalidToken(String),

    /// The JWT token has expired.
    #[error("token expired")]
    TokenExpired,

    /// The principal lacks a required scope.
    #[error("insufficient scopes: missing {0}")]
    InsufficientScopes(String),

    /// Generic unauthorized — no credentials provided.
    #[error("unauthorized")]
    Unauthorized,
}
