pub mod entity_store;
pub mod stored_entity;
pub mod workspace_store;

use std::path::PathBuf;

use crate::domain::ids::{EntityId, WorkspaceId};

/// Which storage backend to use at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackendKind {
    /// Local bare git repos on disk (libgit2).
    Git,
    /// Redis-protocol key-value store (Redis, Valkey, Dragonfly, etc.).
    Kv,
}

/// Manages the on-disk layout of git repos.
///
/// Layout:
///   {data_dir}/{workspace_id}/{entity_id}.git  — entity repos
///   {data_dir}/{workspace_id}/_workspace.git   — workspace repo
pub struct RepoLayout {
    data_dir: PathBuf,
}

impl RepoLayout {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn entity_repo_path(&self, workspace_id: WorkspaceId, entity_id: EntityId) -> PathBuf {
        self.data_dir
            .join(workspace_id.to_string())
            .join(format!("{}.git", entity_id))
    }

    pub fn workspace_repo_path(&self, workspace_id: WorkspaceId) -> PathBuf {
        self.data_dir
            .join(workspace_id.to_string())
            .join("_workspace.git")
    }

    /// List all workspace IDs by scanning the top-level data directory.
    pub fn list_workspace_ids(&self) -> Vec<WorkspaceId> {
        let mut ids = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.data_dir) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|t| t.is_dir()) {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if let Ok(id) = name_str.parse() {
                        ids.push(id);
                    }
                }
            }
        }
        ids
    }

    /// List all entity IDs in a workspace by scanning the filesystem for `{uuid}.git` dirs.
    pub fn list_entity_ids(&self, workspace_id: WorkspaceId) -> Vec<EntityId> {
        let ws_dir = self.data_dir.join(workspace_id.to_string());
        let mut ids = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&ws_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if let Some(uuid_str) = name_str.strip_suffix(".git") {
                    if uuid_str == "_workspace" {
                        continue;
                    }
                    if let Ok(id) = uuid_str.parse() {
                        ids.push(id);
                    }
                }
            }
        }
        ids
    }
}

/// List workspace IDs using either filesystem scan or Valkey.
pub fn list_workspace_ids(
    layout: &RepoLayout,
    backend: StorageBackendKind,
    valkey_client: Option<&redis::Client>,
) -> Result<Vec<WorkspaceId>, crate::git::error::GitStorageError> {
    match backend {
        StorageBackendKind::Git => Ok(layout.list_workspace_ids()),
        StorageBackendKind::Kv => {
            let client = valkey_client.expect("Redis-protocol client required for KV backend");
            let mut con = client
                .get_connection()
                .map_err(|e| crate::git::error::GitStorageError::Git(e.to_string()))?;
            let ids = corp_store::store::list_workspaces(&mut con)?
                .into_iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            Ok(ids)
        }
    }
}

/// List entity IDs in a workspace using either filesystem scan or Valkey.
pub fn list_entity_ids(
    layout: &RepoLayout,
    backend: StorageBackendKind,
    valkey_client: Option<&redis::Client>,
    workspace_id: WorkspaceId,
) -> Result<Vec<EntityId>, crate::git::error::GitStorageError> {
    match backend {
        StorageBackendKind::Git => Ok(layout.list_entity_ids(workspace_id)),
        StorageBackendKind::Kv => {
            let client = valkey_client.expect("Redis-protocol client required for KV backend");
            let mut con = client
                .get_connection()
                .map_err(|e| crate::git::error::GitStorageError::Git(e.to_string()))?;
            let ids = corp_store::store::list_entities(&mut con, &workspace_id.to_string())?
                .into_iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            Ok(ids)
        }
    }
}
