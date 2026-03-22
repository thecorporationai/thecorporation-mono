use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("git error: {0}")]
    Git(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("ref not found: {0}")]
    RefNotFound(String),

    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("internal error: {0}")]
    Internal(String),
}
