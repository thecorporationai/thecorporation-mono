//! Secrets proxy routes — resolve opaque tokens for agent containers.
//!
//! The agent runtime (Pi) calls these endpoints to exchange opaque tokens
//! for real secret values, so secrets never exist inside the container.

use axum::{Json, Router, extract::State, routing::post};
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::error::AppError;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResolveRequest {
    pub token: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ResolveResponse {
    pub value: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct InterpolateRequest {
    pub execution_id: String,
    pub template: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct InterpolateResponse {
    pub result: String,
}

#[utoipa::path(
    post,
    path = "/v1/secrets/resolve",
    tag = "secrets",
    request_body = ResolveRequest,
    responses((status = 200, body = ResolveResponse)),
)]
async fn resolve_token(
    State(state): State<AppState>,
    Json(req): Json<ResolveRequest>,
) -> Result<Json<ResolveResponse>, AppError> {
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| AppError::Internal("redis not configured".to_owned()))?;
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis: {e}")))?;

    // Reverse lookup: token -> execution_id
    let reverse_key = format!("aw:tokens:reverse:{}", req.token);
    let execution_id: Option<String> = conn
        .get(&reverse_key)
        .await
        .map_err(|e| AppError::Internal(format!("redis get: {e}")))?;

    let execution_id =
        execution_id.ok_or_else(|| AppError::Forbidden("invalid or expired token".to_owned()))?;

    // Check execution is still active
    let exec_key = format!("aw:exec:{execution_id}");
    let status: Option<String> = conn
        .hget(&exec_key, "status")
        .await
        .map_err(|e| AppError::Internal(format!("redis hget: {e}")))?;

    match status.as_deref() {
        Some("queued") | Some("running") => {}
        _ => {
            return Err(AppError::Forbidden(
                "execution is no longer active".to_owned(),
            ));
        }
    }

    // Resolve token to real value
    let tokens_key = format!("aw:tokens:{execution_id}");
    let value: Option<String> = conn
        .hget(&tokens_key, &req.token)
        .await
        .map_err(|e| AppError::Internal(format!("redis hget: {e}")))?;

    let value = value.ok_or_else(|| AppError::Forbidden("token not found".to_owned()))?;

    Ok(Json(ResolveResponse { value }))
}

#[utoipa::path(
    post,
    path = "/v1/secrets/interpolate",
    tag = "secrets",
    request_body = InterpolateRequest,
    responses((status = 200, body = InterpolateResponse)),
)]
async fn interpolate_template(
    State(state): State<AppState>,
    Json(req): Json<InterpolateRequest>,
) -> Result<Json<InterpolateResponse>, AppError> {
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| AppError::Internal("redis not configured".to_owned()))?;
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::Internal(format!("redis: {e}")))?;

    // Check execution is still active
    let exec_key = format!("aw:exec:{}", req.execution_id);
    let status: Option<String> = conn
        .hget(&exec_key, "status")
        .await
        .map_err(|e| AppError::Internal(format!("redis hget: {e}")))?;

    match status.as_deref() {
        Some("queued") | Some("running") => {}
        _ => {
            return Err(AppError::Forbidden(
                "execution is no longer active".to_owned(),
            ));
        }
    }

    // Get all tokens for this execution
    let tokens_key = format!("aw:tokens:{}", req.execution_id);
    let tokens: std::collections::HashMap<String, String> = conn
        .hgetall(&tokens_key)
        .await
        .map_err(|e| AppError::Internal(format!("redis hgetall: {e}")))?;

    // Replace all opaque tokens in the template with real values
    let mut result = req.template;
    for (token, value) in &tokens {
        result = result.replace(token, value);
    }

    Ok(Json(InterpolateResponse { result }))
}

pub fn secrets_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/secrets/resolve", post(resolve_token))
        .route("/v1/secrets/interpolate", post(interpolate_template))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(resolve_token, interpolate_template),
    components(schemas(
        ResolveRequest,
        ResolveResponse,
        InterpolateRequest,
        InterpolateResponse
    ))
)]
pub struct SecretsProxyApi;
