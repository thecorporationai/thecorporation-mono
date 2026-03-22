//! Compliance HTTP routes.
//!
//! Endpoints for tax filings, deadlines, and contractor classification.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::AppState;
use super::validation::{
    normalize_slug, require_non_empty_trimmed_max, validate_max_len, validate_reasonable_year,
};
use crate::auth::RequireAdmin;
use crate::domain::formation::{
    contractor::{ClassificationResult, ContractorClassification, RiskLevel},
    deadline::{Deadline, DeadlineSeverity, DeadlineStatus, Recurrence},
    escalation::ComplianceEscalation,
    evidence_link::ComplianceEvidenceLink,
    tax_filing::{TaxFiling, TaxFilingStatus},
    types::FormationStatus,
};
use crate::domain::governance::incident::{GovernanceIncident, IncidentSeverity, IncidentStatus};
use crate::domain::governance::trigger::{GovernanceTriggerSource, GovernanceTriggerType};
use crate::domain::ids::*;
use crate::domain::{
    execution::{
        obligation::Obligation,
        transaction_packet::TransactionPacket,
        types::{AssigneeType, ObligationStatus, ObligationType},
    },
    formation::escalation::EscalationStatus,
};
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::routes::governance_enforcement::{LockdownTriggerInput, apply_lockdown_trigger};
use crate::store::entity_store::EntityStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct FileTaxDocumentRequest {
    pub entity_id: EntityId,
    pub document_type: String,
    pub tax_year: i32,
    /// Optional contact for per-person filings (e.g. 83(b) elections).
    #[serde(default)]
    pub filer_contact_id: Option<crate::domain::ids::ContactId>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDeadlineRequest {
    pub entity_id: EntityId,
    pub deadline_type: String,
    pub due_date: NaiveDate,
    pub description: String,
    #[serde(default = "default_recurrence")]
    pub recurrence: Recurrence,
    #[serde(default = "default_deadline_severity")]
    pub severity: DeadlineSeverity,
}

fn default_recurrence() -> Recurrence {
    Recurrence::OneTime
}

fn default_deadline_severity() -> DeadlineSeverity {
    DeadlineSeverity::Medium
}

fn allowed_tax_document_type(document_type: &str) -> bool {
    allowed_tax_document_types().contains(&document_type)
}

fn canonical_tax_document_type(document_type: &str) -> &str {
    match document_type {
        "1120" | "form_1120" => "form_1120",
        "1120s" | "form_1120s" => "form_1120s",
        "1065" | "form_1065" => "form_1065",
        "1099_nec" | "form_1099_nec" => "form_1099_nec",
        "k1" | "form_k1" => "form_k1",
        "941" | "form_941" => "form_941",
        "w2" | "form_w2" => "form_w2",
        other => other,
    }
}

fn allowed_tax_document_types() -> &'static [&'static str] {
    &[
        "1120",
        "1120s",
        "1065",
        "franchise_tax",
        "annual_report",
        "83b",
        "form_1120",
        "form_1120s",
        "form_1065",
        "1099_nec",
        "form_1099_nec",
        "k1",
        "form_k1",
        "941",
        "form_941",
        "w2",
        "form_w2",
    ]
}

fn validate_tax_document_type(document_type: &str) -> Result<(), AppError> {
    if allowed_tax_document_type(document_type) {
        return Ok(());
    }
    Err(AppError::BadRequest(format!(
        "unsupported tax document type: {}. Allowed values: {}",
        document_type,
        allowed_tax_document_types().join(", ")
    )))
}

fn validate_deadline_recurrence(
    deadline_type: &str,
    recurrence: Recurrence,
) -> Result<(), AppError> {
    let expected = match deadline_type {
        "annual_report" | "franchise_tax" => Some(Recurrence::Annual),
        "quarterly_tax" => Some(Recurrence::Quarterly),
        "monthly_payroll_tax" => Some(Recurrence::Monthly),
        _ => None,
    };
    if let Some(expected) = expected
        && recurrence != expected
    {
        return Err(AppError::BadRequest(format!(
            "deadline type {deadline_type} requires recurrence {expected:?}"
        )));
    }
    Ok(())
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ScanComplianceRequest {
    pub entity_id: EntityId,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ClassifyContractorRequest {
    pub entity_id: EntityId,
    pub contractor_name: String,
    #[serde(default = "default_state")]
    pub state: String,
    #[serde(default)]
    pub hours_per_week: Option<u32>,
    #[serde(default)]
    pub exclusive_client: Option<bool>,
    #[serde(default)]
    pub duration_months: Option<u32>,
    #[serde(default)]
    pub provides_tools: Option<bool>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub factors: serde_json::Value,
}

fn default_state() -> String {
    "CA".to_owned()
}

const VALID_US_STATE_CODES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS",
    "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY",
    "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY", "DC",
];

fn normalize_state_code(state: &str) -> Result<String, AppError> {
    let normalized = state.trim().to_ascii_uppercase();
    if VALID_US_STATE_CODES.contains(&normalized.as_str()) {
        return Ok(normalized);
    }
    Err(AppError::BadRequest(format!(
        "unsupported state code: {}",
        state
    )))
}

fn json_u32(value: &serde_json::Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|raw| u32::try_from(raw).ok())
}

fn json_bool(value: &serde_json::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(serde_json::Value::as_bool)
}

fn classify_contractor_inputs(
    req: &ClassifyContractorRequest,
) -> (RiskLevel, Vec<String>, ClassificationResult) {
    let mut score = 0u32;
    let mut flags = Vec::new();

    match req.state.as_str() {
        "CA" | "MA" | "NJ" | "NY" => {
            score += 2;
            flags.push(format!(
                "{}_strict_classification_laws",
                req.state.to_lowercase()
            ));
        }
        "TX" | "FL" | "WA" => {}
        _ => {
            score += 1;
        }
    }

    let hours_per_week = req
        .hours_per_week
        .or_else(|| json_u32(&req.factors, "hours_per_week"));
    if hours_per_week.is_some_and(|hours| hours >= 35) {
        score += 2;
        flags.push("full_time_schedule".to_owned());
    }

    let exclusive_client = req
        .exclusive_client
        .or_else(|| json_bool(&req.factors, "exclusive_client"))
        .unwrap_or(false);
    if exclusive_client {
        score += 2;
        flags.push("exclusive_client".to_owned());
    }

    let duration_months = req
        .duration_months
        .or_else(|| json_u32(&req.factors, "duration_months"));
    if duration_months.is_some_and(|months| months >= 12) {
        score += 1;
        flags.push("long_term_engagement".to_owned());
    }

    let provides_tools = req
        .provides_tools
        .or_else(|| json_bool(&req.factors, "provides_tools"));
    if provides_tools == Some(true) {
        score += 2;
        flags.push("company_provides_tools".to_owned());
    }

    let risk_level = if score >= 5 {
        RiskLevel::High
    } else if score >= 2 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };
    let classification = if score >= 6 {
        ClassificationResult::Employee
    } else if score >= 3 {
        ClassificationResult::Uncertain
    } else {
        ClassificationResult::Independent
    };

    (risk_level, flags, classification)
}

fn ensure_entity_ready_for_compliance(
    store: &EntityStore<'_>,
    action: &str,
) -> Result<(), AppError> {
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    if entity.formation_status() != FormationStatus::Active {
        return Err(AppError::BadRequest(format!(
            "{action} requires an active entity, current status is {}",
            entity.formation_status()
        )));
    }
    Ok(())
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResolveEscalationWithEvidenceRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub packet_id: Option<PacketId>,
    #[serde(default)]
    pub filing_reference: Option<String>,
    #[serde(default)]
    pub evidence_type: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub resolve_obligation: bool,
    #[serde(default)]
    pub resolve_incident: bool,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct TaxFilingResponse {
    pub filing_id: TaxFilingId,
    pub entity_id: EntityId,
    pub document_type: String,
    pub tax_year: i32,
    pub document_id: DocumentId,
    pub status: TaxFilingStatus,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DeadlineResponse {
    pub deadline_id: DeadlineId,
    pub entity_id: EntityId,
    pub deadline_type: String,
    pub due_date: NaiveDate,
    pub description: String,
    pub recurrence: Recurrence,
    pub severity: DeadlineSeverity,
    pub status: DeadlineStatus,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
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

#[derive(Serialize, utoipa::ToSchema)]
pub struct ComplianceEscalationResponse {
    pub escalation_id: ComplianceEscalationId,
    pub entity_id: EntityId,
    pub deadline_id: DeadlineId,
    pub milestone: String,
    pub action: String,
    pub authority: String,
    pub status: EscalationStatus,
    pub obligation_id: Option<ObligationId>,
    pub incident_id: Option<IncidentId>,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ComplianceScanResponse {
    pub scanned_deadlines: usize,
    pub escalations_created: usize,
    pub incidents_created: usize,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ResolveEscalationWithEvidenceResponse {
    pub escalation: ComplianceEscalationResponse,
    pub evidence_link_id: ComplianceEvidenceLinkId,
    pub obligation_resolved: bool,
    pub incident_resolved: bool,
}

// ── Helper ───────────────────────────────────────────────────────────

fn open_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    allowed_entity_ids: Option<&[EntityId]>,
    entity_id: EntityId,
    valkey_client: Option<&redis::Client>,
) -> Result<EntityStore<'a>, AppError> {
    if let Some(ids) = allowed_entity_ids
        && !ids.contains(&entity_id)
    {
        return Err(AppError::Forbidden(format!(
            "principal is not authorized for entity {}",
            entity_id
        )));
    }
    EntityStore::open(layout, workspace_id, entity_id, valkey_client).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {} not found", entity_id))
        }
        other => AppError::Internal(other.to_string()),
    })
}

fn deadline_to_response(deadline: &Deadline) -> DeadlineResponse {
    DeadlineResponse {
        deadline_id: deadline.deadline_id(),
        entity_id: deadline.entity_id(),
        deadline_type: deadline.deadline_type().to_owned(),
        due_date: deadline.due_date(),
        description: deadline.description().to_owned(),
        recurrence: deadline.recurrence(),
        severity: deadline.severity(),
        status: deadline.status(),
        completed_at: deadline.completed_at().map(|t| t.to_rfc3339()),
        created_at: deadline.created_at().to_rfc3339(),
    }
}

fn tax_filing_to_response(filing: &TaxFiling) -> TaxFilingResponse {
    TaxFilingResponse {
        filing_id: filing.filing_id(),
        entity_id: filing.entity_id(),
        document_type: filing.document_type().to_owned(),
        tax_year: filing.tax_year(),
        document_id: filing.document_id(),
        status: filing.status(),
        created_at: filing.created_at().to_rfc3339(),
    }
}

fn classification_to_response(classification: &ContractorClassification) -> ClassificationResponse {
    ClassificationResponse {
        classification_id: classification.classification_id(),
        entity_id: classification.entity_id(),
        contractor_name: classification.contractor_name().to_owned(),
        state: classification.state().to_owned(),
        risk_level: classification.risk_level(),
        flags: classification.flags().to_vec(),
        classification: classification.classification(),
        created_at: classification.created_at().to_rfc3339(),
    }
}

fn escalation_to_response(escalation: &ComplianceEscalation) -> ComplianceEscalationResponse {
    ComplianceEscalationResponse {
        escalation_id: escalation.escalation_id(),
        entity_id: escalation.entity_id(),
        deadline_id: escalation.deadline_id(),
        milestone: escalation.milestone().to_owned(),
        action: escalation.action().to_owned(),
        authority: escalation.authority().to_owned(),
        status: escalation.status(),
        obligation_id: escalation.obligation_id(),
        incident_id: escalation.incident_id(),
        created_at: escalation.created_at().to_rfc3339(),
    }
}

fn milestone_specs() -> [(&'static str, i64, &'static str, &'static str); 6] {
    [
        (
            "D-30",
            30,
            "Prepare filing package and evidence",
            "operator",
        ),
        (
            "D-14",
            14,
            "Escalate with owner reminder and checklist",
            "operator",
        ),
        ("D-7", 7, "Require execution plan with assignee", "officer"),
        (
            "D-1",
            1,
            "Immediate owner notification and hold risky actions",
            "officer",
        ),
        (
            "D+0",
            0,
            "Deadline due today: block non-essential changes",
            "officer",
        ),
        (
            "D+1",
            -1,
            "Missed deadline: incident + board escalation",
            "board",
        ),
    ]
}

// ── Handlers ─────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/tax/filings",
    tag = "compliance",
    request_body = FileTaxDocumentRequest,
    responses(
        (status = 200, description = "Tax document filed", body = TaxFilingResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn file_tax_document(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<FileTaxDocumentRequest>,
) -> Result<Json<TaxFilingResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("compliance.tax_filing.create", workspace_id, 60, 60)?;
    validate_tax_document_type(&req.document_type)?;
    validate_reasonable_year("tax_year", req.tax_year, 1900, 2)?;

    let filing = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            ensure_entity_ready_for_compliance(&store, "tax filing")?;
            let document_type = canonical_tax_document_type(&req.document_type).to_owned();
            let existing_ids = store
                .list_ids::<TaxFiling>("main")
                .map_err(|e| AppError::Internal(format!("list tax filings: {e}")))?;
            for existing_id in existing_ids {
                let existing = store.read::<TaxFiling>("main", existing_id).map_err(|e| {
                    AppError::Internal(format!("read tax filing {existing_id}: {e}"))
                })?;
                if canonical_tax_document_type(existing.document_type()) == document_type
                    && existing.tax_year() == req.tax_year
                    && existing.filer_contact_id() == req.filer_contact_id
                {
                    let scope = match req.filer_contact_id {
                        Some(cid) => format!(" for filer {cid}"),
                        None => String::new(),
                    };
                    return Err(AppError::Conflict(format!(
                        "tax filing already exists for {} tax year {}{}",
                        document_type, req.tax_year, scope
                    )));
                }
            }

            let filing_id = TaxFilingId::new();
            let document_id = DocumentId::new();
            let filing = TaxFiling::new(
                filing_id,
                entity_id,
                document_type,
                req.tax_year,
                document_id,
            ).with_filer_contact(req.filer_contact_id);

            let path = format!("tax/filings/{}.json", filing_id);
            store
                .write_json(
                    "main",
                    &path,
                    &filing,
                    &format!("File tax document {filing_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(filing)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

#[utoipa::path(
    post,
    path = "/v1/deadlines",
    tag = "compliance",
    request_body = CreateDeadlineRequest,
    responses(
        (status = 200, description = "Deadline created", body = DeadlineResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_deadline(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateDeadlineRequest>,
) -> Result<Json<DeadlineResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("compliance.deadline.create", workspace_id, 120, 60)?;
    let deadline_type = normalize_slug(&req.deadline_type, "deadline_type", 128)?;
    require_non_empty_trimmed_max(&req.description, "description", 2000)?;
    validate_deadline_recurrence(&deadline_type, req.recurrence)?;

    let deadline = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let deadline_type = deadline_type.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;

            let deadline_id = DeadlineId::new();
            let deadline = Deadline::new(
                deadline_id,
                entity_id,
                deadline_type,
                req.due_date,
                req.description,
                req.recurrence,
                req.severity,
            );

            let path = format!("deadlines/{}.json", deadline_id);
            store
                .write_json(
                    "main",
                    &path,
                    &deadline,
                    &format!("COMPLIANCE: create deadline {deadline_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(deadline)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(deadline_to_response(&deadline)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/tax-filings",
    tag = "compliance",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of tax filings", body = Vec<TaxFilingResponse>),
    ),
)]
async fn list_tax_filings(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<TaxFilingResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let filings = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<TaxFiling>("main")
                .map_err(|e| AppError::Internal(format!("list tax filings: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let filing = store
                    .read::<TaxFiling>("main", id)
                    .map_err(|e| AppError::Internal(format!("read tax filing {id}: {e}")))?;
                results.push(tax_filing_to_response(&filing));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(filings))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/deadlines",
    tag = "compliance",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of deadlines", body = Vec<DeadlineResponse>),
    ),
)]
async fn list_deadlines(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<DeadlineResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let deadlines = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<Deadline>("main")
                .map_err(|e| AppError::Internal(format!("list deadlines: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let deadline = store
                    .read::<Deadline>("main", id)
                    .map_err(|e| AppError::Internal(format!("read deadline {id}: {e}")))?;
                results.push(deadline_to_response(&deadline));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(deadlines))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/contractor-classifications",
    tag = "compliance",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of contractor classifications", body = Vec<ClassificationResponse>),
    ),
)]
async fn list_contractor_classifications(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ClassificationResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let classifications = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<ContractorClassification>("main")
                .map_err(|e| AppError::Internal(format!("list contractor classifications: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let classification =
                    store
                        .read::<ContractorClassification>("main", id)
                        .map_err(|e| {
                            AppError::Internal(format!("read contractor classification {id}: {e}"))
                        })?;
                results.push(classification_to_response(&classification));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(classifications))
}

#[utoipa::path(
    post,
    path = "/v1/contractors/classify",
    tag = "compliance",
    request_body = ClassifyContractorRequest,
    responses(
        (status = 200, description = "Contractor classified", body = ClassificationResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn classify_contractor(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<ClassifyContractorRequest>,
) -> Result<Json<ClassificationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit(
        "compliance.contractor_classification.create",
        workspace_id,
        60,
        60,
    )?;
    if req.contractor_name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "contractor_name cannot be empty".to_owned(),
        ));
    }
    validate_max_len(&req.contractor_name, "contractor_name", 256)?;
    let normalized_state = normalize_state_code(&req.state)?;
    let (risk_level, flags, classification_result) =
        classify_contractor_inputs(&ClassifyContractorRequest {
            state: normalized_state.clone(),
            contractor_name: req.contractor_name.clone(),
            entity_id,
            hours_per_week: req.hours_per_week,
            exclusive_client: req.exclusive_client,
            duration_months: req.duration_months,
            provides_tools: req.provides_tools,
            factors: req.factors.clone(),
        });

    let classification = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let contractor_name = req.contractor_name.trim().to_owned();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;

            let classification_id = ClassificationId::new();
            let classification = ContractorClassification::new(
                classification_id,
                entity_id,
                contractor_name,
                normalized_state,
                risk_level,
                flags,
                classification_result,
            );

            let path = format!("contractors/{}.json", classification_id);
            store
                .write_json(
                    "main",
                    &path,
                    &classification,
                    &format!("Classify contractor {classification_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(classification)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

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

#[utoipa::path(
    post,
    path = "/v1/compliance/escalations/scan",
    tag = "compliance",
    request_body = ScanComplianceRequest,
    responses(
        (status = 200, description = "Compliance scan completed", body = ComplianceScanResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn scan_compliance_escalations(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Json(req): Json<ScanComplianceRequest>,
) -> Result<Json<ComplianceScanResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let deadline_ids = store
                .list_ids::<Deadline>("main")
                .map_err(|e| AppError::Internal(format!("list deadlines: {e}")))?;
            let escalation_ids = store
                .list_ids::<ComplianceEscalation>("main")
                .map_err(|e| AppError::Internal(format!("list escalations: {e}")))?;

            let mut existing = std::collections::HashSet::<(DeadlineId, String)>::new();
            for escalation_id in escalation_ids {
                if let Ok(escalation) = store.read::<ComplianceEscalation>("main", escalation_id) {
                    existing.insert((escalation.deadline_id(), escalation.milestone().to_owned()));
                }
            }

            let today = Utc::now().date_naive();
            let mut scanned_deadlines = 0usize;
            let mut escalations_created = 0usize;
            let mut incidents_created = 0usize;

            for deadline_id in deadline_ids {
                let deadline = match store.read::<Deadline>("main", deadline_id) {
                    Ok(deadline) => deadline,
                    Err(_) => continue,
                };
                if deadline.status() == DeadlineStatus::Completed {
                    continue;
                }
                scanned_deadlines += 1;
                let days_until = (deadline.due_date() - today).num_days();

                for (milestone, threshold, action, authority) in milestone_specs() {
                    if days_until > threshold {
                        continue;
                    }
                    let key = (deadline.deadline_id(), milestone.to_owned());
                    if existing.contains(&key) {
                        continue;
                    }

                    let obligation_id = ObligationId::new();
                    let obligation = Obligation::new(
                        obligation_id,
                        entity_id,
                        None,
                        ObligationType::new(format!(
                            "compliance_escalation_{}",
                            milestone.to_lowercase().replace('+', "plus")
                        )),
                        AssigneeType::Human,
                        None,
                        format!(
                            "{} for deadline {} ({})",
                            action,
                            deadline.deadline_type(),
                            deadline.deadline_id()
                        ),
                        Some(deadline.due_date()),
                    );
                    store
                        .write_json(
                            "main",
                            &format!("execution/obligations/{}.json", obligation_id),
                            &obligation,
                            &format!(
                                "COMPLIANCE: create escalation obligation {obligation_id} for {}",
                                deadline.deadline_id()
                            ),
                        )
                        .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

                    let incident_id = if milestone == "D+1" {
                        let severity = match deadline.severity() {
                            DeadlineSeverity::Low => IncidentSeverity::Low,
                            DeadlineSeverity::Medium => IncidentSeverity::Medium,
                            DeadlineSeverity::High => IncidentSeverity::High,
                            DeadlineSeverity::Critical => IncidentSeverity::Critical,
                        };
                        let lockdown = apply_lockdown_trigger(
                            &store,
                            entity_id,
                            LockdownTriggerInput {
                                source: GovernanceTriggerSource::ComplianceScanner,
                                trigger_type: GovernanceTriggerType::ComplianceDeadlineMissedDPlus1,
                                severity,
                                title: format!("Compliance miss: {}", deadline.deadline_type()),
                                description: format!(
                                    "Deadline {} missed by at least one day",
                                    deadline.deadline_id()
                                ),
                                evidence_refs: vec![format!("deadline:{}", deadline.deadline_id())],
                                linked_intent_id: None,
                                linked_escalation_id: None,
                                idempotency_key: Some(format!(
                                    "compliance-d-plus-1:{}",
                                    deadline.deadline_id()
                                )),
                                existing_incident_id: None,
                                updated_by: None,
                            },
                        )?;
                        if lockdown.incident_created {
                            incidents_created += 1;
                        }
                        Some(lockdown.incident.incident_id())
                    } else {
                        None
                    };

                    let escalation = ComplianceEscalation::new(
                        ComplianceEscalationId::new(),
                        entity_id,
                        deadline.deadline_id(),
                        milestone.to_owned(),
                        action.to_owned(),
                        authority.to_owned(),
                        Some(obligation_id),
                        incident_id,
                    );
                    store
                        .write_json(
                            "main",
                            &format!("compliance/escalations/{}.json", escalation.escalation_id()),
                            &escalation,
                            &format!(
                                "COMPLIANCE: create escalation {} {}",
                                escalation.escalation_id(),
                                milestone
                            ),
                        )
                        .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
                    existing.insert((deadline.deadline_id(), milestone.to_owned()));
                    escalations_created += 1;
                }
            }

            Ok::<_, AppError>(ComplianceScanResponse {
                scanned_deadlines,
                escalations_created,
                incidents_created,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/compliance/escalations",
    tag = "compliance",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of compliance escalations", body = Vec<ComplianceEscalationResponse>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_entity_escalations(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ComplianceEscalationResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let escalations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let ids = store
                .list_ids::<ComplianceEscalation>("main")
                .map_err(|e| AppError::Internal(format!("list escalations: {e}")))?;
            let mut out = Vec::new();
            for id in ids {
                if let Ok(escalation) = store.read::<ComplianceEscalation>("main", id) {
                    out.push(escalation_to_response(&escalation));
                }
            }
            out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok::<_, AppError>(out)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(escalations))
}

#[utoipa::path(
    post,
    path = "/v1/compliance/escalations/{escalation_id}/resolve-with-evidence",
    tag = "compliance",
    params(
        ("escalation_id" = ComplianceEscalationId, Path, description = "Escalation ID"),
    ),
    request_body = ResolveEscalationWithEvidenceRequest,
    responses(
        (status = 200, description = "Escalation resolved with evidence", body = ResolveEscalationWithEvidenceResponse),
        (status = 404, description = "Escalation not found"),
    ),
)]
async fn resolve_escalation_with_evidence(
    RequireAdmin(auth): RequireAdmin,
    State(state): State<AppState>,
    Path(escalation_id): Path<ComplianceEscalationId>,
    Json(req): Json<ResolveEscalationWithEvidenceRequest>,
) -> Result<Json<ResolveEscalationWithEvidenceResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_scope.as_deref(), entity_id, valkey_client.as_ref())?;
            let mut escalation = store
                .read::<ComplianceEscalation>("main", escalation_id)
                .map_err(|_| {
                    AppError::NotFound(format!("escalation {} not found", escalation_id))
                })?;
            if escalation.entity_id() != entity_id {
                return Err(AppError::Forbidden(
                    "escalation belongs to a different entity".to_owned(),
                ));
            }

            let evidence_link_id = ComplianceEvidenceLinkId::new();
            let evidence_type = req
                .evidence_type
                .unwrap_or_else(|| "escalation_resolution_evidence".to_owned());
            let evidence_link = ComplianceEvidenceLink::new(
                evidence_link_id,
                entity_id,
                escalation_id,
                evidence_type,
                req.packet_id,
                req.filing_reference,
                req.notes,
            );

            let mut files = vec![
                FileWrite::json(
                    format!(
                        "compliance/evidence-links/{}.json",
                        evidence_link.evidence_link_id()
                    ),
                    &evidence_link,
                )
                .map_err(|e| AppError::Internal(format!("serialize evidence link: {e}")))?,
            ];

            if let Some(packet_id) = req.packet_id {
                let mut packet = store
                    .read::<TransactionPacket>("main", packet_id)
                    .map_err(|_| AppError::NotFound(format!("packet {} not found", packet_id)))?;
                if packet.entity_id() != entity_id {
                    return Err(AppError::Forbidden(
                        "packet belongs to a different entity".to_owned(),
                    ));
                }
                packet.add_evidence_ref(format!("escalation:{}", escalation_id));
                packet.add_evidence_ref(format!("evidence_link:{}", evidence_link_id));
                files.push(
                    FileWrite::json(format!("execution/packets/{}.json", packet_id), &packet)
                        .map_err(|e| AppError::Internal(format!("serialize packet: {e}")))?,
                );
            }

            let mut obligation_resolved = false;
            if req.resolve_obligation
                && let Some(obligation_id) = escalation.obligation_id()
            {
                let mut obligation =
                    store
                        .read::<Obligation>("main", obligation_id)
                        .map_err(|_| {
                            AppError::NotFound(format!("obligation {} not found", obligation_id))
                        })?;
                if matches!(
                    obligation.status(),
                    ObligationStatus::Required | ObligationStatus::InProgress
                ) {
                    obligation.fulfill()?;
                    obligation_resolved = true;
                    files.push(
                        FileWrite::json(
                            format!("execution/obligations/{}.json", obligation_id),
                            &obligation,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize obligation: {e}")))?,
                    );
                }
            }

            let mut incident_resolved = false;
            if req.resolve_incident
                && let Some(incident_id) = escalation.incident_id()
            {
                let mut incident = store
                    .read::<GovernanceIncident>("main", incident_id)
                    .map_err(|_| {
                        AppError::NotFound(format!("incident {} not found", incident_id))
                    })?;
                if incident.status() == IncidentStatus::Open {
                    incident.resolve();
                    incident_resolved = true;
                    files.push(
                        FileWrite::json(
                            format!("governance/incidents/{}.json", incident_id),
                            &incident,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize incident: {e}")))?,
                    );
                }
            }

            escalation.resolve();
            files.push(
                FileWrite::json(
                    format!("compliance/escalations/{}.json", escalation_id),
                    &escalation,
                )
                .map_err(|e| AppError::Internal(format!("serialize escalation: {e}")))?,
            );

            store
                .commit(
                    "main",
                    &format!(
                        "COMPLIANCE: resolve escalation {} with evidence {}",
                        escalation_id, evidence_link_id
                    ),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(ResolveEscalationWithEvidenceResponse {
                escalation: escalation_to_response(&escalation),
                evidence_link_id,
                obligation_resolved,
                incident_resolved,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn compliance_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/tax/filings", post(file_tax_document))
        .route(
            "/v1/entities/{entity_id}/tax-filings",
            get(list_tax_filings),
        )
        .route("/v1/deadlines", post(create_deadline))
        .route("/v1/entities/{entity_id}/deadlines", get(list_deadlines))
        .route("/v1/contractors/classify", post(classify_contractor))
        .route(
            "/v1/entities/{entity_id}/contractor-classifications",
            get(list_contractor_classifications),
        )
        .route(
            "/v1/compliance/escalations/scan",
            post(scan_compliance_escalations),
        )
        .route(
            "/v1/entities/{entity_id}/compliance/escalations",
            get(list_entity_escalations),
        )
        .route(
            "/v1/compliance/escalations/{escalation_id}/resolve-with-evidence",
            post(resolve_escalation_with_evidence),
        )
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        file_tax_document,
        list_tax_filings,
        create_deadline,
        list_deadlines,
        classify_contractor,
        list_contractor_classifications,
        scan_compliance_escalations,
        list_entity_escalations,
        resolve_escalation_with_evidence,
    ),
    components(schemas(
        FileTaxDocumentRequest,
        CreateDeadlineRequest,
        ScanComplianceRequest,
        ClassifyContractorRequest,
        ResolveEscalationWithEvidenceRequest,
        TaxFilingResponse,
        DeadlineResponse,
        ClassificationResponse,
        ComplianceEscalationResponse,
        ComplianceScanResponse,
        ResolveEscalationWithEvidenceResponse,
    )),
    tags((name = "compliance", description = "Compliance checks and monitoring")),
)]
pub struct ComplianceApi;

#[cfg(test)]
mod tests {
    use super::{
        ClassificationResult, ClassifyContractorRequest, RiskLevel, classify_contractor_inputs,
        normalize_state_code, validate_tax_document_type,
    };
    use crate::domain::ids::EntityId;
    use crate::error::AppError;

    #[test]
    fn invalid_tax_document_type_lists_allowed_values() {
        let err = validate_tax_document_type("SS-4").expect_err("invalid type should fail");
        match err {
            AppError::BadRequest(message) => {
                assert!(message.contains("unsupported tax document type: SS-4"));
                assert!(message.contains("1120"));
                assert!(message.contains("annual_report"));
            }
            other => panic!("expected bad request, got {other:?}"),
        }
    }

    #[test]
    fn normalize_state_code_rejects_invalid_values() {
        let err = normalize_state_code("XX").expect_err("invalid state should fail");
        match err {
            AppError::BadRequest(message) => {
                assert!(message.contains("unsupported state code"));
            }
            other => panic!("expected bad request, got {other:?}"),
        }
    }

    #[test]
    fn contractor_classification_uses_behavioral_inputs() {
        let req = ClassifyContractorRequest {
            entity_id: EntityId::new(),
            contractor_name: "Jane Consultant".to_owned(),
            state: "CA".to_owned(),
            hours_per_week: Some(40),
            exclusive_client: Some(true),
            duration_months: Some(18),
            provides_tools: Some(true),
            factors: serde_json::json!({}),
        };
        let (risk_level, flags, classification) = classify_contractor_inputs(&req);
        assert_eq!(risk_level, RiskLevel::High);
        assert_eq!(classification, ClassificationResult::Employee);
        assert!(flags.iter().any(|flag| flag == "full_time_schedule"));
        assert!(flags.iter().any(|flag| flag == "exclusive_client"));
        assert!(flags.iter().any(|flag| flag == "company_provides_tools"));
    }
}
