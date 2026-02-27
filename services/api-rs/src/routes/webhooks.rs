//! Webhook HTTP routes.
//!
//! Endpoints for receiving external webhook events (Stripe, etc.).
//! These accept POST payloads and store events in workspace repos.

use axum::{
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;

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
    Json(payload): Json<StripeWebhookPayload>,
) -> Json<WebhookAckResponse> {
    // In production: verify Stripe signature, dispatch event to handler.
    // For now: acknowledge receipt. Events would be stored in the
    // relevant workspace repo as `webhooks/stripe/{event_id}.json`.
    tracing::info!(
        event_type = ?payload.event_type,
        event_id = ?payload.id,
        "Received Stripe webhook"
    );

    Json(WebhookAckResponse {
        received: true,
        event_id: payload.id,
    })
}

async fn stripe_billing_webhook(
    Json(payload): Json<StripeWebhookPayload>,
) -> Json<WebhookAckResponse> {
    tracing::info!(
        event_type = ?payload.event_type,
        event_id = ?payload.id,
        "Received Stripe billing webhook"
    );

    Json(WebhookAckResponse {
        received: true,
        event_id: payload.id,
    })
}

// ── Router ────────────────────────────────────────────────────────────

pub fn webhook_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/webhooks/stripe", post(stripe_webhook))
        .route("/v1/webhooks/stripe-billing", post(stripe_billing_webhook))
}
