//! Formation HTTP routes.
//!
//! Endpoints for creating entities, signing documents, and advancing through
//! the formation lifecycle.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::domain::formation::{
    content::MemberInput,
    contract::{Contract, ContractStatus, ContractTemplateType},
    document::SignatureRequest,
    service,
    types::*,
};
use crate::domain::ids::{ContractId, DocumentId, EntityId, SignatureId, WorkspaceId};
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Request / Response types ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateFormationRequest {
    pub entity_type: EntityType,
    pub legal_name: String,
    pub jurisdiction: String,
    #[serde(default)]
    pub registered_agent_name: Option<String>,
    #[serde(default)]
    pub registered_agent_address: Option<String>,
    pub members: Vec<MemberInput>,
    #[serde(default)]
    pub authorized_shares: Option<i64>,
    #[serde(default)]
    pub par_value: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Serialize)]
pub struct FormationResponse {
    pub formation_id: EntityId,
    pub entity_id: EntityId,
    pub formation_status: FormationStatus,
    pub document_ids: Vec<DocumentId>,
    pub next_action: Option<String>,
}

#[derive(Serialize)]
pub struct FormationStatusResponse {
    pub entity_id: EntityId,
    pub legal_name: String,
    pub entity_type: EntityType,
    pub jurisdiction: String,
    pub formation_state: FormationState,
    pub formation_status: FormationStatus,
    pub next_action: Option<String>,
}

#[derive(Serialize)]
pub struct DocumentSummary {
    pub document_id: DocumentId,
    pub document_type: DocumentType,
    pub title: String,
    pub status: DocumentStatus,
    pub signature_count: usize,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct DocumentResponse {
    pub document_id: DocumentId,
    pub entity_id: EntityId,
    pub document_type: DocumentType,
    pub title: String,
    pub status: DocumentStatus,
    pub content: serde_json::Value,
    pub content_hash: String,
    pub version: u32,
    pub signatures: Vec<SignatureSummary>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SignatureSummary {
    pub signature_id: SignatureId,
    pub signer_name: String,
    pub signer_role: String,
    pub signed_at: String,
}

#[derive(Deserialize)]
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

#[derive(Serialize)]
pub struct SignDocumentResponse {
    pub signature_id: SignatureId,
    pub document_id: DocumentId,
    pub document_status: DocumentStatus,
    pub signed_at: String,
}

/// Query params for identifying which workspace an entity belongs to.
#[derive(Deserialize)]
pub struct EntityQuery {
    pub workspace_id: WorkspaceId,
}

#[derive(Deserialize)]
pub struct ConfirmFilingRequest {
    pub external_filing_id: String,
    #[serde(default)]
    pub receipt_reference: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Deserialize)]
pub struct ConfirmEinRequest {
    pub ein: String,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

/// Open an entity store, mapping git errors to formation errors.
fn open_formation_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<crate::store::entity_store::EntityStore<'a>, crate::domain::formation::error::FormationError> {
    crate::store::entity_store::EntityStore::open(layout, workspace_id, entity_id).map_err(
        |e| match e {
            crate::git::error::GitStorageError::RepoNotFound(_) => {
                crate::domain::formation::error::FormationError::EntityNotFound(entity_id)
            }
            other => crate::domain::formation::error::FormationError::Validation(
                other.to_string(),
            ),
        },
    )
}

// ── Handlers ────────────────────────────────────────────────────────────

async fn create_formation(
    State(state): State<AppState>,
    Json(req): Json<CreateFormationRequest>,
) -> Result<Json<FormationResponse>, AppError> {
    if req.members.is_empty() {
        return Err(AppError::BadRequest("at least one member is required".to_owned()));
    }
    if req.jurisdiction.is_empty() {
        return Err(AppError::BadRequest("jurisdiction is required".to_owned()));
    }
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

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

async fn get_formation(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = query.workspace_id;

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        next_action,
    }))
}

async fn list_documents(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<DocumentSummary>>, AppError> {
    let workspace_id = query.workspace_id;

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

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

async fn get_document(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<DocumentResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            store.read_document("main", document_id).map_err(|e| match e {
                crate::git::error::GitStorageError::NotFound(_) => {
                    crate::domain::formation::error::FormationError::DocumentNotFound(document_id)
                }
                other => crate::domain::formation::error::FormationError::Validation(
                    other.to_string(),
                ),
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

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

async fn sign_document(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
    Json(req): Json<SignDocumentRequest>,
) -> Result<Json<SignDocumentResponse>, AppError> {
    if req.signer_name.is_empty() || req.signer_name.len() > 256 {
        return Err(AppError::BadRequest("signer_name must be between 1 and 256 characters".to_owned()));
    }
    if req.signer_email.is_empty() || !req.signer_email.contains('@') {
        return Err(AppError::BadRequest("signer_email must be a valid email address".to_owned()));
    }
    if req.signature_text.is_empty() {
        return Err(AppError::BadRequest("signature_text is required".to_owned()));
    }
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;

            let mut doc =
                store.read_document("main", document_id).map_err(|e| match e {
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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    // The last signature is the one we just added.
    let last_sig = doc.signatures().last().ok_or_else(|| {
        AppError::Internal("signature was added but not found".to_string())
    })?;

    Ok(Json(SignDocumentResponse {
        signature_id: last_sig.signature_id(),
        document_id: doc.document_id(),
        document_status: doc.status(),
        signed_at: last_sig.signed_at().to_rfc3339(),
    }))
}

async fn confirm_filing(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<ConfirmFilingRequest>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;

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
                filing.confirm(
                    req.external_filing_id,
                    req.receipt_reference,
                );
                store.commit(
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
                ).map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(
                        format!("failed to update filing: {e}")
                    )
                })?;
            }

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        next_action,
    }))
}

async fn confirm_ein(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<ConfirmEinRequest>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;

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
            entity.activate();

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
                store.commit(
                    "main",
                    "Update tax profile with EIN",
                    vec![
                        crate::git::commit::FileWrite::json("tax/profile.json", &tax)
                            .map_err(|e| {
                                crate::domain::formation::error::FormationError::Validation(
                                    e.to_string(),
                                )
                            })?,
                    ],
                ).map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(
                        format!("failed to update tax profile: {e}")
                    )
                })?;
            }

            Ok::<_, crate::domain::formation::error::FormationError>(entity)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        next_action,
    }))
}

// ── Contract / Document management ──────────────────────────────────

#[derive(Deserialize)]
pub struct GenerateContractRequest {
    pub entity_id: EntityId,
    pub template_type: ContractTemplateType,
    pub counterparty_name: String,
    pub effective_date: chrono::NaiveDate,
    #[serde(default = "default_params")]
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

fn default_params() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Serialize)]
pub struct ContractResponse {
    pub contract_id: ContractId,
    pub entity_id: EntityId,
    pub template_type: ContractTemplateType,
    pub counterparty_name: String,
    pub effective_date: chrono::NaiveDate,
    pub status: ContractStatus,
    pub document_id: DocumentId,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SigningLinkResponse {
    pub document_id: DocumentId,
    pub signing_url: String,
}

#[derive(Serialize)]
pub struct DocumentPdfResponse {
    pub document_id: DocumentId,
    pub content_type: String,
    pub url: String,
}

#[derive(Serialize)]
pub struct AmendmentHistoryEntry {
    pub version: u32,
    pub amended_at: String,
    pub description: String,
}

async fn generate_contract(
    State(state): State<AppState>,
    Json(req): Json<GenerateContractRequest>,
) -> Result<Json<ContractResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
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
                .write_json("main", &path, &contract, &format!("Generate contract {contract_id}"))
                .map_err(|e| {
                    crate::domain::formation::error::FormationError::Validation(
                        format!("commit error: {e}"),
                    )
                })?;

            Ok::<_, crate::domain::formation::error::FormationError>(contract)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

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

async fn get_signing_link(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<SigningLinkResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    // Verify the document exists in storage
    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            store.read_document("main", document_id).map_err(|_| {
                AppError::NotFound(format!("document {} not found", document_id))
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(SigningLinkResponse {
        document_id,
        signing_url: format!("/human/sign/{}", document_id),
    }))
}

async fn get_document_pdf(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<DocumentPdfResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    // Verify the document exists and return metadata-derived URL
    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            store.read_document("main", document_id).map_err(|_| {
                AppError::NotFound(format!("document {} not found", document_id))
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(DocumentPdfResponse {
        document_id: doc.document_id(),
        content_type: "application/pdf".to_owned(),
        url: format!(
            "/v1/documents/{}/pdf/download?entity_id={}&workspace_id={}",
            doc.document_id(),
            entity_id,
            workspace_id
        ),
    }))
}

#[derive(Deserialize)]
pub struct DocumentCopyRequest {
    pub workspace_id: WorkspaceId,
    pub entity_id: EntityId,
    #[serde(default)]
    pub recipient_email: Option<String>,
}

#[derive(Serialize)]
pub struct DocumentCopyResponse {
    pub document_id: DocumentId,
    pub request_id: String,
    pub status: String,
    pub title: String,
    pub recipient_email: Option<String>,
    pub created_at: String,
}

async fn request_document_copy(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Json(req): Json<DocumentCopyRequest>,
) -> Result<Json<DocumentCopyResponse>, AppError> {
    let workspace_id = req.workspace_id;
    let entity_id = req.entity_id;

    // Verify the document exists and read its title
    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            store.read_document("main", document_id).map_err(|_| {
                AppError::NotFound(format!("document {} not found", document_id))
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(DocumentCopyResponse {
        document_id,
        request_id: format!("req_{}", uuid::Uuid::new_v4()),
        status: "requested".to_owned(),
        title: doc.title().to_owned(),
        recipient_email: req.recipient_email,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

async fn get_amendment_history(
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<Vec<AmendmentHistoryEntry>>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let doc = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_formation_store(&layout, workspace_id, entity_id)?;
            store.read_document("main", document_id).map_err(|e| match e {
                crate::git::error::GitStorageError::NotFound(_) => {
                    crate::domain::formation::error::FormationError::DocumentNotFound(document_id)
                }
                other => crate::domain::formation::error::FormationError::Validation(
                    other.to_string(),
                ),
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    // Current version is the only entry (no amendment tracking yet)
    let entries = vec![AmendmentHistoryEntry {
        version: doc.version(),
        amended_at: doc.created_at().to_rfc3339(),
        description: "Original document".to_owned(),
    }];

    Ok(Json(entries))
}

// ── Governance documents ────────────────────────────────────────────

async fn list_governance_documents(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<DocumentSummary>>, AppError> {
    let workspace_id = query.workspace_id;

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(docs))
}

async fn get_current_governance_document(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<DocumentSummary>, AppError> {
    let workspace_id = query.workspace_id;

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
                    if doc_type.contains("articles") || doc_type.contains("bylaws") || doc_type.contains("operating_agreement") {
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
                .ok_or_else(|| crate::domain::formation::error::FormationError::Validation(
                    "no governance documents found".to_owned(),
                ))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(doc))
}

// ── Entity lifecycle ────────────────────────────────────────────────

async fn list_entities(
    State(state): State<AppState>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<FormationStatusResponse>>, AppError> {
    let workspace_id = query.workspace_id;

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
                            next_action,
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

    Ok(Json(entities))
}

#[derive(Deserialize)]
pub struct ConvertEntityRequest {
    pub target_type: EntityType,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

async fn convert_entity(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<ConvertEntityRequest>,
) -> Result<Json<FormationStatusResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    let next_action = service::next_formation_action(entity.formation_status()).map(String::from);

    Ok(Json(FormationStatusResponse {
        entity_id: entity.entity_id(),
        legal_name: entity.legal_name().to_owned(),
        entity_type: entity.entity_type(),
        jurisdiction: entity.jurisdiction().to_owned(),
        formation_state: entity.formation_state(),
        formation_status: entity.formation_status(),
        next_action,
    }))
}

#[derive(Deserialize)]
pub struct DissolveEntityRequest {
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

async fn dissolve_entity(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<DissolveEntityRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let reason = req.reason.unwrap_or_else(|| "voluntary dissolution".to_owned());

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
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(serde_json::json!({
        "entity_id": entity.entity_id(),
        "legal_name": entity.legal_name(),
        "status": "dissolved",
        "reason": reason,
    })))
}

// ── Router ──────────────────────────────────────────────────────────────

pub fn formation_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/formations", post(create_formation))
        .route("/v1/formations/{entity_id}", get(get_formation))
        .route(
            "/v1/formations/{entity_id}/documents",
            get(list_documents),
        )
        .route(
            "/v1/formations/{entity_id}/filing-confirmation",
            post(confirm_filing),
        )
        .route(
            "/v1/formations/{entity_id}/ein-confirmation",
            post(confirm_ein),
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
        // Entity lifecycle
        .route("/v1/entities", get(list_entities))
        .route(
            "/v1/entities/{entity_id}/convert",
            post(convert_entity),
        )
        .route(
            "/v1/entities/{entity_id}/dissolve",
            post(dissolve_entity),
        )
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
