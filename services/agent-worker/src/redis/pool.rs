//! Redis connection pool initialization.

use deadpool_redis::{Config, Pool, Runtime};

use crate::error::WorkerError;

/// Create a Redis connection pool from a URL.
pub fn create_pool(redis_url: &str) -> Result<Pool, WorkerError> {
    let cfg = Config::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .map_err(|e| WorkerError::Internal(format!("failed to create redis pool: {e}")))
}
