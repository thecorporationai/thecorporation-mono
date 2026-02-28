//! Billing HTTP routes.
//!
//! Endpoints for checkout, portal, status, and plans.
//! Subscription state is stored in the workspace repo at `billing/subscription.json`.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::RequireAdmin;
use crate::domain::billing::subscription::Subscription;
use crate::domain::ids::{SubscriptionId, WorkspaceId};
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CheckoutRequest {
    pub plan_id: String,
    #[serde(default)]
    pub success_url: Option<String>,
    #[serde(default)]
    pub cancel_url: Option<String>,
}

#[derive(Deserialize)]
pub struct PortalRequest {
    #[serde(default)]
    pub return_url: Option<String>,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
    pub session_id: String,
    pub workspace_id: WorkspaceId,
    pub plan_id: String,
}

#[derive(Serialize)]
pub struct PortalResponse {
    pub portal_url: String,
    pub workspace_id: WorkspaceId,
}

#[derive(Serialize)]
pub struct BillingStatusResponse {
    pub workspace_id: WorkspaceId,
    pub plan: String,
    pub status: String,
    pub current_period_end: Option<String>,
}

#[derive(Serialize)]
pub struct BillingPlan {
    pub plan_id: String,
    pub name: String,
    pub price_cents: i64,
    pub interval: String,
    pub features: Vec<String>,
}

// ── Plan catalog (stored in memory, same for all workspaces) ────────

fn plan_catalog() -> Vec<BillingPlan> {
    vec![
        BillingPlan {
            plan_id: "free".to_owned(),
            name: "Free".to_owned(),
            price_cents: 0,
            interval: "month".to_owned(),
            features: vec!["1 entity".to_owned(), "Basic features".to_owned()],
        },
        BillingPlan {
            plan_id: "pro".to_owned(),
            name: "Pro".to_owned(),
            price_cents: 4900,
            interval: "month".to_owned(),
            features: vec![
                "Unlimited entities".to_owned(),
                "API access".to_owned(),
                "Agent management".to_owned(),
            ],
        },
        BillingPlan {
            plan_id: "enterprise".to_owned(),
            name: "Enterprise".to_owned(),
            price_cents: 19900,
            interval: "month".to_owned(),
            features: vec![
                "Everything in Pro".to_owned(),
                "Priority support".to_owned(),
                "Custom integrations".to_owned(),
                "SSO".to_owned(),
            ],
        },
    ]
}

// ── Helpers ─────────────────────────────────────────────────────────

fn read_or_create_subscription(
    ws_store: &WorkspaceStore<'_>,
    workspace_id: WorkspaceId,
    plan: &str,
) -> Result<Subscription, AppError> {
    match ws_store.read_json::<Subscription>("billing/subscription.json") {
        Ok(sub) => Ok(sub),
        Err(_) => {
            let sub = Subscription::new(SubscriptionId::new(), workspace_id, plan.to_owned());
            ws_store
                .write_json("billing/subscription.json", &sub, "Init billing subscription")
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok(sub)
        }
    }
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn checkout(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<CheckoutResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let plan_id = req.plan_id.clone();

    // Validate plan exists
    if !plan_catalog().iter().any(|p| p.plan_id == plan_id) {
        return Err(AppError::BadRequest(format!("unknown plan: {plan_id}")));
    }

    // Update subscription in workspace repo
    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let plan_id = plan_id.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let mut sub = read_or_create_subscription(&ws_store, workspace_id, "free")?;
            sub.set_plan(plan_id);
            sub.set_status("pending_checkout".to_owned());

            ws_store
                .write_json("billing/subscription.json", &sub, "Checkout initiated")
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let session_id = format!("cs_{}", uuid::Uuid::new_v4());
    Ok(Json(CheckoutResponse {
        checkout_url: format!(
            "https://checkout.stripe.com/c/pay?plan={}&ws={}&session={}",
            req.plan_id, workspace_id, session_id
        ),
        session_id,
        workspace_id,
        plan_id: req.plan_id,
    }))
}

async fn portal(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(_req): Json<PortalRequest>,
) -> Result<Json<PortalResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    // Verify workspace exists
    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;
            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(PortalResponse {
        portal_url: format!(
            "https://billing.stripe.com/p/portal?ws={}",
            workspace_id
        ),
        workspace_id,
    }))
}

async fn billing_status(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<BillingStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let sub = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            read_or_create_subscription(&ws_store, workspace_id, "free")
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(BillingStatusResponse {
        workspace_id: sub.workspace_id(),
        plan: sub.plan().to_owned(),
        status: sub.status().to_owned(),
        current_period_end: sub.current_period_end().map(|s| s.to_owned()),
    }))
}

async fn list_plans() -> Json<Vec<BillingPlan>> {
    Json(plan_catalog())
}

// ── Handlers: Subscriptions ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateSubscriptionRequest {
    pub plan: String,
}

#[derive(Serialize)]
pub struct SubscriptionResponse {
    pub subscription_id: SubscriptionId,
    pub workspace_id: WorkspaceId,
    pub plan: String,
    pub status: String,
    pub current_period_end: Option<String>,
}

fn subscription_to_response(sub: &Subscription) -> SubscriptionResponse {
    SubscriptionResponse {
        subscription_id: sub.subscription_id(),
        workspace_id: sub.workspace_id(),
        plan: sub.plan().to_owned(),
        status: sub.status().to_owned(),
        current_period_end: sub.current_period_end().map(|s| s.to_owned()),
    }
}

async fn create_subscription(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateSubscriptionRequest>,
) -> Result<Json<SubscriptionResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let sub = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let plan = req.plan.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let sub = Subscription::new(SubscriptionId::new(), workspace_id, plan);
            ws_store
                .write_json("billing/subscription.json", &sub, "Create subscription")
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(sub)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(subscription_to_response(&sub)))
}

async fn get_subscription(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(subscription_id): Path<SubscriptionId>,
) -> Result<Json<SubscriptionResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let sub = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let sub: Subscription = ws_store.read_json("billing/subscription.json")
                .map_err(|_| AppError::NotFound(format!("subscription {} not found", subscription_id)))?;
            Ok::<_, AppError>(sub)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(subscription_to_response(&sub)))
}

#[derive(Serialize)]
pub struct TickResponse {
    pub workspaces_processed: usize,
    pub renewals: usize,
}

async fn tick_subscriptions(
    State(state): State<AppState>,
) -> Result<Json<TickResponse>, AppError> {
    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let workspace_ids = layout.list_workspace_ids();
            let mut processed = 0;
            let mut renewals = 0;

            for ws_id in workspace_ids {
                if let Ok(ws_store) = WorkspaceStore::open(&layout, ws_id) {
                    processed += 1;
                    if let Ok(mut sub) = ws_store.read_json::<Subscription>("billing/subscription.json") {
                        if sub.status() == "active" {
                            renewals += 1;
                            sub.set_status("active".to_owned());
                            let _ = ws_store.write_json("billing/subscription.json", &sub, "Subscription tick");
                        }
                    }
                }
            }

            Ok::<_, AppError>(TickResponse {
                workspaces_processed: processed,
                renewals,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(result))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn billing_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/billing/checkout", post(checkout))
        .route("/v1/billing/portal", post(portal))
        .route("/v1/billing/status", get(billing_status))
        .route("/v1/billing/plans", get(list_plans))
        .route("/v1/subscriptions", post(create_subscription))
        .route("/v1/subscriptions/{subscription_id}", get(get_subscription))
        .route("/v1/subscriptions/tick", post(tick_subscriptions))
}
