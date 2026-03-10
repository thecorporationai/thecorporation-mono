//! Formation HTTP routes.
//!
//! Endpoints for creating entities, signing documents, and advancing through
//! the formation lifecycle.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::AppState;
use crate::auth::{RequireFormationCreate, RequireFormationRead, RequireFormationSign};
use crate::domain::formation::{
    content::{InvestorType, MemberInput, MemberRole, OfficerTitle},
    contract::{Contract, ContractStatus, ContractTemplateType},
    document::{Document, SignatureRequest},
    entity::Entity,
    filing::Filing,
    service,
    types::*,
};
use crate::domain::governance::{
    doc_ast, doc_generator,
    profile::{
        CompanyAddress, DocumentOptions, FiscalYearEnd, GOVERNANCE_PROFILE_PATH, GovernanceProfile,
    },
    typst_renderer,
};
use crate::domain::ids::{ContractId, DocumentId, EntityId, SignatureId, WorkspaceId};
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Request / Response types ────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateFormationRequest {
    pub entity_type: EntityType,
    pub legal_name: String,
    pub jurisdiction: Jurisdiction,
    #[serde(default)]
    pub registered_agent_name: Option<String>,
    #[serde(default)]
    pub registered_agent_address: Option<String>,
    pub members: Vec<MemberInput>,
    #[serde(default)]
    pub authorized_shares: Option<i64>,
    #[serde(default)]
    pub par_value: Option<String>,
    /// Optional formation date for importing pre-formed entities.
    #[serde(default)]
    pub formation_date: Option<String>,
    /// Fiscal year end, e.g. "12-31". Defaults to "12-31".
    #[serde(default)]
    pub fiscal_year_end: Option<String>,
    /// Whether the company will elect S-Corp tax treatment.
    #[serde(default)]
    pub s_corp_election: Option<bool>,
    /// Include transfer restrictions in bylaws (corp). Default true.
    #[serde(default)]
    pub transfer_restrictions: Option<bool>,
    /// Include right of first refusal in bylaws (corp). Default true.
    #[serde(default)]
    pub right_of_first_refusal: Option<bool>,
    /// Company address.
    #[serde(default)]
    pub company_address: Option<crate::domain::formation::content::Address>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct FormationResponse {
    pub formation_id: EntityId,
    pub entity_id: EntityId,
    pub formation_status: FormationStatus,
    pub document_ids: Vec<DocumentId>,
    pub next_action: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct FormationWithCapTableResponse {
    pub formation_id: EntityId,
    pub entity_id: EntityId,
    pub formation_status: FormationStatus,
    pub document_ids: Vec<DocumentId>,
    pub next_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_entity_id: Option<crate::domain::ids::LegalEntityId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instrument_id: Option<crate::domain::ids::InstrumentId>,
    pub holders: Vec<service::HolderSummary>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct FormationStatusResponse {
    pub entity_id: EntityId,
    pub legal_name: String,
    pub entity_type: EntityType,
    pub jurisdiction: Jurisdiction,
    pub formation_state: FormationState,
    pub formation_status: FormationStatus,
    pub formation_date: Option<String>,
    pub next_action: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentSummary {
    pub document_id: DocumentId,
    pub document_type: DocumentType,
    pub title: String,
    pub status: DocumentStatus,
    pub signature_count: usize,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentResponse {
    pub document_id: DocumentId,
    pub entity_id: EntityId,
    pub document_type: DocumentType,
    pub title: String,
    pub status: DocumentStatus,
    #[schema(value_type = Object)]
    pub content: serde_json::Value,
    pub content_hash: String,
    pub version: u32,
    pub signatures: Vec<SignatureSummary>,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SignatureSummary {
    pub signature_id: SignatureId,
    pub signer_name: String,
    pub signer_role: String,
    pub signed_at: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SignDocumentRequest {
    pub signer_name: String,
    pub signer_role: String,
    pub signer_email: String,
    pub signature_text: String,
    #[serde(default = "default_consent")]
    pub consent_text: String,
    pub signature_svg: Option<String>,
}

fn default_consent() -> String {
    "I agree to sign this document electronically.".to_string()
}

fn default_filing_attestation_consent() -> String {
    "I attest the filing information is accurate and authorized.".to_owned()
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SignDocumentResponse {
    pub signature_id: SignatureId,
    pub document_id: DocumentId,
    pub document_status: DocumentStatus,
    pub signed_at: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConfirmFilingRequest {
    pub external_filing_id: String,
    #[serde(default)]
    pub receipt_reference: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConfirmEinRequest {
    pub ein: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct FilingAttestationRequest {
    pub signer_name: String,
    pub signer_role: String,
    pub signer_email: String,
    #[serde(default = "default_filing_attestation_consent")]
    pub consent_text: String,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RegisteredAgentConsentEvidenceRequest {
    pub evidence_uri: String,
    #[serde(default)]
    pub evidence_type: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ExecuteServiceAgreementRequest {
    #[serde(default)]
    pub contract_id: Option<ContractId>,
    #[serde(default)]
    pub document_id: Option<DocumentId>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct FormationGatesResponse {
    pub entity_id: EntityId,
    pub filing_submission_blockers: Vec<String>,
    pub requires_natural_person_attestation: bool,
    pub designated_attestor_name: String,
    pub designated_attestor_email: Option<String>,
    pub designated_attestor_role: String,
    pub attestation_recorded: bool,
    pub requires_registered_agent_consent_evidence: bool,
    pub registered_agent_consent_evidence_count: usize,
    pub service_agreement_required_for_tier1_autonomy: bool,
    pub service_agreement_executed: bool,
    pub service_agreement_executed_at: Option<String>,
    pub service_agreement_contract_id: Option<ContractId>,
    pub service_agreement_document_id: Option<DocumentId>,
    pub service_agreement_notes: Option<String>,
}

// ── Staged formation request / response types ──────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreatePendingFormationRequest {
    pub entity_type: EntityType,
    pub legal_name: String,
    #[serde(default = "default_staged_jurisdiction")]
    pub jurisdiction: Option<Jurisdiction>,
    #[serde(default)]
    pub registered_agent_name: Option<String>,
    #[serde(default)]
    pub registered_agent_address: Option<String>,
    #[serde(default)]
    pub formation_date: Option<String>,
    #[serde(default)]
    pub fiscal_year_end: Option<String>,
    #[serde(default)]
    pub s_corp_election: Option<bool>,
    #[serde(default)]
    pub transfer_restrictions: Option<bool>,
    #[serde(default)]
    pub right_of_first_refusal: Option<bool>,
    #[serde(default)]
    pub company_address: Option<crate::domain::formation::content::Address>,
}

fn default_staged_jurisdiction() -> Option<Jurisdiction> {
    None
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PendingFormationResponse {
    pub entity_id: EntityId,
    pub legal_name: String,
    pub entity_type: EntityType,
    pub jurisdiction: Jurisdiction,
    pub formation_status: FormationStatus,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddFounderRequest {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub role: Option<MemberRole>,
    #[serde(default)]
    pub ownership_pct: Option<f64>,
    #[serde(default)]
    pub officer_title: Option<OfficerTitle>,
    #[serde(default)]
    pub is_incorporator: Option<bool>,
    #[serde(default)]
    pub address: Option<crate::domain::formation::content::Address>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct FinalizePendingFormationRequest {
    #[serde(default)]
    pub authorized_shares: Option<i64>,
    #[serde(default)]
    pub par_value: Option<String>,
    #[serde(default)]
    pub registered_agent_name: Option<String>,
    #[serde(default)]
    pub registered_agent_address: Option<String>,
    #[serde(default)]
    pub formation_date: Option<String>,
    #[serde(default)]
    pub fiscal_year_end: Option<String>,
    #[serde(default)]
    pub s_corp_election: Option<bool>,
    #[serde(default)]
    pub transfer_restrictions: Option<bool>,
    #[serde(default)]
    pub right_of_first_refusal: Option<bool>,
    #[serde(default)]
    pub company_address: Option<crate::domain::formation::content::Address>,
    #[serde(default)]
    pub incorporator_name: Option<String>,
    #[serde(default)]
    pub incorporator_address: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AddFounderResponse {
    pub entity_id: EntityId,
    pub member_count: usize,
    pub members: Vec<FounderSummary>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct FounderSummary {
    pub name: String,
    pub email: Option<String>,
    pub role: Option<MemberRole>,
    pub ownership_pct: Option<f64>,
    pub address: Option<crate::domain::formation::content::Address>,
}

fn build_formation_gates_response(entity: &Entity, filing: &Filing) -> FormationGatesResponse {
    FormationGatesResponse {
        entity_id: entity.entity_id(),
        filing_submission_blockers: filing.submission_blockers(),
        requires_natural_person_attestation: filing.requires_natural_person_attestation(),
        designated_attestor_name: filing.designated_attestor_name().to_owned(),
        designated_attestor_email: filing.designated_attestor_email().map(ToOwned::to_owned),
        designated_attestor_role: filing.designated_attestor_role().to_owned(),
        attestation_recorded: filing.attestation().is_some(),
        requires_registered_agent_consent_evidence: filing
            .requires_registered_agent_consent_evidence(),
        registered_agent_consent_evidence_count: filing.registered_agent_consent_evidence().len(),
        service_agreement_required_for_tier1_autonomy: matches!(
            entity.formation_status(),
            FormationStatus::Active
        ),
        service_agreement_executed: entity.service_agreement_executed(),
        service_agreement_executed_at: entity
            .service_agreement_executed_at()
            .map(|ts| ts.to_rfc3339()),
        service_agreement_contract_id: entity.service_agreement_contract_id(),
        service_agreement_document_id: entity.service_agreement_document_id(),
        service_agreement_notes: entity.service_agreement_notes().map(ToOwned::to_owned),
    }
}

/// Open an entity store, mapping git errors to formation errors.
fn open_formation_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<
    crate::store::entity_store::EntityStore<'a>,
    crate::domain::formation::error::FormationError,
> {
    crate::store::entity_store::EntityStore::open(layout, workspace_id, entity_id).map_err(|e| {
        match e {
            crate::git::error::GitStorageError::RepoNotFound(_) => {
                crate::domain::formation::error::FormationError::EntityNotFound(entity_id)
            }
            other => crate::domain::formation::error::FormationError::Validation(other.to_string()),
        }
    })
}

fn resolve_document_entity_id(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    requested_entity_id: EntityId,
    allowed_entity_ids: Option<&[EntityId]>,
    document_id: DocumentId,
) -> Result<EntityId, AppError> {
    resolve_document_entity_id_inner(
        layout,
        workspace_id,
        requested_entity_id,
        allowed_entity_ids,
        document_id,
        false,
    )
}

fn resolve_document_entity_id_with_fallback(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    requested_entity_id: EntityId,
    allowed_entity_ids: Option<&[EntityId]>,
    document_id: DocumentId,
) -> Result<EntityId, AppError> {
    resolve_document_entity_id_inner(
        layout,
        workspace_id,
        requested_entity_id,
        allowed_entity_ids,
        document_id,
        true,
    )
}

fn resolve_document_entity_id_inner(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    requested_entity_id: EntityId,
    allowed_entity_ids: Option<&[EntityId]>,
    document_id: DocumentId,
    fallback_across_entities: bool,
) -> Result<EntityId, AppError> {
    if let Some(ids) = allowed_entity_ids {
        if !ids.contains(&requested_entity_id) {
            return Err(AppError::NotFound(format!(
                "document {} not found",
                document_id
            )));
        }
    }

    let has_document = EntityStore::open(layout, workspace_id, requested_entity_id)
        .ok()
        .and_then(|store| store.read_document("main", document_id).ok())
        .is_some();

    if has_document {
        return Ok(requested_entity_id);
    }

    if fallback_across_entities {
        // Scan all entities in the workspace for the document.
        for entity_id in layout.list_entity_ids(workspace_id) {
            if entity_id == requested_entity_id {
                continue;
            }
            if let Some(ids) = allowed_entity_ids {
                if !ids.contains(&entity_id) {
                    continue;
                }
            }
            let found = EntityStore::open(layout, workspace_id, entity_id)
                .ok()
                .and_then(|store| store.read_document("main", document_id).ok())
                .is_some();
            if found {
                return Ok(entity_id);
            }
        }
    }

    Err(AppError::NotFound(format!(
        "document {} not found",
        document_id
    )))
}

fn parse_formation_date(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, AppError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(raw) {
        return Ok(Some(timestamp.with_timezone(&Utc)));
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| {
        AppError::BadRequest("formation_date must be RFC3339 or YYYY-MM-DD".to_owned())
    })?;
    Ok(Some(
        date.and_hms_opt(0, 0, 0)
            .expect("midnight is a valid timestamp")
            .and_utc(),
    ))
}

fn parse_fiscal_year_end(raw: Option<&str>) -> Result<Option<FiscalYearEnd>, AppError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let (month, day) = raw
        .split_once('-')
        .ok_or_else(|| AppError::BadRequest("fiscal_year_end must use MM-DD format".to_owned()))?;
    let month = month
        .parse::<u32>()
        .map_err(|_| AppError::BadRequest("fiscal_year_end month must be numeric".to_owned()))?;
    let day = day
        .parse::<u32>()
        .map_err(|_| AppError::BadRequest("fiscal_year_end day must be numeric".to_owned()))?;
    NaiveDate::from_ymd_opt(2024, month, day).ok_or_else(|| {
        AppError::BadRequest("fiscal_year_end must be a real calendar date".to_owned())
    })?;
    Ok(Some(FiscalYearEnd { month, day }))
}

fn request_company_address(
    address: Option<crate::domain::formation::content::Address>,
) -> Option<CompanyAddress> {
    address.map(|address| CompanyAddress {
        street: match address.street2 {
            Some(street2) if !street2.trim().is_empty() => {
                format!("{}, {}", address.street, street2)
            }
            _ => address.street,
        },
        city: address.city,
        county: None,
        state: address.state,
        zip: address.zip,
    })
}

fn cleaned_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn build_profile_overrides_from_fields(
    formation_date: Option<&str>,
    fiscal_year_end: Option<&str>,
    s_corp_election: Option<bool>,
    transfer_restrictions: Option<bool>,
    right_of_first_refusal: Option<bool>,
    company_address: Option<crate::domain::formation::content::Address>,
) -> Result<service::FormationProfileOverrides, AppError> {
    Ok(service::FormationProfileOverrides {
        formation_date: parse_formation_date(formation_date)?,
        fiscal_year_end: parse_fiscal_year_end(fiscal_year_end)?,
        document_options: Some(DocumentOptions {
            dating_format: "blank_line".to_owned(),
            transfer_restrictions: transfer_restrictions.unwrap_or(true),
            right_of_first_refusal: right_of_first_refusal.unwrap_or(true),
            s_corp_election: s_corp_election.unwrap_or(false),
        }),
        company_address: request_company_address(company_address),
    })
}

fn is_core_governance_document(doc: &Document) -> bool {
    matches!(
        doc.document_type(),
        DocumentType::ArticlesOfIncorporation
            | DocumentType::ArticlesOfOrganization
            | DocumentType::Bylaws
            | DocumentType::IncorporatorAction
            | DocumentType::InitialBoardConsent
            | DocumentType::OperatingAgreement
            | DocumentType::InitialWrittenConsent
    )
}

// ── Handlers ────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/formations",
    tag = "formation",
    request_body = CreateFormationRequest,
    responses(
        (status = 200, description = "Formation created", body = FormationResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_formation(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Json(req): Json<CreateFormationRequest>,
) -> Result<Json<FormationResponse>, AppError> {
    if req.members.is_empty() {
        return Err(AppError::BadRequest(
            "at least one member is required".to_owned(),
        ));
    }
    let workspace_id = auth.workspace_id();
    let profile_overrides = build_profile_overrides_from_fields(
        req.formation_date.as_deref(),
        req.fiscal_year_end.as_deref(),
        req.s_corp_election,
        req.transfer_restrictions,
        req.right_of_first_refusal,
        req.company_address.clone(),
    )?;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let legal_name = req.legal_name;
        let jurisdiction = req.jurisdiction;
        let members = req.members;
        let entity_type = req.entity_type;
        let ra_name = cleaned_optional_string(req.registered_agent_name);
        let ra_addr = cleaned_optional_string(req.registered_agent_address);
        let shares = req.authorized_shares;
        let par_value = req.par_value;
        let profile_overrides = profile_overrides.clone();
        move || {
            if workspace_has_legal_name(&layout, workspace_id, &legal_name, None).map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(format!("{e:?}"))
            })? {
                return Err(crate::domain::formation::error::FormationError::Validation(
                    format!("entity legal name already exists in workspace: {legal_name}"),
                ));
            }
            service::create_entity_with_profile_overrides(
                &layout,
                workspace_id,
                legal_name,
                entity_type,
                jurisdiction,
                ra_name,
                ra_addr,
                &members,
                shares,
                par_value.as_deref(),
                profile_overrides,
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let formation_status = result.entity.formation_status();
    let next_action = service::next_formation_action(formation_status).map(String::from);
    let entity_id = result.entity.entity_id();

    Ok(Json(FormationResponse {
        formation_id: entity_id,
        entity_id,
        formation_status,
        document_ids: result.document_ids,
        next_action,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/formations/with-cap-table",
    tag = "formation",
    request_body = CreateFormationRequest,
    responses(
        (status = 200, description = "Formation created with cap table", body = FormationWithCapTableResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_formation_with_cap_table(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Json(req): Json<CreateFormationRequest>,
) -> Result<Json<FormationWithCapTableResponse>, AppError> {
    if req.members.is_empty() {
        return Err(AppError::BadRequest(
            "at least one member is required".to_owned(),
        ));
    }
    let workspace_id = auth.workspace_id();
    let profile_overrides = build_profile_overrides_from_fields(
        req.formation_date.as_deref(),
        req.fiscal_year_end.as_deref(),
        req.s_corp_election,
        req.transfer_restrictions,
        req.right_of_first_refusal,
        req.company_address.clone(),
    )?;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let legal_name = req.legal_name;
        let jurisdiction = req.jurisdiction;
        let members = req.members;
        let entity_type = req.entity_type;
        let ra_name = cleaned_optional_string(req.registered_agent_name);
        let ra_addr = cleaned_optional_string(req.registered_agent_address);
        let shares = req.authorized_shares;
        let par_value = req.par_value;
        let profile_overrides = profile_overrides.clone();
        move || {
            if workspace_has_legal_name(&layout, workspace_id, &legal_name, None).map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(format!("{e:?}"))
            })? {
                return Err(crate::domain::formation::error::FormationError::Validation(
                    format!("entity legal name already exists in workspace: {legal_name}"),
                ));
            }
            // Step 1: Create the entity (formation documents, filing, tax profile)
            let formation = service::create_entity_with_profile_overrides(
                &layout,
                workspace_id,
                legal_name.clone(),
                entity_type,
                jurisdiction,
                ra_name,
                ra_addr,
                &members,
                shares,
                par_value.as_deref(),
                profile_overrides,
            )?;

            // Step 2: Set up the cap table (contacts, legal entity, instrument, holders, positions)
            let cap_table = service::setup_cap_table(
                &layout,
                workspace_id,
                formation.entity.entity_id(),
                entity_type,
                &legal_name,
                &members,
                shares,
                par_value.as_deref(),
            )?;

            Ok::<_, crate::domain::formation::error::FormationError>((formation, cap_table))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let (formation, cap_table) = result;
    let formation_status = formation.entity.formation_status();
    let next_action = service::next_formation_action(formation_status).map(String::from);
    let entity_id = formation.entity.entity_id();

    Ok(Json(FormationWithCapTableResponse {
        formation_id: entity_id,
        entity_id,
        formation_status,
        document_ids: formation.document_ids,
        next_action,
        legal_entity_id: Some(cap_table.legal_entity_id),
        instrument_id: Some(cap_table.instrument_id),
        holders: cap_table.holders,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/formations/{entity_id}",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Formation status", body = FormationStatusResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn get_formation(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        formation_date: entity.formation_date().map(|d| d.to_rfc3339()),
        next_action,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/formations/{entity_id}/documents",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of formation documents", body = Vec<DocumentSummary>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_documents(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<DocumentSummary>>, AppError> {
    let workspace_id = auth.workspace_id();

    let docs = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            let doc_ids = store.list_document_ids("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            let mut documents = Vec::new();
            for id in doc_ids {
                let doc = store.read_document("main", id).map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(e.to_string())
                })?;
                documents.push(doc);
            }
            Ok::<_, crate::domain::formation::error::FormationError>(documents)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let summaries = docs
        .into_iter()
        .map(|doc| DocumentSummary {
            document_id: doc.document_id(),
            document_type: doc.document_type(),
            title: doc.title().to_owned(),
            status: doc.status(),
            signature_count: doc.signatures().len(),
            created_at: doc.created_at().to_rfc3339(),
        })
        .collect();

    Ok(Json(summaries))
}

#[utoipa::path(
    get,
    path = "/v1/documents/{document_id}",
    tag = "formation",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Document details", body = DocumentResponse),
        (status = 404, description = "Document not found"),
    ),
)]
async fn get_document(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<DocumentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            store
                .read_document("main", document_id)
                .map_err(|e| match e {
                    crate::git::error::GitStorageError::NotFound(_) => {
                        crate::domain::formation::error::FormationError::DocumentNotFound(
                            document_id,
                        )
                    }
                    other => crate::domain::formation::error::FormationError::Validation(
                        other.to_string(),
                    ),
                })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let signatures = doc
        .signatures()
        .iter()
        .map(|sig| SignatureSummary {
            signature_id: sig.signature_id(),
            signer_name: sig.signer_name().to_owned(),
            signer_role: sig.signer_role().to_owned(),
            signed_at: sig.signed_at().to_rfc3339(),
        })
        .collect();

    Ok(Json(DocumentResponse {
        document_id: doc.document_id(),
        entity_id: doc.entity_id(),
        document_type: doc.document_type(),
        title: doc.title().to_owned(),
        status: doc.status(),
        content: doc.content().clone(),
        content_hash: doc.content_hash().to_owned(),
        version: doc.version(),
        signatures,
        created_at: doc.created_at().to_rfc3339(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/documents/{document_id}/sign",
    tag = "formation",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    request_body = SignDocumentRequest,
    responses(
        (status = 200, description = "Document signed", body = SignDocumentResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Document not found"),
    ),
)]
async fn sign_document(
    RequireFormationSign(auth): RequireFormationSign,
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::EntityIdQuery>,
    Json(req): Json<SignDocumentRequest>,
) -> Result<Json<SignDocumentResponse>, AppError> {
    if req.signer_name.is_empty() || req.signer_name.len() > 256 {
        return Err(AppError::BadRequest(
            "signer_name must be between 1 and 256 characters".to_owned(),
        ));
    }
    if req.signer_email.is_empty() || !req.signer_email.contains('@') {
        return Err(AppError::BadRequest(
            "signer_email must be a valid email address".to_owned(),
        ));
    }
    if req.signature_text.is_empty() {
        return Err(AppError::BadRequest(
            "signature_text is required".to_owned(),
        ));
    }
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            let mut doc = store
                .read_document("main", document_id)
                .map_err(|e| match e {
                    crate::git::error::GitStorageError::NotFound(_) => {
                        crate::domain::formation::error::FormationError::DocumentNotFound(
                            document_id,
                        )
                    }
                    other => crate::domain::formation::error::FormationError::Validation(
                        other.to_string(),
                    ),
                })?;

            let sig_request = SignatureRequest {
                signer_name: req.signer_name,
                signer_role: req.signer_role,
                signer_email: req.signer_email,
                signature_text: req.signature_text,
                consent_text: req.consent_text,
                signature_svg: req.signature_svg,
                ip_address: None,
            };

            doc.sign(sig_request)?;

            store
                .write_document("main", &doc, &format!("Sign document {document_id}"))
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(doc)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    // The last signature is the one we just added.
    let last_sig = doc
        .signatures()
        .last()
        .ok_or_else(|| AppError::Internal("signature was added but not found".to_string()))?;

    Ok(Json(SignDocumentResponse {
        signature_id: last_sig.signature_id(),
        document_id: doc.document_id(),
        document_status: doc.status(),
        signed_at: last_sig.signed_at().to_rfc3339(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/mark-documents-signed",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Documents marked as signed", body = FormationStatusResponse),
        (status = 400, description = "Not all documents are fully signed"),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn mark_documents_signed(
    RequireFormationSign(auth): RequireFormationSign,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            let mut entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            let doc_ids = store.list_document_ids("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;
            if doc_ids.is_empty() {
                return Err(crate::domain::formation::error::FormationError::Validation(
                    "no formation documents found".to_owned(),
                ));
            }

            for doc_id in doc_ids {
                let doc = store.read_document("main", doc_id).map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(e.to_string())
                })?;
                if !doc.is_fully_signed() {
                    return Err(crate::domain::formation::error::FormationError::Validation(
                        format!("document {doc_id} is not fully signed"),
                    ));
                }
            }

            entity.advance_status(FormationStatus::DocumentsSigned)?;
            store
                .write_entity("main", &entity, "Mark formation documents as signed")
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        formation_date: entity.formation_date().map(|d| d.to_rfc3339()),
        next_action,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/submit-filing",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Filing submitted", body = FormationStatusResponse),
        (status = 400, description = "Filing submission blocked"),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn submit_filing(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let mut entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;
            let filing = store.read_filing("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;
            let blockers = filing.submission_blockers();
            if !blockers.is_empty() {
                return Err(crate::domain::formation::error::FormationError::Validation(
                    format!("filing submission blocked: {}", blockers.join("; ")),
                ));
            }

            entity.advance_status(FormationStatus::FilingSubmitted)?;
            store
                .write_entity("main", &entity, "Submit formation filing")
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        formation_date: entity.formation_date().map(|d| d.to_rfc3339()),
        next_action,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/filing-attestation",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = FilingAttestationRequest,
    responses(
        (status = 200, description = "Filing attestation recorded", body = FormationGatesResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn record_filing_attestation(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<FilingAttestationRequest>,
) -> Result<Json<FormationGatesResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;
            let mut filing = store.read_filing("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            filing.record_attestation(
                req.signer_name,
                req.signer_role,
                req.signer_email,
                req.consent_text,
                req.notes,
            )?;
            store
                .write_json(
                    "main",
                    "formation/filing.json",
                    &filing,
                    "Record filing attestation",
                )
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(
                build_formation_gates_response(&entity, &filing),
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/registered-agent-consent-evidence",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = RegisteredAgentConsentEvidenceRequest,
    responses(
        (status = 200, description = "Registered agent consent evidence added", body = FormationGatesResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn add_registered_agent_consent_evidence(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<RegisteredAgentConsentEvidenceRequest>,
) -> Result<Json<FormationGatesResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;
            let mut filing = store.read_filing("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            filing.add_registered_agent_evidence(req.evidence_uri, req.evidence_type, req.notes)?;
            store
                .write_json(
                    "main",
                    "formation/filing.json",
                    &filing,
                    "Record registered-agent consent evidence",
                )
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(
                build_formation_gates_response(&entity, &filing),
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/service-agreement/execute",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = ExecuteServiceAgreementRequest,
    responses(
        (status = 200, description = "Service agreement executed", body = FormationGatesResponse),
        (status = 404, description = "Entity or contract not found"),
        (status = 422, description = "Validation error"),
    ),
)]
async fn execute_service_agreement(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<ExecuteServiceAgreementRequest>,
) -> Result<Json<FormationGatesResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || -> Result<FormationGatesResponse, AppError> {
            let store =
                open_formation_store(&layout, workspace_id, entity_id).map_err(AppError::from)?;
            let mut entity = store
                .read_entity("main")
                .map_err(|e| AppError::UnprocessableEntity(format!("validation error: {e}")))?;
            let filing = store
                .read_filing("main")
                .map_err(|e| AppError::UnprocessableEntity(format!("validation error: {e}")))?;
            let contract_id = req.contract_id;
            let document_id = req.document_id;
            let notes = req.notes;

            if let Some(contract_id) = contract_id {
                let contract = store.read::<Contract>("main", contract_id).map_err(|_| {
                    AppError::NotFound(format!("contract {} not found", contract_id))
                })?;
                if contract.entity_id() != entity_id {
                    return Err(AppError::UnprocessableEntity(format!(
                        "contract {} belongs to a different entity",
                        contract_id
                    )));
                }
            }
            if let Some(document_id) = document_id {
                let document = store.read_document("main", document_id).map_err(|_| {
                    AppError::NotFound(format!("document {} not found", document_id))
                })?;
                if document.entity_id() != entity_id {
                    return Err(AppError::UnprocessableEntity(format!(
                        "document {} belongs to a different entity",
                        document_id
                    )));
                }
            }

            entity.record_service_agreement_execution(contract_id, document_id, notes);
            store
                .write_entity("main", &entity, "Record service agreement execution")
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok(build_formation_gates_response(&entity, &filing))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/v1/formations/{entity_id}/gates",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Formation gates status", body = FormationGatesResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn get_formation_gates(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<FormationGatesResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;
            let filing = store.read_filing("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;
            Ok::<_, crate::domain::formation::error::FormationError>(
                build_formation_gates_response(&entity, &filing),
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/filing-confirmation",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = ConfirmFilingRequest,
    responses(
        (status = 200, description = "Filing confirmed", body = FormationStatusResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn confirm_filing(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<ConfirmFilingRequest>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            let mut entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            entity.advance_status(FormationStatus::Filed)?;

            store
                .write_entity("main", &entity, "Confirm state filing")
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            // Also update the filing record if it exists.
            if let Ok(mut filing) = store.read_filing("main") {
                filing.confirm(req.external_filing_id, req.receipt_reference);
                store
                    .commit(
                        "main",
                        "Update filing record",
                        vec![
                            crate::git::commit::FileWrite::json("formation/filing.json", &filing)
                                .map_err(|e| {
                                crate::domain::formation::error::FormationError::Validation(
                                    e.to_string(),
                                )
                            })?,
                        ],
                    )
                    .map_err(|e| {
                        crate::domain::formation::error::FormationError::Validation(format!(
                            "failed to update filing: {e}"
                        ))
                    })?;
            }

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        formation_date: entity.formation_date().map(|d| d.to_rfc3339()),
        next_action,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/apply-ein",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "EIN application submitted", body = FormationStatusResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn apply_ein(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let mut entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            entity.advance_status(FormationStatus::EinApplied)?;
            store
                .write_entity("main", &entity, "Submit EIN application")
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        formation_date: entity.formation_date().map(|d| d.to_rfc3339()),
        next_action,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/ein-confirmation",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = ConfirmEinRequest,
    responses(
        (status = 200, description = "EIN confirmed", body = FormationStatusResponse),
        (status = 400, description = "Invalid EIN format"),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn confirm_ein(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<ConfirmEinRequest>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    // Validate EIN format: XX-XXXXXXX (2 digits, hyphen, 7 digits)
    let ein_bytes = req.ein.as_bytes();
    let valid_ein = ein_bytes.len() == 10
        && ein_bytes[0].is_ascii_digit()
        && ein_bytes[1].is_ascii_digit()
        && ein_bytes[2] == b'-'
        && ein_bytes[3..].iter().all(|b| b.is_ascii_digit());
    if !valid_ein {
        return Err(AppError::BadRequest(
            "EIN must be in format XX-XXXXXXX (2 digits, hyphen, 7 digits)".to_owned(),
        ));
    }

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let ein = req.ein;
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            let mut entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            entity.advance_status(FormationStatus::Active)?;

            store
                .write_entity("main", &entity, &format!("Confirm EIN: {ein}"))
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            // Also update the tax profile if it exists.
            if let Ok(mut tax) = store.read_tax_profile("main") {
                tax.confirm_ein(ein);
                store
                    .commit(
                        "main",
                        "Update tax profile with EIN",
                        vec![
                            crate::git::commit::FileWrite::json("tax/profile.json", &tax).map_err(
                                |e| {
                                    crate::domain::formation::error::FormationError::Validation(
                                        e.to_string(),
                                    )
                                },
                            )?,
                        ],
                    )
                    .map_err(|e| {
                        crate::domain::formation::error::FormationError::Validation(format!(
                            "failed to update tax profile: {e}"
                        ))
                    })?;
            }

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        formation_date: entity.formation_date().map(|d| d.to_rfc3339()),
        next_action,
    }))
}

// ── Contract / Document management ──────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GenerateContractRequest {
    pub entity_id: EntityId,
    pub template_type: ContractTemplateType,
    pub counterparty_name: String,
    #[schema(value_type = String)]
    pub effective_date: chrono::NaiveDate,
    #[serde(default = "default_params")]
    #[schema(value_type = Object)]
    pub parameters: serde_json::Value,
}

fn default_params() -> serde_json::Value {
    serde_json::json!({})
}

fn normalize_legal_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn workspace_has_legal_name(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    legal_name: &str,
    skip_entity_id: Option<EntityId>,
) -> Result<bool, AppError> {
    let normalized = normalize_legal_name(legal_name);
    for entity_id in layout.list_entity_ids(workspace_id) {
        if skip_entity_id.is_some_and(|skip| skip == entity_id) {
            continue;
        }
        if let Ok(store) = EntityStore::open(layout, workspace_id, entity_id)
            && let Ok(entity) = store.read_entity("main")
            && normalize_legal_name(entity.legal_name()) == normalized
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ContractResponse {
    pub contract_id: ContractId,
    pub entity_id: EntityId,
    pub template_type: ContractTemplateType,
    pub counterparty_name: String,
    #[schema(value_type = String)]
    pub effective_date: chrono::NaiveDate,
    pub status: ContractStatus,
    pub document_id: DocumentId,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SigningLinkResponse {
    pub document_id: DocumentId,
    pub signing_url: String,
    pub token: String,
}

/// Persisted signing token that maps a hashed token to a document.
/// The raw token is never stored — only its SHA-256 hash is persisted
/// (similar to Django CSRF verification). The raw token is returned
/// to the user once and used as a bearer credential for the signing UI.
#[derive(Serialize, Deserialize)]
struct SigningToken {
    /// SHA-256 hash of the raw token (hex-encoded). The raw token is
    /// never stored at rest.
    token_hash: String,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    document_id: DocumentId,
    expires_at: String,
}

/// Hash a raw signing token to its storage key using SHA-256.
fn hash_signing_token(raw_token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Response for the public resolve endpoint.
#[derive(Serialize, utoipa::ToSchema)]
pub struct SigningResolveResponse {
    pub document_id: DocumentId,
    pub entity_id: EntityId,
    pub document_title: String,
    pub document_status: String,
    pub signatures: Vec<SignatureSummary>,
    /// Public PDF preview URL for the signing page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_url: Option<String>,
    /// Plain-text preview fallback when a PDF is unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_text: Option<String>,
    /// Contract details when the document references a contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract: Option<SigningContractDetails>,
    /// Entity legal name for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
}

/// Contract details included in signing resolve response.
#[derive(Serialize, utoipa::ToSchema)]
pub struct SigningContractDetails {
    pub template_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_label: Option<String>,
    pub counterparty_name: String,
    pub effective_date: String,
    pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_text: Option<String>,
}

/// Query param for signing token.
#[derive(Deserialize, utoipa::IntoParams)]
pub struct SigningTokenQuery {
    pub token: String,
}

fn fmt_bool(v: bool) -> &'static str {
    if v { "Yes" } else { "No" }
}

fn fmt_param_label(key: &str) -> String {
    key.replace('_', " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn template_label(template_type: ContractTemplateType, parameters: &Value) -> String {
    if matches!(
        template_type,
        ContractTemplateType::SafeAgreement | ContractTemplateType::Custom
    ) && parameters
        .get("contract_type")
        .and_then(Value::as_str)
        .is_some_and(|s| s.eq_ignore_ascii_case("safe agreement"))
    {
        return "SAFE Agreement".to_owned();
    }
    match template_type {
        ContractTemplateType::ConsultingAgreement => "Consulting Agreement".to_owned(),
        ContractTemplateType::EmploymentOffer => "Employment Offer Letter".to_owned(),
        ContractTemplateType::ContractorAgreement => {
            "Independent Contractor Services Agreement".to_owned()
        }
        ContractTemplateType::Nda => "Mutual Non-Disclosure Agreement".to_owned(),
        ContractTemplateType::SafeAgreement => "SAFE Agreement".to_owned(),
        ContractTemplateType::Custom => "Custom Agreement".to_owned(),
    }
}

fn param_string<'a>(parameters: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| parameters.get(*key).and_then(Value::as_str))
}

fn param_bool(parameters: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| parameters.get(*key).and_then(Value::as_bool))
}

fn param_rendered(parameters: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        parameters.get(*key).and_then(|value| match value {
            Value::String(v) => Some(v.clone()),
            Value::Bool(v) => Some(if *v { "Yes" } else { "No" }.to_owned()),
            Value::Number(v) => Some(v.to_string()),
            _ => None,
        })
    })
}

fn signature_requirement(role: &str, signer_name: &str) -> Value {
    serde_json::json!({
        "role": role,
        "signer_name": signer_name,
        "required": true
    })
}

fn company_legal_name(store: &EntityStore<'_>) -> String {
    store
        .read_entity("main")
        .ok()
        .map(|entity| entity.legal_name().to_owned())
        .unwrap_or_else(|| "Company".to_owned())
}

fn format_profile_company_address(address: &CompanyAddress) -> String {
    let mut parts = vec![address.street.clone(), address.city.clone()];
    if let Some(county) = &address.county {
        parts.push(county.clone());
    }
    parts.push(address.state.clone());
    parts.push(address.zip.clone());
    parts.join(", ")
}

fn company_governing_law(store: &EntityStore<'_>) -> String {
    store
        .read_entity("main")
        .ok()
        .map(|entity| entity.jurisdiction().to_string())
        .unwrap_or_else(|| "Delaware".to_owned())
}

fn company_notice_address(store: &EntityStore<'_>) -> Option<String> {
    store
        .read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH)
        .ok()
        .and_then(|profile| {
            profile
                .company_address()
                .map(format_profile_company_address)
                .or_else(|| profile.registered_agent_address().map(ToOwned::to_owned))
        })
        .or_else(|| {
            store
                .read_entity("main")
                .ok()
                .and_then(|entity| entity.registered_agent_address().map(ToOwned::to_owned))
        })
}

fn required_param_rendered(
    parameters: &Value,
    keys: &[&str],
    label: &str,
) -> Result<String, String> {
    param_rendered(parameters, keys)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{label} is required for a production-grade document"))
}

fn validate_governance_document_content(
    store: &EntityStore<'_>,
    governance_tag: &str,
    content: &Value,
) -> Result<(), AppError> {
    let ast = doc_ast::default_doc_ast();
    let doc_def = find_ast_document_definition(ast, governance_tag).ok_or_else(|| {
        AppError::NotFound(format!(
            "no AST document definition matches governance_tag '{}'",
            governance_tag
        ))
    })?;
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    let profile = store
        .read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH)
        .unwrap_or_else(|_| GovernanceProfile::default_for_entity(&entity));
    let rendered = doc_generator::render_document_from_ast_with_context(
        doc_def,
        ast,
        entity_type_key(entity.entity_type()),
        &profile,
        content,
    );
    let warnings = doc_generator::detect_placeholder_warnings_for_text(governance_tag, &rendered);
    if warnings.is_empty() {
        Ok(())
    } else {
        Err(AppError::UnprocessableEntity(format!(
            "document '{}' is incomplete for production use: {}",
            governance_tag,
            warnings.join("; ")
        )))
    }
}

fn app_error_detail(error: AppError) -> String {
    match error {
        AppError::BadRequest(msg)
        | AppError::Unauthorized(msg)
        | AppError::Forbidden(msg)
        | AppError::NotFound(msg)
        | AppError::Conflict(msg)
        | AppError::UnprocessableEntity(msg)
        | AppError::NotImplemented(msg)
        | AppError::ServiceUnavailable(msg)
        | AppError::Internal(msg) => msg,
        AppError::RateLimited {
            limit,
            window_seconds,
        } => format!("rate limited: limit={limit}, window_seconds={window_seconds}"),
    }
}

fn contract_document_binding(
    store: &EntityStore<'_>,
    contract: &Contract,
) -> Result<(DocumentType, Option<String>, Value), String> {
    let entity_legal_name = company_legal_name(store);
    let governing_law = company_governing_law(store);
    let company_notice_address = company_notice_address(store);
    let effective_date = contract.effective_date().to_string();
    let counterparty_name = contract.counterparty_name().to_owned();
    let parameters = contract.parameters();

    match contract.template_type() {
        ContractTemplateType::ConsultingAgreement => Ok((
            DocumentType::ConsultingAgreement,
            Some("consulting_agreement".to_owned()),
            serde_json::json!({
                "fields": {
                    "entity_legal_name": entity_legal_name,
                    "effective_date": effective_date,
                    "consultant_name": counterparty_name,
                    "services_description": param_rendered(parameters, &["services_description", "services", "scope_of_services", "scope"]).unwrap_or_else(|| "Strategic, business, and advisory consulting services requested by the Company from time to time.".to_owned()),
                    "start_date": param_rendered(parameters, &["start_date"]).unwrap_or_else(|| contract.effective_date().to_string()),
                    "term_description": param_rendered(parameters, &["term_description", "term", "term_length"]).unwrap_or_else(|| "The agreement continues until terminated in accordance with its terms.".to_owned()),
                    "compensation_terms": param_rendered(parameters, &["compensation_terms", "compensation", "fee", "retainer", "rate"]).unwrap_or_else(|| "Consultant will be compensated as set forth in an applicable statement of work, fee schedule, or invoice accepted by the Company.".to_owned()),
                    "expense_policy": param_rendered(parameters, &["expense_policy", "expenses", "expense_reimbursement"]).unwrap_or_else(|| "The Company will reimburse pre-approved, reasonable out-of-pocket business expenses supported by customary documentation.".to_owned()),
                    "payment_terms": param_rendered(parameters, &["payment_terms"]).unwrap_or_else(|| "Undisputed invoices are due within thirty (30) days after receipt.".to_owned()),
                    "termination_notice_days": param_rendered(parameters, &["termination_notice_days", "notice_days"]).unwrap_or_else(|| "14".to_owned()),
                    "governing_law": param_rendered(parameters, &["governing_law"]).unwrap_or_else(|| governing_law.clone())
                },
                "signature_requirements": [
                    signature_requirement("Company", &entity_legal_name),
                    signature_requirement("Consultant", &counterparty_name)
                ]
            }),
        )),
        ContractTemplateType::EmploymentOffer => Ok((
            DocumentType::EmploymentOfferLetter,
            Some("employment_offer_letter".to_owned()),
            serde_json::json!({
                "fields": {
                    "entity_legal_name": entity_legal_name,
                    "effective_date": effective_date,
                    "candidate_name": counterparty_name,
                    "position_title": param_rendered(parameters, &["position_title", "job_title", "title"]).unwrap_or_else(|| "Employee".to_owned()),
                    "reporting_manager": param_rendered(parameters, &["reporting_manager", "reports_to", "manager"]).unwrap_or_else(|| "the Chief Executive Officer or such other manager as the Company may designate".to_owned()),
                    "work_location": param_rendered(parameters, &["work_location", "location"]).unwrap_or_else(|| "the Company's principal office, remotely, or such other location as reasonably required".to_owned()),
                    "start_date": param_rendered(parameters, &["start_date"]).unwrap_or_else(|| contract.effective_date().to_string()),
                    "classification": param_rendered(parameters, &["classification", "exempt_status"]).unwrap_or_else(|| "at-will exempt employee".to_owned()),
                    "base_salary": required_param_rendered(parameters, &["base_salary", "annual_salary", "salary"], "base_salary")?,
                    "bonus_terms": param_rendered(parameters, &["bonus_terms", "bonus_target", "target_bonus"]).unwrap_or_else(|| "Eligibility for any bonus program will be determined under Company plans adopted from time to time and is not guaranteed.".to_owned()),
                    "equity_terms": param_rendered(parameters, &["equity_terms", "equity_award", "equity", "option_grant"]).unwrap_or_else(|| "Any equity award will be subject to separate board approval and definitive equity documents.".to_owned()),
                    "benefits_summary": param_rendered(parameters, &["benefits_summary", "benefits"]).unwrap_or_else(|| "Participation in employee benefit plans made available to similarly situated employees, subject to plan terms and Company policies.".to_owned()),
                    "offer_expiration_date": param_rendered(parameters, &["offer_expiration_date", "expiration_date"]).unwrap_or_else(|| contract.effective_date().to_string()),
                    "governing_law": param_rendered(parameters, &["governing_law"]).unwrap_or_else(|| governing_law.clone())
                },
                "signature_requirements": [
                    signature_requirement("Company", &entity_legal_name),
                    signature_requirement("Candidate", &counterparty_name)
                ]
            }),
        )),
        ContractTemplateType::ContractorAgreement => Ok((
            DocumentType::ContractorServicesAgreement,
            Some("contractor_services_agreement".to_owned()),
            serde_json::json!({
                "fields": {
                    "entity_legal_name": entity_legal_name,
                    "effective_date": effective_date,
                    "contractor_name": counterparty_name,
                    "services_description": param_rendered(parameters, &["services_description", "services", "scope_of_services", "scope"]).unwrap_or_else(|| "Independent contractor services requested by the Company from time to time.".to_owned()),
                    "start_date": param_rendered(parameters, &["start_date"]).unwrap_or_else(|| contract.effective_date().to_string()),
                    "term_description": param_rendered(parameters, &["term_description", "term", "term_length"]).unwrap_or_else(|| "The engagement continues until completed or earlier terminated in accordance with this agreement.".to_owned()),
                    "compensation_terms": param_rendered(parameters, &["compensation_terms", "compensation", "fee", "retainer", "rate"]).unwrap_or_else(|| "Contractor will be paid the fees set forth in the applicable scope, schedule, or invoice accepted by the Company.".to_owned()),
                    "expense_policy": param_rendered(parameters, &["expense_policy", "expenses", "expense_reimbursement"]).unwrap_or_else(|| "The Company will reimburse only reasonable pre-approved expenses supported by customary documentation.".to_owned()),
                    "payment_terms": param_rendered(parameters, &["payment_terms"]).unwrap_or_else(|| "Undisputed invoices are due within thirty (30) days after receipt.".to_owned()),
                    "termination_notice_days": param_rendered(parameters, &["termination_notice_days", "notice_days"]).unwrap_or_else(|| "14".to_owned()),
                    "governing_law": param_rendered(parameters, &["governing_law"]).unwrap_or_else(|| governing_law.clone())
                },
                "signature_requirements": [
                    signature_requirement("Company", &entity_legal_name),
                    signature_requirement("Contractor", &counterparty_name)
                ]
            }),
        )),
        ContractTemplateType::Nda => Ok((
            DocumentType::MutualNondisclosureAgreement,
            Some("mutual_nondisclosure_agreement".to_owned()),
            serde_json::json!({
                "fields": {
                    "entity_legal_name": entity_legal_name,
                    "effective_date": effective_date,
                    "counterparty_name": counterparty_name,
                    "purpose": param_rendered(parameters, &["purpose"]).unwrap_or_else(|| "evaluating and discussing a potential business relationship".to_owned()),
                    "term_years": param_rendered(parameters, &["term_years", "term"]).unwrap_or_else(|| "2".to_owned()),
                    "confidentiality_period_years": param_rendered(parameters, &["confidentiality_period_years", "survival_years"]).unwrap_or_else(|| "3".to_owned()),
                    "return_materials_days": param_rendered(parameters, &["return_materials_days"]).unwrap_or_else(|| "10".to_owned()),
                    "governing_law": param_rendered(parameters, &["governing_law"]).unwrap_or_else(|| governing_law.clone())
                },
                "signature_requirements": [
                    signature_requirement("Company", &entity_legal_name),
                    signature_requirement("Counterparty", &counterparty_name)
                ]
            }),
        )),
        ContractTemplateType::SafeAgreement => Ok((
            DocumentType::SafeAgreement,
            Some("safe_agreement".to_owned()),
            serde_json::json!({
                "fields": {
                    "entity_legal_name": entity_legal_name,
                    "effective_date": effective_date,
                    "investor_name": counterparty_name,
                    "purchase_amount": required_param_rendered(parameters, &["investment_amount", "investment_amount_display"], "purchase_amount")?,
                    "safe_type": parameters.get("safe_type").and_then(Value::as_str).unwrap_or("post-money"),
                    "valuation_cap": required_param_rendered(parameters, &["valuation_cap", "valuation_cap_display"], "valuation_cap")?,
                    "discount_rate": parameters.get("discount_rate").and_then(Value::as_str).unwrap_or("None"),
                    "pro_rata_rights": parameters.get("pro_rata_rights").and_then(Value::as_bool).map(|v| if v { "Yes" } else { "No" }).unwrap_or("No"),
                    "governing_law": parameters.get("governing_law").and_then(Value::as_str).unwrap_or(&governing_law),
                    "company_notice_address": param_string(parameters, &["company_notice_address"])
                        .map(ToOwned::to_owned)
                        .or_else(|| company_notice_address.clone())
                        .ok_or_else(|| "company_notice_address is required for a production-grade SAFE".to_owned())?,
                    "investor_notice_address": required_param_rendered(parameters, &["investor_notice_address"], "investor_notice_address")?,
                },
                "signature_requirements": [
                    signature_requirement("Company", &entity_legal_name),
                    signature_requirement("Investor", &counterparty_name)
                ]
            }),
        )),
        ContractTemplateType::Custom => Ok((
            DocumentType::Contract,
            None,
            serde_json::json!({ "contract_id": contract.contract_id().to_string() }),
        )),
    }
}

fn is_safe_contract(contract: &Contract) -> bool {
    contract.template_type() == ContractTemplateType::SafeAgreement
        || contract.template_type() == ContractTemplateType::Custom
            && contract
                .parameters()
                .get("contract_type")
                .and_then(Value::as_str)
                .is_some_and(|s| s.eq_ignore_ascii_case("safe agreement"))
}

fn render_generic_contract_preview(contract: &Contract, entity_name: &str) -> String {
    let tpl = template_label(contract.template_type(), contract.parameters());
    let mut out = format!(
        "{tpl}\n\nThis {tpl} is entered into as of {}.\n\nBETWEEN:\n\n  (1) {} (\"Company\")\n\n  (2) {} (\"Counterparty\")\n\n",
        contract.effective_date(),
        entity_name,
        contract.counterparty_name()
    );

    if let Some(obj) = contract.parameters().as_object()
        && !obj.is_empty()
    {
        out.push_str("TERMS:\n\n");
        for (key, value) in obj {
            let rendered = match value {
                Value::Bool(v) => fmt_bool(*v).to_owned(),
                Value::String(v) => v.clone(),
                _ => value.to_string(),
            };
            out.push_str(&format!("  {}: {}\n", fmt_param_label(key), rendered));
        }
        out.push('\n');
    }

    out.push_str(
        "This agreement is subject to the terms and conditions stated below. By signing, each party agrees to be bound by it.\n",
    );
    out
}

fn render_safe_contract_text(contract: &Contract, entity_name: &str) -> String {
    let params = contract.parameters();
    let investment_amount = param_string(
        params,
        &[
            "investment_amount",
            "investment_amount_display",
            "principal_amount",
            "principal_amount_display",
        ],
    )
    .unwrap_or("the Purchase Amount");
    let valuation_cap = param_string(params, &["valuation_cap", "valuation_cap_display"])
        .unwrap_or("the Valuation Cap");
    let discount_rate = param_string(params, &["discount_rate", "discount"]);
    let safe_type = param_string(params, &["safe_type"]).unwrap_or("post-money");
    let pro_rata_rights = param_bool(params, &["pro_rata_rights"]).unwrap_or(false);
    let governing_law = param_string(params, &["governing_law"]).unwrap_or("Delaware");
    let company_notice = param_string(params, &["company_notice_address"]).unwrap_or(entity_name);
    let investor_notice =
        param_string(params, &["investor_notice_address"]).unwrap_or(contract.counterparty_name());
    let signed_year = contract.effective_date().year();

    let mut out = String::new();
    out.push_str("SAFE\n");
    out.push_str("(Simple Agreement for Future Equity)\n\n");
    out.push_str(&format!(
        "This SAFE is made as of {} by and between {} (the \"Company\") and {} (the \"Investor\"). The Investor pays {} to the Company on the date of this SAFE as the purchase amount.\n\n",
        contract.effective_date(),
        entity_name,
        contract.counterparty_name(),
        investment_amount
    ));
    out.push_str("1. Triggering Events and Conversion.\n");
    out.push_str(&format!(
        "Upon the closing of an Equity Financing, this SAFE will automatically convert into the class of capital stock sold in that financing or into a shadow series intended to preserve the economic terms of this SAFE. The conversion price will be determined using the most favorable price resulting from the valuation cap of {}",
        valuation_cap
    ));
    if let Some(discount_rate) = discount_rate {
        out.push_str(&format!(" and the discount rate of {}", discount_rate));
    }
    out.push_str(&format!(
        ", consistent with a {} SAFE structure. The definitive financing documents may include customary mechanics needed to implement that conversion.\n\n",
        safe_type
    ));
    out.push_str("2. Liquidity Event.\n");
    out.push_str("If a Change of Control, merger, asset sale, public listing, or other liquidity event occurs before conversion of this SAFE, the Investor will be entitled, immediately prior to closing, to receive either cash equal to the purchase amount or the amount payable as if this SAFE had converted into the number of shares implied by the valuation cap mechanics, whichever yields the greater economic result under the transaction documents.\n\n");
    out.push_str("3. Dissolution Event.\n");
    out.push_str("If the Company dissolves, winds up, or commences a general assignment for the benefit of creditors before this SAFE converts, the Investor will receive payment of the purchase amount, subject to the rights of creditors and any senior preferred stock liquidation preferences that are expressly senior to this SAFE.\n\n");
    out.push_str("4. Pro Rata Participation.\n");
    out.push_str(&format!(
        "Investor pro rata participation rights: {}. If granted, the Investor may participate in future equity financings on a pro rata basis pursuant to the terms of the applicable financing documents and any side letter issued by the Company.\n\n",
        fmt_bool(pro_rata_rights)
    ));
    out.push_str("5. Company Representations.\n");
    out.push_str("The Company represents that it is duly organized, validly existing, and has authority to enter into and perform this SAFE; that execution and delivery of this SAFE have been duly authorized; and that this SAFE constitutes a binding obligation of the Company, enforceable in accordance with its terms except as limited by bankruptcy, insolvency, reorganization, moratorium, and similar laws and by general principles of equity.\n\n");
    out.push_str("6. Investor Representations.\n");
    out.push_str("The Investor represents that it is acquiring this SAFE for investment for its own account and not with a view to distribution in violation of applicable securities laws; that it has sufficient knowledge and experience to evaluate the investment; and that it is able to bear the economic risk of a complete loss.\n\n");
    out.push_str("7. Nature of the Instrument.\n");
    out.push_str("This SAFE is not debt and bears no interest. It has no maturity date. Until conversion, the Investor has no voting rights, dividend rights, or rights as a stockholder of the Company, except as expressly stated in this SAFE or required by law.\n\n");
    out.push_str("8. Transfers; Amendments; Notices.\n");
    out.push_str("The Investor may not transfer this SAFE except to an affiliate, estate planning vehicle, or with the Company’s prior written consent, and any permitted transferee takes subject to this SAFE. Any amendment, waiver, or modification must be in writing and signed by the Company and either the Investor or the holders of a majority-in-interest of substantially similar SAFEs, as applicable. Notices to the Company may be sent to ");
    out.push_str(company_notice);
    out.push_str("; notices to the Investor may be sent to ");
    out.push_str(investor_notice);
    out.push_str(".\n\n");
    out.push_str(&format!(
        "9. Governing Law; Counterparts; Electronic Signatures.\nThis SAFE is governed by the laws of {} without regard to conflicts principles. It may be executed in counterparts, including by electronic signature, each of which is deemed an original and all of which together form one instrument.\n\n",
        governing_law
    ));
    out.push_str(&format!(
        "IN WITNESS WHEREOF, the parties have executed this SAFE as of {}.\n\nCOMPANY:\n{}\n\nBy: __________________________\nName: ________________________\nTitle: _________________________\nDate: _________________________\n\nINVESTOR:\n{}\n\nBy: __________________________\nName: ________________________\nTitle: _________________________\nDate: _________________________\n",
        signed_year,
        entity_name,
        contract.counterparty_name()
    ));
    out
}

fn render_signing_preview_text(contract: &Contract, entity_name: &str) -> String {
    if is_safe_contract(contract) {
        return render_safe_contract_text(contract, entity_name);
    }
    render_generic_contract_preview(contract, entity_name)
}

fn escape_typst_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '#' | '*' | '_' | '<' | '>' | '@' | '$' | '`' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn render_text_preview_pdf(title: &str, body_text: &str) -> Result<Vec<u8>, AppError> {
    let mut typst_source = String::from(
        "#set page(paper: \"us-letter\", margin: (top: 1in, bottom: 1in, left: 1in, right: 1in), numbering: \"1\")\n#set text(font: \"New Computer Modern\", size: 10.5pt)\n#set par(justify: true, leading: 0.7em)\n",
    );
    typst_source.push_str(&format!("= {}\n\n", escape_typst_text(title)));
    for para in body_text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rendered = trimmed
            .lines()
            .map(escape_typst_text)
            .collect::<Vec<_>>()
            .join(" \\\n");
        typst_source.push_str(&rendered);
        typst_source.push_str("\n\n");
    }
    typst_renderer::render_source_pdf(&typst_source)
        .map_err(|e| AppError::Internal(format!("PDF rendering failed: {e}")))
}

fn entity_type_key(entity_type: EntityType) -> doc_ast::EntityTypeKey {
    match entity_type {
        EntityType::CCorp => doc_ast::EntityTypeKey::Corporation,
        EntityType::Llc => doc_ast::EntityTypeKey::Llc,
    }
}

fn find_ast_document_definition<'a>(
    ast: &'a doc_ast::GovernanceDocAst,
    governance_tag: &str,
) -> Option<&'a doc_ast::DocumentDefinition> {
    ast.documents
        .iter()
        .find(|d| d.id == governance_tag || d.path.contains(governance_tag))
}

fn render_governance_document_markdown(
    store: &EntityStore<'_>,
    doc: &Document,
) -> Result<Option<String>, AppError> {
    let Some(governance_tag) = doc.governance_tag() else {
        return Ok(None);
    };

    let ast = doc_ast::default_doc_ast();
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    let profile: GovernanceProfile =
        match store.read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH) {
            Ok(p) => p,
            Err(_) => GovernanceProfile::default_for_entity(&entity),
        };
    let Some(doc_def) = find_ast_document_definition(ast, governance_tag) else {
        return Ok(None);
    };

    let rendered = doc_generator::render_document_from_ast_with_context(
        doc_def,
        ast,
        entity_type_key(entity.entity_type()),
        &profile,
        doc.content(),
    );
    let warnings = doc_generator::detect_placeholder_warnings_for_text(governance_tag, &rendered);
    if warnings.is_empty() {
        Ok(Some(rendered))
    } else {
        Err(AppError::UnprocessableEntity(format!(
            "document '{}' is incomplete for production use: {}",
            governance_tag,
            warnings.join("; ")
        )))
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AmendmentHistoryEntry {
    pub version: u32,
    pub amended_at: String,
    pub description: String,
}

#[utoipa::path(
    post,
    path = "/v1/contracts",
    tag = "formation",
    request_body = GenerateContractRequest,
    responses(
        (status = 200, description = "Contract generated", body = ContractResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn generate_contract(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Json(req): Json<GenerateContractRequest>,
) -> Result<Json<ContractResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    let is_legacy_safe = req.template_type == ContractTemplateType::Custom
        && req
            .parameters
            .get("contract_type")
            .and_then(Value::as_str)
            .is_some_and(|s| s.eq_ignore_ascii_case("safe agreement"));
    let template_type = if is_legacy_safe {
        ContractTemplateType::SafeAgreement
    } else {
        req.template_type
    };
    if template_type == ContractTemplateType::Custom {
        let has_payload = req
            .parameters
            .as_object()
            .is_some_and(|value| !value.is_empty());
        if !has_payload {
            return Err(AppError::BadRequest(
                "custom templates require non-empty parameters".to_owned(),
            ));
        }
    }

    let contract = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            let contract_id = ContractId::new();
            let document_id = DocumentId::new();
            let contract = Contract::new(
                contract_id,
                entity_id,
                template_type,
                req.counterparty_name,
                req.effective_date,
                req.parameters,
                document_id,
            );

            // Persist a Document so the contract appears in the documents list.
            let title = format!(
                "{} — {}",
                template_label(contract.template_type(), contract.parameters()),
                contract.counterparty_name()
            );
            let (document_type, governance_tag, content) =
                contract_document_binding(&store, &contract)
                    .map_err(crate::domain::formation::error::FormationError::Validation)?;
            if let Some(governance_tag) = governance_tag.as_deref() {
                validate_governance_document_content(&store, governance_tag, &content).map_err(
                    |error| {
                        crate::domain::formation::error::FormationError::Validation(
                            app_error_detail(error),
                        )
                    },
                )?;
            }
            let path = format!("contracts/{}.json", contract_id);
            store
                .write_json(
                    "main",
                    &path,
                    &contract,
                    &format!("Generate contract {contract_id}"),
                )
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;
            let doc = Document::new(
                document_id,
                entity_id,
                workspace_id,
                document_type,
                title,
                content,
                governance_tag,
                None,
            );
            store
                .write_document(
                    "main",
                    &doc,
                    &format!("Add document for contract {contract_id}"),
                )
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(contract)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(ContractResponse {
        contract_id: contract.contract_id(),
        entity_id: contract.entity_id(),
        template_type: contract.template_type(),
        counterparty_name: contract.counterparty_name().to_owned(),
        effective_date: contract.effective_date(),
        status: contract.status(),
        document_id: contract.document_id(),
        created_at: contract.created_at().to_rfc3339(),
    }))
}

#[utoipa::path(
    get,
    path = "/v1/sign/{document_id}",
    tag = "formation",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Signing link", body = SigningLinkResponse),
        (status = 404, description = "Document not found"),
    ),
)]
async fn get_signing_link(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<SigningLinkResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let requested_entity_id = query.entity_id;
    let allowed_entity_ids = auth.entity_ids().map(|ids| ids.to_vec());

    // Generate a signing token — only the hash is stored at rest
    let raw_token = format!("sig_{}", uuid::Uuid::new_v4().simple());
    let token_hash = hash_signing_token(&raw_token);
    let expires_at = (chrono::Utc::now() + chrono::Duration::hours(72)).to_rfc3339();

    // Verify document exists and persist the signing token (hashed)
    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let token_hash = token_hash.clone();
        let expires_at = expires_at.clone();
        move || {
            let resolved_entity_id = resolve_document_entity_id_with_fallback(
                &layout,
                workspace_id,
                requested_entity_id,
                allowed_entity_ids.as_deref(),
                document_id,
            )?;
            let store = open_formation_store(&layout, workspace_id, resolved_entity_id)?;
            let signing_token = SigningToken {
                token_hash: token_hash.clone(),
                workspace_id,
                entity_id: resolved_entity_id,
                document_id,
                expires_at,
            };

            // Persist signing token keyed by hash (raw token never stored)
            let path = format!("signing-tokens/{}.json", token_hash);
            store
                .write_json(
                    "main",
                    &path,
                    &signing_token,
                    &format!("Create signing token for document {document_id}"),
                )
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(SigningLinkResponse {
        document_id,
        signing_url: format!("/human/sign/{}", document_id),
        token: raw_token,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/documents/{document_id}/pdf",
    tag = "formation",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "PDF document", content_type = "application/pdf"),
        (status = 404, description = "Document not found"),
        (status = 422, description = "Document has no governance tag"),
    ),
)]
async fn get_document_pdf(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<impl IntoResponse, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let pdf_bytes = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let doc = store
                .read_document("main", document_id)
                .map_err(|_| AppError::NotFound(format!("document {} not found", document_id)))?;

            // Load AST and profile
            let ast = doc_ast::default_doc_ast();
            let profile: GovernanceProfile =
                match store.read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH) {
                    Ok(p) => p,
                    Err(_) => {
                        let entity = store
                            .read_entity("main")
                            .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
                        GovernanceProfile::default_for_entity(&entity)
                    }
                };

            // Map entity type
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            let entity_type = entity_type_key(entity.entity_type());

            // Find matching AST document definition via governance_tag
            let governance_tag = doc.governance_tag().ok_or_else(|| {
                AppError::UnprocessableEntity(format!(
                    "document {} has no governance_tag — cannot render PDF",
                    document_id
                ))
            })?;

            let doc_def = find_ast_document_definition(ast, governance_tag).ok_or_else(|| {
                AppError::NotFound(format!(
                    "no AST document definition matches governance_tag '{}'",
                    governance_tag
                ))
            })?;
            validate_governance_document_content(&store, governance_tag, doc.content())?;

            // Render PDF
            let pdf = typst_renderer::render_pdf_with_context(
                doc_def,
                ast,
                entity_type,
                &profile,
                doc.content(),
                doc.signatures(),
            )
            .map_err(|e| AppError::Internal(format!("PDF rendering failed: {e}")))?;

            Ok::<_, AppError>(pdf)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let disposition = format!("inline; filename=\"{document_id}.pdf\"");
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_owned()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        pdf_bytes,
    ))
}

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct PreviewDocumentQuery {
    pub entity_id: EntityId,
    pub document_id: String,
}

/// Validate that a document definition exists and applies to the entity type.
/// Does NOT render the PDF — returns instantly.
fn validate_preview_document(
    layout: &crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
    doc_id: &str,
) -> Result<(), AppError> {
    let store = open_formation_store(layout, workspace_id, entity_id)?;
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    let entity_type = match entity.entity_type() {
        EntityType::CCorp => doc_ast::EntityTypeKey::Corporation,
        EntityType::Llc => doc_ast::EntityTypeKey::Llc,
    };
    let ast = doc_ast::default_doc_ast();
    let doc_def = ast
        .documents
        .iter()
        .find(|d| d.id == doc_id)
        .ok_or_else(|| {
            AppError::NotFound(format!("no AST document definition matches id '{doc_id}'"))
        })?;
    if !doc_def.entity_scope.matches(entity_type) {
        return Err(AppError::UnprocessableEntity(format!(
            "document '{doc_id}' does not apply to entity type {entity_type:?}"
        )));
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/v1/documents/preview/pdf/validate",
    tag = "formation",
    params(PreviewDocumentQuery),
    responses(
        (status = 200, description = "Document definition is valid"),
        (status = 404, description = "Document definition not found"),
        (status = 422, description = "Document does not apply to entity type"),
    ),
)]
/// Validate a document definition without rendering. Returns instantly.
async fn validate_preview_document_pdf(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Query(query): Query<PreviewDocumentQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    let doc_id = query.document_id;

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || validate_preview_document(&layout, workspace_id, entity_id, &doc_id)
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(serde_json::json!({ "valid": true })))
}

#[utoipa::path(
    get,
    path = "/v1/documents/preview/pdf",
    tag = "formation",
    params(PreviewDocumentQuery),
    responses(
        (status = 200, description = "Preview PDF document", content_type = "application/pdf"),
        (status = 404, description = "Document definition not found"),
        (status = 422, description = "Document does not apply to entity type"),
    ),
)]
/// Preview a governance document as PDF without requiring a saved Document record.
async fn preview_document_pdf(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Query(query): Query<PreviewDocumentQuery>,
) -> Result<impl IntoResponse, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    let doc_id = query.document_id;

    let pdf_bytes = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::task::spawn_blocking({
            let layout = state.layout.clone();
            let doc_id = doc_id.clone();
            move || {
                let store = open_formation_store(&layout, workspace_id, entity_id)?;
                let entity = store
                    .read_entity("main")
                    .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
                let entity_type = match entity.entity_type() {
                    EntityType::CCorp => doc_ast::EntityTypeKey::Corporation,
                    EntityType::Llc => doc_ast::EntityTypeKey::Llc,
                };
                let profile: GovernanceProfile =
                    match store.read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH) {
                        Ok(p) => p,
                        Err(_) => GovernanceProfile::default_for_entity(&entity),
                    };
                let ast = doc_ast::default_doc_ast();
                let doc_def = ast
                    .documents
                    .iter()
                    .find(|d| d.id == doc_id)
                    .ok_or_else(|| {
                        AppError::NotFound(format!(
                            "no AST document definition matches id '{doc_id}'"
                        ))
                    })?;
                if !doc_def.entity_scope.matches(entity_type) {
                    return Err(AppError::UnprocessableEntity(format!(
                        "document '{doc_id}' does not apply to entity type {entity_type:?}"
                    )));
                }
                let pdf = typst_renderer::render_pdf(doc_def, ast, entity_type, &profile, &[])
                    .map_err(|e| AppError::Internal(format!("PDF rendering failed: {e}")))?;
                Ok::<_, AppError>(pdf)
            }
        }),
    )
    .await
    .map_err(|_| AppError::Internal("PDF rendering timed out after 30 seconds".to_owned()))?
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let disposition = format!("inline; filename=\"preview-{doc_id}.pdf\"");
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_owned()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        pdf_bytes,
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct DocumentCopyRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub recipient_email: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DocumentCopyResponse {
    pub document_id: DocumentId,
    pub request_id: String,
    pub status: String,
    pub title: String,
    pub recipient_email: Option<String>,
    pub created_at: String,
}

#[utoipa::path(
    post,
    path = "/v1/documents/{document_id}/request-copy",
    tag = "formation",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
    ),
    request_body = DocumentCopyRequest,
    responses(
        (status = 200, description = "Document copy requested", body = DocumentCopyResponse),
        (status = 404, description = "Document not found"),
    ),
)]
async fn request_document_copy(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Json(req): Json<DocumentCopyRequest>,
) -> Result<Json<DocumentCopyResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;

    // Verify the document exists and read its title
    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            store
                .read_document("main", document_id)
                .map_err(|_| AppError::NotFound(format!("document {} not found", document_id)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(DocumentCopyResponse {
        document_id,
        request_id: format!("req_{}", uuid::Uuid::new_v4()),
        status: "requested".to_owned(),
        title: doc.title().to_owned(),
        recipient_email: req.recipient_email,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

#[utoipa::path(
    get,
    path = "/v1/documents/{document_id}/amendment-history",
    tag = "formation",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
        ("entity_id" = EntityId, Query, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Amendment history", body = Vec<AmendmentHistoryEntry>),
        (status = 404, description = "Document not found"),
    ),
)]
async fn get_amendment_history(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<Vec<AmendmentHistoryEntry>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;

    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            store
                .read_document("main", document_id)
                .map_err(|e| match e {
                    crate::git::error::GitStorageError::NotFound(_) => {
                        crate::domain::formation::error::FormationError::DocumentNotFound(
                            document_id,
                        )
                    }
                    other => crate::domain::formation::error::FormationError::Validation(
                        other.to_string(),
                    ),
                })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    // Current version is the only entry (no amendment tracking yet)
    let entries = vec![AmendmentHistoryEntry {
        version: doc.version(),
        amended_at: doc.created_at().to_rfc3339(),
        description: "Original document".to_owned(),
    }];

    Ok(Json(entries))
}

// ── Governance documents ────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance-documents",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "List of governance documents", body = Vec<DocumentSummary>),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn list_governance_documents(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<DocumentSummary>>, AppError> {
    let workspace_id = auth.workspace_id();

    let docs = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_document_ids("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            let mut results = Vec::new();
            for id in ids {
                if let Ok(doc) = store.read_document("main", id) {
                    if is_core_governance_document(&doc) {
                        results.push(DocumentSummary {
                            document_id: doc.document_id(),
                            document_type: doc.document_type(),
                            title: doc.title().to_owned(),
                            status: doc.status(),
                            signature_count: doc.signatures().len(),
                            created_at: doc.created_at().to_rfc3339(),
                        });
                    }
                }
            }
            Ok::<_, crate::domain::formation::error::FormationError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(docs))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/governance-documents/current",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    responses(
        (status = 200, description = "Current governance document", body = DocumentSummary),
        (status = 404, description = "Entity or governance document not found"),
    ),
)]
async fn get_current_governance_document(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<DocumentSummary>, AppError> {
    let workspace_id = auth.workspace_id();

    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_document_ids("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            let mut latest_doc = None;
            for id in ids {
                if let Ok(doc) = store.read_document("main", id) {
                    if is_core_governance_document(&doc)
                        && latest_doc
                            .as_ref()
                            .map(|current: &Document| doc.created_at() > current.created_at())
                            .unwrap_or(true)
                    {
                        latest_doc = Some(doc);
                    }
                }
            }

            latest_doc
                .map(|d| DocumentSummary {
                    document_id: d.document_id(),
                    document_type: d.document_type(),
                    title: d.title().to_owned(),
                    status: d.status(),
                    signature_count: d.signatures().len(),
                    created_at: d.created_at().to_rfc3339(),
                })
                .ok_or_else(|| {
                    crate::domain::formation::error::FormationError::Validation(
                        "no governance documents found".to_owned(),
                    )
                })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(doc))
}

// ── Entity lifecycle ────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/entities",
    tag = "formation",
    responses(
        (status = 200, description = "List of entities", body = Vec<FormationStatusResponse>),
    ),
)]
async fn list_entities(
    RequireFormationRead(auth): RequireFormationRead,
    State(state): State<AppState>,
) -> Result<Json<Vec<FormationStatusResponse>>, AppError> {
    let workspace_id = auth.workspace_id();

    let entities = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let entity_ids = layout.list_entity_ids(workspace_id);
            let mut results = Vec::new();
            for eid in entity_ids {
                if let Ok(store) = EntityStore::open(&layout, workspace_id, eid) {
                    if let Ok(entity) = store.read_entity("main") {
                        let next_action = service::next_formation_action(entity.formation_status())
                            .map(String::from);
                        results.push(FormationStatusResponse {
                            entity_id: entity.entity_id(),
                            legal_name: entity.legal_name().to_owned(),
                            entity_type: entity.entity_type(),
                            jurisdiction: entity.jurisdiction().to_owned(),
                            formation_state: entity.formation_state(),
                            formation_status: entity.formation_status(),
                            formation_date: entity.formation_date().map(|d| d.to_rfc3339()),
                            next_action,
                        });
                    }
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(entities))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConvertEntityRequest {
    #[serde(alias = "new_entity_type")]
    pub target_type: EntityType,
    #[serde(default, alias = "new_jurisdiction")]
    pub jurisdiction: Option<Jurisdiction>,
}

#[utoipa::path(
    post,
    path = "/v1/entities/{entity_id}/convert",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = ConvertEntityRequest,
    responses(
        (status = 200, description = "Entity converted", body = FormationStatusResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn convert_entity(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<ConvertEntityRequest>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let mut entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            entity.set_entity_type(req.target_type)?;
            if let Some(jurisdiction) = req.jurisdiction {
                entity.set_jurisdiction(jurisdiction)?;
            }

            store
                .write_entity(
                    "main",
                    &entity,
                    &format!("Convert entity to {}", req.target_type),
                )
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        formation_date: entity.formation_date().map(|d| d.to_rfc3339()),
        next_action,
    }))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct DissolveEntityRequest {
    #[serde(default)]
    pub reason: Option<String>,
}

#[utoipa::path(
    post,
    path = "/v1/entities/{entity_id}/dissolve",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = DissolveEntityRequest,
    responses(
        (status = 200, description = "Entity dissolved"),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn dissolve_entity(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<DissolveEntityRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let workspace_id = auth.workspace_id();
    let reason = req
        .reason
        .unwrap_or_else(|| "voluntary dissolution".to_owned());

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let reason = reason.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            let mut entity = store.read_entity("main").map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(e.to_string())
            })?;

            entity.dissolve()?;

            store
                .write_entity("main", &entity, &format!("Dissolve entity: {reason}"))
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(serde_json::json!({
        "entity_id": entity.entity_id(),
        "legal_name": entity.legal_name(),
        "status": "dissolved",
        "reason": reason,
    })))
}

// ── Staged formation handlers ────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/formations/pending",
    tag = "formation",
    request_body = CreatePendingFormationRequest,
    responses(
        (status = 200, description = "Pending formation created", body = PendingFormationResponse),
        (status = 400, description = "Invalid request"),
    ),
)]
async fn create_pending_formation(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Json(req): Json<CreatePendingFormationRequest>,
) -> Result<Json<PendingFormationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_type = req.entity_type;
    let profile_overrides = build_profile_overrides_from_fields(
        req.formation_date.as_deref(),
        req.fiscal_year_end.as_deref(),
        req.s_corp_election,
        req.transfer_restrictions,
        req.right_of_first_refusal,
        req.company_address.clone(),
    )?;
    let jurisdiction = req.jurisdiction.unwrap_or_else(|| {
        let j = match entity_type {
            EntityType::Llc => "US-WY",
            EntityType::CCorp => "US-DE",
        };
        Jurisdiction::new(j).unwrap()
    });

    let entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let legal_name = req.legal_name;
        let jurisdiction = jurisdiction.clone();
        let registered_agent_name = cleaned_optional_string(req.registered_agent_name);
        let registered_agent_address = cleaned_optional_string(req.registered_agent_address);
        let profile_overrides = profile_overrides.clone();
        move || {
            if workspace_has_legal_name(&layout, workspace_id, &legal_name, None).map_err(|e| {
                crate::domain::formation::error::FormationError::Validation(format!("{e:?}"))
            })? {
                return Err(crate::domain::formation::error::FormationError::Validation(
                    format!("entity legal name already exists in workspace: {legal_name}"),
                ));
            }
            service::create_pending_entity_with_profile_overrides(
                &layout,
                workspace_id,
                legal_name,
                entity_type,
                jurisdiction,
                registered_agent_name,
                registered_agent_address,
                profile_overrides,
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(PendingFormationResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().clone(),
        formation_status: entity.formation_status(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/founders",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = AddFounderRequest,
    responses(
        (status = 200, description = "Founder added", body = AddFounderResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn add_founder(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<AddFounderRequest>,
) -> Result<Json<AddFounderResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let member = MemberInput {
        name: req.name,
        investor_type: InvestorType::NaturalPerson,
        email: req.email,
        agent_id: None,
        entity_id: None,
        ownership_pct: req.ownership_pct,
        membership_units: None,
        share_count: None,
        share_class: None,
        role: req.role,
        address: req.address,
        officer_title: req.officer_title,
        shares_purchased: None,
        vesting: None,
        ip_description: None,
        is_incorporator: req.is_incorporator,
    };

    let members = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || service::add_pending_member(&layout, workspace_id, entity_id, member)
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let summaries: Vec<FounderSummary> = members
        .iter()
        .map(|m| FounderSummary {
            name: m.name.clone(),
            email: m.email.clone(),
            role: m.role,
            ownership_pct: m.ownership_pct,
            address: m.address.clone(),
        })
        .collect();

    Ok(Json(AddFounderResponse {
        entity_id,
        member_count: summaries.len(),
        members: summaries,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/formations/{entity_id}/finalize",
    tag = "formation",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
    ),
    request_body = FinalizePendingFormationRequest,
    responses(
        (status = 200, description = "Formation finalized with cap table", body = FormationWithCapTableResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn finalize_pending_formation(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<FinalizePendingFormationRequest>,
) -> Result<Json<FormationWithCapTableResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let profile_overrides = build_profile_overrides_from_fields(
        req.formation_date.as_deref(),
        req.fiscal_year_end.as_deref(),
        req.s_corp_election,
        req.transfer_restrictions,
        req.right_of_first_refusal,
        req.company_address.clone(),
    )?;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let registered_agent_name = cleaned_optional_string(req.registered_agent_name);
        let registered_agent_address = cleaned_optional_string(req.registered_agent_address);
        let incorporator_name = cleaned_optional_string(req.incorporator_name);
        let incorporator_address = cleaned_optional_string(req.incorporator_address);
        let authorized_shares = req.authorized_shares;
        let par_value = req.par_value;
        let profile_overrides = profile_overrides.clone();
        move || {
            service::finalize_formation_with_profile_overrides(
                &layout,
                workspace_id,
                entity_id,
                authorized_shares,
                par_value.as_deref(),
                registered_agent_name,
                registered_agent_address,
                incorporator_name,
                incorporator_address,
                profile_overrides,
            )
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let (formation, cap_table) = result;
    let formation_status = formation.entity.formation_status();
    let next_action = service::next_formation_action(formation_status).map(String::from);

    Ok(Json(FormationWithCapTableResponse {
        formation_id: entity_id,
        entity_id,
        formation_status,
        document_ids: formation.document_ids,
        next_action,
        legal_entity_id: Some(cap_table.legal_entity_id),
        instrument_id: Some(cap_table.instrument_id),
        holders: cap_table.holders,
    }))
}

// ── Router ──────────────────────────────────────────────────────────────

// ── Public signing endpoints (token-authenticated, no API key needed) ──

/// Helper: look up a signing token by hashing the raw token and scanning
/// workspaces/entities for the matching hash file.
fn resolve_signing_token(
    layout: &crate::store::RepoLayout,
    raw_token: &str,
) -> Result<SigningToken, AppError> {
    let token_hash = hash_signing_token(raw_token);
    for workspace_id in layout.list_workspace_ids() {
        for entity_id in layout.list_entity_ids(workspace_id) {
            let store = match EntityStore::open(layout, workspace_id, entity_id) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let path = format!("signing-tokens/{}.json", token_hash);
            if let Ok(st) = store.read_json::<SigningToken>("main", &path) {
                // Verify the stored hash matches (constant-time not critical here
                // since the hash is already derived from the token)
                if st.token_hash != token_hash {
                    continue;
                }
                // Check expiration
                if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&st.expires_at) {
                    if expires < chrono::Utc::now() {
                        return Err(AppError::BadRequest("signing token has expired".to_owned()));
                    }
                }
                return Ok(st);
            }
        }
    }
    Err(AppError::NotFound("invalid signing token".to_owned()))
}

#[utoipa::path(
    get,
    path = "/v1/human/sign/{document_id}/resolve",
    tag = "signing",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
        ("token" = String, Query, description = "Signing token"),
    ),
    responses(
        (status = 200, description = "Document metadata for signing UI", body = SigningResolveResponse),
        (status = 404, description = "Invalid token or document not found"),
    ),
)]
async fn resolve_signing_link(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<SigningTokenQuery>,
) -> Result<Json<SigningResolveResponse>, AppError> {
    let token = query.token;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let st = resolve_signing_token(&layout, &token)?;
            if st.document_id != document_id {
                return Err(AppError::BadRequest(
                    "token does not match document".to_owned(),
                ));
            }

            let store = open_formation_store(&layout, st.workspace_id, st.entity_id)?;
            let doc = store
                .read_document("main", document_id)
                .map_err(|_| AppError::NotFound(format!("document {} not found", document_id)))?;

            let signatures = doc
                .signatures()
                .iter()
                .map(|s| SignatureSummary {
                    signature_id: s.signature_id(),
                    signer_name: s.signer_name().to_owned(),
                    signer_role: s.signer_role().to_owned(),
                    signed_at: s.signed_at().to_rfc3339(),
                })
                .collect();

            // Load entity name for display
            let entity_name = store
                .read_entity("main")
                .ok()
                .map(|e| e.legal_name().to_owned());
            let preview_text = render_governance_document_markdown(&store, &doc)?;

            // If document references a contract, load contract details
            let contract = doc
                .content()
                .get("contract_id")
                .and_then(|v| v.as_str())
                .and_then(|cid| {
                    let path = format!("contracts/{cid}.json");
                    store.read_json::<Contract>("main", &path).ok()
                })
                .map(|c| {
                    let rendered_text = render_signing_preview_text(
                        &c,
                        entity_name.as_deref().unwrap_or("Company"),
                    );
                    SigningContractDetails {
                        template_type: format!("{:?}", c.template_type()),
                        template_label: Some(template_label(c.template_type(), c.parameters())),
                        counterparty_name: c.counterparty_name().to_owned(),
                        effective_date: c.effective_date().to_string(),
                        parameters: c.parameters().clone(),
                        rendered_text: Some(rendered_text),
                    }
                });

            Ok(SigningResolveResponse {
                document_id: doc.document_id(),
                entity_id: st.entity_id,
                document_title: doc.title().to_owned(),
                document_status: format!("{:?}", doc.status()),
                signatures,
                pdf_url: Some(format!("/api/human/sign/{document_id}/pdf?token={token}")),
                preview_text: preview_text
                    .or_else(|| contract.as_ref().and_then(|c| c.rendered_text.clone())),
                contract,
                entity_name,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/v1/human/sign/{document_id}/pdf",
    tag = "signing",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
        ("token" = String, Query, description = "Signing token"),
    ),
    responses(
        (status = 200, description = "PDF preview for signing", content_type = "application/pdf"),
        (status = 404, description = "Invalid token or document not found"),
    ),
)]
async fn get_signing_pdf(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<SigningTokenQuery>,
) -> Result<impl IntoResponse, AppError> {
    let token = query.token;

    let pdf_bytes = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let st = resolve_signing_token(&layout, &token)?;
            if st.document_id != document_id {
                return Err(AppError::BadRequest("token does not match document".to_owned()));
            }

            let store = open_formation_store(&layout, st.workspace_id, st.entity_id)?;
            let doc = store
                .read_document("main", document_id)
                .map_err(|_| AppError::NotFound(format!("document {} not found", document_id)))?;
            let entity_name = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?
                .legal_name()
                .to_owned();

            let body_text = render_governance_document_markdown(&store, &doc)?.unwrap_or_else(|| {
                doc.content()
                    .get("contract_id")
                    .and_then(|v| v.as_str())
                    .and_then(|cid| {
                        let path = format!("contracts/{cid}.json");
                        store.read_json::<Contract>("main", &path).ok()
                    })
                    .map(|c| render_signing_preview_text(&c, &entity_name))
                    .unwrap_or_else(|| {
                        format!(
                            "{}\n\nThis document is available for signature, but no contract preview text is stored for it.",
                            doc.title()
                        )
                    })
            });

            render_text_preview_pdf(doc.title(), &body_text)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let disposition = format!("inline; filename=\"{document_id}.pdf\"");
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_owned()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        pdf_bytes,
    ))
}

#[utoipa::path(
    post,
    path = "/v1/human/sign/{document_id}/submit",
    tag = "signing",
    params(
        ("document_id" = DocumentId, Path, description = "Document ID"),
        ("token" = String, Query, description = "Signing token"),
    ),
    request_body = SignDocumentRequest,
    responses(
        (status = 200, description = "Document signed", body = SignDocumentResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Invalid token or document not found"),
    ),
)]
async fn submit_signing(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<SigningTokenQuery>,
    Json(req): Json<SignDocumentRequest>,
) -> Result<Json<SignDocumentResponse>, AppError> {
    if req.signer_name.is_empty() || req.signer_name.len() > 256 {
        return Err(AppError::BadRequest(
            "signer_name must be between 1 and 256 characters".to_owned(),
        ));
    }
    if req.signer_email.is_empty() || !req.signer_email.contains('@') {
        return Err(AppError::BadRequest(
            "signer_email must be a valid email address".to_owned(),
        ));
    }
    if req.signature_text.is_empty() {
        return Err(AppError::BadRequest(
            "signature_text is required".to_owned(),
        ));
    }

    let token = query.token;

    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let st = resolve_signing_token(&layout, &token)?;
            if st.document_id != document_id {
                return Err(AppError::BadRequest(
                    "token does not match document".to_owned(),
                ));
            }

            let store = open_formation_store(&layout, st.workspace_id, st.entity_id)?;
            let mut doc = store
                .read_document("main", document_id)
                .map_err(|_| AppError::NotFound(format!("document {} not found", document_id)))?;

            let sig_request = SignatureRequest {
                signer_name: req.signer_name,
                signer_role: req.signer_role,
                signer_email: req.signer_email,
                signature_text: req.signature_text,
                consent_text: req.consent_text,
                signature_svg: req.signature_svg,
                ip_address: None,
            };

            doc.sign(sig_request)
                .map_err(|e| AppError::BadRequest(format!("signing failed: {e}")))?;

            store
                .write_document(
                    "main",
                    &doc,
                    &format!("Sign document {document_id} (human UI)"),
                )
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(format!(
                        "commit error: {e}"
                    ))
                })?;

            Ok::<_, AppError>(doc)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    let last_sig = doc
        .signatures()
        .last()
        .ok_or_else(|| AppError::Internal("signature was added but not found".to_string()))?;

    Ok(Json(SignDocumentResponse {
        signature_id: last_sig.signature_id(),
        document_id: doc.document_id(),
        document_status: doc.status(),
        signed_at: last_sig.signed_at().to_rfc3339(),
    }))
}

pub fn formation_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/formations", post(create_formation))
        .route(
            "/v1/formations/with-cap-table",
            post(create_formation_with_cap_table),
        )
        // Staged formation flow
        .route("/v1/formations/pending", post(create_pending_formation))
        .route("/v1/formations/{entity_id}/founders", post(add_founder))
        .route(
            "/v1/formations/{entity_id}/finalize",
            post(finalize_pending_formation),
        )
        .route("/v1/formations/{entity_id}", get(get_formation))
        .route("/v1/formations/{entity_id}/documents", get(list_documents))
        .route(
            "/v1/formations/{entity_id}/mark-documents-signed",
            post(mark_documents_signed),
        )
        .route(
            "/v1/formations/{entity_id}/filing-attestation",
            post(record_filing_attestation),
        )
        .route(
            "/v1/formations/{entity_id}/registered-agent-consent-evidence",
            post(add_registered_agent_consent_evidence),
        )
        .route(
            "/v1/formations/{entity_id}/service-agreement/execute",
            post(execute_service_agreement),
        )
        .route("/v1/formations/{entity_id}/gates", get(get_formation_gates))
        .route(
            "/v1/formations/{entity_id}/submit-filing",
            post(submit_filing),
        )
        .route(
            "/v1/formations/{entity_id}/filing-confirmation",
            post(confirm_filing),
        )
        .route("/v1/formations/{entity_id}/apply-ein", post(apply_ein))
        .route(
            "/v1/formations/{entity_id}/ein-confirmation",
            post(confirm_ein),
        )
        .route("/v1/documents/preview/pdf", get(preview_document_pdf))
        .route(
            "/v1/documents/preview/pdf/validate",
            get(validate_preview_document_pdf),
        )
        .route("/v1/documents/{document_id}", get(get_document))
        .route("/v1/documents/{document_id}/sign", post(sign_document))
        .route("/v1/documents/{document_id}/pdf", get(get_document_pdf))
        .route(
            "/v1/documents/{document_id}/request-copy",
            post(request_document_copy),
        )
        .route(
            "/v1/documents/{document_id}/amendment-history",
            get(get_amendment_history),
        )
        // Contracts
        .route("/v1/contracts", post(generate_contract))
        // Signing links
        .route("/v1/sign/{document_id}", get(get_signing_link))
        // Public signing endpoints (token-authenticated, no API key)
        .route(
            "/v1/human/sign/{document_id}/resolve",
            get(resolve_signing_link),
        )
        .route("/v1/human/sign/{document_id}/pdf", get(get_signing_pdf))
        .route("/v1/human/sign/{document_id}/submit", post(submit_signing))
        // Entity lifecycle
        .route("/v1/entities", get(list_entities))
        .route("/v1/entities/{entity_id}/convert", post(convert_entity))
        .route("/v1/entities/{entity_id}/dissolve", post(dissolve_entity))
        // Governance documents
        .route(
            "/v1/entities/{entity_id}/governance-documents",
            get(list_governance_documents),
        )
        .route(
            "/v1/entities/{entity_id}/governance-documents/current",
            get(get_current_governance_document),
        )
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create_formation,
        create_formation_with_cap_table,
        get_formation,
        list_documents,
        get_document,
        sign_document,
        mark_documents_signed,
        submit_filing,
        record_filing_attestation,
        add_registered_agent_consent_evidence,
        execute_service_agreement,
        get_formation_gates,
        confirm_filing,
        apply_ein,
        confirm_ein,
        generate_contract,
        get_signing_link,
        resolve_signing_link,
        get_signing_pdf,
        submit_signing,
        get_document_pdf,
        preview_document_pdf,
        request_document_copy,
        get_amendment_history,
        list_governance_documents,
        get_current_governance_document,
        list_entities,
        convert_entity,
        dissolve_entity,
        create_pending_formation,
        add_founder,
        finalize_pending_formation,
    ),
    components(schemas(
        CreateFormationRequest,
        FormationResponse,
        FormationWithCapTableResponse,
        FormationStatusResponse,
        DocumentSummary,
        DocumentResponse,
        SignatureSummary,
        SignDocumentRequest,
        SignDocumentResponse,
        ConfirmFilingRequest,
        ConfirmEinRequest,
        FilingAttestationRequest,
        RegisteredAgentConsentEvidenceRequest,
        ExecuteServiceAgreementRequest,
        FormationGatesResponse,
        CreatePendingFormationRequest,
        PendingFormationResponse,
        AddFounderRequest,
        AddFounderResponse,
        FounderSummary,
        FinalizePendingFormationRequest,
        GenerateContractRequest,
        ContractResponse,
        SigningLinkResponse,
        SigningResolveResponse,
        AmendmentHistoryEntry,
        PreviewDocumentQuery,
        DocumentCopyRequest,
        DocumentCopyResponse,
        ConvertEntityRequest,
        DissolveEntityRequest,
    )),
    tags((name = "formation", description = "Entity formation and document management")),
)]
pub struct FormationApi;

#[cfg(test)]
mod tests {
    use super::{resolve_document_entity_id, workspace_has_legal_name};
    use crate::domain::formation::{
        content::{InvestorType, MemberInput, MemberRole},
        service::{self, FormationProfileOverrides},
        types::{EntityType, Jurisdiction},
    };
    use crate::domain::ids::WorkspaceId;
    use crate::error::AppError;
    use crate::store::RepoLayout;
    use tempfile::TempDir;

    fn founder(name: &str, email: &str) -> MemberInput {
        MemberInput {
            name: name.to_owned(),
            investor_type: InvestorType::NaturalPerson,
            email: Some(email.to_owned()),
            agent_id: None,
            entity_id: None,
            ownership_pct: Some(100.0),
            membership_units: None,
            share_count: None,
            share_class: None,
            role: Some(MemberRole::Member),
            address: None,
            officer_title: None,
            shares_purchased: None,
            vesting: None,
            ip_description: None,
            is_incorporator: None,
        }
    }

    #[test]
    fn resolve_document_entity_id_requires_the_requested_entity_to_own_the_document() {
        let tmp = TempDir::new().expect("temp dir");
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let owner = service::create_pending_entity_with_profile_overrides(
            &layout,
            workspace_id,
            "Owner LLC".to_owned(),
            EntityType::Llc,
            Jurisdiction::new("Wyoming").expect("jurisdiction"),
            Some("Wyoming Registered Agent LLC".to_owned()),
            Some("123 Capitol Ave, Cheyenne, WY 82001".to_owned()),
            FormationProfileOverrides::default(),
        )
        .expect("owner entity");
        service::add_pending_member(
            &layout,
            workspace_id,
            owner.entity_id(),
            founder("Alice Owner", "alice@example.com"),
        )
        .expect("owner founder");
        let (formation, _) =
            service::finalize_formation(&layout, workspace_id, owner.entity_id(), None, None)
                .expect("owner formation");
        let document_id = formation.document_ids[0];

        let other = service::create_pending_entity_with_profile_overrides(
            &layout,
            workspace_id,
            "Other LLC".to_owned(),
            EntityType::Llc,
            Jurisdiction::new("Wyoming").expect("jurisdiction"),
            Some("Wyoming Registered Agent LLC".to_owned()),
            Some("456 Frontier Mall Dr, Cheyenne, WY 82009".to_owned()),
            FormationProfileOverrides::default(),
        )
        .expect("other entity");

        let resolved =
            resolve_document_entity_id(&layout, workspace_id, owner.entity_id(), None, document_id)
                .expect("document should resolve for owning entity");
        assert_eq!(resolved, owner.entity_id());

        let err =
            resolve_document_entity_id(&layout, workspace_id, other.entity_id(), None, document_id)
                .expect_err("document should not resolve through a different entity");
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn workspace_has_legal_name_detects_existing_entities() {
        let tmp = TempDir::new().expect("temp dir");
        let layout = RepoLayout::new(tmp.path().to_path_buf());
        let workspace_id = WorkspaceId::new();

        let entity = service::create_entity_with_profile_overrides(
            &layout,
            workspace_id,
            "Nexus AI Labs LLC".to_owned(),
            EntityType::Llc,
            Jurisdiction::new("US-WY").expect("jurisdiction"),
            None,
            None,
            &[founder("Alice", "alice@example.com")],
            None,
            None,
            FormationProfileOverrides {
                company_address: Some(crate::domain::governance::profile::CompanyAddress {
                    street: "1 Market St".to_owned(),
                    city: "Cheyenne".to_owned(),
                    county: None,
                    state: "WY".to_owned(),
                    zip: "82001".to_owned(),
                }),
                ..FormationProfileOverrides::default()
            },
        )
        .expect("entity");

        assert!(
            workspace_has_legal_name(&layout, workspace_id, "Nexus AI Labs LLC", None)
                .expect("name lookup")
        );
        assert!(
            !workspace_has_legal_name(
                &layout,
                workspace_id,
                "Nexus AI Labs LLC",
                Some(entity.entity.entity_id()),
            )
            .expect("skip current entity")
        );
    }
}
