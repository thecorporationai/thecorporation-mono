//! Compliance HTTP routes.
//!
//! Endpoints for tax filings, deadlines, and contractor classification.

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::domain::formation::{
    contractor::{ClassificationResult, ContractorClassification, RiskLevel},
    deadline::{Deadline, DeadlineStatus, Recurrence},
    tax_filing::{TaxFiling, TaxFilingStatus},
};
use crate::domain::ids::*;
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FileTaxDocumentRequest {
    pub entity_id: EntityId,
    pub document_type: String,
    pub tax_year: i32,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Deserialize)]
pub struct CreateDeadlineRequest {
    pub entity_id: EntityId,
    pub deadline_type: String,
    pub due_date: NaiveDate,
    pub description: String,
    #[serde(default = "default_recurrence")]
    pub recurrence: Recurrence,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

fn default_recurrence() -> Recurrence {
    Recurrence::OneTime
}

#[derive(Deserialize)]
pub struct ClassifyContractorRequest {
    pub entity_id: EntityId,
    pub contractor_name: String,
    #[serde(default = "default_state")]
    pub state: String,
    #[serde(default)]
    pub factors: serde_json::Value,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

fn default_state() -> String {
    "CA".to_owned()
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TaxFilingResponse {
    pub filing_id: TaxFilingId,
    pub entity_id: EntityId,
    pub document_type: String,
    pub tax_year: i32,
    pub document_id: DocumentId,
    pub status: TaxFilingStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct DeadlineResponse {
    pub deadline_id: DeadlineId,
    pub entity_id: EntityId,
    pub deadline_type: String,
    pub due_date: NaiveDate,
    pub description: String,
    pub recurrence: Recurrence,
    pub status: DeadlineStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ClassificationResponse {
    pub classification_id: ClassificationId,
    pub entity_id: EntityId,
    pub contractor_name: String,
    pub state: String,
    pub risk_level: RiskLevel,
    pub flags: Vec<String>,
    pub classification: ClassificationResult,
    pub created_at: String,
}

// ── Helper ───────────────────────────────────────────────────────────

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

// ── Handlers ─────────────────────────────────────────────────────────

async fn file_tax_document(
    State(state): State<AppState>,
    Json(req): Json<FileTaxDocumentRequest>,
) -> Result<Json<TaxFilingResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let filing = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let filing_id = TaxFilingId::new();
            let document_id = DocumentId::new();
            let filing = TaxFiling::new(
                filing_id,
                entity_id,
                req.document_type,
                req.tax_year,
                document_id,
            );

            let path = format!("tax/filings/{}.json", filing_id);
            store
                .write_json("main", &path, &filing, &format!("File tax document {filing_id}"))
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(filing)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(TaxFilingResponse {
        filing_id: filing.filing_id(),
        entity_id: filing.entity_id(),
        document_type: filing.document_type().to_owned(),
        tax_year: filing.tax_year(),
        document_id: filing.document_id(),
        status: filing.status(),
        created_at: filing.created_at().to_rfc3339(),
    }))
}

async fn create_deadline(
    State(state): State<AppState>,
    Json(req): Json<CreateDeadlineRequest>,
) -> Result<Json<DeadlineResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let deadline = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let deadline_id = DeadlineId::new();
            let deadline = Deadline::new(
                deadline_id,
                entity_id,
                req.deadline_type,
                req.due_date,
                req.description,
                req.recurrence,
            );

            let path = format!("deadlines/{}.json", deadline_id);
            store
                .write_json("main", &path, &deadline, &format!("Create deadline {deadline_id}"))
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(deadline)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(DeadlineResponse {
        deadline_id: deadline.deadline_id(),
        entity_id: deadline.entity_id(),
        deadline_type: deadline.deadline_type().to_owned(),
        due_date: deadline.due_date(),
        description: deadline.description().to_owned(),
        recurrence: deadline.recurrence(),
        status: deadline.status(),
        created_at: deadline.created_at().to_rfc3339(),
    }))
}

async fn classify_contractor(
    State(state): State<AppState>,
    Json(req): Json<ClassifyContractorRequest>,
) -> Result<Json<ClassificationResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let classification = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let classification_id = ClassificationId::new();
            // Simple risk assessment based on state
            let risk_level = match req.state.as_str() {
                "CA" | "NY" | "MA" => RiskLevel::High,
                "TX" | "FL" | "WA" => RiskLevel::Low,
                _ => RiskLevel::Medium,
            };
            let flags: Vec<String> = if risk_level == RiskLevel::High {
                vec![format!("{}_strict_classification_laws", req.state.to_lowercase())]
            } else {
                vec![]
            };

            let classification = ContractorClassification::new(
                classification_id,
                entity_id,
                req.contractor_name,
                req.state,
                risk_level,
                flags,
                ClassificationResult::Independent,
            );

            let path = format!("contractors/{}.json", classification_id);
            store
                .write_json("main", &path, &classification, &format!("Classify contractor {classification_id}"))
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(classification)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(ClassificationResponse {
        classification_id: classification.classification_id(),
        entity_id: classification.entity_id(),
        contractor_name: classification.contractor_name().to_owned(),
        state: classification.state().to_owned(),
        risk_level: classification.risk_level(),
        flags: classification.flags().to_vec(),
        classification: classification.classification(),
        created_at: classification.created_at().to_rfc3339(),
    }))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn compliance_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/tax/filings", post(file_tax_document))
        .route("/v1/deadlines", post(create_deadline))
        .route("/v1/contractors/classify", post(classify_contractor))
}
