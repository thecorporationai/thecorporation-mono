//! Git storage error types.
//!
//! All git-layer operations return `Result<_, GitStorageError>`.
//! The application error layer (`crate::error::AppError`) converts these
//! into appropriate HTTP status codes.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitStorageError {
    /// The bare repo directory does not exist on disk.
    #[error("repository not found: {0}")]
    RepoNotFound(String),

    /// A path (file or directory) was not found in the tree at the given ref.
    #[error("not found: {0}")]
    NotFound(String),

    /// The requested branch does not exist.
    #[error("branch not found: {0}")]
    BranchNotFound(String),

    /// Attempted to create a branch that already exists.
    #[error("branch already exists: {0}")]
    BranchAlreadyExists(String),

    /// A merge could not be completed as a fast-forward.
    #[error("merge conflict: {0}")]
    MergeConflict(String),

    /// JSON serialization or deserialization failed.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Wrapped `git2` library error.
    #[error("git error: {0}")]
    Git(String),

    /// Commit signing failed.
    #[error("signing error: {0}")]
    SigningError(String),

    /// Filesystem I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<git2::Error> for GitStorageError {
    fn from(e: git2::Error) -> Self {
        GitStorageError::Git(e.message().to_owned())
    }
}
