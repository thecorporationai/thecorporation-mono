//! Webhook HTTP routes.
//!
//! Endpoints for receiving external webhook events (Stripe, etc.).
//! These accept POST payloads and store events in workspace repos.

use axum::{Json, Router, body::Bytes, http::HeaderMap, routing::post};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::error::AppError;

// ── Request / Response types ──────────────────────────────────────────

#[derive(Deserialize)]
pub struct StripeWebhookPayload {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type", default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Serialize)]
pub struct WebhookAckResponse {
    pub received: bool,
    pub event_id: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────

async fn stripe_webhook(
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, AppError> {
    verify_webhook_secret(&headers, "STRIPE_WEBHOOK_SECRET")?;
    let payload: StripeWebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("invalid webhook JSON: {e}")))?;
    tracing::info!(
        event_type = ?payload.event_type,
        event_id = ?payload.id,
        "Received Stripe webhook"
    );

    Ok(Json(WebhookAckResponse {
        received: true,
        event_id: payload.id,
    }))
}

async fn stripe_billing_webhook(
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, AppError> {
    verify_webhook_secret(&headers, "STRIPE_BILLING_WEBHOOK_SECRET")?;
    let payload: StripeWebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("invalid webhook JSON: {e}")))?;
    tracing::info!(
        event_type = ?payload.event_type,
        event_id = ?payload.id,
        "Received Stripe billing webhook"
    );

    Ok(Json(WebhookAckResponse {
        received: true,
        event_id: payload.id,
    }))
}

fn verify_webhook_secret(headers: &HeaderMap, env_name: &str) -> Result<(), AppError> {
    let expected = std::env::var(env_name)
        .map_err(|_| AppError::Internal(format!("{env_name} is not configured")))?;
    let provided = headers
        .get("x-webhook-secret")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing webhook secret".to_owned()))?;
    // Constant-time comparison to prevent timing attacks
    let expected_bytes = expected.as_bytes();
    let provided_bytes = provided.as_bytes();
    if expected_bytes.len() != provided_bytes.len()
        || expected_bytes
            .iter()
            .zip(provided_bytes.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            != 0
    {
        return Err(AppError::Unauthorized("invalid webhook secret".to_owned()));
    }
    Ok(())
}

// ── Router ────────────────────────────────────────────────────────────

pub fn webhook_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/webhooks/stripe", post(stripe_webhook))
        .route("/v1/webhooks/stripe-billing", post(stripe_billing_webhook))
}
