//! Contacts route handlers.
//!
//! ## Route map
//!
//! | Method | Path | Scope |
//! |--------|------|-------|
//! | GET    | `/entities/{entity_id}/contacts` | `ContactsRead` |
//! | POST   | `/entities/{entity_id}/contacts` | `ContactsWrite` |
//! | GET    | `/entities/{entity_id}/contacts/{contact_id}` | `ContactsRead` |
//! | PATCH  | `/entities/{entity_id}/contacts/{contact_id}` | `ContactsWrite` |
//! | POST   | `/entities/{entity_id}/contacts/{contact_id}/deactivate` | `ContactsWrite` |

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use corp_auth::{RequireContactsRead, RequireContactsWrite};
use corp_core::contacts::{CapTableAccess, Contact, ContactCategory, ContactType};
use corp_core::ids::{ContactId, EntityId};
use crate::error::AppError;
use crate::state::AppState;

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/entities/{entity_id}/contacts",
            get(list_contacts).post(create_contact),
        )
        .route(
            "/entities/{entity_id}/contacts/{contact_id}",
            get(get_contact).patch(update_contact),
        )
        .route(
            "/entities/{entity_id}/contacts/{contact_id}/deactivate",
            post(deactivate_contact),
        )
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateContactRequest {
    pub contact_type: ContactType,
    pub name: String,
    pub category: ContactCategory,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mailing_address: Option<String>,
    pub cap_table_access: Option<CapTableAccess>,
    pub notes: Option<String>,
}

/// All fields are optional; only those present are applied to the stored record.
#[derive(Debug, Deserialize)]
pub struct UpdateContactRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mailing_address: Option<String>,
    pub category: Option<ContactCategory>,
    pub cap_table_access: Option<CapTableAccess>,
    pub notes: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn list_contacts(
    RequireContactsRead(principal): RequireContactsRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Contact>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let contacts = store
        .read_all::<Contact>("main")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(contacts))
}

async fn create_contact(
    RequireContactsWrite(principal): RequireContactsWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateContactRequest>,
) -> Result<Json<Contact>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;

    let mut contact = Contact::new(
        entity_id,
        principal.workspace_id,
        body.contact_type,
        body.name,
        body.category,
    )
    .map_err(|e| AppError::BadRequest(e.to_string()))?;

    contact.set_email(body.email);
    contact.set_phone(body.phone);
    contact.set_mailing_address(body.mailing_address);
    if let Some(access) = body.cap_table_access {
        contact.set_cap_table_access(access);
    }
    contact.set_notes(body.notes);

    store
        .write::<Contact>(&contact, contact.contact_id, "main", "create contact")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(contact))
}

async fn get_contact(
    RequireContactsRead(principal): RequireContactsRead,
    State(state): State<AppState>,
    Path((entity_id, contact_id)): Path<(EntityId, ContactId)>,
) -> Result<Json<Contact>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let contact = store
        .read::<Contact>(contact_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::NotFound(format!("contact {} not found", contact_id))
                }
                other => AppError::Storage(other),
            }
        })?;
    Ok(Json(contact))
}

async fn update_contact(
    RequireContactsWrite(principal): RequireContactsWrite,
    State(state): State<AppState>,
    Path((entity_id, contact_id)): Path<(EntityId, ContactId)>,
    Json(body): Json<UpdateContactRequest>,
) -> Result<Json<Contact>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut contact = store
        .read::<Contact>(contact_id, "main")
        .await
        .map_err(AppError::Storage)?;

    if let Some(name) = body.name {
        contact
            .set_name(name)
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
    }
    if let Some(email) = body.email {
        contact.set_email(Some(email));
    }
    if let Some(phone) = body.phone {
        contact.set_phone(Some(phone));
    }
    if let Some(addr) = body.mailing_address {
        contact.set_mailing_address(Some(addr));
    }
    if let Some(category) = body.category {
        contact.set_category(category);
    }
    if let Some(access) = body.cap_table_access {
        contact.set_cap_table_access(access);
    }
    if let Some(notes) = body.notes {
        contact.set_notes(Some(notes));
    }

    store
        .write::<Contact>(&contact, contact_id, "main", "update contact")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(contact))
}

async fn deactivate_contact(
    RequireContactsWrite(principal): RequireContactsWrite,
    State(state): State<AppState>,
    Path((entity_id, contact_id)): Path<(EntityId, ContactId)>,
) -> Result<Json<Contact>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut contact = store
        .read::<Contact>(contact_id, "main")
        .await
        .map_err(AppError::Storage)?;
    contact
        .deactivate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Contact>(&contact, contact_id, "main", "deactivate contact")
        .await
        .map_err(AppError::Storage)?;
    Ok(Json(contact))
}
