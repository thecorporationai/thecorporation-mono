//! LLM proxy route — forwards container LLM requests to an upstream provider.
//!
//! Containers call `POST /v1/llm/proxy/{path}` with `Authorization: Bearer tok_xxx`.
//! The proxy resolves the opaque token to a real API key, forwards the request
//! to the configured upstream (e.g. OpenRouter), extracts usage from the response,
//! and accumulates per-model token counts + cost in Redis.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::post,
    Router,
};
use deadpool_redis::redis::AsyncCommands;
use serde::Deserialize;

use super::AppState;
use crate::error::AppError;

/// Usage key TTL: 7 days.
const USAGE_TTL_SECS: i64 = 7 * 24 * 3600;

/// Minimal struct to extract usage from the LLM response JSON.
#[derive(Deserialize, Default)]
struct LlmResponse {
    usage: Option<LlmUsage>,
    model: Option<String>,
}

#[derive(Deserialize, Default)]
struct LlmUsage {
    prompt_tokens: i64,
    completion_tokens: i64,
}

/// Redis key for proxy-accumulated usage for an execution.
fn usage_key(execution_id: &str) -> String {
    format!("aw:usage:{execution_id}")
}

async fn proxy_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, AppError> {
    // 1. Extract Bearer token
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing Authorization header".to_owned()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("invalid Authorization format".to_owned()))?;

    // 2. Redis: reverse lookup token -> execution_id
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| AppError::Internal("redis not configured".to_owned()))?;
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis: {e}")))?;

    let reverse_key = format!("aw:tokens:reverse:{token}");
    let execution_id: Option<String> = conn
        .get(&reverse_key)
        .await
        .map_err(|e| AppError::Internal(format!("redis get: {e}")))?;

    let execution_id = execution_id
        .ok_or_else(|| AppError::Forbidden("invalid or expired token".to_owned()))?;

    // 3. Verify execution is still active
    let exec_key = format!("aw:exec:{execution_id}");
    let status: Option<String> = conn
        .hget(&exec_key, "status")
        .await
        .map_err(|e| AppError::Internal(format!("redis hget: {e}")))?;

    match status.as_deref() {
        Some("queued") | Some("running") => {}
        _ => return Err(AppError::Forbidden("execution is no longer active".to_owned())),
    }

    // 4. Resolve token -> real API key
    let tokens_key = format!("aw:tokens:{execution_id}");
    let api_key: Option<String> = conn
        .hget(&tokens_key, token)
        .await
        .map_err(|e| AppError::Internal(format!("redis hget: {e}")))?;

    let api_key =
        api_key.ok_or_else(|| AppError::Forbidden("token not found".to_owned()))?;

    // 5. Forward request to upstream
    let upstream_url = format!("{}/{path}", state.llm_upstream_url);

    let body_bytes = axum::body::to_bytes(body, 10 * 1024 * 1024)
        .await
        .map_err(|e| AppError::BadRequest(format!("failed to read request body: {e}")))?;

    let mut upstream_req = state
        .http_client
        .post(&upstream_url)
        .header("authorization", format!("Bearer {api_key}"))
        .header("content-type", "application/json");

    // Forward select headers
    if let Some(v) = headers.get("x-request-id") {
        upstream_req = upstream_req.header("x-request-id", v);
    }

    let upstream_resp = upstream_req
        .body(body_bytes.clone())
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("upstream request failed: {e}")))?;

    let resp_status = upstream_resp.status();
    let resp_headers = upstream_resp.headers().clone();
    let resp_bytes = upstream_resp
        .bytes()
        .await
        .map_err(|e| AppError::Internal(format!("failed to read upstream response: {e}")))?;

    // 6. Extract usage and accumulate in Redis (best-effort, don't fail the response)
    if resp_status.is_success() {
        if let Ok(parsed) = serde_json::from_slice::<LlmResponse>(&resp_bytes) {
            if let Some(usage) = parsed.usage {
                let model = parsed.model.unwrap_or_default();
                let key = usage_key(&execution_id);

                // Compute cost
                let cost = state
                    .model_pricing
                    .get(&model)
                    .map(|p| {
                        (usage.prompt_tokens as f64 * p.input as f64
                            + usage.completion_tokens as f64 * p.output as f64)
                            / 100_000_000.0
                    })
                    .unwrap_or(0.0);

                // Pipeline all Redis updates
                let res: Result<(), _> = async {
                    let prompt_field = format!("model:{model}:prompt_tokens");
                    let completion_field = format!("model:{model}:completion_tokens");
                    let cost_field = format!("model:{model}:cost");

                    conn.hincr::<_, _, _, i64>(&key, &prompt_field, usage.prompt_tokens).await?;
                    conn.hincr::<_, _, _, i64>(&key, &completion_field, usage.completion_tokens).await?;
                    conn.hincr::<_, _, _, f64>(&key, &cost_field, cost).await?;
                    conn.hincr::<_, _, _, f64>(&key, "total_cost", cost).await?;
                    conn.hincr::<_, _, _, i64>(&key, "request_count", 1i64).await?;
                    conn.expire::<_, ()>(&key, USAGE_TTL_SECS).await?;
                    Ok::<(), deadpool_redis::redis::RedisError>(())
                }
                .await;

                if let Err(e) = res {
                    tracing::warn!(execution_id = %execution_id, error = %e, "failed to record LLM usage");
                }
            }
        }
    }

    // 7. Return original response to container
    let mut response = Response::builder().status(StatusCode::from_u16(resp_status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));

    // Forward content-type from upstream
    if let Some(ct) = resp_headers.get("content-type") {
        response = response.header("content-type", ct);
    }

    Ok(response
        .body(Body::from(resp_bytes))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        }))
}

pub fn llm_proxy_routes() -> Router<AppState> {
    Router::new().route("/v1/llm/proxy/{*path}", post(proxy_handler))
}
