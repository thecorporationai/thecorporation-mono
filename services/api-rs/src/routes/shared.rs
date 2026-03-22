//! Shared helpers used across route modules.
//!
//! Centralizes patterns that were previously copy-pasted into each route file.

use crate::domain::ids::EntityId;
use crate::error::AppError;
use crate::store::entity_store::EntityStore;
use crate::store::RepoLayout;

/// Open an entity store with mandatory entity-scope authorization.
///
/// This is the single canonical way to open an entity store in route handlers.
/// It replaces the `open_store` helpers that were copy-pasted across every
/// route module — some of which omitted the entity-scope check entirely.
///
/// When `allowed_entity_ids` is `Some`, the entity must be in the allowed set
/// or `AppError::Forbidden` is returned. Pass `auth.entity_ids()` from any
/// scoped auth extractor.
pub fn open_entity_store<'a>(
    layout: &'a RepoLayout,
    workspace_id: crate::domain::ids::WorkspaceId,
    entity_id: EntityId,
    allowed_entity_ids: Option<&[EntityId]>,
    valkey_client: Option<&redis::Client>,
) -> Result<EntityStore<'a>, AppError> {
    // Entity-scope authorization: if the token is scoped to specific entities,
    // verify this entity is in the allowed set.
    if let Some(ids) = allowed_entity_ids {
        if !ids.contains(&entity_id) {
            return Err(AppError::Forbidden(format!(
                "token is not authorized for entity {}",
                entity_id
            )));
        }
    }

    EntityStore::open(layout, workspace_id, entity_id, valkey_client).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {} not found", entity_id))
        }
        other => AppError::Internal(other.to_string()),
    })
}
