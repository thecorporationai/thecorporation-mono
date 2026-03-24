//! Execution route handlers — intents, obligations, and receipts.
//!
//! ## Route map
//!
//! | Method | Path | Scope |
//! |--------|------|-------|
//! | GET    | `/entities/{entity_id}/intents` | `ExecutionRead` |
//! | POST   | `/entities/{entity_id}/intents` | `ExecutionWrite` |
//! | GET    | `/entities/{entity_id}/intents/{intent_id}` | `ExecutionRead` |
//! | PATCH  | `/entities/{entity_id}/intents/{intent_id}` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/intents/{intent_id}/evaluate` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/intents/{intent_id}/authorize` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/intents/{intent_id}/execute` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/intents/{intent_id}/cancel` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/intents/{intent_id}/fail` | `ExecutionWrite` |
//! | GET    | `/entities/{entity_id}/obligations` | `ExecutionRead` |
//! | POST   | `/entities/{entity_id}/obligations` | `ExecutionWrite` |
//! | GET    | `/entities/{entity_id}/obligations/{obligation_id}` | `ExecutionRead` |
//! | PATCH  | `/entities/{entity_id}/obligations/{obligation_id}` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/obligations/{obligation_id}/start` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/obligations/{obligation_id}/fulfill` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/obligations/{obligation_id}/waive` | `ExecutionWrite` |
//! | POST   | `/entities/{entity_id}/obligations/{obligation_id}/expire` | `ExecutionWrite` |
//! | GET    | `/entities/{entity_id}/receipts` | `ExecutionRead` |
//! | POST   | `/entities/{entity_id}/receipts` | `ExecutionWrite` |
//! | GET    | `/entities/{entity_id}/receipts/{receipt_id}` | `ExecutionRead` |

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::Deserialize;

use corp_auth::{RequireExecutionRead, RequireExecutionWrite};
use corp_core::execution::{AssigneeType, Intent, Obligation, Receipt};
use corp_core::governance::capability::AuthorityTier;
use corp_core::ids::{ContactId, EntityId, IntentId, ObligationId, ReceiptId};
use crate::error::AppError;
use crate::state::AppState;

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        // Intents
        .route(
            "/entities/{entity_id}/intents",
            get(list_intents).post(create_intent),
        )
        .route(
            "/entities/{entity_id}/intents/{intent_id}",
            get(get_intent).patch(update_intent),
        )
        .route(
            "/entities/{entity_id}/intents/{intent_id}/evaluate",
            post(evaluate_intent),
        )
        .route(
            "/entities/{entity_id}/intents/{intent_id}/authorize",
            post(authorize_intent),
        )
        .route(
            "/entities/{entity_id}/intents/{intent_id}/execute",
            post(execute_intent),
        )
        .route(
            "/entities/{entity_id}/intents/{intent_id}/cancel",
            post(cancel_intent),
        )
        .route(
            "/entities/{entity_id}/intents/{intent_id}/fail",
            post(fail_intent),
        )
        // Obligations
        .route(
            "/entities/{entity_id}/obligations",
            get(list_obligations).post(create_obligation),
        )
        .route(
            "/entities/{entity_id}/obligations/{obligation_id}",
            get(get_obligation).patch(update_obligation),
        )
        .route(
            "/entities/{entity_id}/obligations/{obligation_id}/start",
            post(start_obligation),
        )
        .route(
            "/entities/{entity_id}/obligations/{obligation_id}/fulfill",
            post(fulfill_obligation),
        )
        .route(
            "/entities/{entity_id}/obligations/{obligation_id}/waive",
            post(waive_obligation),
        )
        .route(
            "/entities/{entity_id}/obligations/{obligation_id}/expire",
            post(expire_obligation),
        )
        // Receipts
        .route(
            "/entities/{entity_id}/receipts",
            get(list_receipts).post(create_receipt),
        )
        .route(
            "/entities/{entity_id}/receipts/{receipt_id}",
            get(get_receipt),
        )
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateIntentRequest {
    pub intent_type: String,
    pub authority_tier: AuthorityTier,
    pub description: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateObligationRequest {
    pub obligation_type: String,
    pub assignee_type: AssigneeType,
    pub assignee_id: Option<ContactId>,
    pub description: String,
    pub due_date: Option<NaiveDate>,
    pub intent_id: Option<IntentId>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIntentRequest {
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateObligationRequest {
    pub description: Option<String>,
    pub assignee_id: Option<ContactId>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReceiptRequest {
    pub intent_id: IntentId,
    pub idempotency_key: String,
    pub request_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct FailIntentRequest {
    pub reason: String,
}

// ── Intent handlers ───────────────────────────────────────────────────────────

async fn list_intents(
    RequireExecutionRead(principal): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Intent>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let intents = store
        .read_all::<Intent>("main")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(intents))
}

async fn create_intent(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateIntentRequest>,
) -> Result<Json<Intent>, AppError> {
    if body.description.trim().is_empty() {
        return Err(AppError::BadRequest("intent description must not be empty".into()));
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let intent = Intent::new(
        entity_id,
        principal.workspace_id,
        body.intent_type,
        body.authority_tier,
        body.description,
        body.metadata,
    );
    store
        .write::<Intent>(&intent, intent.intent_id, "main", "create intent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(intent))
}

async fn get_intent(
    RequireExecutionRead(principal): RequireExecutionRead,
    State(state): State<AppState>,
    Path((entity_id, intent_id)): Path<(EntityId, IntentId)>,
) -> Result<Json<Intent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let intent = store
        .read::<Intent>(intent_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::NotFound(format!("intent {} not found", intent_id))
                }
                other => AppError::Storage(other),
            }
        })?;
    Ok(Json(intent))
}

async fn evaluate_intent(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, intent_id)): Path<(EntityId, IntentId)>,
) -> Result<Json<Intent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut intent = store
        .read::<Intent>(intent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    intent
        .evaluate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Intent>(&intent, intent_id, "main", "evaluate intent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(intent))
}

async fn authorize_intent(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, intent_id)): Path<(EntityId, IntentId)>,
) -> Result<Json<Intent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut intent = store
        .read::<Intent>(intent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    intent
        .authorize()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Intent>(&intent, intent_id, "main", "authorize intent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(intent))
}

async fn execute_intent(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, intent_id)): Path<(EntityId, IntentId)>,
) -> Result<Json<Intent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut intent = store
        .read::<Intent>(intent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    intent
        .mark_executed()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Intent>(&intent, intent_id, "main", "execute intent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(intent))
}

async fn cancel_intent(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, intent_id)): Path<(EntityId, IntentId)>,
) -> Result<Json<Intent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut intent = store
        .read::<Intent>(intent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    intent
        .cancel()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Intent>(&intent, intent_id, "main", "cancel intent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(intent))
}

async fn update_intent(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, intent_id)): Path<(EntityId, IntentId)>,
    Json(body): Json<UpdateIntentRequest>,
) -> Result<Json<Intent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut intent = store
        .read::<Intent>(intent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    if let Some(description) = body.description {
        intent.description = description;
    }
    if let Some(metadata) = body.metadata {
        intent.metadata = metadata;
    }
    store
        .write::<Intent>(&intent, intent_id, "main", "update intent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(intent))
}

async fn fail_intent(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, intent_id)): Path<(EntityId, IntentId)>,
    Json(body): Json<FailIntentRequest>,
) -> Result<Json<Intent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut intent = store
        .read::<Intent>(intent_id, "main")
        .await
        .map_err(AppError::Storage)?;
    intent
        .mark_failed(body.reason)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Intent>(&intent, intent_id, "main", "fail intent")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(intent))
}

// ── Obligation handlers ───────────────────────────────────────────────────────

async fn list_obligations(
    RequireExecutionRead(principal): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Obligation>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let obligations = store
        .read_all::<Obligation>("main")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(obligations))
}

async fn create_obligation(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateObligationRequest>,
) -> Result<Json<Obligation>, AppError> {
    if body.description.trim().is_empty() {
        return Err(AppError::BadRequest("intent description must not be empty".into()));
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let obligation = Obligation::new(
        entity_id,
        body.intent_id,
        body.obligation_type,
        body.assignee_type,
        body.assignee_id,
        body.description,
        body.due_date,
    );
    store
        .write::<Obligation>(
            &obligation,
            obligation.obligation_id,
            "main",
            "create obligation",
        )
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(obligation))
}

async fn get_obligation(
    RequireExecutionRead(principal): RequireExecutionRead,
    State(state): State<AppState>,
    Path((entity_id, obligation_id)): Path<(EntityId, ObligationId)>,
) -> Result<Json<Obligation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let obligation = store
        .read::<Obligation>(obligation_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => AppError::NotFound(
                    format!("obligation {} not found", obligation_id),
                ),
                other => AppError::Storage(other),
            }
        })?;
    Ok(Json(obligation))
}

async fn start_obligation(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, obligation_id)): Path<(EntityId, ObligationId)>,
) -> Result<Json<Obligation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut obligation = store
        .read::<Obligation>(obligation_id, "main")
        .await
        .map_err(AppError::Storage)?;
    obligation
        .start()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Obligation>(&obligation, obligation_id, "main", "start obligation")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(obligation))
}

async fn fulfill_obligation(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, obligation_id)): Path<(EntityId, ObligationId)>,
) -> Result<Json<Obligation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut obligation = store
        .read::<Obligation>(obligation_id, "main")
        .await
        .map_err(AppError::Storage)?;
    obligation
        .fulfill()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Obligation>(
            &obligation,
            obligation_id,
            "main",
            "fulfill obligation",
        )
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(obligation))
}

async fn waive_obligation(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, obligation_id)): Path<(EntityId, ObligationId)>,
) -> Result<Json<Obligation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut obligation = store
        .read::<Obligation>(obligation_id, "main")
        .await
        .map_err(AppError::Storage)?;
    obligation
        .waive()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Obligation>(&obligation, obligation_id, "main", "waive obligation")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(obligation))
}

async fn update_obligation(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, obligation_id)): Path<(EntityId, ObligationId)>,
    Json(body): Json<UpdateObligationRequest>,
) -> Result<Json<Obligation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut obligation = store
        .read::<Obligation>(obligation_id, "main")
        .await
        .map_err(AppError::Storage)?;
    if let Some(description) = body.description {
        obligation.description = description;
    }
    if let Some(assignee_id) = body.assignee_id {
        obligation.assignee_id = Some(assignee_id);
    }
    store
        .write::<Obligation>(&obligation, obligation_id, "main", "update obligation")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(obligation))
}

async fn expire_obligation(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path((entity_id, obligation_id)): Path<(EntityId, ObligationId)>,
) -> Result<Json<Obligation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut obligation = store
        .read::<Obligation>(obligation_id, "main")
        .await
        .map_err(AppError::Storage)?;
    obligation
        .expire()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Obligation>(&obligation, obligation_id, "main", "expire obligation")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(obligation))
}

// ── Receipt handlers ──────────────────────────────────────────────────────────

async fn list_receipts(
    RequireExecutionRead(principal): RequireExecutionRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Receipt>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let receipts = store
        .read_all::<Receipt>("main")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(receipts))
}

async fn create_receipt(
    RequireExecutionWrite(principal): RequireExecutionWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateReceiptRequest>,
) -> Result<Json<Receipt>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let receipt = Receipt::new(body.intent_id, body.idempotency_key, body.request_hash);
    store
        .write::<Receipt>(&receipt, receipt.receipt_id, "main", "create receipt")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(receipt))
}

async fn get_receipt(
    RequireExecutionRead(principal): RequireExecutionRead,
    State(state): State<AppState>,
    Path((entity_id, receipt_id)): Path<(EntityId, ReceiptId)>,
) -> Result<Json<Receipt>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let receipt = store
        .read::<Receipt>(receipt_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::NotFound(format!("receipt {} not found", receipt_id))
                }
                other => AppError::Storage(other),
            }
        })?;
    Ok(Json(receipt))
}
