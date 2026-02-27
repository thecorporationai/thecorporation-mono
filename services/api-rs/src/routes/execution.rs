//! Execution HTTP routes.
//!
//! Endpoints for intents, obligations, and receipts.

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::domain::execution::{
    intent::Intent,
    obligation::Obligation,
    receipt::Receipt,
    types::*,
};
use crate::domain::ids::{
    ContactId, EntityId, IntentId, ObligationId, ReceiptId, WorkspaceId,
};
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Query types ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct EntityQuery {
    pub workspace_id: WorkspaceId,
}

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateIntentRequest {
    pub entity_id: EntityId,
    pub intent_type: String,
    pub authority_tier: AuthorityTier,
    pub description: String,
    #[serde(default = "default_metadata")]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

fn default_metadata() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

#[derive(Deserialize)]
pub struct CreateObligationRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub intent_id: Option<IntentId>,
    pub obligation_type: String,
    pub assignee_type: AssigneeType,
    #[serde(default)]
    pub assignee_id: Option<ContactId>,
    pub description: String,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct IntentResponse {
    pub intent_id: IntentId,
    pub entity_id: EntityId,
    pub intent_type: String,
    pub authority_tier: AuthorityTier,
    pub status: IntentStatus,
    pub description: String,
    pub evaluated_at: Option<String>,
    pub authorized_at: Option<String>,
    pub executed_at: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ObligationResponse {
    pub obligation_id: ObligationId,
    pub entity_id: EntityId,
    pub intent_id: Option<IntentId>,
    pub obligation_type: String,
    pub assignee_type: AssigneeType,
    pub assignee_id: Option<ContactId>,
    pub description: String,
    pub due_date: Option<NaiveDate>,
    pub status: ObligationStatus,
    pub fulfilled_at: Option<String>,
    pub waived_at: Option<String>,
    pub created_at: String,
}

// ── Conversion helpers ───────────────────────────────────────────────

fn intent_to_response(i: &Intent) -> IntentResponse {
    IntentResponse {
        intent_id: i.intent_id(),
        entity_id: i.entity_id(),
        intent_type: i.intent_type().to_owned(),
        authority_tier: i.authority_tier(),
        status: i.status(),
        description: i.description().to_owned(),
        evaluated_at: i.evaluated_at().map(|t| t.to_rfc3339()),
        authorized_at: i.authorized_at().map(|t| t.to_rfc3339()),
        executed_at: i.executed_at().map(|t| t.to_rfc3339()),
        failure_reason: i.failure_reason().map(|s| s.to_owned()),
        created_at: i.created_at().to_rfc3339(),
    }
}

fn obligation_to_response(o: &Obligation) -> ObligationResponse {
    ObligationResponse {
        obligation_id: o.obligation_id(),
        entity_id: o.entity_id(),
        intent_id: o.intent_id(),
        obligation_type: o.obligation_type().as_str().to_owned(),
        assignee_type: o.assignee_type(),
        assignee_id: o.assignee_id(),
        description: o.description().to_owned(),
        due_date: o.due_date(),
        status: o.status(),
        fulfilled_at: o.fulfilled_at().map(|t| t.to_rfc3339()),
        waived_at: o.waived_at().map(|t| t.to_rfc3339()),
        created_at: o.created_at().to_rfc3339(),
    }
}

// ── Helper to open a store ───────────────────────────────────────────

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

// ── Handlers: Intents ────────────────────────────────────────────────

async fn create_intent(
    State(state): State<AppState>,
    Json(req): Json<CreateIntentRequest>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let intent_id = IntentId::new();
            let intent = Intent::new(
                intent_id,
                entity_id,
                workspace_id,
                req.intent_type,
                req.authority_tier,
                req.description,
                req.metadata,
            );

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("Create intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(intent_to_response(&intent)))
}

async fn list_intents(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<IntentResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let intents = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_intent_ids("main").map_err(|e| {
                AppError::Internal(format!("list intents: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let i = store.read_intent("main", id).map_err(|e| {
                    AppError::Internal(format!("read intent {id}: {e}"))
                })?;
                results.push(intent_to_response(&i));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(intents))
}

async fn evaluate_intent(
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut intent = store.read_intent("main", intent_id).map_err(|_| {
                AppError::NotFound(format!("intent {} not found", intent_id))
            })?;

            intent.evaluate()?;

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("Evaluate intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(intent_to_response(&intent)))
}

async fn authorize_intent(
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut intent = store.read_intent("main", intent_id).map_err(|_| {
                AppError::NotFound(format!("intent {} not found", intent_id))
            })?;

            intent.authorize()?;

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("Authorize intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(intent_to_response(&intent)))
}

async fn execute_intent(
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<IntentResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let intent = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut intent = store.read_intent("main", intent_id).map_err(|_| {
                AppError::NotFound(format!("intent {} not found", intent_id))
            })?;

            intent.mark_executed()?;

            let path = format!("execution/intents/{}.json", intent_id);
            store
                .write_json(
                    "main",
                    &path,
                    &intent,
                    &format!("Execute intent {intent_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(intent)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(intent_to_response(&intent)))
}

// ── Handlers: Obligations ────────────────────────────────────────────

async fn create_obligation(
    State(state): State<AppState>,
    Json(req): Json<CreateObligationRequest>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let obligation_id = ObligationId::new();
            let obligation = Obligation::new(
                obligation_id,
                entity_id,
                req.intent_id,
                ObligationType::new(req.obligation_type),
                req.assignee_type,
                req.assignee_id,
                req.description,
                req.due_date,
            );

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json(
                    "main",
                    &path,
                    &obligation,
                    &format!("Create obligation {obligation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(obligation_to_response(&obligation)))
}

async fn list_obligations(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<ObligationResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let obligations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_obligation_ids("main").map_err(|e| {
                AppError::Internal(format!("list obligations: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let o = store.read_obligation("main", id).map_err(|e| {
                    AppError::Internal(format!("read obligation {id}: {e}"))
                })?;
                results.push(obligation_to_response(&o));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(obligations))
}

async fn fulfill_obligation(
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut obligation =
                store
                    .read_obligation("main", obligation_id)
                    .map_err(|_| {
                        AppError::NotFound(format!("obligation {} not found", obligation_id))
                    })?;

            obligation.fulfill()?;

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json(
                    "main",
                    &path,
                    &obligation,
                    &format!("Fulfill obligation {obligation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(obligation_to_response(&obligation)))
}

async fn waive_obligation(
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut obligation =
                store
                    .read_obligation("main", obligation_id)
                    .map_err(|_| {
                        AppError::NotFound(format!("obligation {} not found", obligation_id))
                    })?;

            obligation.waive()?;

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json(
                    "main",
                    &path,
                    &obligation,
                    &format!("Waive obligation {obligation_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(obligation_to_response(&obligation)))
}

// ── Receipt handlers ────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ReceiptResponse {
    pub receipt_id: ReceiptId,
    pub intent_id: IntentId,
    pub idempotency_key: String,
    pub status: ReceiptStatus,
    pub request_hash: String,
    pub response_hash: Option<String>,
    pub executed_at: Option<String>,
    pub created_at: String,
}

fn receipt_to_response(r: &Receipt) -> ReceiptResponse {
    ReceiptResponse {
        receipt_id: r.receipt_id(),
        intent_id: r.intent_id(),
        idempotency_key: r.idempotency_key().to_owned(),
        status: r.status(),
        request_hash: r.request_hash().to_owned(),
        response_hash: r.response_hash().map(|s| s.to_owned()),
        executed_at: r.executed_at().map(|t| t.to_rfc3339()),
        created_at: r.created_at().to_rfc3339(),
    }
}

async fn get_receipt(
    State(state): State<AppState>,
    Path(receipt_id): Path<ReceiptId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<ReceiptResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let receipt = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            store.read_receipt("main", receipt_id).map_err(|_| {
                AppError::NotFound(format!("receipt {} not found", receipt_id))
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(receipt_to_response(&receipt)))
}

async fn list_receipts_by_intent(
    State(state): State<AppState>,
    Path(intent_id): Path<IntentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<Vec<ReceiptResponse>>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let receipts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_receipt_ids("main").map_err(|e| {
                AppError::Internal(format!("list receipts: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(r) = store.read_receipt("main", id) {
                    if r.intent_id() == intent_id {
                        results.push(receipt_to_response(&r));
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(receipts))
}

// ── Obligation extended ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AssignObligationRequest {
    pub workspace_id: WorkspaceId,
    pub entity_id: EntityId,
    pub assignee_id: ContactId,
}

async fn assign_obligation(
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Json(req): Json<AssignObligationRequest>,
) -> Result<Json<ObligationResponse>, AppError> {
    let workspace_id = req.workspace_id;
    let entity_id = req.entity_id;

    let obligation = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut obligation = store
                .read_obligation("main", obligation_id)
                .map_err(|_| AppError::NotFound(format!("obligation {} not found", obligation_id)))?;

            obligation.assign(req.assignee_id)?;

            let path = format!("execution/obligations/{}.json", obligation_id);
            store
                .write_json("main", &path, &obligation, &format!("Assign obligation {obligation_id}"))
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(obligation)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(obligation_to_response(&obligation)))
}

#[derive(Serialize)]
pub struct ObligationsSummaryResponse {
    pub total: usize,
    pub pending: usize,
    pub fulfilled: usize,
    pub waived: usize,
}

async fn obligations_summary(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<ObligationsSummaryResponse>, AppError> {
    let workspace_id = query.workspace_id;

    let summary = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_obligation_ids("main").map_err(|e| {
                AppError::Internal(format!("list obligations: {e}"))
            })?;

            let mut total = 0;
            let mut pending = 0;
            let mut fulfilled = 0;
            let mut waived = 0;

            for id in ids {
                if let Ok(o) = store.read_obligation("main", id) {
                    total += 1;
                    match o.status() {
                        ObligationStatus::Required | ObligationStatus::InProgress => pending += 1,
                        ObligationStatus::Fulfilled => fulfilled += 1,
                        ObligationStatus::Waived => waived += 1,
                        _ => {}
                    }
                }
            }

            Ok::<_, AppError>(ObligationsSummaryResponse {
                total,
                pending,
                fulfilled,
                waived,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(summary))
}

async fn list_human_obligations(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<ObligationResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let obligations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_obligation_ids("main").map_err(|e| {
                AppError::Internal(format!("list obligations: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(o) = store.read_obligation("main", id) {
                    if o.assignee_type() == AssigneeType::Human {
                        results.push(obligation_to_response(&o));
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(obligations))
}

// ── Handlers: Global human obligations ──────────────────────────────

async fn list_global_human_obligations(
    State(state): State<AppState>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<ObligationResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let obligations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut results = Vec::new();

            for entity_id in entity_ids {
                if let Ok(store) = EntityStore::open(&layout, workspace_id, entity_id) {
                    if let Ok(ids) = store.list_obligation_ids("main") {
                        for id in ids {
                            if let Ok(o) = store.read_obligation("main", id) {
                                if o.assignee_type() == AssigneeType::Human
                                    && o.status() != ObligationStatus::Fulfilled
                                    && o.status() != ObligationStatus::Waived
                                {
                                    results.push(obligation_to_response(&o));
                                }
                            }
                        }
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(obligations))
}

// ── Handlers: Signer token ──────────────────────────────────────────

#[derive(Serialize)]
pub struct SignerTokenResponse {
    pub obligation_id: ObligationId,
    pub token: String,
    pub expires_at: String,
}

async fn generate_signer_token(
    Path(obligation_id): Path<ObligationId>,
) -> Json<SignerTokenResponse> {
    let token = format!("signer_{}", uuid::Uuid::new_v4().simple());
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339();
    Json(SignerTokenResponse {
        obligation_id,
        token,
        expires_at,
    })
}

// ── Handlers: Human obligation fulfill ──────────────────────────────

async fn fulfill_human_obligation(
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<ObligationResponse>, AppError> {
    fulfill_obligation(
        State(state),
        Path(obligation_id),
        Query(query),
    )
    .await
}

// ── Handlers: Document requests ─────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateDocumentRequestPayload {
    pub description: String,
    pub document_type: String,
    pub workspace_id: WorkspaceId,
    pub entity_id: EntityId,
}

#[derive(Serialize)]
pub struct DocumentRequestResponse {
    pub request_id: String,
    pub obligation_id: ObligationId,
    pub description: String,
    pub document_type: String,
    pub status: String,
    pub created_at: String,
}

async fn create_document_request(
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Json(req): Json<CreateDocumentRequestPayload>,
) -> Result<Json<DocumentRequestResponse>, AppError> {
    let workspace_id = req.workspace_id;
    let entity_id = req.entity_id;
    let request_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let request_id = request_id.clone();
        let description = req.description.clone();
        let document_type = req.document_type.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Verify obligation exists
            store.read_obligation("main", obligation_id).map_err(|_| {
                AppError::NotFound(format!("obligation {} not found", obligation_id))
            })?;

            store
                .write_json(
                    "main",
                    &format!("execution/document-requests/{}.json", request_id),
                    &serde_json::json!({
                        "request_id": request_id,
                        "obligation_id": obligation_id,
                        "description": description,
                        "document_type": document_type,
                        "status": "pending",
                        "created_at": now.to_rfc3339(),
                    }),
                    &format!("Create document request {request_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(DocumentRequestResponse {
        request_id,
        obligation_id,
        description: req.description,
        document_type: req.document_type,
        status: "pending".to_owned(),
        created_at: now.to_rfc3339(),
    }))
}

async fn list_document_requests(
    State(state): State<AppState>,
    Path(obligation_id): Path<ObligationId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<Vec<DocumentRequestResponse>>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let requests = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // List all document requests and filter by obligation_id
            let dir = "execution/document-requests";
            let ids: Vec<String> = store.list_ids_in_dir("main", dir).unwrap_or_default();

            let mut results = Vec::new();
            for id in ids {
                let path = format!("{}/{}.json", dir, id);
                if let Ok(val) = store.read_json::<serde_json::Value>("main", &path) {
                    if val.get("obligation_id").and_then(|v| v.as_str()) == Some(&obligation_id.to_string()) {
                        results.push(DocumentRequestResponse {
                            request_id: val.get("request_id").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                            obligation_id,
                            description: val.get("description").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                            document_type: val.get("document_type").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                            status: val.get("status").and_then(|v| v.as_str()).unwrap_or("pending").to_owned(),
                            created_at: val.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                        });
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(requests))
}

async fn fulfill_document_request(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<DocumentRequestResponse>, AppError> {
    update_document_request_status(state, request_id, query, "fulfilled").await
}

async fn mark_document_request_na(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<DocumentRequestResponse>, AppError> {
    update_document_request_status(state, request_id, query, "not_applicable").await
}

async fn update_document_request_status(
    state: AppState,
    request_id: String,
    query: super::WorkspaceEntityQuery,
    new_status: &'static str,
) -> Result<Json<DocumentRequestResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let request_id = request_id.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let path = format!("execution/document-requests/{}.json", request_id);
            let mut val: serde_json::Value = store.read_json("main", &path).map_err(|_| {
                AppError::NotFound(format!("document request {} not found", request_id))
            })?;

            val["status"] = serde_json::json!(new_status);

            store
                .write_json("main", &path, &val, &format!("Update doc request {request_id} to {new_status}"))
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(val)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let obl_id_str = result.get("obligation_id").and_then(|v| v.as_str()).unwrap_or("");
    let obligation_id = obl_id_str.parse::<uuid::Uuid>()
        .map(ObligationId::from_uuid)
        .unwrap_or_else(|_| ObligationId::new());

    Ok(Json(DocumentRequestResponse {
        request_id,
        obligation_id,
        description: result.get("description").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
        document_type: result.get("document_type").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
        status: new_status.to_owned(),
        created_at: result.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
    }))
}

// ── Handlers: Global obligations summary ────────────────────────────

async fn global_obligations_summary(
    State(state): State<AppState>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<ObligationsSummaryResponse>, AppError> {
    let workspace_id = query.workspace_id;

    let summary = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut total = 0;
            let mut pending = 0;
            let mut fulfilled = 0;
            let mut waived = 0;

            for entity_id in entity_ids {
                if let Ok(store) = EntityStore::open(&layout, workspace_id, entity_id) {
                    if let Ok(ids) = store.list_obligation_ids("main") {
                        for id in ids {
                            if let Ok(o) = store.read_obligation("main", id) {
                                total += 1;
                                match o.status() {
                                    ObligationStatus::Required | ObligationStatus::InProgress => pending += 1,
                                    ObligationStatus::Fulfilled => fulfilled += 1,
                                    ObligationStatus::Waived => waived += 1,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            Ok::<_, AppError>(ObligationsSummaryResponse {
                total,
                pending,
                fulfilled,
                waived,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(summary))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn execution_routes() -> Router<AppState> {
    Router::new()
        // Intents
        .route("/v1/execution/intents", post(create_intent))
        .route("/v1/entities/{entity_id}/intents", get(list_intents))
        .route(
            "/v1/intents/{intent_id}/evaluate",
            post(evaluate_intent),
        )
        .route(
            "/v1/intents/{intent_id}/authorize",
            post(authorize_intent),
        )
        .route(
            "/v1/intents/{intent_id}/execute",
            post(execute_intent),
        )
        // Obligations
        .route("/v1/execution/obligations", post(create_obligation))
        .route(
            "/v1/entities/{entity_id}/obligations",
            get(list_obligations),
        )
        .route(
            "/v1/obligations/{obligation_id}/fulfill",
            post(fulfill_obligation),
        )
        .route(
            "/v1/obligations/{obligation_id}/waive",
            post(waive_obligation),
        )
        .route(
            "/v1/obligations/{obligation_id}/assign",
            post(assign_obligation),
        )
        .route(
            "/v1/entities/{entity_id}/obligations/summary",
            get(obligations_summary),
        )
        // Receipts
        .route("/v1/receipts/{receipt_id}", get(get_receipt))
        .route(
            "/v1/intents/{intent_id}/receipts",
            get(list_receipts_by_intent),
        )
        // Human obligations
        .route(
            "/v1/entities/{entity_id}/obligations/human",
            get(list_human_obligations),
        )
        .route("/v1/human-obligations", get(list_global_human_obligations))
        .route(
            "/v1/human-obligations/{obligation_id}/signer-token",
            post(generate_signer_token),
        )
        .route(
            "/v1/human-obligations/{obligation_id}/fulfill",
            post(fulfill_human_obligation),
        )
        // Document requests
        .route(
            "/v1/obligations/{obligation_id}/document-requests",
            post(create_document_request).get(list_document_requests),
        )
        .route(
            "/v1/document-requests/{request_id}/fulfill",
            patch(fulfill_document_request),
        )
        .route(
            "/v1/document-requests/{request_id}/not-applicable",
            patch(mark_document_request_na),
        )
        // Global obligations summary
        .route("/v1/obligations/summary", get(global_obligations_summary))
}
