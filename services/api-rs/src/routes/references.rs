use axum::{Json, Router, extract::State, routing::post};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{AppState, validation::validate_max_len};
use crate::auth::Principal;
use crate::domain::auth::scopes::Scope;
use crate::domain::ids::EntityId;
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::git::error::GitStorageError;
use crate::store::entity_store::EntityStore;
use crate::store::workspace_store::WorkspaceStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceKind {
    Entity,
    Contact,
    ShareTransfer,
    Invoice,
    BankAccount,
    Payment,
    PayrollRun,
    Distribution,
    Reconciliation,
    TaxFiling,
    Deadline,
    Classification,
    Body,
    Meeting,
    Seat,
    AgendaItem,
    Resolution,
    Document,
    WorkItem,
    Agent,
    Valuation,
    SafeNote,
    Instrument,
    ShareClass,
    Round,
}

impl ReferenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Contact => "contact",
            Self::ShareTransfer => "share_transfer",
            Self::Invoice => "invoice",
            Self::BankAccount => "bank_account",
            Self::Payment => "payment",
            Self::PayrollRun => "payroll_run",
            Self::Distribution => "distribution",
            Self::Reconciliation => "reconciliation",
            Self::TaxFiling => "tax_filing",
            Self::Deadline => "deadline",
            Self::Classification => "classification",
            Self::Body => "body",
            Self::Meeting => "meeting",
            Self::Seat => "seat",
            Self::AgendaItem => "agenda_item",
            Self::Resolution => "resolution",
            Self::Document => "document",
            Self::WorkItem => "work_item",
            Self::Agent => "agent",
            Self::Valuation => "valuation",
            Self::SafeNote => "safe_note",
            Self::Instrument => "instrument",
            Self::ShareClass => "share_class",
            Self::Round => "round",
        }
    }

    fn is_entity_scoped(self) -> bool {
        !matches!(self, Self::Entity | Self::Agent)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReferenceHandleRecord {
    pub kind: ReferenceKind,
    pub resource_id: String,
    pub handle: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<EntityId>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SyncReferenceItem {
    pub resource_id: String,
    pub label: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SyncReferencesRequest {
    pub kind: ReferenceKind,
    #[serde(default)]
    pub entity_id: Option<EntityId>,
    pub items: Vec<SyncReferenceItem>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SyncReferencesResponse {
    pub references: Vec<ReferenceHandleRecord>,
}

const MAX_REFERENCE_LABEL_LEN: usize = 256;
const MAX_REFERENCE_RESOURCE_ID_LEN: usize = 128;
const MAX_HANDLE_SUFFIX_ATTEMPTS: usize = 128;

fn references_dir(kind: ReferenceKind) -> String {
    format!("references/{}", kind.as_str())
}

fn reference_path(kind: ReferenceKind, resource_id: &str) -> String {
    format!("{}/{}.json", references_dir(kind), resource_id)
}

fn slugify_handle(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !out.is_empty() && !last_dash {
            out.push('-');
            last_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        "ref".to_owned()
    } else {
        out
    }
}

fn allocate_handle(
    base_handle: &str,
    resource_id: &str,
    existing: &[ReferenceHandleRecord],
) -> Result<String, AppError> {
    if !existing
        .iter()
        .any(|record| record.handle == base_handle && record.resource_id != resource_id)
    {
        return Ok(base_handle.to_owned());
    }

    let mut index = 2usize;
    while index <= MAX_HANDLE_SUFFIX_ATTEMPTS {
        let candidate = format!("{base_handle}-{index}");
        if !existing
            .iter()
            .any(|record| record.handle == candidate && record.resource_id != resource_id)
        {
            return Ok(candidate);
        }
        index += 1;
    }

    let mut hasher = Sha256::new();
    hasher.update(resource_id.as_bytes());
    let digest = hex::encode(hasher.finalize());
    let candidate = format!("{base_handle}-{}", &digest[..8]);
    if !existing
        .iter()
        .any(|record| record.handle == candidate && record.resource_id != resource_id)
    {
        return Ok(candidate);
    }

    Err(AppError::BadRequest(format!(
        "unable to allocate a unique handle for resource {} after {} attempts and hashed fallback",
        resource_id, MAX_HANDLE_SUFFIX_ATTEMPTS
    )))
}

fn normalized_label(item: &SyncReferenceItem) -> Result<String, AppError> {
    let raw = if item.label.trim().is_empty() {
        item.resource_id.trim()
    } else {
        item.label.trim()
    };
    validate_max_len(raw, "label", MAX_REFERENCE_LABEL_LEN)?;
    Ok(raw.to_owned())
}

fn normalized_resource_id(item: &SyncReferenceItem) -> Result<String, AppError> {
    let resource_id = item.resource_id.trim();
    if resource_id.is_empty() {
        return Err(AppError::BadRequest(
            "reference resource_id must not be empty".to_owned(),
        ));
    }
    validate_max_len(resource_id, "resource_id", MAX_REFERENCE_RESOURCE_ID_LEN)?;
    if resource_id.contains('/')
        || resource_id.contains('\\')
        || resource_id.contains("..")
        || resource_id.chars().any(char::is_control)
    {
        return Err(AppError::BadRequest(
            "reference resource_id cannot contain path separators, parent traversals, or control characters".to_owned(),
        ));
    }
    Ok(resource_id.to_owned())
}

fn read_workspace_reference_records(
    store: &WorkspaceStore<'_>,
    kind: ReferenceKind,
) -> Result<Vec<ReferenceHandleRecord>, AppError> {
    let dir = references_dir(kind);
    let entries = match store.list_dir(&dir) {
        Ok(entries) => entries,
        Err(GitStorageError::NotFound(_)) => return Ok(Vec::new()),
        Err(err) => return Err(AppError::from(err)),
    };

    let mut out = Vec::new();
    for (name, is_dir) in entries {
        if is_dir || !name.ends_with(".json") {
            continue;
        }
        let path = format!("{dir}/{name}");
        let record: ReferenceHandleRecord =
            store.read_json(&path).map_err(AppError::from)?;
        out.push(record);
    }
    Ok(out)
}

fn read_entity_reference_records(
    store: &EntityStore<'_>,
    kind: ReferenceKind,
) -> Result<Vec<ReferenceHandleRecord>, AppError> {
    let dir = references_dir(kind);
    let entries = match store.list_dir("main", &dir) {
        Ok(entries) => entries,
        Err(GitStorageError::NotFound(_)) => return Ok(Vec::new()),
        Err(err) => return Err(AppError::from(err)),
    };

    let mut out = Vec::new();
    for (name, is_dir) in entries {
        if is_dir || !name.ends_with(".json") {
            continue;
        }
        let path = format!("{dir}/{name}");
        let record: ReferenceHandleRecord =
            store.read_json("main", &path).map_err(AppError::from)?;
        out.push(record);
    }
    Ok(out)
}

fn sync_workspace_reference_records(
    store: &WorkspaceStore<'_>,
    kind: ReferenceKind,
    items: &[SyncReferenceItem],
) -> Result<Vec<ReferenceHandleRecord>, AppError> {
    let mut existing = read_workspace_reference_records(store, kind)?;
    let mut files = Vec::new();
    let mut out = Vec::with_capacity(items.len());
    let now = Utc::now().to_rfc3339();

    for item in items {
        let resource_id = normalized_resource_id(item)?;
        let label = normalized_label(item)?;

        if let Some(existing_record) = existing
            .iter_mut()
            .find(|record| record.resource_id == resource_id)
        {
            if existing_record.label != label {
                // Handles are intentionally stable once issued; rename the label snapshot
                // without rewriting the persisted handle.
                existing_record.label = label.clone();
                existing_record.updated_at = now.clone();
                files.push(
                    FileWrite::json(reference_path(kind, &resource_id), existing_record)
                        .map_err(|err| AppError::Internal(err.to_string()))?,
                );
            }
            out.push(existing_record.clone());
            continue;
        }

        let base_handle = slugify_handle(&label);
        let handle = allocate_handle(&base_handle, &resource_id, &existing)?;
        let record = ReferenceHandleRecord {
            kind,
            resource_id: resource_id.to_owned(),
            handle,
            label,
            entity_id: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        files.push(
            FileWrite::json(reference_path(kind, &resource_id), &record)
                .map_err(|err| AppError::Internal(err.to_string()))?,
        );
        existing.push(record.clone());
        out.push(record);
    }

    if !files.is_empty() {
        store
            .commit_files(
                &format!("Sync {} reference handles", kind.as_str()),
                &files,
            )
            .map_err(AppError::from)?;
    }

    Ok(out)
}

fn sync_entity_reference_records(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    kind: ReferenceKind,
    items: &[SyncReferenceItem],
) -> Result<Vec<ReferenceHandleRecord>, AppError> {
    let mut existing = read_entity_reference_records(store, kind)?;
    let mut files = Vec::new();
    let mut out = Vec::with_capacity(items.len());
    let now = Utc::now().to_rfc3339();

    for item in items {
        let resource_id = normalized_resource_id(item)?;
        let label = normalized_label(item)?;

        if let Some(existing_record) = existing
            .iter_mut()
            .find(|record| record.resource_id == resource_id)
        {
            if existing_record.label != label {
                // Handles are intentionally stable once issued; rename the label snapshot
                // without rewriting the persisted handle.
                existing_record.label = label.clone();
                existing_record.updated_at = now.clone();
                files.push(
                    FileWrite::json(reference_path(kind, &resource_id), existing_record)
                        .map_err(|err| AppError::Internal(err.to_string()))?,
                );
            }
            out.push(existing_record.clone());
            continue;
        }

        let base_handle = slugify_handle(&label);
        let handle = allocate_handle(&base_handle, &resource_id, &existing)?;
        let record = ReferenceHandleRecord {
            kind,
            resource_id: resource_id.to_owned(),
            handle,
            label,
            entity_id: Some(entity_id),
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        files.push(
            FileWrite::json(reference_path(kind, &resource_id), &record)
                .map_err(|err| AppError::Internal(err.to_string()))?,
        );
        existing.push(record.clone());
        out.push(record);
    }

    if !files.is_empty() {
        store
            .commit(
                "main",
                &format!("Sync {} reference handles", kind.as_str()),
                files,
            )
            .map_err(AppError::from)?;
    }

    Ok(out)
}

#[utoipa::path(
    post,
    path = "/v1/references/sync",
    tag = "references",
    security(("bearer_auth" = [])),
    request_body = SyncReferencesRequest,
    responses((status = 200, description = "Reference handles ensured", body = SyncReferencesResponse)),
)]
async fn sync_references(
    auth: Principal,
    State(state): State<AppState>,
    Json(req): Json<SyncReferencesRequest>,
) -> Result<Json<SyncReferencesResponse>, AppError> {
    if req.items.is_empty() {
        return Err(AppError::BadRequest(
            "reference sync requires at least one item".to_owned(),
        ));
    }
    if req.items.len() > 500 {
        return Err(AppError::BadRequest(
            "reference sync supports at most 500 items per request".to_owned(),
        ));
    }

    let workspace_id = auth.workspace_id();
    state.enforce_creation_rate_limit("references.sync", workspace_id, 120, 60)?;
    let references = if req.kind.is_entity_scoped() {
        let entity_id = req.entity_id.ok_or_else(|| {
            AppError::BadRequest(format!(
                "entity_id is required for {} references",
                req.kind.as_str()
            ))
        })?;
        if !auth.allows_entity(entity_id) {
            return Err(AppError::Forbidden("entity access denied".to_owned()));
        }
        let store =
            EntityStore::open(&state.layout, workspace_id, entity_id, state.valkey_client.as_ref(), state.s3_backend.as_ref()).map_err(|err| match err {
                GitStorageError::RepoNotFound(_) => {
                    AppError::NotFound(format!("entity {} not found", entity_id))
                }
                other => AppError::Internal(other.to_string()),
            })?;
        sync_entity_reference_records(&store, entity_id, req.kind, &req.items)?
    } else {
        if req.entity_id.is_some() {
            return Err(AppError::BadRequest(format!(
                "entity_id is not valid for {} references",
                req.kind.as_str()
            )));
        }
        if !auth.scopes().has(Scope::Admin) {
            return Err(AppError::Forbidden(
                "admin scope required for workspace-scoped reference sync".to_owned(),
            ));
        }
        let store = WorkspaceStore::open(&state.layout, workspace_id, state.valkey_client.as_ref()).map_err(AppError::from)?;
        sync_workspace_reference_records(&store, req.kind, &req.items)?
    };

    Ok(Json(SyncReferencesResponse { references }))
}

pub fn references_routes() -> Router<AppState> {
    Router::new().route("/v1/references/sync", post(sync_references))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(sync_references),
    components(schemas(
        SyncReferenceItem,
        SyncReferencesRequest,
        SyncReferencesResponse,
        ReferenceHandleRecord,
        ReferenceKind
    ))
)]
pub struct ReferencesApi;

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{
        MAX_HANDLE_SUFFIX_ATTEMPTS, ReferenceHandleRecord, ReferenceKind, SyncReferenceItem,
        allocate_handle, normalized_label, normalized_resource_id, slugify_handle,
    };

    #[test]
    fn slugify_handle_normalizes_labels() {
        assert_eq!(slugify_handle("Board of Directors"), "board-of-directors");
        assert_eq!(slugify_handle(" Alice   Johnson "), "alice-johnson");
        assert_eq!(slugify_handle("!!!"), "ref");
    }

    #[test]
    fn allocate_handle_adds_numeric_suffix_for_conflicts() {
        let existing = vec![ReferenceHandleRecord {
            kind: ReferenceKind::Contact,
            resource_id: "abc".to_owned(),
            handle: "alice-johnson".to_owned(),
            label: "Alice Johnson".to_owned(),
            entity_id: None,
            created_at: "2026-03-11T00:00:00Z".to_owned(),
            updated_at: "2026-03-11T00:00:00Z".to_owned(),
        }];
        assert_eq!(
            allocate_handle("alice-johnson", "def", &existing).unwrap(),
            "alice-johnson-2"
        );
    }

    #[test]
    fn allocate_handle_errors_after_safety_cap() {
        let mut existing = Vec::with_capacity(MAX_HANDLE_SUFFIX_ATTEMPTS + 1);
        existing.push(ReferenceHandleRecord {
            kind: ReferenceKind::Contact,
            resource_id: "base".to_owned(),
            handle: "alice-johnson".to_owned(),
            label: "Alice Johnson".to_owned(),
            entity_id: None,
            created_at: "2026-03-11T00:00:00Z".to_owned(),
            updated_at: "2026-03-11T00:00:00Z".to_owned(),
        });
        for index in 2..=MAX_HANDLE_SUFFIX_ATTEMPTS {
            existing.push(ReferenceHandleRecord {
                kind: ReferenceKind::Contact,
                resource_id: format!("resource-{index}"),
                handle: format!("alice-johnson-{index}"),
                label: format!("Alice Johnson {index}"),
                entity_id: None,
                created_at: "2026-03-11T00:00:00Z".to_owned(),
                updated_at: "2026-03-11T00:00:00Z".to_owned(),
            });
        }
        let mut hasher = Sha256::new();
        hasher.update(b"overflow");
        let digest = hex::encode(hasher.finalize());
        existing.push(ReferenceHandleRecord {
            kind: ReferenceKind::Contact,
            resource_id: "hashed".to_owned(),
            handle: format!("alice-johnson-{}", &digest[..8]),
            label: "Alice Johnson Hash".to_owned(),
            entity_id: None,
            created_at: "2026-03-11T00:00:00Z".to_owned(),
            updated_at: "2026-03-11T00:00:00Z".to_owned(),
        });
        assert!(allocate_handle("alice-johnson", "overflow", &existing).is_err());
    }

    #[test]
    fn normalized_label_rejects_oversized_values() {
        let item = SyncReferenceItem {
            resource_id: "abc".to_owned(),
            label: "x".repeat(257),
        };
        assert!(normalized_label(&item).is_err());
    }

    #[test]
    fn normalized_resource_id_rejects_path_like_values() {
        let item = SyncReferenceItem {
            resource_id: "../etc/passwd".to_owned(),
            label: "Board".to_owned(),
        };
        assert!(normalized_resource_id(&item).is_err());
    }
}
