pub mod entity_store;
pub mod workspace_store;

use std::path::PathBuf;

use crate::domain::ids::{EntityId, WorkspaceId};

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
