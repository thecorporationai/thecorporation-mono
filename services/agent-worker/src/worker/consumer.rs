//! Queue consumer loop — the heart of the worker.
//!
//! BLPOP from Redis queue, acquire per-agent lock, ack via RPC reply, dispatch lifecycle.

use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::config::WorkerConfig;
use crate::error::WorkerError;
use crate::redis::{lock, queue, rpc};

/// Run the consumer loop until cancelled.
pub async fn run(
    pool: deadpool_redis::Pool,
    docker: bollard::Docker,
    config: Arc<WorkerConfig>,
    worker_id: String,
    shutdown: CancellationToken,
) -> Result<(), WorkerError> {
    let max_concurrency = if config.max_concurrency > 0 {
        config.max_concurrency
    } else {
        super::capacity::detect()
    };
    tracing::info!(max_concurrency, "consumer loop starting");

    let semaphore = Arc::new(Semaphore::new(max_concurrency));

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("consumer loop shutting down");
                break;
            }
            result = queue::dequeue_job(&pool, config.poll_seconds) => {
                let Some((raw_payload, job)) = result? else {
                    continue; // timeout, loop again
                };

                let agent_id_str = job.agent_id.to_string();
                let job_id_str = job.job_id.to_string();
                let exec_id_str = job.execution_id.to_string();

                // Try to acquire per-agent lock
                if !lock::acquire(&pool, &agent_id_str, &worker_id).await? {
                    // Agent busy — re-enqueue and reject this RPC
                    tracing::debug!(agent_id = %agent_id_str, "agent locked, re-enqueueing");
                    queue::enqueue_job(&pool, &job).await?;
                    rpc::send_reply(&pool, &job_id_str, &rpc::RpcReply {
                        status: rpc::RpcStatus::Rejected,
                        execution_id: exec_id_str,
                        message: Some("agent is busy".to_owned()),
                    }).await.ok();
                    continue;
                }

                // Move to processing list for crash recovery
                queue::mark_processing(&pool, &worker_id, &raw_payload).await?;

                // Ack the job immediately — worker has accepted it
                rpc::send_reply(&pool, &job_id_str, &rpc::RpcReply {
                    status: rpc::RpcStatus::Accepted,
                    execution_id: exec_id_str,
                    message: None,
                }).await.ok();

                // Acquire concurrency permit
                let permit = semaphore.clone().acquire_owned().await
                    .expect("semaphore closed");

                let pool = pool.clone();
                let docker = docker.clone();
                let config = config.clone();
                let worker_id = worker_id.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    let exec_id = job.execution_id.to_string();
                    let agent_id = job.agent_id.to_string();

                    if let Err(e) = super::lifecycle::run(
                        job,
                        &raw_payload,
                        &pool,
                        &docker,
                        &config,
                        &worker_id,
                    ).await {
                        tracing::error!(
                            execution_id = %exec_id,
                            agent_id = %agent_id,
                            error = %e,
                            "lifecycle failed"
                        );
                    }
                });
            }
        }
    }

    Ok(())
}
