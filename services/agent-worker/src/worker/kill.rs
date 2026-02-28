//! Kill command listener — subscribes to aw:cmd:kill pub/sub channel.

use agent_types::{ExecutionId, KillCommand, RpcReply, RpcStatus};
use bollard::Docker;
use deadpool_redis::Pool;
use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::error::WorkerError;
use crate::redis::{keys, rpc, state};

/// Listen for kill commands and stop containers.
pub async fn listen(
    pool: Pool,
    docker: Docker,
    redis_url: String,
    shutdown: CancellationToken,
) -> Result<(), WorkerError> {
    // Dedicated connection for pub/sub (can't reuse pool connections for subscribe)
    let client = deadpool_redis::redis::Client::open(redis_url.as_str())
        .map_err(|e| WorkerError::Internal(format!("redis client for kill listener: {e}")))?;

    let mut pubsub = client.get_async_pubsub().await?;
    pubsub.subscribe(keys::CMD_KILL).await?;
    tracing::info!("kill command listener started");

    let mut stream = pubsub.on_message();
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            msg = stream.next() => {
                let Some(msg) = msg else { break; };
                let payload: String = match msg.get_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(error = %e, "bad kill command payload");
                        continue;
                    }
                };
                let cmd: KillCommand = match serde_json::from_str(&payload) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!(error = %e, "bad kill command json");
                        continue;
                    }
                };

                handle_kill(&pool, &docker, &cmd).await;
            }
        }
    }

    Ok(())
}

async fn handle_kill(pool: &Pool, docker: &Docker, cmd: &KillCommand) {
    let execution_id = cmd.execution_id;

    // Look up container ID from execution state
    let exec_state = match state::get_state(pool, execution_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(execution_id = %execution_id, error = %e, "kill: failed to get state");
            send_kill_reply(pool, &cmd.reply_id, execution_id, RpcStatus::NotFound).await;
            return;
        }
    };

    let container_id = match exec_state.container_id {
        Some(ref id) => id.clone(),
        None => {
            tracing::warn!(execution_id = %execution_id, "kill: no container_id");
            send_kill_reply(pool, &cmd.reply_id, execution_id, RpcStatus::NotFound).await;
            return;
        }
    };

    // Kill the container
    match docker.kill_container::<String>(&container_id, None).await {
        Ok(()) => {
            tracing::info!(execution_id = %execution_id, container_id = %container_id, "killed container");
            state::set_failed(pool, execution_id, "killed_by_user").await.ok();
            send_kill_reply(pool, &cmd.reply_id, execution_id, RpcStatus::Killed).await;
        }
        Err(e) => {
            tracing::warn!(execution_id = %execution_id, error = %e, "kill: docker kill failed");
            send_kill_reply(pool, &cmd.reply_id, execution_id, RpcStatus::NotFound).await;
        }
    }
}

async fn send_kill_reply(pool: &Pool, reply_id: &str, execution_id: ExecutionId, status: RpcStatus) {
    rpc::send_reply(pool, reply_id, &RpcReply {
        status,
        execution_id,
        message: None,
    }).await.ok();
}
