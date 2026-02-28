use agent_types::{AgentId, ExecutionId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("redis error: {0}")]
    Redis(#[from] deadpool_redis::redis::RedisError),

    #[error("redis pool error: {0}")]
    Pool(#[from] deadpool_redis::PoolError),

    #[error("docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("lock not acquired for agent {0}")]
    LockNotAcquired(AgentId),

    #[error("execution not found: {0}")]
    ExecutionNotFound(ExecutionId),

    #[error("agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("container failed: {0}")]
    ContainerFailed(String),

    #[error("execution timed out: {0}")]
    Timeout(ExecutionId),

    #[error("queue full ({current}/{max})")]
    QueueFull { current: u64, max: u64 },

    #[error("git error: {0}")]
    Git(String),

    #[error("{0}")]
    Internal(String),
}
