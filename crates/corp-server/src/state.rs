//! Application state threaded through every Axum handler.
// The `s3` feature gate is intentional — it mirrors corp-storage's feature
// name for forward-compatibility. Silence the cfg lint.
#![allow(unexpected_cfgs)]
//!
//! [`AppState`] is cheaply cloneable (all mutable resources are behind `Arc`)
//! and is registered with the Axum router via `.with_state(state)`.
//!
//! # Environment variables
//!
//! | Variable | Default | Notes |
//! |---|---|---|
//! | `CORP_DATA_DIR` | `./data` | Root directory for git repos / data |
//! | `CORP_JWT_SECRET` | — | **Required** HS256 signing secret |
//! | `CORP_STORAGE_BACKEND` | `git` | `git` or `kv` |
//! | `CORP_REDIS_URL` | — | Required when backend is `kv` |
//! | `CORP_S3_BUCKET` | — | Optional; enables S3 blob durability for `kv` |
//!
//! # `FromRef` impls
//!
//! `corp-auth` extractors require `Arc<JwtConfig>: FromRef<S>` and
//! `Arc<dyn ApiKeyResolver>: FromRef<S>`.  Both impls are provided here so the
//! extractors bind automatically against `AppState`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::FromRef;

use corp_auth::{ApiKeyManager, ApiKeyResolver, AuthError, JwtConfig, Principal};
use corp_core::auth::{PrincipalType, ScopeSet};
use corp_core::formation::entity::{Entity, FormationStatus};
use corp_core::ids::{EntityId, WorkspaceId};
use corp_storage::entity_store::{Backend as EntityBackend, EntityStore};
use corp_storage::error::StorageError;
use corp_storage::workspace_store::{Backend as WorkspaceBackend, WorkspaceStore};

use crate::error::AppError;

// ── StorageBackend ────────────────────────────────────────────────────────────

/// Which storage backend the server should use for entity and workspace data.
///
/// Both variants are cheaply cloneable: the `Git` variant uses a string path
/// and the `Kv` variant carries a Redis URL string that is used to open fresh
/// connection managers on demand.
#[derive(Clone, Debug)]
pub enum StorageBackend {
    /// Bare git repositories on the local filesystem (default).
    Git,

    /// Redis / Valkey key-value store.
    Kv {
        redis_url: String,
        #[cfg(feature = "s3")]
        s3_bucket: Option<String>,
    },
}

// ── AppState ──────────────────────────────────────────────────────────────────

/// Shared application state for all route handlers.
///
/// Cheap to clone — all mutable or heavyweight resources are held behind `Arc`.
#[derive(Clone)]
pub struct AppState {
    /// Root directory for all on-disk data (git repos, lock files, etc.).
    pub data_dir: String,

    /// JWT signing and verification configuration.
    pub jwt_config: Arc<JwtConfig>,

    /// Resolves raw API key strings to authenticated [`Principal`] values.
    pub api_key_resolver: Arc<dyn ApiKeyResolver>,

    /// Which storage backend to use when opening entity / workspace stores.
    pub storage_backend: StorageBackend,
}

impl AppState {
    // ── Constructor ───────────────────────────────────────────────────────────

    /// Build [`AppState`] from environment variables.
    ///
    /// Panics if a required variable is absent.  Creates the data directory if
    /// it does not yet exist (git backend only).
    pub async fn from_env() -> Self {
        let data_dir = std::env::var("CORP_DATA_DIR").unwrap_or_else(|_| "./data".to_owned());

        let jwt_secret = std::env::var("CORP_JWT_SECRET").expect("CORP_JWT_SECRET must be set");

        let backend_name =
            std::env::var("CORP_STORAGE_BACKEND").unwrap_or_else(|_| "git".to_owned());

        let storage_backend = match backend_name.as_str() {
            "kv" => {
                let redis_url = std::env::var("CORP_REDIS_URL")
                    .expect("CORP_REDIS_URL must be set when CORP_STORAGE_BACKEND=kv");

                #[cfg(feature = "s3")]
                let s3_bucket = std::env::var("CORP_S3_BUCKET").ok();

                StorageBackend::Kv {
                    redis_url,
                    #[cfg(feature = "s3")]
                    s3_bucket,
                }
            }
            _ => StorageBackend::Git,
        };

        // Ensure the data directory exists for the git backend.
        if matches!(storage_backend, StorageBackend::Git)
            && let Err(e) = tokio::fs::create_dir_all(&data_dir).await
        {
            tracing::warn!(dir = %data_dir, error = %e, "could not create data dir");
        }

        let jwt_config = Arc::new(JwtConfig::new(jwt_secret.as_bytes()));

        // The StoredApiKeyResolver needs the same backend config so it can
        // open workspace stores on demand.
        let api_key_resolver: Arc<dyn ApiKeyResolver> = Arc::new(StoredApiKeyResolver {
            data_dir: data_dir.clone(),
            storage_backend: storage_backend.clone(),
        });

        Self {
            data_dir,
            jwt_config,
            api_key_resolver,
            storage_backend,
        }
    }

    // ── Store factories ───────────────────────────────────────────────────────

    /// Open an entity store for the given workspace + entity pair.
    pub async fn open_entity_store(
        &self,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
    ) -> Result<EntityStore, AppError> {
        let backend = self.entity_backend(workspace_id, entity_id).await?;
        EntityStore::open(backend, workspace_id, entity_id)
            .await
            .map_err(|e| match e {
                StorageError::NotFound(m) => AppError::NotFound(m),
                other => AppError::Storage(other),
            })
    }

    /// Open an entity store for a write operation, rejecting dissolved entities.
    ///
    /// Same as [`open_entity_store`] but additionally loads the `Entity` record
    /// and returns `400 Bad Request` if the entity has been dissolved. Use this
    /// at the top of all write handlers to enforce the lifecycle invariant.
    pub async fn open_entity_store_for_write(
        &self,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
    ) -> Result<EntityStore, AppError> {
        let store = self.open_entity_store(workspace_id, entity_id).await?;
        // Check entity lifecycle — dissolved entities reject all writes.
        if let Ok(entity) = store.read::<Entity>(entity_id, "main").await {
            if entity.formation_status == FormationStatus::Dissolved {
                return Err(AppError::BadRequest(
                    "entity is dissolved; no further modifications allowed".into(),
                ));
            }
        }
        Ok(store)
    }

    /// Open a workspace store for the given workspace.
    pub async fn open_workspace_store(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<WorkspaceStore, AppError> {
        let backend = self.workspace_backend(workspace_id).await?;
        WorkspaceStore::open(backend, workspace_id)
            .await
            .map_err(|e| match e {
                StorageError::NotFound(m) => AppError::NotFound(m),
                other => AppError::Storage(other),
            })
    }

    /// Initialise a new entity store (first-time setup, creates the repo).
    ///
    /// For the git backend, ensures that all parent directories of the
    /// entity's repository path exist before delegating to
    /// [`EntityStore::init`].  `gix::init_bare` does not create intermediate
    /// directories, so we do it here.
    pub async fn init_entity_store(
        &self,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
    ) -> Result<EntityStore, AppError> {
        // Pre-create the repo directory tree so `gix::init_bare` can succeed.
        if let StorageBackend::Git = &self.storage_backend {
            let path = PathBuf::from(&self.data_dir)
                .join(workspace_id.to_string())
                .join("entities")
                .join(entity_id.to_string());
            tokio::fs::create_dir_all(&path).await.map_err(|e| {
                AppError::Internal(format!(
                    "could not create entity dir {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }

        let backend = self.entity_backend(workspace_id, entity_id).await?;
        EntityStore::init(backend, workspace_id, entity_id, b"{}")
            .await
            .map_err(|e| match e {
                StorageError::AlreadyExists(m) => AppError::Conflict(m),
                other => AppError::Storage(other),
            })
    }

    /// Initialise a new workspace store (first-time setup).
    ///
    /// For the git backend, ensures that the workspace directory exists before
    /// delegating to [`WorkspaceStore::init`].
    pub async fn init_workspace_store(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<WorkspaceStore, AppError> {
        // Pre-create the workspace repo directory.
        if let StorageBackend::Git = &self.storage_backend {
            let path = PathBuf::from(&self.data_dir)
                .join(workspace_id.to_string())
                .join("workspace");
            tokio::fs::create_dir_all(&path).await.map_err(|e| {
                AppError::Internal(format!(
                    "could not create workspace dir {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }

        let backend = self.workspace_backend(workspace_id).await?;
        WorkspaceStore::init(backend, workspace_id)
            .await
            .map_err(|e| match e {
                StorageError::AlreadyExists(m) => AppError::Conflict(m),
                other => AppError::Storage(other),
            })
    }

    /// Open an existing workspace store, or init one if it doesn't exist yet.
    ///
    /// This is the common path when creating entities — the workspace store
    /// may or may not exist depending on whether this is the first entity in
    /// the workspace.
    pub async fn init_or_open_workspace_store(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<WorkspaceStore, AppError> {
        match self.open_workspace_store(workspace_id).await {
            Ok(ws) => Ok(ws),
            Err(AppError::NotFound(_)) => self.init_workspace_store(workspace_id).await,
            Err(other) => Err(other),
        }
    }

    // ── Internal backend builders ─────────────────────────────────────────────

    /// Build an [`EntityBackend`] for the given workspace + entity.
    pub(crate) async fn entity_backend(
        &self,
        workspace_id: WorkspaceId,
        entity_id: EntityId,
    ) -> Result<EntityBackend, AppError> {
        match &self.storage_backend {
            StorageBackend::Git => {
                let path = PathBuf::from(&self.data_dir)
                    .join(workspace_id.to_string())
                    .join("entities")
                    .join(entity_id.to_string());
                Ok(EntityBackend::Git {
                    repo_path: Arc::new(path),
                })
            }

            StorageBackend::Kv {
                redis_url,
                #[cfg(feature = "s3")]
                s3_bucket,
            } => {
                let manager = kv_connection_manager(redis_url).await?;

                #[cfg(feature = "s3")]
                let s3 = if let Some(bucket) = s3_bucket {
                    let prefix = format!("{}/{}", workspace_id, entity_id);
                    Some(std::sync::Arc::new(
                        corp_storage::s3::S3Backend::new(bucket.clone(), prefix)
                            .await
                            .map_err(AppError::Storage)?,
                    ))
                } else {
                    None
                };

                Ok(EntityBackend::Kv {
                    pool: manager,
                    #[cfg(feature = "s3")]
                    s3,
                })
            }
        }
    }

    /// Build a [`WorkspaceBackend`] for the given workspace.
    pub(crate) async fn workspace_backend(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<WorkspaceBackend, AppError> {
        match &self.storage_backend {
            StorageBackend::Git => {
                let path = PathBuf::from(&self.data_dir)
                    .join(workspace_id.to_string())
                    .join("workspace");
                Ok(WorkspaceBackend::Git {
                    repo_path: Arc::new(path),
                })
            }

            StorageBackend::Kv { redis_url, .. } => {
                let manager = kv_connection_manager(redis_url).await?;
                Ok(WorkspaceBackend::Kv { pool: manager })
            }
        }
    }
}

// ── FromRef impls ─────────────────────────────────────────────────────────────

/// Allows `corp-auth`'s `Principal` extractor to pull the JWT config out of
/// `AppState` automatically.
///
/// The extractor requires `JwtConfig: FromRef<S>` (not `Arc<JwtConfig>`), so
/// we implement the plain-value variant by cloning out of the `Arc`.
impl FromRef<AppState> for JwtConfig {
    fn from_ref(state: &AppState) -> Self {
        (*state.jwt_config).clone()
    }
}

impl FromRef<AppState> for Arc<JwtConfig> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.jwt_config)
    }
}

/// Allows `corp-auth`'s `Principal` extractor to pull the API key resolver out
/// of `AppState` automatically.
impl FromRef<AppState> for Arc<dyn ApiKeyResolver> {
    fn from_ref(state: &AppState) -> Self {
        Arc::clone(&state.api_key_resolver)
    }
}

// ── StoredApiKeyResolver ──────────────────────────────────────────────────────

/// Resolves raw API keys by looking them up in the [`WorkspaceStore`].
///
/// This is the v2 indexed-lookup approach: instead of a global table, each
/// workspace owns its API key records.  The resolver needs the workspace ID,
/// which it derives from the `CORP_DEFAULT_WORKSPACE` environment variable for
/// single-tenant deployments.
///
/// For multi-tenant deployments where the workspace cannot be derived from the
/// key alone, embed the workspace ID in a well-known key prefix and strip it
/// here before the store lookup.
#[derive(Clone, Debug)]
pub struct StoredApiKeyResolver {
    data_dir: String,
    storage_backend: StorageBackend,
}

#[async_trait]
impl ApiKeyResolver for StoredApiKeyResolver {
    async fn resolve(&self, raw_key: &str) -> Result<Principal, AuthError> {
        // Determine which workspace to search.  In a single-tenant deployment
        // CORP_DEFAULT_WORKSPACE is sufficient.  Multi-tenant setups should
        // embed workspace hints in the key format and parse them here.
        let workspace_id = std::env::var("CORP_DEFAULT_WORKSPACE")
            .ok()
            .and_then(|s| s.parse::<WorkspaceId>().ok())
            .ok_or(AuthError::InvalidApiKey)?;

        let ws_backend = self
            .workspace_backend_sync(workspace_id)
            .map_err(|_| AuthError::InvalidApiKey)?;

        let store = WorkspaceStore::open(ws_backend, workspace_id)
            .await
            .map_err(|_| AuthError::InvalidApiKey)?;

        // Iterate all API key records and verify the Argon2 hash.
        let key_ids = store
            .list_api_key_ids()
            .await
            .map_err(|_| AuthError::InvalidApiKey)?;

        for key_id in key_ids {
            let record = match store.read_api_key(key_id).await {
                Ok(r) => r,
                Err(_) => continue,
            };

            if record.deleted {
                continue;
            }

            let matches = ApiKeyManager::verify(raw_key, &record.key_hash).unwrap_or(false);

            if matches {
                // `Scope` has no `FromStr` impl; deserialize from a JSON
                // quoted string instead (matches the kebab-case serde repr).
                let scopes = record
                    .scopes
                    .iter()
                    .filter_map(|s| {
                        serde_json::from_str::<corp_core::auth::Scope>(&format!("\"{}\"", s)).ok()
                    })
                    .collect::<Vec<_>>();

                let scope_set = ScopeSet::from_vec(scopes);
                let entity_ids = record.entity_id.into_iter().collect::<Vec<_>>();
                let entity_id = entity_ids.first().copied();

                return Ok(Principal {
                    workspace_id,
                    entity_id,
                    contact_id: None,
                    entity_ids,
                    principal_type: PrincipalType::User,
                    scopes: scope_set,
                });
            }
        }

        Err(AuthError::InvalidApiKey)
    }
}

impl StoredApiKeyResolver {
    /// Build a workspace backend synchronously (no I/O; only path construction).
    fn workspace_backend_sync(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<WorkspaceBackend, AppError> {
        match &self.storage_backend {
            StorageBackend::Git => {
                let path = PathBuf::from(&self.data_dir)
                    .join(workspace_id.to_string())
                    .join("workspace");
                Ok(WorkspaceBackend::Git {
                    repo_path: Arc::new(path),
                })
            }

            // For KV, we can't build a ConnectionManager synchronously.
            // The caller must use the async path instead.
            StorageBackend::Kv { .. } => Err(AppError::Internal(
                "StoredApiKeyResolver: use async workspace_backend for kv".into(),
            )),
        }
    }
}

// ── KV helpers ────────────────────────────────────────────────────────────────

/// Open a Redis `ConnectionManager` from a URL string.
///
/// The `redis::Client::open` call is synchronous and cheap, but
/// `ConnectionManager::new` performs an initial connection and is `async`.
pub(crate) async fn kv_connection_manager(
    redis_url: &str,
) -> Result<redis::aio::ConnectionManager, AppError> {
    let client = redis::Client::open(redis_url)
        .map_err(|e| AppError::Storage(StorageError::KvError(e.to_string())))?;

    redis::aio::ConnectionManager::new(client)
        .await
        .map_err(|e| AppError::Storage(StorageError::KvError(e.to_string())))
}
