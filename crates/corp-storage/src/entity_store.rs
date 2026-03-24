//! [`EntityStore`] — the primary storage abstraction for domain entities.
//!
//! `EntityStore` presents a clean, backend-agnostic async API.  Internally it
//! dispatches to either the git or the Redis/Valkey KV backend via the
//! `Backend` enum.
//!
//! ## Git backend
//! Synchronous `gix` calls are wrapped in `tokio::task::spawn_blocking` so
//! they never block the async runtime.  The repo path is cheaply cloned into
//! the closure via `Arc<PathBuf>`.
//!
//! ## KV backend
//! Uses `redis::aio::ConnectionManager` — an arc-backed connection pool with
//! automatic reconnection.  All mutation uses `WATCH`/`MULTI`/`EXEC`
//! transactions; see [`crate::kv`] for details.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{de::DeserializeOwned, Serialize};

use corp_core::ids::{EntityId, WorkspaceId};

use crate::error::StorageError;
use crate::traits::StoredEntity;

#[cfg(feature = "kv")]
use redis::aio::ConnectionManager;

// ── Backend ───────────────────────────────────────────────────────────────────

/// The storage backend variant.
///
/// Each variant carries only the configuration/connection data it needs.
/// There is no `Rc`, no `RefCell`, and no bare connection handle.
pub enum Backend {
    /// Bare git repository on the local filesystem.
    #[cfg(feature = "git")]
    Git {
        /// Shared reference to the repo path, cheap to clone into
        /// `spawn_blocking` closures.
        repo_path: Arc<PathBuf>,
    },
    /// Redis / Valkey connection pool with automatic reconnection.
    #[cfg(feature = "kv")]
    Kv {
        /// Arc-backed connection manager — safe to clone and share across
        /// threads/tasks.
        pool: ConnectionManager,
        /// Optional S3 backend for durable blob/commit storage.
        /// When present, every KV write is also persisted to S3 (best-effort).
        #[cfg(feature = "s3")]
        s3: Option<Arc<crate::s3::S3Backend>>,
    },
}

// ── EntityStore ───────────────────────────────────────────────────────────────

/// An entity-scoped async storage handle.
///
/// One `EntityStore` instance corresponds to one `(workspace_id, entity_id)`
/// pair.  Obtain one via [`EntityStore::init`] (first-time setup) or
/// [`EntityStore::open`] (existing repo/namespace).
pub struct EntityStore {
    backend: Backend,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
}

type Result<T> = std::result::Result<T, StorageError>;

impl EntityStore {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Initialise a new entity store and write `initial_data` to it.
    ///
    /// For the git backend this creates a bare repository and makes an initial
    /// commit on `"main"`.  For the KV backend it registers the namespace and
    /// writes the initial data.
    ///
    /// Returns an error if the store already exists.
    pub async fn init(
        backend: Backend,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
        initial_data: &[u8],
    ) -> Result<Self> {
        // Copy the slice into an owned Vec before any spawn_blocking closures
        // so the 'static bound is satisfied.
        let initial_data_owned: Vec<u8> = initial_data.to_vec();

        match &backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                let path = Arc::clone(repo_path);
                let data = initial_data_owned.clone();
                tokio::task::spawn_blocking(move || {
                    crate::git::init_bare_repo(&path)?;
                    crate::git::write_files(
                        &path,
                        "main",
                        &[("init".to_owned(), data)],
                        "initialise entity store",
                    )
                })
                .await
                .map_err(|e| StorageError::GitError(format!("spawn_blocking: {}", e)))??;
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool, .. } => {
                let mut con = pool.clone();
                let ws = workspace_id.to_string();
                let ent = entity_id.to_string();
                let already: bool = {
                    use redis::AsyncCommands;
                    con.sismember(format!("corp:{}:entities", ws), &ent)
                        .await
                        .map_err(|e| StorageError::KvError(e.to_string()))?
                };
                if already {
                    return Err(StorageError::AlreadyExists(format!(
                        "entity {}/{} already exists",
                        ws, ent
                    )));
                }
                crate::kv::init_entity(&mut con, &ws, &ent).await?;
                let files = &[("init".to_owned(), initial_data_owned)];
                crate::kv::write_files(
                    &mut con,
                    &ws,
                    &ent,
                    "main",
                    files,
                    "initialise entity store",
                )
                .await?;

                // Best-effort S3 durability: persist blobs and a commit
                // entry so the KV state can be rebuilt from S3.
                #[cfg(feature = "s3")]
                if let Backend::Kv { s3: Some(s3), .. } = &backend {
                    sync_files_to_s3(s3, &mut con, &ws, &ent, "main", files, "initialise entity store").await;
                }
            }

            #[allow(unreachable_patterns)]
            _ => {
                return Err(StorageError::InvalidData(
                    "no backend feature enabled".into(),
                ));
            }
        }

        Ok(Self { backend, workspace_id, entity_id })
    }

    /// Open an existing entity store.
    ///
    /// For the git backend this verifies the repository exists.  For the KV
    /// backend it verifies the namespace is registered.
    pub async fn open(
        backend: Backend,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
    ) -> Result<Self> {
        match &backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                if !repo_path.join("HEAD").exists() {
                    return Err(StorageError::NotFound(format!(
                        "git repo at '{}'",
                        repo_path.display()
                    )));
                }
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool, .. } => {
                let mut con = pool.clone();
                let ws = workspace_id.to_string();
                let ent = entity_id.to_string();
                let registered: bool = {
                    use redis::AsyncCommands;
                    con.sismember(format!("corp:{}:entities", ws), &ent)
                        .await
                        .map_err(|e| StorageError::KvError(e.to_string()))?
                };
                if !registered {
                    return Err(StorageError::NotFound(format!(
                        "entity {}/{} not found in KV",
                        ws, ent
                    )));
                }
            }

            #[allow(unreachable_patterns)]
            _ => {
                return Err(StorageError::InvalidData(
                    "no backend feature enabled".into(),
                ));
            }
        }

        Ok(Self { backend, workspace_id, entity_id })
    }

    // ── Entity CRUD ───────────────────────────────────────────────────────────

    /// Read and deserialise an entity by its ID from `branch`.
    pub async fn read<T: StoredEntity>(&self, id: T::Id, branch: &str) -> Result<T> {
        let path = T::storage_path(id);
        let bytes = self.read_raw(&path, branch).await?;
        serde_json::from_slice(&bytes).map_err(StorageError::from)
    }

    /// Serialise and write an entity to `branch`.
    pub async fn write<T: StoredEntity>(
        &self,
        entity: &T,
        id: T::Id,
        branch: &str,
        message: &str,
    ) -> Result<()> {
        let path = T::storage_path(id);
        let bytes = serde_json::to_vec(entity).map_err(StorageError::from)?;
        self.write_raw(&path, bytes, branch, message).await
    }

    /// List all entity IDs of type `T` stored under `T::storage_dir()` on `branch`.
    pub async fn list_ids<T: StoredEntity>(&self, branch: &str) -> Result<Vec<T::Id>>
    where
        <T::Id as std::str::FromStr>::Err: std::fmt::Display,
    {
        let names = self.list_dir(T::storage_dir(), branch).await?;
        let mut ids = Vec::with_capacity(names.len());
        for name in names {
            // Strip the ".json" suffix and parse the ID.
            let stem = name.strip_suffix(".json").unwrap_or(&name);
            // Strip the directory prefix if the backend returns full paths.
            let bare = stem
                .strip_prefix(T::storage_dir())
                .and_then(|s| s.strip_prefix('/'))
                .unwrap_or(stem);
            let id: T::Id = bare
                .parse()
                .map_err(|e: <T::Id as std::str::FromStr>::Err| {
                    StorageError::InvalidData(format!("cannot parse ID '{}': {}", bare, e))
                })?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Read all entities of type `T` from `branch`.
    ///
    /// Internally lists IDs and then reads each entity.  The git backend uses
    /// a single `spawn_blocking` per read; the KV backend uses a
    /// `ConnectionManager` clone per operation.
    pub async fn read_all<T: StoredEntity>(&self, branch: &str) -> Result<Vec<T>>
    where
        <T::Id as std::str::FromStr>::Err: std::fmt::Display,
    {
        let ids = self.list_ids::<T>(branch).await?;
        let mut entities = Vec::with_capacity(ids.len());
        for id in ids {
            entities.push(self.read::<T>(id, branch).await?);
        }
        Ok(entities)
    }

    /// Delete a single entity from `branch`.
    pub async fn delete<T: StoredEntity>(
        &self,
        id: T::Id,
        branch: &str,
        message: &str,
    ) -> Result<()> {
        let path = T::storage_path(id);
        self.delete_raw(&path, branch, message).await
    }

    // ── Low-level path-based API ──────────────────────────────────────────────

    /// Return `true` if `path` exists on `branch`.
    pub async fn path_exists(&self, path: &str, branch: &str) -> Result<bool> {
        match &self.backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                let rp = Arc::clone(repo_path);
                let p = path.to_owned();
                let b = branch.to_owned();
                tokio::task::spawn_blocking(move || crate::git::file_exists(&rp, &b, &p))
                    .await
                    .map_err(|e| StorageError::GitError(format!("spawn_blocking: {}", e)))?
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool, .. } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                let ent = self.entity_id.to_string();
                crate::kv::path_exists(&mut con, &ws, &ent, branch, path).await
            }

            #[allow(unreachable_patterns)]
            _ => Err(StorageError::InvalidData("no backend".into())),
        }
    }

    /// Read and deserialise a JSON value from an arbitrary path.
    pub async fn read_json<T: DeserializeOwned>(
        &self,
        path: &str,
        branch: &str,
    ) -> Result<T> {
        let bytes = self.read_raw(path, branch).await?;
        serde_json::from_slice(&bytes).map_err(StorageError::from)
    }

    /// Serialise a JSON value and write it to an arbitrary path.
    pub async fn write_json<T: Serialize>(
        &self,
        path: &str,
        value: &T,
        branch: &str,
        message: &str,
    ) -> Result<()> {
        let bytes = serde_json::to_vec(value).map_err(StorageError::from)?;
        self.write_raw(path, bytes, branch, message).await
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    async fn read_raw(&self, path: &str, branch: &str) -> Result<Vec<u8>> {
        match &self.backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                let rp = Arc::clone(repo_path);
                let p = path.to_owned();
                let b = branch.to_owned();
                tokio::task::spawn_blocking(move || crate::git::read_file(&rp, &b, &p))
                    .await
                    .map_err(|e| StorageError::GitError(format!("spawn_blocking: {}", e)))?
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool, .. } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                let ent = self.entity_id.to_string();
                crate::kv::read_blob(&mut con, &ws, &ent, branch, path).await
            }

            #[allow(unreachable_patterns)]
            _ => Err(StorageError::InvalidData("no backend".into())),
        }
    }

    async fn write_raw(
        &self,
        path: &str,
        data: Vec<u8>,
        branch: &str,
        message: &str,
    ) -> Result<()> {
        match &self.backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                let rp = Arc::clone(repo_path);
                let p = path.to_owned();
                let b = branch.to_owned();
                let m = message.to_owned();
                tokio::task::spawn_blocking(move || {
                    crate::git::write_files(&rp, &b, &[(p, data)], &m)
                })
                .await
                .map_err(|e| StorageError::GitError(format!("spawn_blocking: {}", e)))?
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool, .. } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                let ent = self.entity_id.to_string();
                let files = &[(path.to_owned(), data)];
                crate::kv::write_files(
                    &mut con,
                    &ws,
                    &ent,
                    branch,
                    files,
                    message,
                )
                .await?;

                // Best-effort S3 durability.
                #[cfg(feature = "s3")]
                if let Backend::Kv { s3: Some(s3), .. } = &self.backend {
                    sync_files_to_s3(s3, &mut con, &ws, &ent, branch, files, message).await;
                }

                Ok(())
            }

            #[allow(unreachable_patterns)]
            _ => Err(StorageError::InvalidData("no backend".into())),
        }
    }

    async fn delete_raw(&self, path: &str, branch: &str, message: &str) -> Result<()> {
        match &self.backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                let rp = Arc::clone(repo_path);
                let p = path.to_owned();
                let b = branch.to_owned();
                let m = message.to_owned();
                tokio::task::spawn_blocking(move || {
                    crate::git::delete_file(&rp, &b, &p, &m)
                })
                .await
                .map_err(|e| StorageError::GitError(format!("spawn_blocking: {}", e)))?
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool, .. } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                let ent = self.entity_id.to_string();
                crate::kv::delete_file(&mut con, &ws, &ent, branch, path, message).await
            }

            #[allow(unreachable_patterns)]
            _ => Err(StorageError::InvalidData("no backend".into())),
        }
    }

    async fn list_dir(&self, dir: &str, branch: &str) -> Result<Vec<String>> {
        match &self.backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                let rp = Arc::clone(repo_path);
                let d = dir.to_owned();
                let b = branch.to_owned();
                tokio::task::spawn_blocking(move || {
                    crate::git::list_directory(&rp, &b, &d)
                })
                .await
                .map_err(|e| StorageError::GitError(format!("spawn_blocking: {}", e)))?
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool, .. } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                let ent = self.entity_id.to_string();
                crate::kv::list_directory(&mut con, &ws, &ent, branch, dir).await
            }

            #[allow(unreachable_patterns)]
            _ => Err(StorageError::InvalidData("no backend".into())),
        }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// The workspace this store is scoped to.
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    /// The entity this store is scoped to.
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }
}

// ── S3 sync helper ──────────────────────────────────────────────────────────

/// Best-effort sync of written files to S3 for durability.
///
/// Persists each file as a content-addressed blob and writes a commit entry
/// so the KV state can be fully rebuilt from S3 alone.  The current sequence
/// number is read from Redis (it was just incremented by `kv::write_files`).
///
/// All S3 errors are logged and swallowed: the KV store is the primary and
/// S3 is the durable backup.
#[cfg(all(feature = "kv", feature = "s3"))]
async fn sync_files_to_s3(
    s3: &crate::s3::S3Backend,
    con: &mut redis::aio::ConnectionManager,
    ws: &str,
    ent: &str,
    branch: &str,
    files: &[(String, Vec<u8>)],
    message: &str,
) {
    use std::collections::HashMap;

    let mut file_shas: HashMap<String, String> = HashMap::with_capacity(files.len());

    for (path, data) in files {
        let sha = crate::kv::sha256_hex(data);
        if let Err(e) = s3.put_blob(&sha, data).await {
            tracing::warn!(
                sha,
                path,
                ws,
                ent,
                error = %e,
                "S3 put_blob failed (best-effort), skipping"
            );
        }
        file_shas.insert(path.clone(), sha);
    }

    // Read the current sequence number that kv::write_files just set.
    let seq: u64 = {
        use redis::AsyncCommands;
        match con
            .get::<_, Option<u64>>(format!("corp:{}:{}:seq", ws, ent))
            .await
        {
            Ok(Some(s)) => s,
            _ => return, // cannot determine seq — skip commit entry
        }
    };

    let entry = crate::kv::CommitEntry {
        seq,
        branch: branch.to_owned(),
        files: file_shas,
        message: message.to_owned(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    if let Ok(entry_json) = serde_json::to_vec(&entry) {
        if let Err(e) = s3.put_commit(ws, ent, seq, &entry_json).await {
            tracing::warn!(
                ws,
                ent,
                seq,
                error = %e,
                "S3 put_commit failed (best-effort), skipping"
            );
        }
    }
}
