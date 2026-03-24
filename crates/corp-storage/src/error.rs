//! Storage error type for the corp-storage crate.
//!
//! All backend errors are mapped into this enum so callers deal with one
//! unified error type regardless of whether the underlying store is git, Redis,
//! or S3.

use thiserror::Error;

/// All errors that can be produced by the storage layer.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The requested key / path / entity was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// An entity or key that must not already exist was found.
    #[error("already exists: {0}")]
    AlreadyExists(String),

    /// The data at the given location was structurally invalid.
    #[error("invalid data: {0}")]
    InvalidData(String),

    /// An error produced by the git backend.
    #[error("git error: {0}")]
    GitError(String),

    /// An error produced by the Redis / Valkey KV backend.
    #[error("kv error: {0}")]
    KvError(String),

    /// An error produced by the S3 backend.
    #[error("s3 error: {0}")]
    S3Error(String),

    /// A (de-)serialization failure.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// An optimistic-locking conflict was detected; the caller should retry.
    #[error("concurrency conflict: {0}")]
    ConcurrencyConflict(String),

    /// A generic I/O error.
    #[error("io error: {0}")]
    Io(String),
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::SerializationError(e.to_string())
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e.to_string())
    }
}
