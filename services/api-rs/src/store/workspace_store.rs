//! Workspace store — reads and writes workspace-scoped data to `_workspace.git` or Valkey.

use std::cell::RefCell;
use std::rc::Rc;

use serde::Serialize;

use crate::domain::auth::api_key::ApiKeyRecord;
use crate::domain::auth::ssh_key::SshKeyRecord;
use crate::domain::ids::{ApiKeyId, SshKeyId, WorkspaceId};
use crate::git::commit::{FileWrite, commit_files};
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

/// Internal backend discriminant.
enum Backend {
    Git {
        repo: CorpRepo,
    },
    Valkey {
        con: Rc<RefCell<redis::Connection>>,
        ws: String,
    },
}

/// The Valkey entity name for workspace repos (matches `_workspace.git` convention).
const VALKEY_WORKSPACE_ENTITY: &str = "_workspace";

/// Operations on a workspace's storage (`{data_dir}/{workspace_id}/_workspace.git` or Valkey).
pub struct WorkspaceStore<'a> {
    backend: Backend,
    workspace_id: WorkspaceId,
    layout: &'a RepoLayout,
}

impl<'a> WorkspaceStore<'a> {
    /// Initialize a new workspace repo.
    pub fn init(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        name: &str,
        valkey_client: Option<&redis::Client>,
    ) -> Result<Self, GitStorageError> {
        let record = WorkspaceRecord {
            workspace_id,
            name: name.to_owned(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let backend = match valkey_client {
            None => {
                let path = layout.workspace_repo_path(workspace_id);
                let repo = CorpRepo::init(&path, None)?;
                let files = vec![FileWrite::json("workspace.json", &record)?];
                commit_files(&repo, "main", "Initialize workspace", &files, None)?;
                Backend::Git { repo }
            }
            Some(client) => {
                let mut con = client
                    .get_connection()
                    .map_err(|e| GitStorageError::Git(e.to_string()))?;
                let ws = workspace_id.to_string();
                let vfiles =
                    vec![corp_store::entry::FileWrite::json("workspace.json", &record)
                        .map_err(|e| GitStorageError::SerializationError(e.to_string()))?];
                corp_store::store::commit_files(
                    &mut con,
                    &ws,
                    VALKEY_WORKSPACE_ENTITY,
                    "main",
                    "Initialize workspace",
                    &vfiles,
                    None,
                    chrono::Utc::now(),
                )?;
                Backend::Valkey {
                    con: Rc::new(RefCell::new(con)),
                    ws,
                }
            }
        };

        Ok(Self {
            backend,
            workspace_id,
            layout,
        })
    }

    /// Open an existing workspace store.
    pub fn open(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        valkey_client: Option<&redis::Client>,
    ) -> Result<Self, GitStorageError> {
        let backend = match valkey_client {
            None => {
                let path = layout.workspace_repo_path(workspace_id);
                let repo = CorpRepo::open(&path)?;
                Backend::Git { repo }
            }
            Some(client) => {
                let mut con = client
                    .get_connection()
                    .map_err(|e| GitStorageError::Git(e.to_string()))?;
                let ws = workspace_id.to_string();
                match corp_store::store::resolve_ref(
                    &mut con,
                    &ws,
                    VALKEY_WORKSPACE_ENTITY,
                    "main",
                ) {
                    Ok(_) => {}
                    Err(corp_store::StoreError::RefNotFound(_)) => {
                        return Err(GitStorageError::RepoNotFound(format!(
                            "{ws}/_workspace"
                        )));
                    }
                    Err(e) => return Err(GitStorageError::from(e)),
                }
                Backend::Valkey {
                    con: Rc::new(RefCell::new(con)),
                    ws,
                }
            }
        };

        Ok(Self {
            backend,
            workspace_id,
            layout,
        })
    }

    /// Open an existing workspace store, reusing a shared Valkey connection.
    pub fn open_shared(
        layout: &'a RepoLayout,
        workspace_id: WorkspaceId,
        shared_con: Option<Rc<RefCell<redis::Connection>>>,
    ) -> Result<Self, GitStorageError> {
        let backend = match shared_con {
            None => {
                let path = layout.workspace_repo_path(workspace_id);
                let repo = CorpRepo::open(&path)?;
                Backend::Git { repo }
            }
            Some(con) => {
                let ws = workspace_id.to_string();
                {
                    let mut c = con.borrow_mut();
                    match corp_store::store::resolve_ref(
                        &mut *c,
                        &ws,
                        VALKEY_WORKSPACE_ENTITY,
                        "main",
                    ) {
                        Ok(_) => {}
                        Err(corp_store::StoreError::RefNotFound(_)) => {
                            return Err(GitStorageError::RepoNotFound(format!(
                                "{ws}/_workspace"
                            )));
                        }
                        Err(e) => return Err(GitStorageError::from(e)),
                    }
                }
                Backend::Valkey { con, ws }
            }
        };

        Ok(Self {
            backend,
            workspace_id,
            layout,
        })
    }

    /// List workspace IDs and return a shared connection for subsequent `open_shared` calls.
    pub fn list_and_prepare(
        layout: &RepoLayout,
        valkey_client: Option<&redis::Client>,
    ) -> Result<(Vec<WorkspaceId>, Option<Rc<RefCell<redis::Connection>>>), GitStorageError> {
        match valkey_client {
            None => Ok((layout.list_workspace_ids(), None)),
            Some(client) => {
                let con = client
                    .get_connection()
                    .map_err(|e| GitStorageError::Git(e.to_string()))?;
                let con = Rc::new(RefCell::new(con));
                let ids = corp_store::store::list_workspaces(&mut *con.borrow_mut())
                    .map_err(GitStorageError::from)?
                    .into_iter()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                Ok((ids, Some(con)))
            }
        }
    }

    /// Read workspace metadata.
    pub fn read_workspace(&self) -> Result<WorkspaceRecord, GitStorageError> {
        self.dispatch_read_json("workspace.json")
    }

    /// Write any serializable value to a JSON path and commit it.
    pub fn write_json<T: Serialize>(
        &self,
        path: &str,
        value: &T,
        message: &str,
    ) -> Result<(), GitStorageError> {
        let files = vec![FileWrite::json(path, value)?];
        self.dispatch_commit(message, &files)
    }

    /// Read an API key record by ID.
    pub fn read_api_key(&self, key_id: ApiKeyId) -> Result<ApiKeyRecord, GitStorageError> {
        self.dispatch_read_json(&format!("api-keys/{}.json", key_id))
    }

    /// List all API key IDs.
    pub fn list_api_key_ids(&self) -> Result<Vec<ApiKeyId>, GitStorageError> {
        self.list_ids_in_dir("api-keys")
    }

    // ── SSH key CRUD ─────────────────────────────────────────────────

    /// Read an SSH key record by ID.
    pub fn read_ssh_key(&self, key_id: SshKeyId) -> Result<SshKeyRecord, GitStorageError> {
        self.dispatch_read_json(&format!("ssh-keys/{}.json", key_id))
    }

    /// List all SSH key IDs.
    pub fn list_ssh_key_ids(&self) -> Result<Vec<SshKeyId>, GitStorageError> {
        self.list_ids_in_dir("ssh-keys")
    }

    // ── API key CRUD (continued) ─────────────────────────────────────

    /// Delete an API key by overwriting with a tombstone marker.
    pub fn delete_api_key(&self, key_id: ApiKeyId) -> Result<(), GitStorageError> {
        let tombstone = serde_json::json!({ "deleted": true, "key_id": key_id });
        let files = vec![FileWrite::json(
            format!("api-keys/{}.json", key_id),
            &tombstone,
        )?];
        self.dispatch_commit(&format!("Revoke API key {key_id}"), &files)
    }

    /// Get the workspace ID.
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    /// Read any deserializable JSON from a path.
    pub fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, GitStorageError> {
        self.dispatch_read_json(path)
    }

    /// Commit raw file writes to the workspace repo.
    pub fn commit_files(&self, message: &str, files: &[FileWrite]) -> Result<(), GitStorageError> {
        self.dispatch_commit(message, files)
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
        let entries = match self.dispatch_list_dir(dir_path) {
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
        match &self.backend {
            Backend::Git { repo } => repo.path_exists("main", path),
            Backend::Valkey { con, ws } => {
                let mut con = con.borrow_mut();
                Ok(corp_store::store::path_exists(
                    &mut *con,
                    ws,
                    VALKEY_WORKSPACE_ENTITY,
                    "main",
                    path,
                )?)
            }
        }
    }

    /// List directory entries.
    pub fn list_dir(&self, dir_path: &str) -> Result<Vec<(String, bool)>, GitStorageError> {
        self.dispatch_list_dir(dir_path)
    }

    /// Get recent commits for audit/logging purposes.
    pub fn recent_commits(
        &self,
        limit: usize,
    ) -> Result<Vec<(String, String, String)>, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => repo.recent_commits("main", limit),
            Backend::Valkey { con, ws } => {
                let mut con = con.borrow_mut();
                let entries = corp_store::store::recent_commits(
                    &mut *con,
                    ws,
                    VALKEY_WORKSPACE_ENTITY,
                    limit,
                )?;
                Ok(entries
                    .into_iter()
                    .map(|e| (e.sha1, e.message, e.timestamp.to_rfc3339()))
                    .collect())
            }
        }
    }

    /// List UUID-style IDs from files in a directory.
    fn list_ids_in_dir<T: std::str::FromStr>(
        &self,
        dir_path: &str,
    ) -> Result<Vec<T>, GitStorageError> {
        let entries = match self.dispatch_list_dir(dir_path) {
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

    // ── Internal dispatch helpers ────────────────────────────────────

    fn dispatch_read_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => repo.read_json("main", path),
            Backend::Valkey { con, ws } => {
                let mut con = con.borrow_mut();
                Ok(corp_store::store::read_json(
                    &mut *con,
                    ws,
                    VALKEY_WORKSPACE_ENTITY,
                    "main",
                    path,
                )?)
            }
        }
    }

    fn dispatch_commit(
        &self,
        message: &str,
        files: &[FileWrite],
    ) -> Result<(), GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => {
                commit_files(repo, "main", message, files, None)?;
                Ok(())
            }
            Backend::Valkey { con, ws } => {
                let mut con = con.borrow_mut();
                let vfiles: Vec<corp_store::entry::FileWrite> = files
                    .iter()
                    .map(|f| corp_store::entry::FileWrite::new(&f.path, f.content.clone()))
                    .collect();
                corp_store::store::commit_files(
                    &mut *con,
                    ws,
                    VALKEY_WORKSPACE_ENTITY,
                    "main",
                    message,
                    &vfiles,
                    None,
                    chrono::Utc::now(),
                )?;
                Ok(())
            }
        }
    }

    fn dispatch_list_dir(
        &self,
        dir_path: &str,
    ) -> Result<Vec<(String, bool)>, GitStorageError> {
        match &self.backend {
            Backend::Git { repo } => repo.list_dir("main", dir_path),
            Backend::Valkey { con, ws } => {
                let mut con = con.borrow_mut();
                Ok(corp_store::store::list_dir(
                    &mut *con,
                    ws,
                    VALKEY_WORKSPACE_ENTITY,
                    "main",
                    dir_path,
                )?)
            }
        }
    }
}
