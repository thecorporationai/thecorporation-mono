//! Contacts HTTP routes.
//!
//! Endpoints for creating and listing contacts.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use super::AppState;
use super::validation::{reject_blank_optional, require_non_empty_trimmed, validate_max_len};
use crate::auth::{RequireContactsRead, RequireContactsWrite};
use crate::domain::contacts::{
    contact::Contact,
    types::{CapTableAccess, ContactCategory, ContactStatus, ContactType},
};
use crate::domain::formation::types::FormationStatus;
use crate::domain::ids::{ContactId, EntityId, WorkspaceId};
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateContactRequest {
    pub entity_id: EntityId,
    pub contact_type: ContactType,
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub mailing_address: Option<String>,
    pub category: ContactCategory,
    #[serde(default)]
    pub cap_table_access: Option<CapTableAccess>,
    #[serde(default)]
    pub notes: Option<String>,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct ContactResponse {
    pub contact_id: ContactId,
    pub entity_id: EntityId,
    pub contact_type: ContactType,
    pub name: String,
    pub email: Option<String>,
    pub mailing_address: Option<String>,
    pub phone: Option<String>,
    pub category: ContactCategory,
    pub cap_table_access: CapTableAccess,
    pub notes: Option<String>,
    pub status: ContactStatus,
    pub created_at: String,
}

// ── Conversion helpers ───────────────────────────────────────────────

fn contact_to_response(c: &Contact) -> ContactResponse {
    ContactResponse {
        contact_id: c.contact_id(),
        entity_id: c.entity_id(),
        contact_type: c.contact_type(),
        name: c.name().to_owned(),
        email: c.email().map(|s| s.to_owned()),
        mailing_address: c.mailing_address().map(|s| s.to_owned()),
        phone: c.phone().map(|s| s.to_owned()),
        category: c.category(),
        cap_table_access: c.cap_table_access(),
        notes: c.notes().map(|s| s.to_owned()),
        status: c.status(),
        created_at: c.created_at().to_rfc3339(),
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

fn ensure_entity_allows_contact_mutation(
    store: &EntityStore<'_>,
    action: &str,
) -> Result<(), AppError> {
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    if entity.formation_status() == FormationStatus::Dissolved {
        return Err(AppError::BadRequest(format!(
            "{action} is not allowed for dissolved entities"
        )));
    }
    Ok(())
}

fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

fn validate_email(email: &str) -> Result<String, AppError> {
    let normalized = normalize_email(email);
    if normalized.is_empty()
        || normalized.chars().any(char::is_whitespace)
        || normalized.contains(',')
        || normalized.matches('@').count() != 1
    {
        return Err(AppError::BadRequest(
            "email must be a valid address".to_owned(),
        ));
    }
    let (local, domain) = normalized
        .split_once('@')
        .ok_or_else(|| AppError::BadRequest("email must be a valid address".to_owned()))?;
    if local.is_empty()
        || domain.is_empty()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.contains("..")
    {
        return Err(AppError::BadRequest(
            "email must be a valid address".to_owned(),
        ));
    }
    Ok(normalized)
}

// ── Handlers ─────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/contacts",
    tag = "contacts",
    request_body = CreateContactRequest,
    responses(
        (status = 200, description = "Contact created", body = ContactResponse),
    ),
)]
async fn create_contact(
    RequireContactsWrite(auth): RequireContactsWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateContactRequest>,
) -> Result<Json<ContactResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    state.enforce_creation_rate_limit("contacts.create", workspace_id, 250, 60)?;
    reject_blank_optional(req.mailing_address.as_deref(), "mailing_address")?;
    if let Some(ref addr) = req.mailing_address {
        validate_max_len(addr, "mailing_address", 1000)?;
    }
    let name = require_non_empty_trimmed(&req.name, "name")?;
    validate_max_len(&name, "name", 256)?;
    if let Some(ref email) = req.email {
        validate_max_len(email, "email", 320)?;
    }
    let normalized_email = req.email.as_deref().map(validate_email).transpose()?;

    let contact = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let name = name.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            ensure_entity_allows_contact_mutation(&store, "contact creation")?;
            let contact_ids = store
                .list_ids::<Contact>("main")
                .map_err(|e| AppError::Internal(format!("list contacts: {e}")))?;
            for existing_id in contact_ids {
                let existing = store
                    .read::<Contact>("main", existing_id)
                    .map_err(|e| AppError::Internal(format!("read contact {existing_id}: {e}")))?;
                if let (Some(existing_email), Some(new_email)) =
                    (existing.email(), normalized_email.as_deref())
                    && normalize_email(existing_email) == new_email
                {
                    return Err(AppError::Conflict(format!(
                        "contact email already exists for entity: {new_email}"
                    )));
                }
            }

            let contact_id = ContactId::new();
            let mut contact = Contact::new(
                contact_id,
                entity_id,
                workspace_id,
                req.contact_type,
                name,
                normalized_email,
                req.category,
            )
            .map_err(AppError::BadRequest)?;
            if let Some(access) = req.cap_table_access {
                contact.set_cap_table_access(access);
            }
            if let Some(mailing_address) = req.mailing_address
                && !mailing_address.trim().is_empty()
            {
                contact.set_mailing_address(Some(mailing_address));
            }
            if let Some(notes) = req.notes
                && !notes.trim().is_empty()
            {
                contact.set_notes(notes);
            }

            let path = format!("contacts/{}.json", contact_id);
            store
                .write_json(
                    "main",
                    &path,
                    &contact,
                    &format!("Create contact {contact_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(contact)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(contact_to_response(&contact)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/contacts",
    tag = "contacts",
    responses(
        (status = 200, description = "List of contacts for entity", body = Vec<ContactResponse>),
    ),
)]
async fn list_contacts(
    RequireContactsRead(auth): RequireContactsRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ContactResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let contacts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store
                .list_ids::<Contact>("main")
                .map_err(|e| AppError::Internal(format!("list contacts: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let c = store
                    .read::<Contact>("main", id)
                    .map_err(|e| AppError::Internal(format!("read contact {id}: {e}")))?;
                results.push(contact_to_response(&c));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(contacts))
}

// ── Extended contact handlers ────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/contacts/{contact_id}",
    tag = "contacts",
    params(
        ("contact_id" = ContactId, Path, description = "Contact ID"),
    ),
    responses(
        (status = 200, description = "Contact details", body = ContactResponse),
        (status = 404, description = "Contact not found"),
    ),
)]
async fn get_contact(
    RequireContactsRead(auth): RequireContactsRead,
    State(state): State<AppState>,
    Path(contact_id): Path<ContactId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ContactResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let contact = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            store
                .read::<Contact>("main", contact_id)
                .map_err(|_| AppError::NotFound(format!("contact {} not found", contact_id)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(contact_to_response(&contact)))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateContactRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub mailing_address: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub category: Option<ContactCategory>,
    #[serde(default)]
    pub cap_table_access: Option<CapTableAccess>,
}

#[utoipa::path(
    patch,
    path = "/v1/contacts/{contact_id}",
    tag = "contacts",
    params(
        ("contact_id" = ContactId, Path, description = "Contact ID"),
    ),
    request_body = UpdateContactRequest,
    responses(
        (status = 200, description = "Contact updated", body = ContactResponse),
        (status = 404, description = "Contact not found"),
    ),
)]
async fn update_contact(
    RequireContactsWrite(auth): RequireContactsWrite,
    State(state): State<AppState>,
    Path(contact_id): Path<ContactId>,
    Json(req): Json<UpdateContactRequest>,
) -> Result<Json<ContactResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }
    let name = req
        .name
        .as_deref()
        .map(|value| require_non_empty_trimmed(value, "name"))
        .transpose()?;
    if req.email.as_deref().is_some_and(|s| s.trim().is_empty()) {
        return Err(AppError::BadRequest("email cannot be empty".to_owned()));
    }
    reject_blank_optional(req.mailing_address.as_deref(), "mailing_address")?;
    let normalized_email = req.email.as_deref().map(validate_email).transpose()?;

    let contact = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let name = name.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            ensure_entity_allows_contact_mutation(&store, "contact updates")?;
            let mut contact = store
                .read::<Contact>("main", contact_id)
                .map_err(|_| AppError::NotFound(format!("contact {} not found", contact_id)))?;

            if let Some(new_email) = normalized_email.as_deref() {
                let contact_ids = store
                    .list_ids::<Contact>("main")
                    .map_err(|e| AppError::Internal(format!("list contacts: {e}")))?;
                for existing_id in contact_ids {
                    if existing_id == contact_id {
                        continue;
                    }
                    let existing = store.read::<Contact>("main", existing_id).map_err(|e| {
                        AppError::Internal(format!("read contact {existing_id}: {e}"))
                    })?;
                    if existing
                        .email()
                        .is_some_and(|email| normalize_email(email) == new_email)
                    {
                        return Err(AppError::Conflict(format!(
                            "contact email already exists for entity: {new_email}"
                        )));
                    }
                }
            }

            if let Some(name) = name {
                contact.set_name(name).map_err(AppError::BadRequest)?;
            }
            if let Some(email) = normalized_email {
                contact.set_email(Some(email));
            }
            if let Some(mailing_address) = req.mailing_address {
                contact.set_mailing_address(Some(mailing_address));
            }
            if let Some(phone) = req.phone {
                contact.set_phone(phone);
            }
            if let Some(notes) = req.notes {
                contact.set_notes(notes);
            }
            if let Some(category) = req.category {
                contact.set_category(category);
            }
            if let Some(access) = req.cap_table_access {
                contact.set_cap_table_access(access);
            }

            let path = format!("contacts/{}.json", contact_id);
            store
                .write_json(
                    "main",
                    &path,
                    &contact,
                    &format!("Update contact {contact_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(contact)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(contact_to_response(&contact)))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ContactProfileResponse {
    pub contact_id: ContactId,
    pub name: String,
    pub email: Option<String>,
    pub mailing_address: Option<String>,
    pub phone: Option<String>,
    pub category: ContactCategory,
    pub notes: Option<String>,
    pub entities: Vec<EntityId>,
}

#[utoipa::path(
    get,
    path = "/v1/contacts/{contact_id}/profile",
    tag = "contacts",
    params(
        ("contact_id" = ContactId, Path, description = "Contact ID"),
    ),
    responses(
        (status = 200, description = "Contact profile", body = ContactProfileResponse),
        (status = 404, description = "Contact not found"),
    ),
)]
async fn get_contact_profile(
    RequireContactsRead(auth): RequireContactsRead,
    State(state): State<AppState>,
    Path(contact_id): Path<ContactId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<ContactProfileResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let contact = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            store
                .read::<Contact>("main", contact_id)
                .map_err(|_| AppError::NotFound(format!("contact {} not found", contact_id)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(ContactProfileResponse {
        contact_id: contact.contact_id(),
        name: contact.name().to_owned(),
        email: contact.email().map(|s| s.to_owned()),
        mailing_address: contact.mailing_address().map(|s| s.to_owned()),
        phone: contact.phone().map(|s| s.to_owned()),
        category: contact.category(),
        notes: contact.notes().map(|s| s.to_owned()),
        entities: vec![contact.entity_id()],
    }))
}

use crate::domain::contacts::notification_prefs::NotificationPrefs as NotifPrefsRecord;

#[derive(Serialize, utoipa::ToSchema)]
pub struct NotificationPrefsResponse {
    pub contact_id: ContactId,
    pub email_enabled: bool,
    pub sms_enabled: bool,
    pub webhook_enabled: bool,
    pub updated_at: String,
}

fn prefs_to_response(p: &NotifPrefsRecord) -> NotificationPrefsResponse {
    NotificationPrefsResponse {
        contact_id: p.contact_id(),
        email_enabled: p.email_enabled(),
        sms_enabled: p.sms_enabled(),
        webhook_enabled: p.webhook_enabled(),
        updated_at: p.updated_at().to_rfc3339(),
    }
}

#[utoipa::path(
    get,
    path = "/v1/contacts/{contact_id}/notification-prefs",
    tag = "contacts",
    params(
        ("contact_id" = ContactId, Path, description = "Contact ID"),
    ),
    responses(
        (status = 200, description = "Notification preferences", body = NotificationPrefsResponse),
        (status = 404, description = "Contact not found"),
    ),
)]
async fn get_notification_prefs(
    RequireContactsRead(auth): RequireContactsRead,
    State(state): State<AppState>,
    Path(contact_id): Path<ContactId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<NotificationPrefsResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = query.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let prefs = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Try to read existing prefs; create defaults if not found
            let path = format!("contacts/{}/notification-prefs.json", contact_id);
            match store.read_json::<NotifPrefsRecord>("main", &path) {
                Ok(p) => Ok::<_, AppError>(p),
                Err(_) => {
                    // Verify the contact exists first
                    store.read::<Contact>("main", contact_id).map_err(|_| {
                        AppError::NotFound(format!("contact {} not found", contact_id))
                    })?;
                    let p = NotifPrefsRecord::new(contact_id);
                    store
                        .write_json(
                            "main",
                            &path,
                            &p,
                            &format!("Init notification prefs for {contact_id}"),
                        )
                        .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
                    Ok(p)
                }
            }
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(prefs_to_response(&prefs)))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateNotificationPrefsRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub email_enabled: Option<bool>,
    #[serde(default)]
    pub sms_enabled: Option<bool>,
    #[serde(default)]
    pub webhook_enabled: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/v1/contacts/{contact_id}/notification-prefs",
    tag = "contacts",
    params(
        ("contact_id" = ContactId, Path, description = "Contact ID"),
    ),
    request_body = UpdateNotificationPrefsRequest,
    responses(
        (status = 200, description = "Notification preferences updated", body = NotificationPrefsResponse),
        (status = 404, description = "Contact not found"),
    ),
)]
async fn update_notification_prefs(
    RequireContactsWrite(auth): RequireContactsWrite,
    State(state): State<AppState>,
    Path(contact_id): Path<ContactId>,
    Json(req): Json<UpdateNotificationPrefsRequest>,
) -> Result<Json<NotificationPrefsResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let prefs = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            ensure_entity_allows_contact_mutation(&store, "notification preference updates")?;
            let path = format!("contacts/{}/notification-prefs.json", contact_id);

            // Read existing or create defaults
            let mut prefs = match store.read_json::<NotifPrefsRecord>("main", &path) {
                Ok(p) => p,
                Err(_) => {
                    store.read::<Contact>("main", contact_id).map_err(|_| {
                        AppError::NotFound(format!("contact {} not found", contact_id))
                    })?;
                    NotifPrefsRecord::new(contact_id)
                }
            };

            if let Some(v) = req.email_enabled {
                prefs.set_email_enabled(v);
            }
            if let Some(v) = req.sms_enabled {
                prefs.set_sms_enabled(v);
            }
            if let Some(v) = req.webhook_enabled {
                prefs.set_webhook_enabled(v);
            }

            store
                .write_json(
                    "main",
                    &path,
                    &prefs,
                    &format!("Update notification prefs for {contact_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;

            Ok::<_, AppError>(prefs)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(prefs_to_response(&prefs)))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn contacts_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/contacts", post(create_contact))
        .route(
            "/v1/contacts/{contact_id}",
            get(get_contact).patch(update_contact),
        )
        .route(
            "/v1/contacts/{contact_id}/profile",
            get(get_contact_profile),
        )
        .route(
            "/v1/contacts/{contact_id}/notification-prefs",
            get(get_notification_prefs).patch(update_notification_prefs),
        )
        .route("/v1/entities/{entity_id}/contacts", get(list_contacts))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create_contact,
        list_contacts,
        get_contact,
        update_contact,
        get_contact_profile,
        get_notification_prefs,
        update_notification_prefs,
    ),
    components(schemas(
        CreateContactRequest,
        ContactResponse,
        UpdateContactRequest,
        ContactProfileResponse,
        NotificationPrefsResponse,
        UpdateNotificationPrefsRequest,
    ))
)]
pub struct ContactsApi;
