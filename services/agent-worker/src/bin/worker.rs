//! Agent worker binary — Redis queue consumer + kill listener + cron scheduler.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use agent_worker::config::WorkerConfig;
use agent_worker::redis::pool as redis_pool;
use agent_worker::worker::{consumer, cron, docker, kill};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_worker=info".parse().unwrap()),
        )
        .json()
        .init();

    dotenvy::dotenv().ok();

    let config = Arc::new(WorkerConfig::from_env()?);
    let worker_id = config.worker_id();
    tracing::info!(worker_id = %worker_id, "agent-worker starting");

    let pool = redis_pool::create_pool(&config.redis_url)?;
    tracing::info!("redis connected");

    let docker_client = docker::connect(&config.docker_host)?;
    tracing::info!("docker connected");

    // Shutdown signal
    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("received SIGINT, shutting down");
        shutdown_clone.cancel();
    });

    // Spawn kill command listener
    let kill_pool = pool.clone();
    let kill_docker = docker_client.clone();
    let kill_shutdown = shutdown.clone();
    let kill_redis_url = config.redis_url.clone();
    tokio::spawn(async move {
        if let Err(e) = kill::listen(kill_pool, kill_docker, kill_redis_url, kill_shutdown).await {
            tracing::error!(error = %e, "kill listener failed");
        }
    });

    // Spawn cron loop
    let cron_pool = pool.clone();
    let cron_config = config.clone();
    let cron_shutdown = shutdown.clone();
    tokio::spawn(async move {
        if let Err(e) = cron::run(cron_pool, cron_config, cron_shutdown).await {
            tracing::error!(error = %e, "cron loop failed");
        }
    });

    // Run consumer loop (blocks until shutdown)
    consumer::run(pool, docker_client, config, worker_id, shutdown).await?;

    tracing::info!("agent-worker stopped");
    Ok(())
}
