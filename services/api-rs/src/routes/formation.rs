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
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::auth::{RequireFormationCreate, RequireFormationRead, RequireFormationSign};
use crate::domain::formation::{
    content::{InvestorType, MemberInput, MemberRole, OfficerTitle},
    contract::{Contract, ContractStatus, ContractTemplateType},
    document::SignatureRequest,
    entity::Entity,
    filing::Filing,
    service,
    types::*,
};
use crate::domain::governance::{
    doc_ast,
    profile::{GOVERNANCE_PROFILE_PATH, GovernanceProfile},
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

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let legal_name = req.legal_name;
        let jurisdiction = req.jurisdiction;
        let members = req.members;
        let entity_type = req.entity_type;
        let ra_name = req.registered_agent_name;
        let ra_addr = req.registered_agent_address;
        let shares = req.authorized_shares;
        let par_value = req.par_value;
        move || {
            service::create_entity(
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

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let legal_name = req.legal_name;
        let jurisdiction = req.jurisdiction;
        let members = req.members;
        let entity_type = req.entity_type;
        let ra_name = req.registered_agent_name;
        let ra_addr = req.registered_agent_address;
        let shares = req.authorized_shares;
        let par_value = req.par_value;
        move || {
            // Step 1: Create the entity (formation documents, filing, tax profile)
            let formation = service::create_entity(
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

    let contract = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            let contract_id = ContractId::new();
            let document_id = DocumentId::new();
            let contract = Contract::new(
                contract_id,
                entity_id,
                req.template_type,
                req.counterparty_name,
                req.effective_date,
                req.parameters,
                document_id,
            );

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
    let entity_id = query.entity_id;

    // Verify the document exists in storage
    tokio::task::spawn_blocking({
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

    Ok(Json(SigningLinkResponse {
        document_id,
        signing_url: format!("/human/sign/{}", document_id),
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
            let entity_type = match entity.entity_type() {
                EntityType::CCorp => doc_ast::EntityTypeKey::Corporation,
                EntityType::Llc => doc_ast::EntityTypeKey::Llc,
            };

            // Find matching AST document definition via governance_tag
            let governance_tag = doc.governance_tag().ok_or_else(|| {
                AppError::UnprocessableEntity(format!(
                    "document {} has no governance_tag — cannot render PDF",
                    document_id
                ))
            })?;

            let doc_def = ast
                .documents
                .iter()
                .find(|d| d.id == governance_tag || d.path.contains(governance_tag))
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "no AST document definition matches governance_tag '{}'",
                        governance_tag
                    ))
                })?;

            // Render PDF
            let pdf = typst_renderer::render_pdf(
                doc_def,
                ast,
                entity_type,
                &profile,
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

    let pdf_bytes = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let doc_id = doc_id.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            // Load entity to determine type
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            let entity_type = match entity.entity_type() {
                EntityType::CCorp => doc_ast::EntityTypeKey::Corporation,
                EntityType::Llc => doc_ast::EntityTypeKey::Llc,
            };

            // Load profile (or default from entity)
            let profile: GovernanceProfile =
                match store.read_json::<GovernanceProfile>("main", GOVERNANCE_PROFILE_PATH) {
                    Ok(p) => p,
                    Err(_) => GovernanceProfile::default_for_entity(&entity),
                };

            // Load AST and find the document definition
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

            // Validate scope
            if !doc_def.entity_scope.matches(entity_type) {
                return Err(AppError::UnprocessableEntity(format!(
                    "document '{doc_id}' does not apply to entity type {entity_type:?}"
                )));
            }

            // Render PDF with empty signatures (preview)
            let pdf = typst_renderer::render_pdf(doc_def, ast, entity_type, &profile, &[])
                .map_err(|e| AppError::Internal(format!("PDF rendering failed: {e}")))?;

            Ok::<_, AppError>(pdf)
        }
    })
    .await
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

            let governance_types = [
                "articles_of_incorporation",
                "bylaws",
                "operating_agreement",
                "certificate_of_formation",
            ];

            let mut results = Vec::new();
            for id in ids {
                if let Ok(doc) = store.read_document("main", id) {
                    let doc_type = format!("{:?}", doc.document_type()).to_lowercase();
                    if governance_types.iter().any(|gt| doc_type.contains(gt)) {
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
                    let doc_type = format!("{:?}", doc.document_type()).to_lowercase();
                    if doc_type.contains("articles")
                        || doc_type.contains("bylaws")
                        || doc_type.contains("operating_agreement")
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
    pub target_type: EntityType,
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
        move || {
            service::create_pending_entity(
                &layout,
                workspace_id,
                legal_name,
                entity_type,
                jurisdiction,
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
        address: None,
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
    responses(
        (status = 200, description = "Formation finalized with cap table", body = FormationWithCapTableResponse),
        (status = 404, description = "Entity not found"),
    ),
)]
async fn finalize_pending_formation(
    RequireFormationCreate(auth): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<FormationWithCapTableResponse>, AppError> {
    let workspace_id = auth.workspace_id();

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || service::finalize_formation(&layout, workspace_id, entity_id, None, None)
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

pub fn formation_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/formations", post(create_formation))
        .route(
            "/v1/formations/with-cap-table",
            post(create_formation_with_cap_table),
        )
        // Staged formation flow
        .route("/v1/formations/pending", post(create_pending_formation))
        .route(
            "/v1/formations/{entity_id}/founders",
            post(add_founder),
        )
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
        GenerateContractRequest,
        ContractResponse,
        SigningLinkResponse,
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
