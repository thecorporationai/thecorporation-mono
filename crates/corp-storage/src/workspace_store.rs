//! [`WorkspaceStore`] — workspace-level storage for cross-entity data.
//!
//! Stores workspace-scoped records that are not tied to a single entity:
//! - API keys (`ApiKeyRecord`)
//! - Entity ID membership lists
//!
//! The same `Backend` enum as `EntityStore` is used, but paths are rooted
//! under the workspace namespace rather than an entity namespace.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use corp_core::ids::{ApiKeyId, EntityId, WorkspaceId};

use crate::error::StorageError;

#[cfg(feature = "kv")]
use redis::aio::ConnectionManager;

// ── ApiKeyRecord ──────────────────────────────────────────────────────────────

/// A persisted API key record.
///
/// The raw secret is never stored; only its Argon2/bcrypt/sha256 `key_hash`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    /// Stable, unique identifier for this key.
    pub key_id: ApiKeyId,

    /// Human-readable display name (e.g. "CI/CD deploy key").
    pub name: String,

    /// Hash of the raw secret.  Never the plaintext.
    pub key_hash: String,

    /// Capability scopes granted to this key (stored as kebab-case strings,
    /// matching `corp_core::auth::Scope`'s serde representation).
    pub scopes: Vec<String>,

    /// Optionally restricts this key to a single entity.
    pub entity_id: Option<EntityId>,

    /// When the key was created.
    pub created_at: DateTime<Utc>,

    /// Soft-deleted keys are retained for audit purposes.
    pub deleted: bool,
}

impl ApiKeyRecord {
    /// Construct a new, non-deleted API key record.
    pub fn new(
        name: impl Into<String>,
        key_hash: impl Into<String>,
        scopes: Vec<String>,
        entity_id: Option<EntityId>,
    ) -> Self {
        Self {
            key_id: ApiKeyId::new(),
            name: name.into(),
            key_hash: key_hash.into(),
            scopes,
            entity_id,
            created_at: Utc::now(),
            deleted: false,
        }
    }
}

// ── Backend ───────────────────────────────────────────────────────────────────

/// Storage backend for workspace-level data.
///
/// Mirrors `EntityStore`'s `Backend` but is kept separate to allow workspace
/// and entity stores to be configured independently.
pub enum Backend {
    #[cfg(feature = "git")]
    Git { repo_path: Arc<PathBuf> },

    #[cfg(feature = "kv")]
    Kv { pool: ConnectionManager },
}

// ── WorkspaceStore ────────────────────────────────────────────────────────────

/// A workspace-scoped storage handle.
pub struct WorkspaceStore {
    backend: Backend,
    workspace_id: WorkspaceId,
}

type Result<T> = std::result::Result<T, StorageError>;

// Path constants for git backend.
const API_KEYS_DIR: &str = "workspace/api_keys";
const ENTITIES_INDEX: &str = "workspace/entities.json";

impl WorkspaceStore {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Initialise a new workspace store (creates repo / KV namespace).
    pub async fn init(backend: Backend, workspace_id: WorkspaceId) -> Result<Self> {
        match &backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                let path = Arc::clone(repo_path);
                tokio::task::spawn_blocking(move || {
                    crate::git::init_bare_repo(&path)?;
                    // Write the initial entity index.
                    let empty_list: Vec<String> = vec![];
                    let bytes = serde_json::to_vec(&empty_list)
                        .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                    crate::git::write_files(
                        &path,
                        "main",
                        &[(ENTITIES_INDEX.to_owned(), bytes)],
                        "initialise workspace store",
                    )
                })
                .await
                .map_err(|e| StorageError::GitError(format!("spawn_blocking: {}", e)))??;
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool } => {
                let mut con = pool.clone();
                let ws = workspace_id.to_string();
                // Register workspace in the global set.
                use redis::AsyncCommands;
                let _: i64 = con
                    .sadd(crate::kv::key_workspaces_static(), &ws)
                    .await
                    .map_err(|e| StorageError::KvError(e.to_string()))?;
            }

            #[allow(unreachable_patterns)]
            _ => {
                return Err(StorageError::InvalidData(
                    "no backend feature enabled".into(),
                ));
            }
        }

        Ok(Self {
            backend,
            workspace_id,
        })
    }

    /// Open an existing workspace store.
    pub async fn open(backend: Backend, workspace_id: WorkspaceId) -> Result<Self> {
        match &backend {
            #[cfg(feature = "git")]
            Backend::Git { repo_path } => {
                if !repo_path.join("HEAD").exists() {
                    return Err(StorageError::NotFound(format!(
                        "workspace git repo at '{}'",
                        repo_path.display()
                    )));
                }
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool } => {
                let mut con = pool.clone();
                let ws = workspace_id.to_string();
                use redis::AsyncCommands;
                let registered: bool = con
                    .sismember(crate::kv::key_workspaces_static(), &ws)
                    .await
                    .map_err(|e| StorageError::KvError(e.to_string()))?;
                if !registered {
                    return Err(StorageError::NotFound(format!(
                        "workspace '{}' not found in KV",
                        ws
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

        Ok(Self {
            backend,
            workspace_id,
        })
    }

    // ── API key operations ────────────────────────────────────────────────────

    /// Persist an API key record.
    pub async fn write_api_key(&self, record: &ApiKeyRecord) -> Result<()> {
        let path = format!("{}/{}.json", API_KEYS_DIR, record.key_id);
        let bytes = serde_json::to_vec(record).map_err(StorageError::from)?;
        self.write_raw(
            &path,
            bytes,
            "main",
            &format!("write api key {}", record.key_id),
        )
        .await
    }

    /// Load an API key record by ID.
    pub async fn read_api_key(&self, key_id: ApiKeyId) -> Result<ApiKeyRecord> {
        let path = format!("{}/{}.json", API_KEYS_DIR, key_id);
        let bytes = self.read_raw(&path, "main").await?;
        serde_json::from_slice(&bytes).map_err(StorageError::from)
    }

    /// List all API key IDs in this workspace (including soft-deleted keys).
    pub async fn list_api_key_ids(&self) -> Result<Vec<ApiKeyId>> {
        let names = self.list_dir(API_KEYS_DIR, "main").await?;
        let mut ids = Vec::with_capacity(names.len());
        for name in names {
            let stem = name.strip_suffix(".json").unwrap_or(&name);
            // Strip directory prefix if present (KV returns full paths).
            let bare = stem
                .strip_prefix(API_KEYS_DIR)
                .and_then(|s| s.strip_prefix('/'))
                .unwrap_or(stem);
            let id: ApiKeyId = bare.parse().map_err(|e| {
                StorageError::InvalidData(format!("cannot parse ApiKeyId '{}': {}", bare, e))
            })?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Soft-delete an API key by setting `deleted = true`.
    pub async fn delete_api_key(&self, key_id: ApiKeyId) -> Result<()> {
        let mut record = self.read_api_key(key_id).await?;
        record.deleted = true;
        self.write_api_key(&record).await
    }

    // ── Entity list operations ────────────────────────────────────────────────

    /// Return all `EntityId`s registered in this workspace.
    pub async fn list_entity_ids(&self) -> Result<Vec<EntityId>> {
        match &self.backend {
            #[cfg(feature = "git")]
            Backend::Git { .. } => {
                let bytes = self.read_raw(ENTITIES_INDEX, "main").await?;
                let ids: Vec<String> =
                    serde_json::from_slice(&bytes).map_err(StorageError::from)?;
                ids.iter()
                    .map(|s| {
                        s.parse::<EntityId>().map_err(|e| {
                            StorageError::InvalidData(format!("bad EntityId '{}': {}", s, e))
                        })
                    })
                    .collect()
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                let raw = crate::kv::list_entities(&mut con, &ws).await?;
                raw.iter()
                    .map(|s| {
                        s.parse::<EntityId>().map_err(|e| {
                            StorageError::InvalidData(format!("bad EntityId '{}': {}", s, e))
                        })
                    })
                    .collect()
            }

            #[allow(unreachable_patterns)]
            _ => Err(StorageError::InvalidData("no backend".into())),
        }
    }

    /// Register a new entity ID in this workspace's entity index.
    ///
    /// Reads the current index, appends `entity_id` if not already present,
    /// and writes it back.  Idempotent.
    pub async fn register_entity(&self, entity_id: EntityId) -> Result<()> {
        let mut ids = match self.list_entity_ids().await {
            Ok(ids) => ids,
            Err(StorageError::NotFound(_)) => vec![],
            Err(e) => return Err(e),
        };
        if ids.contains(&entity_id) {
            return Ok(());
        }
        ids.push(entity_id);
        let id_strings: Vec<String> = ids.iter().map(ToString::to_string).collect();
        let bytes = serde_json::to_vec(&id_strings)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.write_raw(ENTITIES_INDEX, bytes, "main", "register entity")
            .await
    }

    // ── Raw read/write helpers ───────────────────────────────────────────────

    /// Read raw bytes from the workspace store at the given path and branch.
    ///
    /// This is the low-level primitive used by higher-level methods like
    /// `read_api_key`.  It is also available for workspace-scoped data that
    /// does not have a dedicated accessor (e.g. billing records).
    pub async fn read_raw(&self, path: &str, branch: &str) -> Result<Vec<u8>> {
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
            Backend::Kv { pool } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                // WorkspaceStore uses a sentinel entity key "_ws" for its own data.
                crate::kv::read_blob(&mut con, &ws, "_ws", branch, path).await
            }

            #[allow(unreachable_patterns)]
            _ => Err(StorageError::InvalidData("no backend".into())),
        }
    }

    /// Write raw bytes to the workspace store at the given path and branch.
    ///
    /// This is the low-level primitive used by higher-level methods like
    /// `write_api_key`.  It is also available for workspace-scoped data that
    /// does not have a dedicated accessor (e.g. billing records).
    pub async fn write_raw(
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
            Backend::Kv { pool } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                crate::kv::write_files(
                    &mut con,
                    &ws,
                    "_ws",
                    branch,
                    &[(path.to_owned(), data)],
                    message,
                )
                .await
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
                tokio::task::spawn_blocking(move || crate::git::list_directory(&rp, &b, &d))
                    .await
                    .map_err(|e| StorageError::GitError(format!("spawn_blocking: {}", e)))?
            }

            #[cfg(feature = "kv")]
            Backend::Kv { pool } => {
                let mut con = pool.clone();
                let ws = self.workspace_id.to_string();
                crate::kv::list_directory(&mut con, &ws, "_ws", branch, dir).await
            }

            #[allow(unreachable_patterns)]
            _ => Err(StorageError::InvalidData("no backend".into())),
        }
    }

    // ── Accessor ──────────────────────────────────────────────────────────────

    /// The workspace ID this store is scoped to.
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }
}
