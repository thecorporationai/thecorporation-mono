//! Queue consumer loop — the heart of the worker.
//!
//! BLMOVE from Redis queue to processing list, acquire per-agent lock,
//! ack via RPC reply, dispatch lifecycle.

use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use agent_types::{RpcReply, RpcStatus};
use crate::config::WorkerConfig;
use crate::error::WorkerError;
use crate::redis::{lock, queue, rpc};

/// Run the consumer loop until cancelled.
pub async fn run(
    pool: deadpool_redis::Pool,
    docker: bollard::Docker,
    http_client: reqwest::Client,
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
            result = queue::dequeue_to_processing(&pool, &worker_id, config.poll_seconds) => {
                let Some((raw_payload, job)) = result? else {
                    continue; // timeout, loop again
                };

                let job_id_str = job.job_id().to_string();

                // Try to acquire per-agent lock
                if !lock::acquire(&pool, job.agent_id(), &worker_id).await? {
                    // Agent busy — move back to queue and reject this RPC
                    tracing::debug!(agent_id = %job.agent_id(), "agent locked, re-enqueueing");
                    queue::reject_to_queue(&pool, &worker_id, &raw_payload).await?;
                    rpc::send_reply(&pool, &job_id_str, &RpcReply {
                        status: RpcStatus::Rejected,
                        execution_id: job.execution_id(),
                        message: Some("agent is busy".to_owned()),
                    }).await.ok();
                    continue;
                }

                // Ack the job immediately — worker has accepted it
                rpc::send_reply(&pool, &job_id_str, &RpcReply {
                    status: RpcStatus::Accepted,
                    execution_id: job.execution_id(),
                    message: None,
                }).await.ok();

                // Acquire concurrency permit
                let permit = semaphore.clone().acquire_owned().await
                    .expect("semaphore closed");

                let pool = pool.clone();
                let docker = docker.clone();
                let http_client = http_client.clone();
                let config = config.clone();
                let worker_id = worker_id.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    let exec_id = job.execution_id();
                    let agent_id = job.agent_id();

                    if let Err(e) = super::lifecycle::run(
                        job,
                        &raw_payload,
                        &pool,
                        &docker,
                        &http_client,
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
