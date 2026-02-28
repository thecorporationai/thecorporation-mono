//! Kill command listener — subscribes to aw:cmd:kill pub/sub channel.

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
                let cmd: rpc::KillCommand = match serde_json::from_str(&payload) {
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

async fn handle_kill(pool: &Pool, docker: &Docker, cmd: &rpc::KillCommand) {
    let execution_id = &cmd.execution_id;

    // Look up container ID from execution state
    let exec_state = match state::get_state(pool, execution_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(execution_id, error = %e, "kill: failed to get state");
            send_kill_reply(pool, &cmd.reply_key, execution_id, rpc::RpcStatus::NotFound).await;
            return;
        }
    };

    let container_id = match exec_state.get("container_id") {
        Some(id) if !id.is_empty() => id.clone(),
        _ => {
            tracing::warn!(execution_id, "kill: no container_id");
            send_kill_reply(pool, &cmd.reply_key, execution_id, rpc::RpcStatus::NotFound).await;
            return;
        }
    };

    // Kill the container
    match docker.kill_container::<String>(&container_id, None).await {
        Ok(()) => {
            tracing::info!(execution_id, container_id = %container_id, "killed container");
            state::set_failed(pool, execution_id, "killed_by_user").await.ok();
            send_kill_reply(pool, &cmd.reply_key, execution_id, rpc::RpcStatus::Killed).await;
        }
        Err(e) => {
            tracing::warn!(execution_id, error = %e, "kill: docker kill failed");
            send_kill_reply(pool, &cmd.reply_key, execution_id, rpc::RpcStatus::NotFound).await;
        }
    }
}

async fn send_kill_reply(pool: &Pool, reply_key: &str, execution_id: &str, status: rpc::RpcStatus) {
    let reply_id = reply_key.strip_prefix("aw:rpc:reply:").unwrap_or(reply_key);
    rpc::send_reply(pool, reply_id, &rpc::RpcReply {
        status,
        execution_id: execution_id.to_owned(),
        message: None,
    }).await.ok();
}
