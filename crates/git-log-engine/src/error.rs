use thiserror::Error;

#[derive(Debug, Error)]
pub enum LogEngineError {
    #[error("git error: {0}")]
    Git(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("log corrupt: {0}")]
    Corrupt(String),

    #[error("commit not found: {0}")]
    CommitNotFound(String),
}

impl From<git2::Error> for LogEngineError {
    fn from(e: git2::Error) -> Self {
        LogEngineError::Git(e.message().to_owned())
    }
}
