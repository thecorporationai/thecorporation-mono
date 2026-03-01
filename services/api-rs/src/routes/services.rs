//! Fulfillment services HTTP routes.
//!
//! Endpoints for browsing the service catalog, creating service requests
//! (linked to obligations), initiating Stripe checkout, and processing
//! webhooks for payment confirmation and fulfillment.

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::RequireAdmin;
use crate::domain::execution::obligation::Obligation;
use crate::domain::execution::types::{AssigneeType, ObligationType};
use crate::domain::ids::{EntityId, ObligationId, ServiceItemId, ServiceRequestId, WorkspaceId};
use crate::domain::services::catalog::{self, ServiceItem};
use crate::domain::services::error::ServiceError;
use crate::domain::services::request::ServiceRequest;
use crate::domain::services::types::{PriceType, ServiceRequestStatus};
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Helpers ─────────────────────────────────────────────────────────

fn open_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<EntityStore<'a>, AppError> {
    EntityStore::open(layout, workspace_id, entity_id).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {} not found", entity_id))
        }
        other => AppError::Internal(other.to_string()),
    })
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CatalogItemResponse {
    pub item_id: ServiceItemId,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub price_cents: i64,
    pub price_type: PriceType,
    pub obligation_type: String,
    pub active: bool,
}

fn item_to_response(item: &ServiceItem) -> CatalogItemResponse {
    CatalogItemResponse {
        item_id: item.item_id,
        slug: item.slug.clone(),
        name: item.name.clone(),
        description: item.description.clone(),
        price_cents: item.price_cents,
        price_type: item.price_type,
        obligation_type: item.obligation_type.clone(),
        active: item.active,
    }
}

#[derive(Serialize)]
pub struct ServiceRequestResponse {
    pub request_id: ServiceRequestId,
    pub entity_id: EntityId,
    pub obligation_id: ObligationId,
    pub service_item_id: ServiceItemId,
    pub service_slug: String,
    pub amount_cents: i64,
    pub status: ServiceRequestStatus,
    pub stripe_checkout_session_id: Option<String>,
    pub created_at: String,
    pub paid_at: Option<String>,
    pub fulfilled_at: Option<String>,
    pub fulfillment_note: Option<String>,
}

fn request_to_response(r: &ServiceRequest) -> ServiceRequestResponse {
    ServiceRequestResponse {
        request_id: r.request_id(),
        entity_id: r.entity_id(),
        obligation_id: r.obligation_id(),
        service_item_id: r.service_item_id(),
        service_slug: r.service_slug().to_owned(),
        amount_cents: r.amount_cents(),
        status: r.status(),
        stripe_checkout_session_id: r.stripe_checkout_session_id().map(|s| s.to_owned()),
        created_at: r.created_at().to_rfc3339(),
        paid_at: r.paid_at().map(|t| t.to_rfc3339()),
        fulfilled_at: r.fulfilled_at().map(|t| t.to_rfc3339()),
        fulfillment_note: r.fulfillment_note().map(|s| s.to_owned()),
    }
}

// ── Request types ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateServiceRequestBody {
    pub entity_id: EntityId,
    /// Catalog item slug (e.g., "state_filing.incorporation").
    pub service_slug: String,
    /// Existing obligation ID to link, or omit to auto-create one.
    pub obligation_id: Option<ObligationId>,
}

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub request_id: ServiceRequestId,
    pub checkout_url: String,
    pub session_id: String,
}

#[derive(Deserialize)]
pub struct FulfillBody {
    pub note: Option<String>,
}

// ── Handlers: Catalog ───────────────────────────────────────────────

/// GET /v1/services/catalog — list all active catalog items.
async fn list_catalog() -> Json<Vec<CatalogItemResponse>> {
    let items: Vec<CatalogItemResponse> = catalog::service_catalog()
        .iter()
        .filter(|i| i.active)
        .map(item_to_response)
        .collect();
    Json(items)
}

/// GET /v1/services/catalog/{slug} — get a single catalog item by slug.
async fn get_catalog_item(Path(slug): Path<String>) -> Result<Json<CatalogItemResponse>, AppError> {
    let item =
        catalog::find_by_slug(&slug).ok_or_else(|| ServiceError::ItemNotFound(slug.clone()))?;
    Ok(Json(item_to_response(&item)))
}

// ── Handlers: Service Requests ──────────────────────────────────────

/// POST /v1/services/requests — create a service request for a catalog item.
///
/// If `obligation_id` is not provided, an obligation is auto-created with
/// `assignee_type: ThirdParty` and the catalog item's `obligation_type`.
async fn create_service_request(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(body): Json<CreateServiceRequestBody>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = body.entity_id;
    let slug = body.service_slug.clone();

    let item =
        catalog::find_by_slug(&slug).ok_or_else(|| ServiceError::ItemNotFound(slug.clone()))?;

    if !item.active {
        return Err(AppError::BadRequest(format!(
            "service '{}' is not currently available",
            slug
        )));
    }

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let obligation_id_input = body.obligation_id;
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Resolve or create the obligation.
            let obligation_id = if let Some(oid) = obligation_id_input {
                // Verify it exists.
                let _: Obligation = store
                    .read::<Obligation>("main", oid)
                    .map_err(|_| AppError::NotFound(format!("obligation {} not found", oid)))?;
                oid
            } else {
                // Auto-create an obligation assigned to TheCorporation.ai (ThirdParty).
                let oid = ObligationId::new();
                let obligation = Obligation::new(
                    oid,
                    entity_id,
                    None,
                    ObligationType::from(item.obligation_type.as_str()),
                    AssigneeType::ThirdParty,
                    None,
                    format!("Fulfillment: {}", item.name),
                    None,
                );
                store
                    .write::<Obligation>(
                        "main",
                        oid,
                        &obligation,
                        &format!("Create obligation for service request: {}", item.slug),
                    )
                    .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
                oid
            };

            // Create the service request.
            let request_id = ServiceRequestId::new();
            let request = ServiceRequest::new(
                request_id,
                entity_id,
                obligation_id,
                item.item_id,
                item.slug.clone(),
                item.price_cents,
            );
            store
                .write::<ServiceRequest>(
                    "main",
                    request_id,
                    &request,
                    &format!("Create service request {} for {}", request_id, item.slug),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(request_to_response(&result)))
}

/// GET /v1/entities/{entity_id}/services/requests — list service requests for an entity.
async fn list_service_requests(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ServiceRequestResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let requests = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<ServiceRequest>("main")
                .map_err(|e| AppError::Internal(format!("list service requests: {e}")))?;
            let mut results = Vec::new();
            for id in ids {
                let r = store
                    .read::<ServiceRequest>("main", id)
                    .map_err(|e| AppError::Internal(format!("read service request {id}: {e}")))?;
                results.push(request_to_response(&r));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(requests))
}

/// GET /v1/services/requests/{request_id} — get a single service request.
async fn get_service_request(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let r = store
                .read::<ServiceRequest>("main", request_id)
                .map_err(|_| ServiceError::RequestNotFound(request_id))?;
            Ok::<_, AppError>(r)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(request_to_response(&request)))
}

// ── Handlers: Checkout ──────────────────────────────────────────────

/// POST /v1/services/requests/{entity_id}/{request_id}/checkout
///
/// Initiates a Stripe checkout session for the service request and
/// transitions it to `Checkout` status.
async fn initiate_checkout(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
) -> Result<Json<CheckoutResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let (request, session_id) = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut request: ServiceRequest = store
                .read::<ServiceRequest>("main", request_id)
                .map_err(|_| ServiceError::RequestNotFound(request_id))?;

            let session_id = format!("cs_{}", uuid::Uuid::new_v4());
            request
                .begin_checkout(session_id.clone())
                .map_err(AppError::from)?;

            store
                .write::<ServiceRequest>(
                    "main",
                    request_id,
                    &request,
                    &format!("Begin checkout for service request {}", request_id),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>((request, session_id))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let checkout_url = format!(
        "https://checkout.stripe.com/c/pay?service={}&amount={}&session={}",
        request.service_slug(),
        request.amount_cents(),
        session_id
    );

    Ok(Json(CheckoutResponse {
        request_id,
        checkout_url,
        session_id,
    }))
}

// ── Handlers: Webhook ───────────────────────────────────────────────

/// POST /v1/services/webhooks/stripe
///
/// Receives Stripe `checkout.session.completed` events and transitions the
/// matching service request from `Checkout` -> `Paid`.
///
/// Protected by a shared webhook secret (`SERVICES_STRIPE_WEBHOOK_SECRET`).
async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    // Validate webhook secret.
    let expected = std::env::var("SERVICES_STRIPE_WEBHOOK_SECRET").map_err(|_| {
        AppError::Internal("SERVICES_STRIPE_WEBHOOK_SECRET is not configured".to_owned())
    })?;
    let provided = headers
        .get("x-webhook-secret")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing webhook secret".to_owned()))?;
    if provided != expected {
        return Err(AppError::Unauthorized("invalid webhook secret".to_owned()));
    }

    let payload: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("invalid webhook JSON: {e}")))?;

    let event_type = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tracing::info!("Received services Stripe webhook: {}", event_type);

    if event_type == "checkout.session.completed" {
        let _session_id = payload
            .pointer("/data/object/id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let payment_intent = payload
            .pointer("/data/object/payment_intent")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let entity_id_str = payload
            .pointer("/data/object/metadata/entity_id")
            .and_then(|v| v.as_str());
        let request_id_str = payload
            .pointer("/data/object/metadata/request_id")
            .and_then(|v| v.as_str());
        let workspace_id_str = payload
            .pointer("/data/object/metadata/workspace_id")
            .and_then(|v| v.as_str());

        if let (Some(entity_id_s), Some(request_id_s), Some(workspace_id_s)) =
            (entity_id_str, request_id_str, workspace_id_str)
        {
            let entity_id: EntityId = entity_id_s
                .parse()
                .map_err(|_| AppError::BadRequest("invalid entity_id in metadata".to_owned()))?;
            let request_id: ServiceRequestId = request_id_s
                .parse()
                .map_err(|_| AppError::BadRequest("invalid request_id in metadata".to_owned()))?;
            let workspace_id: WorkspaceId = workspace_id_s
                .parse()
                .map_err(|_| AppError::BadRequest("invalid workspace_id in metadata".to_owned()))?;

            tokio::task::spawn_blocking({
                let layout = state.layout.clone();
                move || {
                    let store = open_store(&layout, workspace_id, entity_id)?;
                    let mut request: ServiceRequest = store
                        .read::<ServiceRequest>("main", request_id)
                        .map_err(|_| ServiceError::RequestNotFound(request_id))?;

                    request.mark_paid(payment_intent).map_err(AppError::from)?;

                    store
                        .write::<ServiceRequest>(
                            "main",
                            request_id,
                            &request,
                            &format!("Payment confirmed for service request {}", request_id),
                        )
                        .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

                    // Also start the obligation.
                    if let Ok(mut obligation) =
                        store.read::<Obligation>("main", request.obligation_id())
                    {
                        if obligation.start().is_ok() {
                            let _ = store.write::<Obligation>(
                                "main",
                                obligation.obligation_id(),
                                &obligation,
                                &format!(
                                    "Start obligation {} (payment received)",
                                    obligation.obligation_id()
                                ),
                            );
                        }
                    }

                    Ok::<_, AppError>(())
                }
            })
            .await
            .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;
        }
    }

    Ok(Json(serde_json::json!({
        "received": true,
        "event_type": event_type,
    })))
}

// ── Handlers: Fulfillment (internal/admin) ──────────────────────────

/// POST /v1/services/requests/{entity_id}/{request_id}/begin-fulfillment
///
/// Operator marks that fulfillment work has started. Paid -> Fulfilling.
async fn begin_fulfillment(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut request: ServiceRequest = store
                .read::<ServiceRequest>("main", request_id)
                .map_err(|_| ServiceError::RequestNotFound(request_id))?;

            request.begin_fulfillment().map_err(AppError::from)?;

            store
                .write::<ServiceRequest>(
                    "main",
                    request_id,
                    &request,
                    &format!("Begin fulfillment for service request {}", request_id),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(request_to_response(&request)))
}

/// POST /v1/services/requests/{entity_id}/{request_id}/fulfill
///
/// Operator marks service as fulfilled. Fulfilling -> Fulfilled.
/// Also fulfills the linked obligation.
async fn fulfill_service_request(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
    Json(body): Json<FulfillBody>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut request: ServiceRequest = store
                .read::<ServiceRequest>("main", request_id)
                .map_err(|_| ServiceError::RequestNotFound(request_id))?;

            request.fulfill(body.note).map_err(AppError::from)?;

            store
                .write::<ServiceRequest>(
                    "main",
                    request_id,
                    &request,
                    &format!("Fulfill service request {}", request_id),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            // Also fulfill the linked obligation.
            if let Ok(mut obligation) = store.read::<Obligation>("main", request.obligation_id()) {
                if obligation.fulfill().is_ok() {
                    let _ = store.write::<Obligation>(
                        "main",
                        obligation.obligation_id(),
                        &obligation,
                        &format!(
                            "Fulfill obligation {} (service {} completed)",
                            obligation.obligation_id(),
                            request.service_slug()
                        ),
                    );
                }
            }

            Ok::<_, AppError>(request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(request_to_response(&request)))
}

/// POST /v1/services/requests/{entity_id}/{request_id}/fail
///
/// Mark a service request as failed from any non-terminal state.
async fn fail_service_request(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut request: ServiceRequest = store
                .read::<ServiceRequest>("main", request_id)
                .map_err(|_| ServiceError::RequestNotFound(request_id))?;

            request.fail().map_err(AppError::from)?;

            store
                .write::<ServiceRequest>(
                    "main",
                    request_id,
                    &request,
                    &format!("Fail service request {}", request_id),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(request_to_response(&request)))
}

// ── Handlers: Dashboard ─────────────────────────────────────────────

#[derive(Serialize)]
pub struct PendingFulfillmentResponse {
    pub entity_id: EntityId,
    pub request_id: ServiceRequestId,
    pub service_slug: String,
    pub amount_cents: i64,
    pub status: ServiceRequestStatus,
    pub created_at: String,
    pub paid_at: Option<String>,
}

/// GET /v1/services/pending — list all paid service requests across all
/// entities awaiting fulfillment. Used by the internal ops dashboard.
async fn list_pending_fulfillment(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<PendingFulfillmentResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let results = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut all = Vec::new();

            for eid in entity_ids {
                if let Ok(store) = open_store(&layout, workspace_id, eid) {
                    if let Ok(ids) = store.list_ids::<ServiceRequest>("main") {
                        for rid in ids {
                            if let Ok(r) = store.read::<ServiceRequest>("main", rid) {
                                if matches!(
                                    r.status(),
                                    ServiceRequestStatus::Paid | ServiceRequestStatus::Fulfilling
                                ) {
                                    all.push(PendingFulfillmentResponse {
                                        entity_id: r.entity_id(),
                                        request_id: r.request_id(),
                                        service_slug: r.service_slug().to_owned(),
                                        amount_cents: r.amount_cents(),
                                        status: r.status(),
                                        created_at: r.created_at().to_rfc3339(),
                                        paid_at: r.paid_at().map(|t| t.to_rfc3339()),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            Ok::<_, AppError>(all)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(results))
}

// ── Router ──────────────────────────────────────────────────────────

pub fn services_routes() -> Router<AppState> {
    Router::new()
        // Catalog (public-ish, but behind auth for now)
        .route("/v1/services/catalog", get(list_catalog))
        .route("/v1/services/catalog/{slug}", get(get_catalog_item))
        // Service requests
        .route("/v1/services/requests", post(create_service_request))
        .route(
            "/v1/entities/{entity_id}/services/requests",
            get(list_service_requests),
        )
        .route(
            "/v1/entities/{entity_id}/services/requests/{request_id}",
            get(get_service_request),
        )
        // Checkout
        .route(
            "/v1/services/requests/{entity_id}/{request_id}/checkout",
            post(initiate_checkout),
        )
        // Fulfillment lifecycle
        .route(
            "/v1/services/requests/{entity_id}/{request_id}/begin-fulfillment",
            post(begin_fulfillment),
        )
        .route(
            "/v1/services/requests/{entity_id}/{request_id}/fulfill",
            post(fulfill_service_request),
        )
        .route(
            "/v1/services/requests/{entity_id}/{request_id}/fail",
            post(fail_service_request),
        )
        // Ops dashboard
        .route("/v1/services/pending", get(list_pending_fulfillment))
        // Stripe webhook
        .route("/v1/services/webhooks/stripe", post(stripe_webhook))
}
