//! Admin HTTP routes.
//!
//! Endpoints for workspace listing, audit events, system health, demo seed,
//! and subscriptions. All data is read from git repos on disk.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use super::AppState;
use super::validation::{normalize_slug, require_non_empty_trimmed_max};
use crate::auth::RequireAdmin;
use crate::domain::auth::{
    api_key::generate_api_key,
    scopes::{Scope, ScopeSet},
};
use crate::domain::billing::subscription::Subscription;
use crate::domain::contacts::contact::Contact;
use crate::domain::formation::types::FormationStatus;
use crate::domain::ids::{ApiKeyId, EntityId, SubscriptionId, WorkspaceId};
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkspaceSummary {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub entity_count: usize,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AuditEvent {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: String,
    #[schema(value_type = Object)]
    pub details: serde_json::Value,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SystemHealth {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub git_storage: String,
    pub workspace_count: usize,
}

// ── Handlers ─────────────────────────────────────────────────────────

fn ensure_workspace_access(
    auth_workspace_id: WorkspaceId,
    requested_workspace_id: WorkspaceId,
) -> Result<(), AppError> {
    if auth_workspace_id != requested_workspace_id {
        return Err(AppError::Forbidden("workspace access denied".to_owned()));
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/v1/admin/workspaces",
    tag = "admin",
    responses(
        (status = 200, description = "List all workspaces", body = Vec<WorkspaceSummary>),
    ),
)]
async fn list_workspaces(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkspaceSummary>>, AppError> {
    let workspace_id = auth.workspace_id();
    let summaries = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = match WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref()) {
                Ok(store) => store,
                Err(crate::git::error::GitStorageError::RepoNotFound(_)) => {
                    return Ok::<_, AppError>(Vec::new());
                }
                Err(error) => return Err(AppError::Internal(format!("open workspace: {error}"))),
            };
            let name = ws_store
                .read_workspace()
                .map(|r| r.name)
                .unwrap_or_else(|_| workspace_id.to_string());
            let entity_count = layout.list_entity_ids(workspace_id).len();
            Ok::<_, AppError>(vec![WorkspaceSummary {
                workspace_id,
                name,
                entity_count,
            }])
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(summaries))
}

#[utoipa::path(
    get,
    path = "/v1/admin/audit-events",
    tag = "admin",
    responses(
        (status = 200, description = "List recent audit events", body = Vec<AuditEvent>),
    ),
)]
async fn list_audit_events(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<AuditEvent>>, AppError> {
    let workspace_id = auth.workspace_id();
    let events = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let mut events = Vec::new();
            let ws_store = match WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref()) {
                Ok(store) => store,
                Err(crate::git::error::GitStorageError::RepoNotFound(_)) => {
                    return Ok::<_, AppError>(Vec::new());
                }
                Err(error) => return Err(AppError::Internal(format!("open workspace: {error}"))),
            };
            if let Ok(log_entries) = ws_store.recent_commits(50) {
                for (oid, message, timestamp) in log_entries {
                    events.push(AuditEvent {
                        event_id: oid,
                        event_type: "commit".to_owned(),
                        timestamp,
                        details: serde_json::json!({
                            "workspace_id": workspace_id,
                            "message": message,
                        }),
                    });
                }
            }
            events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            events.truncate(50);

            Ok::<_, AppError>(events)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(events))
}

#[utoipa::path(
    get,
    path = "/v1/admin/system-health",
    tag = "admin",
    responses(
        (status = 200, description = "System health status", body = SystemHealth),
    ),
)]
async fn system_health(
    RequireAdmin(_auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<SystemHealth>, AppError> {
    let workspace_count = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || layout.list_workspace_ids().len()
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?;

    // Check that the data directory is accessible
    let storage_status = if state.layout.data_dir().exists() {
        "operational"
    } else {
        "unavailable"
    };

    Ok(Json(SystemHealth {
        status: "healthy".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        uptime_seconds: 0, // Would need a start-time static to compute
        git_storage: storage_status.to_owned(),
        workspace_count,
    }))
}

// ── Workspace status ────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkspaceStatusResponse {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub status: String,
    pub entity_count: usize,
}

#[utoipa::path(
    get,
    path = "/v1/workspace/status",
    tag = "admin",
    responses(
        (status = 200, description = "Current workspace status", body = WorkspaceStatusResponse),
    ),
)]
async fn workspace_status(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<WorkspaceStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let name = ws_store
                .read_workspace()
                .map(|r| r.name)
                .unwrap_or_else(|_| workspace_id.to_string());

            let entity_count = layout.list_entity_ids(workspace_id).len();

            Ok::<_, AppError>(WorkspaceStatusResponse {
                workspace_id,
                name,
                status: "active".to_owned(),
                entity_count,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkspaceEntitySummary {
    pub entity_id: EntityId,
}

#[utoipa::path(
    get,
    path = "/v1/workspace/entities",
    tag = "admin",
    responses(
        (status = 200, description = "List entities in current workspace", body = Vec<WorkspaceEntitySummary>),
    ),
)]
async fn list_workspace_entities(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkspaceEntitySummary>>, AppError> {
    let workspace_id = auth.workspace_id();

    let entities = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            // Verify workspace exists
            WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let ids = layout.list_entity_ids(workspace_id);
            Ok::<_, AppError>(
                ids.into_iter()
                    .map(|id| WorkspaceEntitySummary { entity_id: id })
                    .collect::<Vec<_>>(),
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(entities))
}

// ── Demo seed ──────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct DemoSeedRequest {
    #[serde(default = "default_scenario")]
    pub scenario: String,
    #[serde(default)]
    pub name: Option<String>,
}

fn default_scenario() -> String {
    "startup".to_owned()
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DemoSeedResponse {
    pub workspace_id: WorkspaceId,
    pub scenario: String,
    pub entity_id: EntityId,
    pub legal_name: String,
    pub entities_created: usize,
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/v1/demo/seed",
    tag = "admin",
    request_body = DemoSeedRequest,
    responses(
        (status = 200, description = "Seed demo data", body = DemoSeedResponse),
    ),
)]
async fn demo_seed(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<DemoSeedRequest>,
) -> Result<Json<DemoSeedResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    state.enforce_creation_rate_limit("admin.demo_seed.create", workspace_id, 5, 60)?;
    let scenario = req.scenario.clone();

    let (entity_id, legal_name, entities_created) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let requested_name = req.name.clone();
        move || {
            // Initialize workspace if it doesn't exist
            let ws_store = match WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref()) {
                Ok(s) => s,
                Err(_) => WorkspaceStore::init(&layout, workspace_id, &format!("Demo: {scenario}"), valkey_client.as_ref())
                    .map_err(|e| AppError::Internal(format!("init workspace: {e}")))?,
            };

            // Create a demo entity based on scenario
            let entity_id = EntityId::new();
            let (entity_type, default_name) = match scenario.as_str() {
                "startup" => (
                    crate::domain::formation::types::EntityType::CCorp,
                    "Demo Startup Inc.",
                ),
                "llc" => (crate::domain::formation::types::EntityType::Llc, "Demo LLC"),
                "restaurant" => (
                    crate::domain::formation::types::EntityType::Llc,
                    "Demo Restaurant LLC",
                ),
                _ => (
                    crate::domain::formation::types::EntityType::CCorp,
                    "Demo Entity",
                ),
            };
            let legal_name = requested_name.unwrap_or_else(|| default_name.to_owned());

            let entity = crate::domain::formation::entity::Entity::new(
                entity_id,
                workspace_id,
                legal_name.clone(),
                entity_type,
                crate::domain::formation::types::Jurisdiction::new("US-DE").unwrap(),
                None,
                None,
            )
            .map_err(|e| AppError::Internal(format!("create entity: {e}")))?;

            // Walk entity through formation statuses to Active so it is
            // fully formed and ready for all operations.
            let mut entity = entity;
            let formation_steps = [
                FormationStatus::DocumentsGenerated,
                FormationStatus::DocumentsSigned,
                FormationStatus::FilingSubmitted,
                FormationStatus::Filed,
                FormationStatus::EinApplied,
                FormationStatus::Active,
            ];
            for step in formation_steps {
                entity
                    .advance_status(step)
                    .map_err(|e| AppError::Internal(format!("advance entity status: {e}")))?;
            }

            crate::store::entity_store::EntityStore::init(
                &layout,
                workspace_id,
                entity_id,
                &entity,
                valkey_client.as_ref(),
            )
            .map_err(|e| AppError::Internal(format!("init entity: {e}")))?;

            // Store a reference in the workspace
            ws_store
                .write_json(
                    &format!("entities/{}.json", entity_id),
                    &serde_json::json!({"entity_id": entity_id, "name": legal_name}),
                    &format!("Register demo entity {entity_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>((entity_id, legal_name, 1usize))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(DemoSeedResponse {
        workspace_id,
        scenario: req.scenario,
        entity_id,
        legal_name,
        entities_created,
        message: format!("Created {} demo entities", entities_created),
    }))
}

// ── Config ───────────────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct ConfigResponse {
    pub version: String,
    pub environment: String,
    pub features: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/v1/config",
    tag = "admin",
    responses(
        (status = 200, description = "System configuration", body = ConfigResponse),
    ),
)]
async fn get_config(RequireAdmin(_auth): RequireAdmin) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        environment: std::env::var("CORP_ENV").unwrap_or_else(|_| "development".to_owned()),
        features: vec![
            "git-storage".to_owned(),
            "branch-workflows".to_owned(),
            "stakeholder-projections".to_owned(),
        ],
    })
}

// ── Workspace link/claim ────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct WorkspaceLinkRequest {
    pub external_id: String,
    pub provider: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkspaceLinkResponse {
    pub workspace_id: WorkspaceId,
    pub linked: bool,
    pub provider: String,
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/link",
    tag = "admin",
    request_body = WorkspaceLinkRequest,
    responses(
        (status = 200, description = "Link workspace to external provider", body = WorkspaceLinkResponse),
    ),
)]
async fn link_workspace(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<WorkspaceLinkRequest>,
) -> Result<Json<WorkspaceLinkResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let provider = normalize_slug(&req.provider, "provider", 64)?;
    let external_id = require_non_empty_trimmed_max(&req.external_id, "external_id", 200)?;
    if external_id.contains('<')
        || external_id.contains('>')
        || external_id.contains("{{")
        || external_id.contains("}}")
        || external_id.chars().any(|ch| ch == '\n' || ch == '\r')
    {
        return Err(AppError::BadRequest(
            "external_id cannot contain markup, template syntax, or newlines".to_owned(),
        ));
    }

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let provider = provider.clone();
        let external_id = external_id.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            ws_store
                .write_json(
                    &format!("links/{}.json", provider),
                    &serde_json::json!({
                        "external_id": external_id,
                        "provider": provider,
                        "linked_at": chrono::Utc::now().to_rfc3339(),
                    }),
                    &format!("Link workspace to {provider}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(WorkspaceLinkResponse {
        workspace_id,
        linked: true,
        provider,
    }))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct WorkspaceClaimRequest {
    pub claim_token: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkspaceClaimResponse {
    pub workspace_id: WorkspaceId,
    pub claimed: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BillingStatusResponse {
    pub workspace_id: WorkspaceId,
    pub plan: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_end: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BillingPlanResponse {
    pub plan_id: String,
    pub name: String,
    pub price_cents: i64,
    pub interval: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BillingPlansResponse {
    pub plans: Vec<BillingPlanResponse>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BillingCheckoutRequest {
    pub plan_id: String,
    #[serde(default)]
    pub entity_id: Option<EntityId>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BillingPortalResponse {
    pub portal_url: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BillingCheckoutResponse {
    pub checkout_url: String,
    pub plan: String,
    pub status: String,
}

fn billing_plans() -> Vec<BillingPlanResponse> {
    vec![
        BillingPlanResponse {
            plan_id: "free".to_owned(),
            name: "Free".to_owned(),
            price_cents: 0,
            interval: "month".to_owned(),
        },
        BillingPlanResponse {
            plan_id: "pro".to_owned(),
            name: "Pro".to_owned(),
            price_cents: 4_900,
            interval: "month".to_owned(),
        },
        BillingPlanResponse {
            plan_id: "enterprise".to_owned(),
            name: "Enterprise".to_owned(),
            price_cents: 29_900,
            interval: "month".to_owned(),
        },
    ]
}

fn billing_checkout_url(workspace_id: WorkspaceId, plan_id: &str) -> String {
    format!("https://billing.thecorporation.ai/checkout?workspace_id={workspace_id}&plan={plan_id}")
}

fn billing_portal_url(workspace_id: WorkspaceId) -> String {
    format!("https://billing.thecorporation.ai/portal?workspace_id={workspace_id}")
}

#[utoipa::path(
    post,
    path = "/v1/workspaces/claim",
    tag = "admin",
    request_body = WorkspaceClaimRequest,
    responses(
        (status = 200, description = "Claim a workspace", body = WorkspaceClaimResponse),
    ),
)]
async fn claim_workspace(
    RequireAdmin(auth): RequireAdmin,
    State(_state): State<AppState>,
    Json(req): Json<WorkspaceClaimRequest>,
) -> Result<Json<WorkspaceClaimResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let _claim_token = require_non_empty_trimmed_max(&req.claim_token, "claim_token", 256)?;
    Err(AppError::NotImplemented(format!(
        "workspace claim tokens are not implemented for workspace {}",
        workspace_id
    )))
}

#[utoipa::path(
    get,
    path = "/v1/billing/status",
    tag = "admin",
    responses(
        (status = 200, description = "Current workspace billing status", body = BillingStatusResponse),
    ),
)]
async fn billing_status(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<BillingStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let status = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;
            match ws_store.read_json::<Subscription>("billing/subscription.json") {
                Ok(subscription) => Ok::<_, AppError>(BillingStatusResponse {
                    workspace_id,
                    plan: subscription.plan().to_owned(),
                    status: subscription.status().to_owned(),
                    current_period_end: subscription.current_period_end().map(ToOwned::to_owned),
                }),
                Err(crate::git::error::GitStorageError::NotFound(_)) => Ok(BillingStatusResponse {
                    workspace_id,
                    plan: "free".to_owned(),
                    status: "active".to_owned(),
                    current_period_end: None,
                }),
                Err(err) => Err(AppError::Internal(format!(
                    "read billing subscription: {err}"
                ))),
            }
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
    Ok(Json(status))
}

#[utoipa::path(
    get,
    path = "/v1/billing/plans",
    tag = "admin",
    responses(
        (status = 200, description = "Available billing plans", body = BillingPlansResponse),
    ),
)]
async fn billing_plans_handler(RequireAdmin(_auth): RequireAdmin) -> Json<BillingPlansResponse> {
    Json(BillingPlansResponse {
        plans: billing_plans(),
    })
}

#[utoipa::path(
    post,
    path = "/v1/billing/portal",
    tag = "admin",
    responses(
        (status = 200, description = "Billing portal URL", body = BillingPortalResponse),
    ),
)]
async fn billing_portal(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<BillingPortalResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;
            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(BillingPortalResponse {
        portal_url: billing_portal_url(workspace_id),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/billing/checkout",
    tag = "admin",
    request_body = BillingCheckoutRequest,
    responses(
        (status = 200, description = "Billing checkout URL", body = BillingCheckoutResponse),
    ),
)]
async fn billing_checkout(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<BillingCheckoutRequest>,
) -> Result<Json<BillingCheckoutResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let plan = normalize_slug(&req.plan_id, "plan_id", 32)?;
    if !billing_plans()
        .iter()
        .any(|candidate| candidate.plan_id == plan)
    {
        return Err(AppError::BadRequest(format!(
            "unsupported plan_id: {}",
            req.plan_id
        )));
    }

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let plan = plan.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;
            let mut subscription =
                match ws_store.read_json::<Subscription>("billing/subscription.json") {
                    Ok(existing) => existing,
                    Err(crate::git::error::GitStorageError::NotFound(_)) => {
                        Subscription::new(SubscriptionId::new(), workspace_id, plan.clone())
                    }
                    Err(err) => {
                        return Err(AppError::Internal(format!(
                            "read billing subscription: {err}"
                        )));
                    }
                };
            subscription.set_plan(plan.clone());
            subscription.set_status("pending_checkout".to_owned());
            ws_store
                .write_json(
                    "billing/subscription.json",
                    &subscription,
                    &format!("Start billing checkout for {plan}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(BillingCheckoutResponse {
        checkout_url: billing_checkout_url(workspace_id, &plan),
        plan,
        status: "pending_checkout".to_owned(),
    }))
}

// ── Handlers: Workspace by path param ───────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/workspaces/{workspace_id}/status",
    tag = "admin",
    params(
        ("workspace_id" = WorkspaceId, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "Workspace status by ID", body = WorkspaceStatusResponse),
    ),
)]
async fn workspace_status_by_path(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<WorkspaceStatusResponse>, AppError> {
    ensure_workspace_access(auth.workspace_id(), workspace_id)?;
    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let name = ws_store
                .read_workspace()
                .map(|r| r.name)
                .unwrap_or_else(|_| workspace_id.to_string());

            let entity_count = layout.list_entity_ids(workspace_id).len();

            Ok::<_, AppError>(WorkspaceStatusResponse {
                workspace_id,
                name,
                status: "active".to_owned(),
                entity_count,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1/workspaces/{workspace_id}/entities",
    tag = "admin",
    params(
        ("workspace_id" = WorkspaceId, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "List entities in workspace", body = Vec<WorkspaceEntitySummary>),
    ),
)]
async fn workspace_entities_by_path(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<Vec<WorkspaceEntitySummary>>, AppError> {
    ensure_workspace_access(auth.workspace_id(), workspace_id)?;
    let entities = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let ids = layout.list_entity_ids(workspace_id);
            Ok::<_, AppError>(
                ids.into_iter()
                    .map(|id| WorkspaceEntitySummary { entity_id: id })
                    .collect::<Vec<_>>(),
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(entities))
}

// ── Handlers: Workspace contacts ────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkspaceContactSummary {
    pub contact_id: String,
    pub entity_id: String,
}

#[utoipa::path(
    get,
    path = "/v1/workspaces/{workspace_id}/contacts",
    tag = "admin",
    params(
        ("workspace_id" = WorkspaceId, Path, description = "Workspace ID"),
    ),
    responses(
        (status = 200, description = "List contacts in workspace", body = Vec<WorkspaceContactSummary>),
    ),
)]
async fn workspace_contacts(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<Vec<WorkspaceContactSummary>>, AppError> {
    ensure_workspace_access(auth.workspace_id(), workspace_id)?;
    let contacts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut results = Vec::new();

            for entity_id in entity_ids {
                if let Ok(store) =
                    crate::store::entity_store::EntityStore::open(&layout, workspace_id, entity_id, valkey_client.as_ref())
                {
                    if let Ok(ids) = store.list_ids::<Contact>("main") {
                        for contact_id in ids {
                            results.push(WorkspaceContactSummary {
                                contact_id: contact_id.to_string(),
                                entity_id: entity_id.to_string(),
                            });
                        }
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(contacts))
}

// ── Handlers: Digests ───────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct DigestSummary {
    pub digest_key: String,
    pub generated_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DigestTriggerResponse {
    pub triggered: bool,
    pub digest_count: usize,
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/v1/digests",
    tag = "admin",
    responses(
        (status = 200, description = "List digests", body = Vec<DigestSummary>),
    ),
)]
async fn list_digests(RequireAdmin(_auth): RequireAdmin) -> Json<Vec<DigestSummary>> {
    Json(vec![])
}

#[utoipa::path(
    post,
    path = "/v1/digests/trigger",
    tag = "admin",
    responses(
        (status = 200, description = "Trigger digest generation", body = DigestTriggerResponse),
    ),
)]
async fn trigger_digests(RequireAdmin(_auth): RequireAdmin) -> Json<DigestTriggerResponse> {
    Json(DigestTriggerResponse {
        triggered: true,
        digest_count: 0,
        message: "Digest generation is not configured in this environment yet, so the trigger was accepted but no digests were produced.".to_owned(),
    })
}

#[utoipa::path(
    get,
    path = "/v1/digests/{digest_key}",
    tag = "admin",
    params(
        ("digest_key" = String, Path, description = "Digest key"),
    ),
    responses(
        (status = 200, description = "Get digest by key", body = Object),
    ),
)]
async fn get_digest(
    RequireAdmin(_auth): RequireAdmin,
    Path(digest_key): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Err(AppError::NotFound(format!(
        "digest {} not found",
        digest_key
    )))
}

// ── Handlers: Service token / JWKS ──────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct ServiceTokenResponse {
    pub api_key_id: ApiKeyId,
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[utoipa::path(
    get,
    path = "/v1/service-token",
    tag = "admin",
    responses(
        (status = 200, description = "Get a service token", body = ServiceTokenResponse),
    ),
)]
async fn get_service_token(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<ServiceTokenResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    state.enforce_creation_rate_limit("admin.service_token.create", workspace_id, 10, 60)?;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id, valkey_client.as_ref())
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;
            let expires_in = 3600u64;
            let scope_set = ScopeSet::from_vec(vec![Scope::Admin]);
            let expires_at = Utc::now() + Duration::seconds(expires_in as i64);
            let (raw_key, record) = generate_api_key(
                workspace_id,
                format!("service-token-{}", Utc::now().timestamp()),
                scope_set,
                Some(expires_at),
                None,
                None,
            )
            .map_err(|e| AppError::Internal(format!("generate service token: {e}")))?;
            let key_id = record.key_id();
            ws_store
                .write_json(
                    &format!("api-keys/{}.json", key_id),
                    &record,
                    &format!("Create service token {key_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit service token: {e}")))?;
            Ok::<_, AppError>(ServiceTokenResponse {
                api_key_id: key_id,
                token: raw_key,
                token_type: "Bearer".to_owned(),
                expires_in,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct JwksResponse {
    #[schema(value_type = Vec<Object>)]
    pub keys: Vec<serde_json::Value>,
}

#[utoipa::path(
    get,
    path = "/v1/jwks",
    tag = "admin",
    responses(
        (status = 200, description = "Get JWKS keys", body = JwksResponse),
    ),
)]
async fn get_jwks(RequireAdmin(_auth): RequireAdmin) -> Json<JwksResponse> {
    Json(JwksResponse { keys: vec![] })
}

// ── Router ───────────────────────────────────────────────────────────

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        list_workspaces,
        list_audit_events,
        system_health,
        workspace_status,
        list_workspace_entities,
        demo_seed,
        get_config,
        link_workspace,
        claim_workspace,
        billing_status,
        billing_plans_handler,
        billing_portal,
        billing_checkout,
        workspace_status_by_path,
        workspace_entities_by_path,
        workspace_contacts,
        list_digests,
        trigger_digests,
        get_digest,
        get_service_token,
        get_jwks,
    ),
    components(schemas(
        WorkspaceSummary,
        AuditEvent,
        SystemHealth,
        WorkspaceStatusResponse,
        WorkspaceEntitySummary,
        DemoSeedRequest,
        DemoSeedResponse,
        ConfigResponse,
        WorkspaceLinkRequest,
        WorkspaceLinkResponse,
        WorkspaceClaimRequest,
        WorkspaceClaimResponse,
        BillingStatusResponse,
        BillingPlanResponse,
        BillingPlansResponse,
        BillingCheckoutRequest,
        BillingPortalResponse,
        BillingCheckoutResponse,
        WorkspaceContactSummary,
        DigestSummary,
        DigestTriggerResponse,
        ServiceTokenResponse,
        JwksResponse,
    )),
    tags(
        (name = "admin", description = "Admin endpoints"),
    ),
)]
pub struct AdminApi;

/// Stub billing routes for standalone api-rs (no Stripe).
/// api-corp overrides these with real Stripe-backed handlers.
pub fn admin_billing_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/billing/status", get(billing_status))
        .route("/v1/billing/plans", get(billing_plans_handler))
        .route("/v1/billing/portal", post(billing_portal))
        .route("/v1/billing/checkout", post(billing_checkout))
}

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/admin/workspaces", get(list_workspaces))
        .route("/v1/admin/audit-events", get(list_audit_events))
        .route("/v1/admin/system-health", get(system_health))
        .route("/v1/workspace/status", get(workspace_status))
        .route("/v1/workspace/entities", get(list_workspace_entities))
        .route("/v1/demo/seed", post(demo_seed))
        .route("/v1/config", get(get_config))
        .route("/v1/workspaces/link", post(link_workspace))
        .route("/v1/workspaces/claim", post(claim_workspace))
        // Workspace by path param (Python-compatible)
        .route(
            "/v1/workspaces/{workspace_id}/status",
            get(workspace_status_by_path),
        )
        .route(
            "/v1/workspaces/{workspace_id}/entities",
            get(workspace_entities_by_path),
        )
        .route(
            "/v1/workspaces/{workspace_id}/contacts",
            get(workspace_contacts),
        )
        // Digests
        .route("/v1/digests", get(list_digests))
        .route("/v1/digests/trigger", post(trigger_digests))
        .route("/v1/digests/{digest_key}", get(get_digest))
        // Auth infrastructure
        .route("/v1/service-token", get(get_service_token))
        .route("/v1/jwks", get(get_jwks))
}
