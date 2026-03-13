//! Service catalog and fulfillment routes.

use axum::extract::{Path, Query, State};
use axum::{Json, Router, routing::{get, post}};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use crate::auth::{RequireAdmin, RequireServicesRead, RequireServicesWrite};
use crate::domain::execution::obligation::Obligation;
use crate::domain::execution::types::{AssigneeType, ObligationType};
use crate::domain::ids::{EntityId, ObligationId, ServiceItemId, ServiceRequestId, WorkspaceId};
use crate::domain::services::catalog;
use crate::domain::services::request::ServiceRequest;
use crate::domain::services::types::{PriceType, ServiceRequestStatus};
use crate::error::AppError;
use crate::routes::{AppState, EntityIdQuery};
use crate::store::entity_store::EntityStore;

// ── Request / Response types ─────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct CatalogItemResponse {
    pub item_id: ServiceItemId,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub price_type: PriceType,
    pub amount_cents: i64,
    pub jurisdiction: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateServiceRequestRequest {
    pub entity_id: EntityId,
    pub service_slug: String,
    pub obligation_id: Option<ObligationId>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceRequestResponse {
    pub request_id: ServiceRequestId,
    pub entity_id: EntityId,
    pub obligation_id: ObligationId,
    pub service_item_id: ServiceItemId,
    pub service_slug: String,
    pub amount_cents: i64,
    pub status: ServiceRequestStatus,
    pub created_at: String,
    pub paid_at: Option<String>,
    pub fulfilled_at: Option<String>,
    pub failed_at: Option<String>,
    pub fulfillment_note: Option<String>,
    pub checkout_url: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BeginCheckoutRequest {
    pub entity_id: EntityId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FulfillRequest {
    pub entity_id: EntityId,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CancelRequest {
    pub entity_id: EntityId,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StripeWebhookPayload {
    pub request_id: ServiceRequestId,
    pub workspace_id: WorkspaceId,
    pub entity_id: EntityId,
    pub stripe_session_id: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
}

// ── Helpers ──────────────────────────────────────────────────────────

fn open_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    valkey_client: Option<&redis::Client>,
) -> Result<EntityStore<'a>, AppError> {
    EntityStore::open(layout, workspace_id, entity_id, valkey_client).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {entity_id} not found"))
        }
        other => AppError::Internal(other.to_string()),
    })
}

fn service_request_to_response(
    req: &ServiceRequest,
    workspace_id: WorkspaceId,
) -> ServiceRequestResponse {
    let checkout_url = if req.status() == ServiceRequestStatus::Checkout {
        Some(format!(
            "https://billing.thecorporation.ai/services/checkout?workspace_id={}&entity_id={}&request_id={}",
            workspace_id,
            req.entity_id(),
            req.request_id(),
        ))
    } else {
        None
    };
    ServiceRequestResponse {
        request_id: req.request_id(),
        entity_id: req.entity_id(),
        obligation_id: req.obligation_id(),
        service_item_id: req.service_item_id(),
        service_slug: req.service_slug().to_owned(),
        amount_cents: req.amount_cents(),
        status: req.status(),
        created_at: req.created_at().to_rfc3339(),
        paid_at: req.paid_at().map(|t| t.to_rfc3339()),
        fulfilled_at: req.fulfilled_at().map(|t| t.to_rfc3339()),
        failed_at: req.failed_at().map(|t| t.to_rfc3339()),
        fulfillment_note: req.fulfillment_note().map(String::from),
        checkout_url,
    }
}

fn read_service_request(
    store: &EntityStore<'_>,
    request_id: ServiceRequestId,
) -> Result<ServiceRequest, AppError> {
    let path = format!("services/requests/{request_id}.json");
    store
        .read_json::<ServiceRequest>("main", &path)
        .map_err(|e| match e {
            crate::git::error::GitStorageError::NotFound(_) => {
                AppError::NotFound(format!("service request {request_id} not found"))
            }
            other => AppError::Internal(other.to_string()),
        })
}

fn write_service_request(
    store: &EntityStore<'_>,
    request: &ServiceRequest,
    message: &str,
) -> Result<(), AppError> {
    let path = format!("services/requests/{}.json", request.request_id());
    store
        .write_json("main", &path, request, message)
        .map_err(|e| AppError::Internal(format!("commit error: {e}")))
}

// ── Handlers ─────────────────────────────────────────────────────────

/// List the service catalog.
#[utoipa::path(
    get,
    path = "/v1/services/catalog",
    tag = "services",
    responses(
        (status = 200, description = "Service catalog", body = Vec<CatalogItemResponse>),
    ),
)]
async fn list_catalog(
    RequireServicesRead(_auth): RequireServicesRead,
) -> Json<Vec<CatalogItemResponse>> {
    let items: Vec<CatalogItemResponse> = catalog::catalog()
        .iter()
        .map(|item| CatalogItemResponse {
            item_id: item.item_id,
            slug: item.slug.to_owned(),
            name: item.name.to_owned(),
            description: item.description.to_owned(),
            price_type: item.price_type,
            amount_cents: item.amount_cents,
            jurisdiction: item.jurisdiction.map(String::from),
        })
        .collect();
    Json(items)
}

/// Create a new service request.
#[utoipa::path(
    post,
    path = "/v1/services/requests",
    tag = "services",
    request_body = CreateServiceRequestRequest,
    responses(
        (status = 200, description = "Service request created", body = ServiceRequestResponse),
        (status = 400, description = "Invalid service slug"),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn create_request(
    RequireServicesWrite(auth): RequireServicesWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateServiceRequestRequest>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden(format!(
            "not authorized for entity {entity_id}"
        )));
    }

    state.enforce_creation_rate_limit("services.request.create", workspace_id, 120, 60)?;

    let catalog_item = catalog::find_by_slug(&req.service_slug).ok_or_else(|| {
        AppError::BadRequest(format!("unknown service slug: {}", req.service_slug))
    })?;

    let service_request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let slug = req.service_slug.clone();
        let item_id = catalog_item.item_id;
        let amount_cents = catalog_item.amount_cents;
        let obligation_id_input = req.obligation_id;
        move || {
            let store = open_store(&layout, workspace_id, entity_id, valkey_client.as_ref())?;

            // Resolve or auto-create obligation.
            let obligation_id = match obligation_id_input {
                Some(id) => id,
                None => {
                    let ob_id = ObligationId::new();
                    let obligation = Obligation::new(
                        ob_id,
                        entity_id,
                        None,
                        ObligationType::new(format!("service_fulfillment_{slug}")),
                        AssigneeType::ThirdParty,
                        None,
                        format!("Service request: {slug}"),
                        None,
                    );
                    let ob_path = format!("execution/obligations/{ob_id}.json");
                    store
                        .write_json("main", &ob_path, &obligation, &format!("Create obligation {ob_id} for service {slug}"))
                        .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
                    ob_id
                }
            };

            let request_id = ServiceRequestId::new();
            let service_request = ServiceRequest::new(
                request_id,
                entity_id,
                obligation_id,
                item_id,
                slug.clone(),
                amount_cents,
            );

            write_service_request(&store, &service_request, &format!("Create service request {request_id} for {slug}"))?;

            Ok::<_, AppError>(service_request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(service_request_to_response(&service_request, workspace_id)))
}

/// Get a service request by ID.
#[utoipa::path(
    get,
    path = "/v1/services/requests/{request_id}",
    tag = "services",
    params(
        ("request_id" = ServiceRequestId, Path, description = "Service request ID"),
        EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Service request", body = ServiceRequestResponse),
        (status = 404, description = "Not found"),
    ),
)]
async fn get_request(
    RequireServicesRead(auth): RequireServicesRead,
    State(state): State<AppState>,
    Path(request_id): Path<ServiceRequestId>,
    Query(q): Query<EntityIdQuery>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = q.entity_id;

    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden(format!(
            "not authorized for entity {entity_id}"
        )));
    }

    let service_request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id, valkey_client.as_ref())?;
            read_service_request(&store, request_id)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(service_request_to_response(&service_request, workspace_id)))
}

/// List service requests for an entity.
#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/service-requests",
    tag = "services",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Service requests", body = Vec<ServiceRequestResponse>),
    ),
)]
async fn list_requests(
    RequireServicesRead(auth): RequireServicesRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ServiceRequestResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden(format!(
            "not authorized for entity {entity_id}"
        )));
    }

    let requests = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id, valkey_client.as_ref())?;
            let ids: Vec<ServiceRequestId> = store
                .list_ids_in_dir("main", "services/requests")
                .unwrap_or_default();
            let mut requests = Vec::new();
            for id in ids {
                let path = format!("services/requests/{id}.json");
                if let Ok(req) = store.read_json::<ServiceRequest>("main", &path) {
                    requests.push(req);
                }
            }
            Ok::<_, AppError>(requests)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let responses: Vec<ServiceRequestResponse> = requests
        .iter()
        .map(|r| service_request_to_response(r, workspace_id))
        .collect();
    Ok(Json(responses))
}

/// Begin checkout for a service request (sets Stripe session ID, returns checkout URL).
#[utoipa::path(
    post,
    path = "/v1/services/requests/{request_id}/checkout",
    tag = "services",
    params(("request_id" = ServiceRequestId, Path, description = "Service request ID")),
    request_body = BeginCheckoutRequest,
    responses(
        (status = 200, description = "Checkout initiated", body = ServiceRequestResponse),
    ),
)]
async fn begin_checkout(
    RequireServicesWrite(auth): RequireServicesWrite,
    State(state): State<AppState>,
    Path(request_id): Path<ServiceRequestId>,
    Json(req): Json<BeginCheckoutRequest>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden(format!(
            "not authorized for entity {entity_id}"
        )));
    }

    let service_request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id, valkey_client.as_ref())?;
            let mut service_request = read_service_request(&store, request_id)?;

            // Generate a deterministic session ID from the request ID.
            let session_id = format!("cs_svc_{request_id}");
            service_request
                .begin_checkout(session_id)
                .map_err(|e| AppError::BadRequest(e.to_string()))?;

            write_service_request(
                &store,
                &service_request,
                &format!("Begin checkout for service request {request_id}"),
            )?;

            Ok::<_, AppError>(service_request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(service_request_to_response(&service_request, workspace_id)))
}

/// Fulfill a service request (operator: Paid -> Fulfilling -> Fulfilled in one call).
#[utoipa::path(
    post,
    path = "/v1/services/requests/{request_id}/fulfill",
    tag = "services",
    params(("request_id" = ServiceRequestId, Path, description = "Service request ID")),
    request_body = FulfillRequest,
    responses(
        (status = 200, description = "Service request fulfilled", body = ServiceRequestResponse),
    ),
)]
async fn fulfill_request(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(request_id): Path<ServiceRequestId>,
    Json(req): Json<FulfillRequest>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    let service_request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id, valkey_client.as_ref())?;
            let mut service_request = read_service_request(&store, request_id)?;

            // Paid -> Fulfilling -> Fulfilled in one atomic call.
            service_request
                .begin_fulfillment()
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            service_request
                .fulfill(req.note)
                .map_err(|e| AppError::BadRequest(e.to_string()))?;

            write_service_request(
                &store,
                &service_request,
                &format!("Fulfill service request {request_id}"),
            )?;

            Ok::<_, AppError>(service_request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(service_request_to_response(&service_request, workspace_id)))
}

/// Cancel a service request.
#[utoipa::path(
    post,
    path = "/v1/services/requests/{request_id}/cancel",
    tag = "services",
    params(("request_id" = ServiceRequestId, Path, description = "Service request ID")),
    request_body = CancelRequest,
    responses(
        (status = 200, description = "Service request cancelled", body = ServiceRequestResponse),
    ),
)]
async fn cancel_request(
    RequireServicesWrite(auth): RequireServicesWrite,
    State(state): State<AppState>,
    Path(request_id): Path<ServiceRequestId>,
    Json(req): Json<CancelRequest>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden(format!(
            "not authorized for entity {entity_id}"
        )));
    }

    let service_request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id, valkey_client.as_ref())?;
            let mut service_request = read_service_request(&store, request_id)?;

            service_request
                .fail()
                .map_err(|e| AppError::BadRequest(e.to_string()))?;

            write_service_request(
                &store,
                &service_request,
                &format!("Cancel service request {request_id}"),
            )?;

            Ok::<_, AppError>(service_request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(service_request_to_response(&service_request, workspace_id)))
}

/// Stripe webhook: confirm payment for a service request (Checkout -> Paid).
#[utoipa::path(
    post,
    path = "/v1/services/webhooks/stripe",
    tag = "services",
    request_body = StripeWebhookPayload,
    responses(
        (status = 200, description = "Payment confirmed", body = ServiceRequestResponse),
        (status = 401, description = "Invalid webhook secret"),
    ),
)]
async fn stripe_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<StripeWebhookPayload>,
) -> Result<Json<ServiceRequestResponse>, AppError> {
    // Validate webhook secret.
    let expected_secret =
        std::env::var("SERVICES_STRIPE_WEBHOOK_SECRET").unwrap_or_default();
    let provided_secret = headers
        .get("x-webhook-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if expected_secret.is_empty() || provided_secret != expected_secret {
        return Err(AppError::Unauthorized("invalid webhook secret".to_owned()));
    }

    let workspace_id = payload.workspace_id;
    let entity_id = payload.entity_id;
    let request_id = payload.request_id;

    let service_request = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let payment_intent_id = payload.stripe_payment_intent_id;
        move || {
            let store = open_store(&layout, workspace_id, entity_id, valkey_client.as_ref())?;
            let mut service_request = read_service_request(&store, request_id)?;

            service_request
                .mark_paid(payment_intent_id)
                .map_err(|e| AppError::BadRequest(e.to_string()))?;

            write_service_request(
                &store,
                &service_request,
                &format!("Payment confirmed for service request {request_id}"),
            )?;

            Ok::<_, AppError>(service_request)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(service_request_to_response(&service_request, workspace_id)))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn services_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/services/catalog", get(list_catalog))
        .route("/v1/services/requests", post(create_request))
        .route("/v1/services/requests/{request_id}", get(get_request))
        .route(
            "/v1/entities/{entity_id}/service-requests",
            get(list_requests),
        )
        .route(
            "/v1/services/requests/{request_id}/checkout",
            post(begin_checkout),
        )
        .route(
            "/v1/services/requests/{request_id}/fulfill",
            post(fulfill_request),
        )
        .route(
            "/v1/services/requests/{request_id}/cancel",
            post(cancel_request),
        )
        .route("/v1/services/webhooks/stripe", post(stripe_webhook))
}

// ── OpenAPI ──────────────────────────────────────────────────────────

#[derive(OpenApi)]
#[openapi(
    paths(
        list_catalog,
        create_request,
        get_request,
        list_requests,
        begin_checkout,
        fulfill_request,
        cancel_request,
        stripe_webhook,
    ),
    components(schemas(
        CatalogItemResponse,
        CreateServiceRequestRequest,
        ServiceRequestResponse,
        BeginCheckoutRequest,
        FulfillRequest,
        CancelRequest,
        StripeWebhookPayload,
        PriceType,
        ServiceRequestStatus,
    )),
    tags(
        (name = "services", description = "Service catalog and fulfillment"),
    ),
)]
pub struct ServicesApi;
