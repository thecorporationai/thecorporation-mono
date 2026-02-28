//! Agent execution HTTP routes — status, logs, kill, streaming.
//!
//! All execution state is stored in Redis (set by agent-worker).

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use deadpool_redis::redis::AsyncCommands;
use serde::Serialize;
use std::collections::HashMap;

use agent_types::KillCommand;
use super::AppState;
use crate::domain::ids::{AgentId, ExecutionId};
use crate::error::AppError;

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ExecutionResponse {
    pub execution_id: ExecutionId,
    pub agent_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct KillResponse {
    pub execution_id: ExecutionId,
    pub status: String,
}

// ── Helpers ──────────────────────────────────────────────────────────

fn require_redis(state: &AppState) -> Result<&deadpool_redis::Pool, AppError> {
    state.redis.as_ref().ok_or_else(|| {
        AppError::Internal("redis not configured".to_owned())
    })
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn get_execution(
    State(state): State<AppState>,
    Path((_agent_id, execution_id)): Path<(AgentId, ExecutionId)>,
) -> Result<Json<ExecutionResponse>, AppError> {
    let redis = require_redis(&state)?;
    let mut conn = redis.get().await
        .map_err(|e| AppError::Internal(format!("redis: {e}")))?;

    let key = format!("aw:exec:{execution_id}");
    let fields: HashMap<String, String> = conn.hgetall(&key).await
        .map_err(|e| AppError::Internal(format!("redis hgetall: {e}")))?;

    if fields.is_empty() {
        return Err(AppError::NotFound(format!("execution {execution_id} not found")));
    }

    Ok(Json(ExecutionResponse {
        execution_id,
        agent_id: fields.get("agent_id").cloned().unwrap_or_default(),
        status: fields.get("status").cloned().unwrap_or_default(),
        container_id: fields.get("container_id").cloned(),
        started_at: fields.get("started_at").cloned(),
        completed_at: fields.get("completed_at").cloned(),
        reason: fields.get("reason").cloned(),
    }))
}

async fn get_execution_result(
    State(state): State<AppState>,
    Path((_agent_id, execution_id)): Path<(AgentId, ExecutionId)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let redis = require_redis(&state)?;
    let mut conn = redis.get().await
        .map_err(|e| AppError::Internal(format!("redis: {e}")))?;

    let key = format!("aw:exec:{execution_id}:result");
    let json: Option<String> = conn.get(&key).await
        .map_err(|e| AppError::Internal(format!("redis get: {e}")))?;

    match json {
        Some(s) => {
            let val: serde_json::Value = serde_json::from_str(&s)
                .map_err(|e| AppError::Internal(format!("parse result: {e}")))?;
            Ok(Json(val))
        }
        None => Err(AppError::NotFound(format!("result for {execution_id} not found"))),
    }
}

async fn get_execution_logs(
    State(state): State<AppState>,
    Path((_agent_id, execution_id)): Path<(AgentId, ExecutionId)>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let redis = require_redis(&state)?;
    let mut conn = redis.get().await
        .map_err(|e| AppError::Internal(format!("redis: {e}")))?;

    let key = format!("aw:logs:{execution_id}:history");
    let entries: Vec<String> = conn.lrange(&key, 0, -1).await
        .map_err(|e| AppError::Internal(format!("redis lrange: {e}")))?;

    let logs: Vec<serde_json::Value> = entries.iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();

    Ok(Json(logs))
}

async fn kill_execution(
    State(state): State<AppState>,
    Path((_agent_id, execution_id)): Path<(AgentId, ExecutionId)>,
) -> Result<Json<KillResponse>, AppError> {
    let redis = require_redis(&state)?;
    let mut conn = redis.get().await
        .map_err(|e| AppError::Internal(format!("redis: {e}")))?;

    // Check execution exists and is running
    let state_key = format!("aw:exec:{execution_id}");
    let status: Option<String> = conn.hget(&state_key, "status").await
        .map_err(|e| AppError::Internal(format!("redis hget: {e}")))?;

    match status.as_deref() {
        None => return Err(AppError::NotFound(format!("execution {execution_id} not found"))),
        Some("queued") => {
            // Cancel directly — no container to kill
            conn.hset_multiple::<_, _, _, ()>(&state_key, &[
                ("status", "cancelled"),
                ("completed_at", &chrono::Utc::now().to_rfc3339()),
            ]).await.map_err(|e| AppError::Internal(format!("redis hset: {e}")))?;

            return Ok(Json(KillResponse {
                execution_id,
                status: "cancelled".to_owned(),
            }));
        }
        Some("running") => {
            // Publish kill command via pub/sub
            let reply_id = uuid::Uuid::new_v4().to_string();
            let cmd = KillCommand {
                execution_id,
                reply_id: reply_id.clone(),
            };
            let cmd_json = serde_json::to_string(&cmd)
                .map_err(|e| AppError::Internal(format!("serialize kill: {e}")))?;
            conn.publish::<_, _, ()>("aw:cmd:kill", &cmd_json)
                .await.map_err(|e| AppError::Internal(format!("redis publish: {e}")))?;

            drop(conn);

            // Wait for ack (2s)
            let mut conn = redis.get().await
                .map_err(|e| AppError::Internal(format!("redis: {e}")))?;
            let reply_key = format!("aw:rpc:reply:{reply_id}");
            let _result: Option<(String, String)> = deadpool_redis::redis::cmd("BLPOP")
                .arg(&reply_key)
                .arg(2.0_f64)
                .query_async(&mut *conn)
                .await
                .map_err(|e| AppError::Internal(format!("redis blpop: {e}")))?;

            Ok(Json(KillResponse {
                execution_id,
                status: "killed".to_owned(),
            }))
        }
        Some(s) => {
            // Already terminal
            Ok(Json(KillResponse {
                execution_id,
                status: s.to_owned(),
            }))
        }
    }
}

// ── Router ───────────────────────────────────────────────────────────

pub fn execution_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/agents/{agent_id}/executions/{execution_id}", get(get_execution))
        .route("/v1/agents/{agent_id}/executions/{execution_id}/result", get(get_execution_result))
        .route("/v1/agents/{agent_id}/executions/{execution_id}/logs", get(get_execution_logs))
        .route("/v1/agents/{agent_id}/executions/{execution_id}/kill", post(kill_execution))
}
