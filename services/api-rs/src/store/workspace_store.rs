//! Workspace store — reads and writes workspace-scoped data to `_workspace.git`.

use serde::Serialize;

use crate::domain::auth::api_key::ApiKeyRecord;
use crate::domain::ids::{ApiKeyId, WorkspaceId};
use crate::git::commit::{commit_files, FileWrite};
use crate::git::error::GitStorageError;
use crate::git::repo::CorpRepo;

use super::RepoLayout;

/// A workspace metadata record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceRecord {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub created_at: String,
}

/// Operations on a workspace's git repo (`{data_dir}/{workspace_id}/_workspace.git`).
pub struct WorkspaceStore<'a> {
    repo: CorpRepo,
    workspace_id: WorkspaceId,
    layout: &'a RepoLayout,
}

impl<'a> WorkspaceStore<'a> {
    /// Initialize a new workspace repo.
    pub fn init(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        name: &str,
    ) -> Result<Self, GitStorageError> {
        let path = layout.workspace_repo_path(workspace_id);
        let repo = CorpRepo::init(&path, None)?;

        let record = WorkspaceRecord {
            workspace_id,
            name: name.to_owned(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let files = vec![FileWrite::json("workspace.json", &record)?];
        commit_files(&repo, "main", "Initialize workspace", &files, None)?;

        Ok(Self {
            repo,
            workspace_id,
            layout,
        })
    }

    /// Open an existing workspace repo.
    pub fn open(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
    ) -> Result<Self, GitStorageError> {
        let path = layout.workspace_repo_path(workspace_id);
        let repo = CorpRepo::open(&path)?;
        Ok(Self {
            repo,
            workspace_id,
            layout,
        })
    }

    /// Read workspace metadata.
    pub fn read_workspace(&self) -> Result<WorkspaceRecord, GitStorageError> {
        self.repo.read_json("main", "workspace.json")
    }

    /// Write any serializable value to a JSON path and commit it.
    pub fn write_json<T: Serialize>(
        &self,
        path: &str,
        value: &T,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let files = vec![FileWrite::json(path, value)?];
        commit_files(&self.repo, "main", message, &files, None)?;
        Ok(())
    }

    /// Read an API key record by ID.
    pub fn read_api_key(
        &self,
        key_id: ApiKeyId,
    ) -> Result<ApiKeyRecord, GitStorageError> {
        self.repo
            .read_json("main", &format!("api-keys/{}.json", key_id))
    }

    /// List all API key IDs.
    pub fn list_api_key_ids(&self) -> Result<Vec<ApiKeyId>, GitStorageError> {
        self.list_ids_in_dir("api-keys")
    }

    /// Delete an API key by overwriting with a tombstone marker.
    pub fn delete_api_key(&self, key_id: ApiKeyId) -> Result<(), GitStorageError> {
        let tombstone = serde_json::json!({ "deleted": true, "key_id": key_id });
        let files = vec![FileWrite::json(
            format!("api-keys/{}.json", key_id),
            &tombstone,
        )?];
        commit_files(
            &self.repo,
            "main",
            &format!("Revoke API key {key_id}"),
            &files,
            None,
        )?;
        Ok(())
    }

    /// Get the underlying repo.
    pub fn repo(&self) -> &CorpRepo {
        &self.repo
    }

    /// Get the workspace ID.
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    /// Read any deserializable JSON from a path.
    pub fn read_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, GitStorageError> {
        self.repo.read_json("main", path)
    }

    /// List UUID-style IDs from files in a directory (public).
    pub fn list_ids_in_dir_pub<T: std::str::FromStr>(
        &self,
        dir_path: &str,
    ) -> Result<Vec<T>, GitStorageError> {
        self.list_ids_in_dir(dir_path)
    }

    /// List subdirectory names in a directory (for non-UUID-keyed directories like `secrets/`).
    pub fn list_names_in_dir(&self, dir_path: &str) -> Result<Vec<String>, GitStorageError> {
        let entries = match self.repo.list_dir("main", dir_path) {
            Ok(entries) => entries,
            Err(GitStorageError::NotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        Ok(entries
            .into_iter()
            .filter(|(_, is_dir)| *is_dir)
            .map(|(name, _)| name)
            .collect())
    }

    /// Check if a path exists in the repo.
    pub fn path_exists(&self, path: &str) -> Result<bool, GitStorageError> {
        self.repo.path_exists("main", path)
    }

    /// List UUID-style IDs from files in a directory.
    fn list_ids_in_dir<T: std::str::FromStr>(
        &self,
        dir_path: &str,
    ) -> Result<Vec<T>, GitStorageError> {
        let entries = match self.repo.list_dir("main", dir_path) {
            Ok(entries) => entries,
            Err(GitStorageError::NotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut ids = Vec::new();
        for (name, is_dir) in entries {
            if is_dir {
                continue;
            }
            if let Some(uuid_str) = name.strip_suffix(".json") {
                if let Ok(id) = uuid_str.parse() {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
    }
}
