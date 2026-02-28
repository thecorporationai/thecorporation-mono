//! Admin HTTP routes.
//!
//! Endpoints for workspace listing, audit events, system health, demo seed,
//! and subscriptions. All data is read from git repos on disk.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::RequireAdmin;
use crate::domain::billing::subscription::Subscription;
use crate::domain::contacts::contact::Contact;
use crate::domain::ids::{EntityId, SubscriptionId, WorkspaceId};
use crate::error::AppError;
use crate::store::workspace_store::WorkspaceStore;

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct WorkspaceSummary {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub entity_count: usize,
}

#[derive(Serialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub details: serde_json::Value,
}

#[derive(Serialize)]
pub struct SystemHealth {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub git_storage: String,
    pub workspace_count: usize,
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn list_workspaces(
    RequireAdmin(_auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkspaceSummary>>, AppError> {
    let summaries = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let workspace_ids = layout.list_workspace_ids();
            let mut results = Vec::new();

            for ws_id in workspace_ids {
                let name = match WorkspaceStore::open(&layout, ws_id) {
                    Ok(ws_store) => ws_store
                        .read_workspace()
                        .map(|r| r.name)
                        .unwrap_or_else(|_| ws_id.to_string()),
                    Err(_) => ws_id.to_string(),
                };

                let entity_count = layout.list_entity_ids(ws_id).len();

                results.push(WorkspaceSummary {
                    workspace_id: ws_id,
                    name,
                    entity_count,
                });
            }

            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(summaries))
}

async fn list_audit_events(
    RequireAdmin(_auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<AuditEvent>>, AppError> {
    let events = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let mut events = Vec::new();
            let workspace_ids = layout.list_workspace_ids();

            for ws_id in workspace_ids {
                // Read git log from workspace repo for recent commits
                if let Ok(ws_store) = WorkspaceStore::open(&layout, ws_id) {
                    if let Ok(log_entries) = ws_store.repo().recent_commits("main", 10) {
                        for (oid, message, timestamp) in log_entries {
                            events.push(AuditEvent {
                                event_id: oid,
                                event_type: "commit".to_owned(),
                                timestamp,
                                details: serde_json::json!({
                                    "workspace_id": ws_id,
                                    "message": message,
                                }),
                            });
                        }
                    }
                }
            }

            // Sort by timestamp descending
            events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            events.truncate(50);

            Ok::<_, AppError>(events)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(events))
}

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

#[derive(Serialize)]
pub struct WorkspaceStatusResponse {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub status: String,
    pub entity_count: usize,
}

async fn workspace_status(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<WorkspaceStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
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

#[derive(Serialize)]
pub struct WorkspaceEntitySummary {
    pub entity_id: EntityId,
}

async fn list_workspace_entities(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkspaceEntitySummary>>, AppError> {
    let workspace_id = auth.workspace_id();

    let entities = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            // Verify workspace exists
            WorkspaceStore::open(&layout, workspace_id)
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

#[derive(Deserialize)]
pub struct DemoSeedRequest {
    #[serde(default = "default_scenario")]
    pub scenario: String,
}

fn default_scenario() -> String {
    "startup".to_owned()
}

#[derive(Serialize)]
pub struct DemoSeedResponse {
    pub workspace_id: WorkspaceId,
    pub scenario: String,
    pub entities_created: usize,
    pub message: String,
}

async fn demo_seed(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<DemoSeedRequest>,
) -> Result<Json<DemoSeedResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let scenario = req.scenario.clone();

    let entities_created = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            // Initialize workspace if it doesn't exist
            let ws_store = match WorkspaceStore::open(&layout, workspace_id) {
                Ok(s) => s,
                Err(_) => WorkspaceStore::init(&layout, workspace_id, &format!("Demo: {scenario}"))
                    .map_err(|e| AppError::Internal(format!("init workspace: {e}")))?,
            };

            // Create a demo entity based on scenario
            let entity_id = EntityId::new();
            let (entity_type, legal_name) = match scenario.as_str() {
                "startup" => (
                    crate::domain::formation::types::EntityType::Corporation,
                    "Demo Startup Inc.",
                ),
                "llc" => (crate::domain::formation::types::EntityType::Llc, "Demo LLC"),
                "restaurant" => (
                    crate::domain::formation::types::EntityType::Llc,
                    "Demo Restaurant LLC",
                ),
                _ => (
                    crate::domain::formation::types::EntityType::Corporation,
                    "Demo Entity",
                ),
            };

            let entity = crate::domain::formation::entity::Entity::new(
                entity_id,
                workspace_id,
                legal_name.to_owned(),
                entity_type,
                crate::domain::formation::types::Jurisdiction::new("Delaware").unwrap(),
                None,
                None,
            )
            .map_err(|e| AppError::Internal(format!("create entity: {e}")))?;

            crate::store::entity_store::EntityStore::init(
                &layout,
                workspace_id,
                entity_id,
                &entity,
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

            Ok::<_, AppError>(1usize)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(DemoSeedResponse {
        workspace_id,
        scenario: req.scenario,
        entities_created,
        message: format!("Created {} demo entities", entities_created),
    }))
}

// ── Subscriptions ──────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SubscriptionResponse {
    pub workspace_id: WorkspaceId,
    pub plan: String,
    pub status: String,
    pub current_period_end: Option<String>,
}

async fn get_subscription(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<SubscriptionResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let sub = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            match ws_store.read_json::<Subscription>("billing/subscription.json") {
                Ok(sub) => Ok::<_, AppError>(sub),
                Err(_) => {
                    // Create default free subscription
                    let sub =
                        Subscription::new(SubscriptionId::new(), workspace_id, "free".to_owned());
                    ws_store
                        .write_json("billing/subscription.json", &sub, "Init subscription")
                        .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
                    Ok(sub)
                }
            }
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(SubscriptionResponse {
        workspace_id: sub.workspace_id(),
        plan: sub.plan().to_owned(),
        status: sub.status().to_owned(),
        current_period_end: sub.current_period_end().map(|s| s.to_owned()),
    }))
}

// ── Config ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ConfigResponse {
    pub version: String,
    pub environment: String,
    pub features: Vec<String>,
}

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

#[derive(Deserialize)]
pub struct WorkspaceLinkRequest {
    pub external_id: String,
    pub provider: String,
}

#[derive(Serialize)]
pub struct WorkspaceLinkResponse {
    pub workspace_id: WorkspaceId,
    pub linked: bool,
    pub provider: String,
}

async fn link_workspace(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<WorkspaceLinkRequest>,
) -> Result<Json<WorkspaceLinkResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let provider = req.provider.clone();
        let external_id = req.external_id.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
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
        provider: req.provider,
    }))
}

#[derive(Deserialize)]
pub struct WorkspaceClaimRequest {
    pub claim_token: String,
}

#[derive(Serialize)]
pub struct WorkspaceClaimResponse {
    pub workspace_id: WorkspaceId,
    pub claimed: bool,
}

async fn claim_workspace(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(_req): Json<WorkspaceClaimRequest>,
) -> Result<Json<WorkspaceClaimResponse>, AppError> {
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(WorkspaceClaimResponse {
        workspace_id,
        claimed: true,
    }))
}

// ── Handlers: Workspace by path param ───────────────────────────────

async fn workspace_status_by_path(
    RequireAdmin(_auth): RequireAdmin,
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<WorkspaceStatusResponse>, AppError> {
    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let ws_store = WorkspaceStore::open(&layout, workspace_id)
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

async fn workspace_entities_by_path(
    RequireAdmin(_auth): RequireAdmin,
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<Vec<WorkspaceEntitySummary>>, AppError> {
    let entities = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            WorkspaceStore::open(&layout, workspace_id)
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

#[derive(Serialize)]
pub struct WorkspaceContactSummary {
    pub contact_id: String,
    pub entity_id: String,
}

async fn workspace_contacts(
    RequireAdmin(_auth): RequireAdmin,
    State(state): State<AppState>,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<Vec<WorkspaceContactSummary>>, AppError> {
    let contacts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            WorkspaceStore::open(&layout, workspace_id)
                .map_err(|e| AppError::NotFound(format!("workspace not found: {e}")))?;

            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut results = Vec::new();

            for entity_id in entity_ids {
                if let Ok(store) =
                    crate::store::entity_store::EntityStore::open(&layout, workspace_id, entity_id)
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

#[derive(Serialize)]
pub struct DigestSummary {
    pub digest_key: String,
    pub generated_at: String,
}

#[derive(Serialize)]
pub struct DigestTriggerResponse {
    pub triggered: bool,
    pub digest_count: usize,
}

async fn list_digests(RequireAdmin(_auth): RequireAdmin) -> Json<Vec<DigestSummary>> {
    Json(vec![])
}

async fn trigger_digests(RequireAdmin(_auth): RequireAdmin) -> Json<DigestTriggerResponse> {
    Json(DigestTriggerResponse {
        triggered: true,
        digest_count: 0,
    })
}

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

#[derive(Serialize)]
pub struct ServiceTokenResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
}

async fn get_service_token(RequireAdmin(_auth): RequireAdmin) -> Json<ServiceTokenResponse> {
    let token = format!("svc_{}", uuid::Uuid::new_v4().simple());
    Json(ServiceTokenResponse {
        token,
        token_type: "Bearer".to_owned(),
        expires_in: 3600,
    })
}

#[derive(Serialize)]
pub struct JwksResponse {
    pub keys: Vec<serde_json::Value>,
}

async fn get_jwks(RequireAdmin(_auth): RequireAdmin) -> Json<JwksResponse> {
    Json(JwksResponse { keys: vec![] })
}

// ── Router ───────────────────────────────────────────────────────────

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/admin/workspaces", get(list_workspaces))
        .route("/v1/admin/audit-events", get(list_audit_events))
        .route("/v1/admin/system-health", get(system_health))
        .route("/v1/workspace/status", get(workspace_status))
        .route("/v1/workspace/entities", get(list_workspace_entities))
        .route("/v1/demo/seed", post(demo_seed))
        .route("/v1/subscription", get(get_subscription))
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
