//! Formation API route handlers.
//!
//! Implements entity lifecycle management, document signing, state-filing,
//! and EIN confirmation for the corporate formation flow.
//!
//! ## Route map
//!
//! | Method | Path | Scope |
//! |--------|------|-------|
//! | POST   | `/entities` | `FormationCreate` |
//! | GET    | `/entities` | `FormationRead` |
//! | GET    | `/entities/{entity_id}` | `FormationRead` |
//! | POST   | `/entities/{entity_id}/dissolve` | `FormationCreate` |
//! | POST   | `/formations/{entity_id}/advance` | `FormationCreate` |
//! | GET    | `/formations/{entity_id}/documents` | `FormationRead` |
//! | GET    | `/formations/{entity_id}/documents/{document_id}` | `FormationRead` |
//! | GET    | `/formations/{entity_id}/documents/{document_id}/html` | `FormationRead` |
//! | POST   | `/documents/{document_id}/sign` | `FormationSign` |
//! | GET    | `/formations/{entity_id}/filing` | `FormationRead` |
//! | POST   | `/formations/{entity_id}/filing/confirm` | `FormationCreate` |
//! | GET    | `/formations/{entity_id}/tax` | `FormationRead` |
//! | POST   | `/formations/{entity_id}/tax/confirm-ein` | `FormationCreate` |

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use sha2::Digest;

use corp_auth::{RequireFormationCreate, RequireFormationRead, RequireFormationSign};
use corp_core::formation::{
    Document, DocumentStatus, DocumentType, Entity, EntityType, Filing, FilingType,
    FormationStatus, IrsTaxClassification, Jurisdiction, Signature, TaxProfile,
};
use corp_core::ids::{DocumentId, EntityId};
use corp_storage::entity_store::EntityStore;

use crate::error::AppError;
use crate::state::AppState;

// ── Governance AST template helpers ─────────────────────────────────────────

/// Load the embedded governance AST template for the given formation document type.
///
/// Templates are compiled into the binary via `include_str!` so deployment
/// requires no external file access.  Returns `None` for document types that
/// have no corresponding governance AST template (e.g. board resolutions).
fn load_formation_template(doc_type: &DocumentType) -> Option<serde_json::Value> {
    let json_str = match doc_type {
        DocumentType::CertificateOfIncorporation => {
            include_str!("../../../../governance/ast/documents/certificate-of-incorporation.json")
        }
        DocumentType::Bylaws => {
            include_str!("../../../../governance/ast/documents/bylaws.json")
        }
        DocumentType::IncorporatorAction => {
            include_str!("../../../../governance/ast/documents/incorporator-action.json")
        }
        DocumentType::ArticlesOfOrganization => {
            include_str!("../../../../governance/ast/documents/articles-of-organization.json")
        }
        DocumentType::OperatingAgreement => {
            include_str!("../../../../governance/ast/documents/operating-agreement.json")
        }
        _ => return None,
    };
    serde_json::from_str(json_str).ok()
}

/// Recursively walk a [`serde_json::Value`] tree and replace `{{key}}`
/// placeholders in every string value with the corresponding entry from `vars`.
///
/// Variables that are not present in `vars` are left as-is so they can be
/// filled in during later workflow steps (e.g. `{{registered_agent_address}}`).
fn substitute_variables(
    value: &mut serde_json::Value,
    vars: &std::collections::HashMap<String, String>,
) {
    match value {
        serde_json::Value::String(s) => {
            for (key, val) in vars {
                *s = s.replace(&format!("{{{{{}}}}}", key), val);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                substitute_variables(item, vars);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map {
                substitute_variables(v, vars);
            }
        }
        _ => {}
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

/// Build the formation sub-router.
///
/// Mount this under a prefix (e.g. `/v1`) in the top-level router.
pub fn routes() -> Router<AppState> {
    Router::new()
        // Entity CRUD
        .route("/entities", post(create_entity))
        .route("/entities", get(list_entities))
        .route("/entities/{entity_id}", get(get_entity))
        .route("/entities/{entity_id}/dissolve", post(dissolve_entity))
        // Formation flow
        .route("/formations/{entity_id}/advance", post(advance_formation))
        .route("/formations/{entity_id}/documents", get(list_documents))
        .route(
            "/formations/{entity_id}/documents/{document_id}",
            get(get_document),
        )
        .route(
            "/formations/{entity_id}/documents/{document_id}/html",
            get(render_document_html),
        )
        // Document signing
        .route("/documents/{document_id}/sign", post(sign_document))
        // Filing
        .route("/formations/{entity_id}/filing", get(get_filing))
        .route(
            "/formations/{entity_id}/filing/confirm",
            post(confirm_filing),
        )
        // Tax
        .route("/formations/{entity_id}/tax", get(get_tax_profile))
        .route("/formations/{entity_id}/tax/confirm-ein", post(confirm_ein))
}

// ── Request / response types ──────────────────────────────────────────────────

/// Request body for `POST /entities`.
#[derive(Debug, Deserialize)]
pub struct CreateEntityRequest {
    /// Full legal name of the entity (1–500 characters).
    pub legal_name: String,
    /// The legal form of the business (e.g. `"c_corp"`, `"llc"`).
    pub entity_type: EntityType,
    /// Two-letter US state code for the jurisdiction of formation (e.g. `"DE"`).
    pub jurisdiction: String,
}

/// Request body for `POST /documents/{document_id}/sign`.
#[derive(Debug, Deserialize)]
pub struct SignDocumentRequest {
    /// Full legal name of the signer.
    pub signer_name: String,
    /// Organizational role of the signer (e.g. `"CEO"`).
    pub signer_role: String,
    /// Email address of the signer — used for duplicate-signer detection.
    pub signer_email: String,
    /// Typed or drawn signature text.
    pub signature_text: String,
    /// Consent statement the signer acknowledged (stored for audit trail).
    pub consent_text: String,
    /// Optional SVG representation of a handwritten signature.
    pub signature_svg: Option<String>,
}

/// Request body for `POST /formations/{entity_id}/filing/confirm`.
#[derive(Debug, Deserialize)]
pub struct ConfirmFilingRequest {
    /// State-issued confirmation number, if received.
    pub confirmation_number: Option<String>,
}

/// Request body for `POST /formations/{entity_id}/tax/confirm-ein`.
#[derive(Debug, Deserialize)]
pub struct ConfirmEinRequest {
    /// IRS-assigned Employer Identification Number.
    ///
    /// Accepts either raw 9-digit form (`"123456789"`) or the hyphenated form
    /// (`"12-3456789"`).  Stored normalised to the hyphenated form.
    pub ein: String,
}

// ── Helper: open entity store + read entity ───────────────────────────────────

/// Open the entity store and read the [`Entity`] for `entity_id`.
///
/// Handles the common pattern of opening the store under the principal's
/// workspace and then reading the entity record.
async fn load_entity(
    state: &AppState,
    workspace_id: corp_core::ids::WorkspaceId,
    entity_id: EntityId,
) -> Result<(EntityStore, Entity), AppError> {
    let store = state.open_entity_store(workspace_id, entity_id).await?;
    let entity: Entity = store.read::<Entity>(entity_id, "main").await.map_err(|e| {
        use corp_storage::error::StorageError;
        match e {
            StorageError::NotFound(_) => {
                AppError::NotFound(format!("entity {} not found", entity_id))
            }
            other => AppError::Storage(other),
        }
    })?;
    Ok((store, entity))
}

// ── Entity CRUD ───────────────────────────────────────────────────────────────

/// `POST /entities` — create a new legal entity.
///
/// Creates an [`Entity`] in `Pending` formation status together with an
/// empty [`Filing`] and a `Pending`-EIN [`TaxProfile`].  All three records
/// are written to the entity's store in a single logical transaction.
async fn create_entity(
    RequireFormationCreate(principal): RequireFormationCreate,
    State(state): State<AppState>,
    Json(body): Json<CreateEntityRequest>,
) -> Result<Json<Entity>, AppError> {
    use crate::routes::validation::{validate_name, validate_jurisdiction};
    validate_name("legal_name", &body.legal_name)?;
    validate_jurisdiction(&body.jurisdiction)?;

    // Validate jurisdiction.
    let jurisdiction =
        Jurisdiction::new(&body.jurisdiction).map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Build the root entity.
    let entity = Entity::new(
        principal.workspace_id,
        body.legal_name,
        body.entity_type,
        jurisdiction.clone(),
    )
    .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let entity_id = entity.entity_id;

    // Initialise the store (creates the git repo / KV namespace).
    let store = state
        .init_entity_store(principal.workspace_id, entity_id)
        .await?;

    // Register this entity in the workspace's entity index so
    // list_entities / sign_document can discover it.
    let ws_store = state
        .init_or_open_workspace_store(principal.workspace_id)
        .await?;
    ws_store
        .register_entity(entity_id)
        .await
        .map_err(AppError::Storage)?;

    // Persist entity.
    store
        .write::<Entity>(&entity, entity_id, "main", "create entity")
        .await
        .map_err(AppError::Storage)?;

    // Create the associated Filing record.
    let filing_type = match body.entity_type {
        EntityType::CCorp => FilingType::CertificateOfIncorporation,
        EntityType::Llc => FilingType::CertificateOfFormation,
    };
    let filing = Filing::new(
        entity_id,
        principal.workspace_id,
        filing_type,
        jurisdiction.as_str(),
    );
    store
        .write::<Filing>(&filing, filing.filing_id, "main", "create filing")
        .await
        .map_err(AppError::Storage)?;

    // Create the associated TaxProfile.
    let classification = match body.entity_type {
        EntityType::CCorp => IrsTaxClassification::CCorporation,
        EntityType::Llc => IrsTaxClassification::DisregardedEntity,
    };
    let tax_profile = TaxProfile::new(entity_id, principal.workspace_id, classification);
    store
        .write::<TaxProfile>(
            &tax_profile,
            tax_profile.tax_profile_id,
            "main",
            "create tax profile",
        )
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(entity))
}

/// `GET /entities` — list all entities in the caller's workspace.
///
/// Iterates the workspace entity-ID index, opens each entity store, and reads
/// the root [`Entity`] record.  Entities whose stores cannot be opened (e.g.
/// partially-initialized) are silently skipped.
async fn list_entities(
    RequireFormationRead(principal): RequireFormationRead,
    State(state): State<AppState>,
) -> Result<Json<Vec<Entity>>, AppError> {
    // List entity IDs registered for this workspace.
    // On a fresh data directory the workspace store may not exist yet —
    // treat that as an empty entity list rather than an error.
    let workspace_store = match state.open_workspace_store(principal.workspace_id).await {
        Ok(ws) => ws,
        Err(AppError::NotFound(_)) => return Ok(Json(Vec::new())),
        Err(other) => return Err(other),
    };
    let entity_ids = workspace_store
        .list_entity_ids()
        .await
        .map_err(AppError::Storage)?;

    let mut entities = Vec::with_capacity(entity_ids.len());
    for entity_id in entity_ids {
        // Skip stores that fail to open (e.g. still being initialised).
        let Ok(store) = state
            .open_entity_store(principal.workspace_id, entity_id)
            .await
        else {
            continue;
        };
        if let Ok(entity) = store.read::<Entity>(entity_id, "main").await {
            entities.push(entity);
        }
    }

    Ok(Json(entities))
}

/// `GET /entities/{entity_id}` — fetch a single entity by ID.
async fn get_entity(
    RequireFormationRead(principal): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Entity>, AppError> {
    let (_store, entity) = load_entity(&state, principal.workspace_id, entity_id).await?;
    Ok(Json(entity))
}

/// `POST /entities/{entity_id}/dissolve` — dissolve a legal entity.
///
/// Sets the entity's `formation_status` to `Dissolved` and records today's
/// date as the effective dissolution date.
async fn dissolve_entity(
    RequireFormationCreate(principal): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Entity>, AppError> {
    let (store, mut entity) = load_entity(&state, principal.workspace_id, entity_id).await?;

    let today = Utc::now().date_naive();
    entity
        .dissolve(today)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    store
        .write::<Entity>(&entity, entity_id, "main", "dissolve entity")
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(entity))
}

// ── Formation flow ────────────────────────────────────────────────────────────

/// `POST /formations/{entity_id}/advance` — advance the entity's formation status.
///
/// Moves the entity one step forward along the happy-path FSM:
/// `Pending → DocumentsGenerated → … → Active`.
async fn advance_formation(
    RequireFormationCreate(principal): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Entity>, AppError> {
    let (store, mut entity) = load_entity(&state, principal.workspace_id, entity_id).await?;

    let previous_status = entity.formation_status;

    entity
        .advance_status()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    // When transitioning DocumentsGenerated → DocumentsSigned, verify all
    // documents have been signed before allowing the advance.
    if previous_status == FormationStatus::DocumentsGenerated {
        let documents: Vec<Document> = store
            .read_all::<Document>("main")
            .await
            .map_err(AppError::Storage)?;
        let unsigned = documents.iter().any(|d| d.status != DocumentStatus::Signed);
        if unsigned {
            return Err(AppError::BadRequest(
                "all formation documents must be signed before advancing".to_string(),
            ));
        }
    }

    // When transitioning Pending → DocumentsGenerated, auto-create the
    // standard formation documents so they can be signed.
    if previous_status == FormationStatus::Pending {
        let doc_types = match entity.entity_type {
            EntityType::CCorp => vec![
                (
                    DocumentType::CertificateOfIncorporation,
                    "Certificate of Incorporation",
                ),
                (DocumentType::Bylaws, "Bylaws"),
                (DocumentType::IncorporatorAction, "Action of Incorporator"),
            ],
            EntityType::Llc => vec![
                (
                    DocumentType::ArticlesOfOrganization,
                    "Articles of Organization",
                ),
                (DocumentType::OperatingAgreement, "Operating Agreement"),
            ],
        };
        // Build the variable map for template substitution.
        let mut vars = std::collections::HashMap::new();
        vars.insert("legal_name".to_string(), entity.legal_name.clone());
        vars.insert("entity_legal_name".to_string(), entity.legal_name.clone());
        vars.insert(
            "jurisdiction".to_string(),
            entity.jurisdiction.as_str().to_string(),
        );
        let today = Utc::now().format("%Y-%m-%d").to_string();
        vars.insert("effective_date".to_string(), today.clone());

        // Populate registered agent from entity data if available.
        if let Some(ref ra_name) = entity.registered_agent_name {
            vars.insert("registered_agent_name".to_string(), ra_name.clone());
        }
        if let Some(ref ra_addr) = entity.registered_agent_address {
            vars.insert("registered_agent_address".to_string(), ra_addr.clone());
        }

        // Common defaults for C-Corp formation.
        vars.insert("authorized_shares".to_string(), "10,000,000".to_string());
        vars.insert("par_value".to_string(), "$0.00001".to_string());
        vars.insert("board_size".to_string(), "1".to_string());
        vars.insert("fiscal_year_end".to_string(), "December 31".to_string());
        vars.insert("incorporator_name".to_string(), entity.legal_name.clone());
        vars.insert("principal_name".to_string(), entity.legal_name.clone());
        vars.insert("directors_list".to_string(), "As designated by the Incorporator".to_string());
        vars.insert("officers_list".to_string(), "As designated by the Board of Directors".to_string());
        vars.insert("founders_table".to_string(), "See cap table for founder allocations".to_string());
        // Use registered agent address as fallback for company/incorporator address
        // (common for newly formed entities). Left unsubstituted if no address on file.
        if let Some(ref ra_addr) = entity.registered_agent_address {
            vars.insert("company_address".to_string(), ra_addr.clone());
            vars.insert("incorporator_address".to_string(), ra_addr.clone());
        }

        for (doc_type, title) in doc_types {
            // Use the full governance AST template when available, falling
            // back to the minimal stub for unknown document types.
            let content = if let Some(mut template) = load_formation_template(&doc_type) {
                substitute_variables(&mut template, &vars);
                template
            } else {
                serde_json::json!({
                    "entity_name": entity.legal_name,
                    "entity_type": format!("{:?}", entity.entity_type),
                    "jurisdiction": entity.jurisdiction.as_str(),
                    "document_type": title,
                })
            };
            let content_bytes = serde_json::to_vec(&content).unwrap_or_default();
            let content_hash = format!("{:x}", sha2::Sha256::digest(&content_bytes));
            let doc = Document::new(
                entity_id,
                principal.workspace_id,
                doc_type,
                title.to_owned(),
                content,
                content_hash,
            );
            store
                .write::<Document>(&doc, doc.document_id, "main", &format!("create {}", title))
                .await
                .map_err(AppError::Storage)?;
        }
    }

    store
        .write::<Entity>(&entity, entity_id, "main", "advance formation status")
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(entity))
}

/// `GET /formations/{entity_id}/documents` — list all documents for an entity.
async fn list_documents(
    RequireFormationRead(principal): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Document>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;

    let documents: Vec<Document> = store
        .read_all::<Document>("main")
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(documents))
}

/// `GET /formations/{entity_id}/documents/{document_id}` — fetch a single document.
async fn get_document(
    RequireFormationRead(principal): RequireFormationRead,
    State(state): State<AppState>,
    Path((entity_id, document_id)): Path<(EntityId, DocumentId)>,
) -> Result<Json<Document>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;

    let document: Document = store
        .read::<Document>(document_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::NotFound(format!("document {} not found", document_id))
                }
                other => AppError::Storage(other),
            }
        })?;

    Ok(Json(document))
}

// ── Document HTML rendering ───────────────────────────────────────────────────

/// `GET /formations/{entity_id}/documents/{document_id}/html` — render document as HTML.
///
/// Loads the document from the entity store and walks the governance AST
/// `content` array, producing a full print-ready HTML page.  The output is
/// suitable for opening in a browser and printing to PDF via the browser's
/// native print dialog, or for piping through a headless renderer such as
/// `wkhtmltopdf`.
async fn render_document_html(
    RequireFormationRead(principal): RequireFormationRead,
    State(state): State<AppState>,
    Path((entity_id, document_id)): Path<(EntityId, DocumentId)>,
) -> Result<axum::response::Html<String>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;

    let document: Document = store
        .read::<Document>(document_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::NotFound(format!("document {} not found", document_id))
                }
                other => AppError::Storage(other),
            }
        })?;

    let html = render_document_to_html(&document);
    Ok(axum::response::Html(html))
}

/// Render a [`Document`] to a self-contained HTML page with print-ready CSS.
fn render_document_to_html(document: &Document) -> String {
    let mut body = String::with_capacity(8 * 1024);

    // Extract document-level fields from content.
    let title = document
        .content
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&document.title);
    let preamble = document.content.get("preamble").and_then(|v| v.as_str());

    body.push_str(&format!("<h1>{}</h1>\n", escape_html(title)));

    if let Some(pre) = preamble {
        body.push_str(&format!(
            "<p class=\"preamble\">{}</p>\n",
            inline_markup(pre)
        ));
    }

    // Walk the AST content array.
    if let Some(nodes) = document.content.get("content").and_then(|v| v.as_array()) {
        for node in nodes {
            render_ast_node(node, &mut body);
        }
    }

    // Render any collected signatures.
    if !document.signatures.is_empty() {
        body.push_str("<div class=\"signatures\">\n");
        body.push_str("<h2>Signatures</h2>\n");
        for sig in &document.signatures {
            render_signature(sig, &mut body);
        }
        body.push_str("</div>\n");
    }

    wrap_html_page(title, &body)
}

/// Render a single AST node to HTML, appending to `out`.
fn render_ast_node(node: &serde_json::Value, out: &mut String) {
    let node_type = match node.get("type").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return,
    };

    match node_type {
        "heading" => {
            let level = node
                .get("level")
                .and_then(|v| v.as_u64())
                .unwrap_or(2)
                .clamp(1, 6);
            let text = node.get("text").and_then(|v| v.as_str()).unwrap_or("");
            out.push_str(&format!(
                "<h{l}>{t}</h{l}>\n",
                l = level,
                t = inline_markup(text)
            ));
        }

        "paragraph" => {
            let text = node.get("text").and_then(|v| v.as_str()).unwrap_or("");
            // Preserve explicit newlines in the source as line breaks.
            let rendered = text
                .split('\n')
                .map(inline_markup)
                .collect::<Vec<_>>()
                .join("<br>\n");
            out.push_str(&format!("<p>{}</p>\n", rendered));
        }

        "unordered_list" => {
            if let Some(items) = node.get("items").and_then(|v| v.as_array()) {
                out.push_str("<ul>\n");
                for item in items {
                    if let Some(text) = item.as_str() {
                        out.push_str(&format!("  <li>{}</li>\n", inline_markup(text)));
                    }
                }
                out.push_str("</ul>\n");
            }
        }

        "ordered_list" => {
            if let Some(items) = node.get("items").and_then(|v| v.as_array()) {
                out.push_str("<ol>\n");
                for item in items {
                    if let Some(text) = item.as_str() {
                        out.push_str(&format!("  <li>{}</li>\n", inline_markup(text)));
                    }
                }
                out.push_str("</ol>\n");
            }
        }

        "table" => {
            let headers = node.get("headers").and_then(|v| v.as_array());
            let rows = node.get("rows").and_then(|v| v.as_array());
            out.push_str("<table>\n");
            if let Some(hdrs) = headers {
                out.push_str("  <thead><tr>\n");
                for h in hdrs {
                    let text = h.as_str().unwrap_or("");
                    out.push_str(&format!("    <th>{}</th>\n", inline_markup(text)));
                }
                out.push_str("  </tr></thead>\n");
            }
            if let Some(row_list) = rows {
                out.push_str("  <tbody>\n");
                for row in row_list {
                    if let Some(cells) = row.as_array() {
                        out.push_str("  <tr>\n");
                        for cell in cells {
                            let text = cell.as_str().unwrap_or("");
                            out.push_str(&format!("    <td>{}</td>\n", inline_markup(text)));
                        }
                        out.push_str("  </tr>\n");
                    }
                }
                out.push_str("  </tbody>\n");
            }
            out.push_str("</table>\n");
        }

        "note" => {
            let text = node.get("text").and_then(|v| v.as_str()).unwrap_or("");
            out.push_str(&format!(
                "<div class=\"note\"><strong>Note:</strong> {}</div>\n",
                inline_markup(text)
            ));
        }

        "signature_block" => {
            let role = node
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("Signer");
            out.push_str("<div class=\"signature-block\">\n");
            out.push_str("  <div class=\"signature-line\"></div>\n");
            out.push_str(&format!(
                "  <div class=\"signature-role\">{}</div>\n",
                escape_html(role)
            ));
            if let Some(fields) = node.get("fields").and_then(|v| v.as_array()) {
                for field in fields {
                    if let Some(name) = field.as_str() {
                        let label = match name {
                            "name" => "Name",
                            "title" => "Title",
                            "date" => "Date",
                            other => other,
                        };
                        out.push_str(&format!(
                            "  <div class=\"signature-field\">{}: _______________</div>\n",
                            escape_html(label)
                        ));
                    }
                }
            }
            out.push_str("</div>\n");
        }

        "horizontal_rule" => {
            out.push_str("<hr>\n");
        }

        "code_block" => {
            out.push_str("<pre><code>");
            if let Some(lines) = node.get("lines").and_then(|v| v.as_array()) {
                for (i, line) in lines.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    let text = line.as_str().unwrap_or("");
                    out.push_str(&escape_html(text));
                }
            }
            out.push_str("</code></pre>\n");
        }

        // Unknown node types are silently ignored.
        _ => {}
    }
}

/// Render a collected signature into HTML.
fn render_signature(sig: &Signature, out: &mut String) {
    out.push_str("<div class=\"signature-entry\">\n");

    // Render SVG if present.
    if let Some(ref svg) = sig.signature_svg {
        out.push_str(&format!("  <div class=\"signature-svg\">{}</div>\n", svg));
    } else {
        out.push_str(&format!(
            "  <div class=\"signature-text-display\">{}</div>\n",
            escape_html(&sig.signature_text)
        ));
    }

    out.push_str("  <div class=\"signature-line\"></div>\n");
    out.push_str(&format!(
        "  <div class=\"signature-meta\">{}, {}</div>\n",
        escape_html(&sig.signer_name),
        escape_html(&sig.signer_role)
    ));
    out.push_str(&format!(
        "  <div class=\"signature-date\">Signed: {}</div>\n",
        sig.signed_at.format("%Y-%m-%d %H:%M UTC")
    ));
    out.push_str("</div>\n");
}

/// Convert inline markup in a text string to HTML.
///
/// Handles:
/// - `**bold**` → `<strong>bold</strong>`
/// - `` `code` `` → `<code>code</code>`
///
/// Text is HTML-escaped before markup conversion.
fn inline_markup(text: &str) -> String {
    let escaped = escape_html(text);
    // Convert **bold** → <strong>bold</strong>
    let with_bold = bold_re(&escaped);
    // Convert `code` → <code>code</code>
    inline_code_re(&with_bold)
}

/// Replace `**text**` with `<strong>text</strong>`.
fn bold_re(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        if let Some(start) = rest.find("**") {
            result.push_str(&rest[..start]);
            let after_open = &rest[start + 2..];
            if let Some(end) = after_open.find("**") {
                result.push_str("<strong>");
                result.push_str(&after_open[..end]);
                result.push_str("</strong>");
                rest = &after_open[end + 2..];
            } else {
                // No closing **, output literally.
                result.push_str("**");
                rest = after_open;
            }
        } else {
            result.push_str(rest);
            break;
        }
    }
    result
}

/// Replace `` `text` `` with `<code>text</code>`.
fn inline_code_re(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        if let Some(start) = rest.find('`') {
            result.push_str(&rest[..start]);
            let after_open = &rest[start + 1..];
            if let Some(end) = after_open.find('`') {
                result.push_str("<code>");
                result.push_str(&after_open[..end]);
                result.push_str("</code>");
                rest = &after_open[end + 1..];
            } else {
                result.push('`');
                rest = after_open;
            }
        } else {
            result.push_str(rest);
            break;
        }
    }
    result
}

/// Minimal HTML entity escaping for safe text output.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Wrap a body fragment in a complete HTML page with print-ready CSS.
fn wrap_html_page(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
  @page {{
    size: letter;
    margin: 1in;
  }}
  body {{
    font-family: "Times New Roman", Times, Georgia, serif;
    font-size: 12pt;
    line-height: 1.5;
    color: #111;
    max-width: 7.5in;
    margin: 0 auto;
    padding: 1in;
  }}
  h1 {{
    text-align: center;
    font-size: 18pt;
    margin-bottom: 0.5em;
    page-break-after: avoid;
  }}
  h2 {{
    font-size: 14pt;
    margin-top: 1.5em;
    margin-bottom: 0.5em;
    page-break-after: avoid;
  }}
  h3 {{
    font-size: 13pt;
    margin-top: 1.2em;
    margin-bottom: 0.4em;
    page-break-after: avoid;
  }}
  h4, h5, h6 {{
    font-size: 12pt;
    margin-top: 1em;
    margin-bottom: 0.3em;
    page-break-after: avoid;
  }}
  p {{
    margin: 0.6em 0;
    text-align: justify;
  }}
  .preamble {{
    font-style: italic;
    margin-bottom: 1.5em;
  }}
  ul, ol {{
    margin: 0.6em 0;
    padding-left: 2em;
  }}
  li {{
    margin-bottom: 0.3em;
  }}
  table {{
    width: 100%;
    border-collapse: collapse;
    margin: 1em 0;
    font-size: 11pt;
  }}
  th, td {{
    border: 1px solid #333;
    padding: 6px 10px;
    text-align: left;
  }}
  th {{
    background: #f5f5f5;
    font-weight: bold;
  }}
  .note {{
    background: #fffde7;
    border-left: 4px solid #fbc02d;
    padding: 0.8em 1em;
    margin: 1em 0;
    font-size: 11pt;
  }}
  pre {{
    background: #f5f5f5;
    border: 1px solid #ddd;
    padding: 0.8em 1em;
    font-family: "Courier New", Courier, monospace;
    font-size: 10pt;
    overflow-x: auto;
    white-space: pre-wrap;
    page-break-inside: avoid;
  }}
  code {{
    font-family: "Courier New", Courier, monospace;
    font-size: 11pt;
    background: #f0f0f0;
    padding: 1px 4px;
    border-radius: 2px;
  }}
  pre code {{
    background: none;
    padding: 0;
    font-size: inherit;
  }}
  hr {{
    border: none;
    border-top: 1px solid #666;
    margin: 2em 0;
  }}
  .signature-block {{
    margin-top: 3em;
    page-break-inside: avoid;
  }}
  .signature-block .signature-line {{
    border-bottom: 1px solid #111;
    width: 60%;
    margin-bottom: 4px;
    height: 2em;
  }}
  .signature-block .signature-role {{
    font-weight: bold;
    margin-bottom: 0.3em;
  }}
  .signature-block .signature-field {{
    margin: 0.2em 0;
  }}
  .signatures {{
    margin-top: 3em;
    page-break-before: auto;
  }}
  .signature-entry {{
    margin-bottom: 2em;
    page-break-inside: avoid;
  }}
  .signature-entry .signature-line {{
    border-bottom: 1px solid #111;
    width: 60%;
    margin-bottom: 4px;
  }}
  .signature-entry .signature-svg {{
    max-width: 300px;
    max-height: 80px;
    margin-bottom: 4px;
  }}
  .signature-entry .signature-svg svg {{
    max-width: 100%;
    max-height: 80px;
  }}
  .signature-entry .signature-text-display {{
    font-family: "Brush Script MT", "Segoe Script", cursive;
    font-size: 18pt;
    margin-bottom: 4px;
  }}
  .signature-entry .signature-meta {{
    font-weight: bold;
  }}
  .signature-entry .signature-date {{
    font-size: 10pt;
    color: #555;
  }}
  @media print {{
    body {{
      padding: 0;
    }}
    .note {{
      background: none;
      border-left: 3px solid #999;
    }}
  }}
</style>
</head>
<body>
{body}
</body>
</html>"#,
        title = escape_html(title),
        body = body
    )
}

// ── Document signing ──────────────────────────────────────────────────────────

/// `POST /documents/{document_id}/sign` — apply a signature to a document.
///
/// The handler must locate which entity store holds the document.  It
/// searches all entities associated with the principal's workspace; the first
/// match wins.  For production use with large workspaces, consider storing a
/// `document_id → entity_id` index.
async fn sign_document(
    RequireFormationSign(principal): RequireFormationSign,
    State(state): State<AppState>,
    Path(document_id): Path<DocumentId>,
    Json(body): Json<SignDocumentRequest>,
) -> Result<Json<Document>, AppError> {
    if body.signer_name.trim().is_empty() {
        return Err(AppError::BadRequest("signer_name must not be empty".into()));
    }
    if body.signer_email.trim().is_empty() {
        return Err(AppError::BadRequest(
            "signer_email must not be empty".into(),
        ));
    }
    if body.consent_text.trim().is_empty() {
        return Err(AppError::BadRequest(
            "consent_text must not be empty".into(),
        ));
    }
    // Locate the entity that owns this document by scanning workspace entities.
    let workspace_store = state.open_workspace_store(principal.workspace_id).await?;
    let entity_ids = workspace_store
        .list_entity_ids()
        .await
        .map_err(AppError::Storage)?;

    for entity_id in entity_ids {
        let Ok(store) = state
            .open_entity_store(principal.workspace_id, entity_id)
            .await
        else {
            continue;
        };

        // Try to read the document from this store.
        match store.read::<Document>(document_id, "main").await {
            Ok(mut document) => {
                // Build the signature using the current content hash so the
                // document's integrity check passes.
                let signature = Signature::new(
                    document_id,
                    &body.signer_name,
                    &body.signer_role,
                    &body.signer_email,
                    &body.signature_text,
                    body.signature_svg.clone(),
                    document.content_hash.clone(),
                );

                // Sign with no required-signers list — transitions to Signed
                // immediately once any signature is applied.  Callers that
                // need multi-party signing should extend this with a required
                // signers record.
                document
                    .sign(signature, &[])
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;

                store
                    .write::<Document>(
                        &document,
                        document_id,
                        "main",
                        &format!("sign document {} by {}", document_id, body.signer_email),
                    )
                    .await
                    .map_err(AppError::Storage)?;

                return Ok(Json(document));
            }
            Err(corp_storage::error::StorageError::NotFound(_)) => {
                // Not in this entity's store — try the next one.
                continue;
            }
            Err(e) => return Err(AppError::Storage(e)),
        }
    }

    Err(AppError::NotFound(format!(
        "document {} not found in any entity store",
        document_id
    )))
}

// ── Filing ────────────────────────────────────────────────────────────────────

/// `GET /formations/{entity_id}/filing` — fetch the filing record for an entity.
async fn get_filing(
    RequireFormationRead(principal): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Filing>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;

    // Read the first (and normally only) filing.
    let filings: Vec<Filing> = store
        .read_all::<Filing>("main")
        .await
        .map_err(AppError::Storage)?;

    filings
        .into_iter()
        .next()
        .ok_or_else(|| AppError::NotFound(format!("no filing found for entity {}", entity_id)))
        .map(Json)
}

/// `POST /formations/{entity_id}/filing/confirm` — confirm state acceptance of the filing.
///
/// Records the confirmation number and transitions the filing to `Filed`.
async fn confirm_filing(
    RequireFormationCreate(principal): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<ConfirmFilingRequest>,
) -> Result<Json<Filing>, AppError> {
    let (store, entity) = load_entity(&state, principal.workspace_id, entity_id).await?;

    if entity.formation_status != FormationStatus::FilingSubmitted {
        return Err(AppError::BadRequest(format!(
            "entity must be in filing_submitted state to confirm filing, currently: {:?}",
            entity.formation_status
        )));
    }

    let mut filings: Vec<Filing> = store
        .read_all::<Filing>("main")
        .await
        .map_err(AppError::Storage)?;

    let filing = filings
        .iter_mut()
        .next()
        .ok_or_else(|| AppError::NotFound(format!("no filing found for entity {}", entity_id)))?;

    let confirmation = body
        .confirmation_number
        .clone()
        .unwrap_or_else(|| format!("CONF-{}", filing.filing_id));

    filing
        .confirm(confirmation, Utc::now())
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let filing_id = filing.filing_id;
    store
        .write::<Filing>(filing, filing_id, "main", "confirm filing")
        .await
        .map_err(AppError::Storage)?;

    // Return the updated filing by value (clone from the vec).
    let updated = store
        .read::<Filing>(filing_id, "main")
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(updated))
}

// ── Tax ───────────────────────────────────────────────────────────────────────

/// `GET /formations/{entity_id}/tax` — fetch the tax profile for an entity.
async fn get_tax_profile(
    RequireFormationRead(principal): RequireFormationRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<TaxProfile>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;

    let profiles: Vec<TaxProfile> = store
        .read_all::<TaxProfile>("main")
        .await
        .map_err(AppError::Storage)?;

    profiles
        .into_iter()
        .next()
        .ok_or_else(|| AppError::NotFound(format!("no tax profile found for entity {}", entity_id)))
        .map(Json)
}

/// `POST /formations/{entity_id}/tax/confirm-ein` — record an IRS-assigned EIN.
///
/// Activates the entity's EIN and transitions `ein_status` to `Active`.
async fn confirm_ein(
    RequireFormationCreate(principal): RequireFormationCreate,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<ConfirmEinRequest>,
) -> Result<Json<TaxProfile>, AppError> {
    let (store, entity) = load_entity(&state, principal.workspace_id, entity_id).await?;

    if entity.formation_status != FormationStatus::EinApplied {
        return Err(AppError::BadRequest(format!(
            "entity must be in ein_applied state to confirm EIN, currently: {:?}",
            entity.formation_status
        )));
    }

    let mut profiles: Vec<TaxProfile> = store
        .read_all::<TaxProfile>("main")
        .await
        .map_err(AppError::Storage)?;

    let profile = profiles.iter_mut().next().ok_or_else(|| {
        AppError::NotFound(format!("no tax profile found for entity {}", entity_id))
    })?;

    profile
        .assign_ein(&body.ein)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let tax_profile_id = profile.tax_profile_id;
    store
        .write::<TaxProfile>(profile, tax_profile_id, "main", "confirm EIN")
        .await
        .map_err(AppError::Storage)?;

    let updated = store
        .read::<TaxProfile>(tax_profile_id, "main")
        .await
        .map_err(AppError::Storage)?;

    Ok(Json(updated))
}
