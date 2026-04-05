//! Service request route handlers.
//!
//! ## Route map
//!
//! | Method | Path | Scope |
//! |--------|------|-------|
//! | GET    | `/entities/{entity_id}/service-requests` | `ServicesRead` |
//! | POST   | `/entities/{entity_id}/service-requests` | `ServicesWrite` |
//! | GET    | `/entities/{entity_id}/service-requests/{request_id}` | `ServicesRead` |
//! | POST   | `/entities/{entity_id}/service-requests/{request_id}/checkout` | `ServicesWrite` |
//! | POST   | `/entities/{entity_id}/service-requests/{request_id}/pay` | `ServicesWrite` |
//! | POST   | `/entities/{entity_id}/service-requests/{request_id}/fulfill` | `ServicesWrite` |

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use corp_auth::{RequireServicesRead, RequireServicesWrite};
use corp_core::ids::{EntityId, ServiceRequestId};
use corp_core::services::ServiceRequest;

use crate::error::AppError;
use crate::state::AppState;

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/entities/{entity_id}/service-requests",
            get(list_service_requests).post(create_service_request),
        )
        .route(
            "/entities/{entity_id}/service-requests/{request_id}",
            get(get_service_request),
        )
        .route(
            "/entities/{entity_id}/service-requests/{request_id}/checkout",
            post(begin_checkout),
        )
        .route(
            "/entities/{entity_id}/service-requests/{request_id}/pay",
            post(mark_paid),
        )
        .route(
            "/entities/{entity_id}/service-requests/{request_id}/fulfill",
            post(fulfill_service_request),
        )
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateServiceRequestRequest {
    /// Slug identifying the service product (e.g. `"registered-agent-de"`).
    pub service_slug: String,
    /// Price in whole cents.
    pub amount_cents: i64,
}

#[derive(Debug, Deserialize)]
pub struct FulfillServiceRequestRequest {
    /// Optional note recorded alongside the fulfillment (e.g. confirmation number).
    pub fulfillment_note: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_service_requests(
    RequireServicesRead(principal): RequireServicesRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ServiceRequest>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let requests = store
        .read_all::<ServiceRequest>("main")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(requests))
}

async fn create_service_request(
    RequireServicesWrite(principal): RequireServicesWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateServiceRequestRequest>,
) -> Result<Json<ServiceRequest>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let request = ServiceRequest::new(entity_id, body.service_slug, body.amount_cents);
    store
        .write::<ServiceRequest>(
            &request,
            request.request_id,
            "main",
            "create service request",
        )
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(request))
}

async fn get_service_request(
    RequireServicesRead(principal): RequireServicesRead,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
) -> Result<Json<ServiceRequest>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let request = store
        .read::<ServiceRequest>(request_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::NotFound(format!("service request {} not found", request_id))
                }
                other => AppError::Storage(other),
            }
        })?;
    Ok(Json(request))
}

async fn begin_checkout(
    RequireServicesWrite(principal): RequireServicesWrite,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
) -> Result<Json<ServiceRequest>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut request = store
        .read::<ServiceRequest>(request_id, "main")
        .await
        .map_err(AppError::Storage)?;
    request
        .begin_checkout()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<ServiceRequest>(&request, request_id, "main", "begin checkout")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(request))
}

async fn mark_paid(
    RequireServicesWrite(principal): RequireServicesWrite,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
) -> Result<Json<ServiceRequest>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut request = store
        .read::<ServiceRequest>(request_id, "main")
        .await
        .map_err(AppError::Storage)?;
    request
        .mark_paid()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<ServiceRequest>(&request, request_id, "main", "mark service request paid")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(request))
}

async fn fulfill_service_request(
    RequireServicesWrite(principal): RequireServicesWrite,
    State(state): State<AppState>,
    Path((entity_id, request_id)): Path<(EntityId, ServiceRequestId)>,
    Json(body): Json<FulfillServiceRequestRequest>,
) -> Result<Json<ServiceRequest>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut request = store
        .read::<ServiceRequest>(request_id, "main")
        .await
        .map_err(AppError::Storage)?;

    // Advance through Paid → Fulfilling first if still in Paid state.
    if matches!(
        request.status,
        corp_core::services::ServiceRequestStatus::Paid
    ) {
        request
            .begin_fulfillment()
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
    }

    request
        .fulfill(body.fulfillment_note)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<ServiceRequest>(&request, request_id, "main", "fulfill service request")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(request))
}
