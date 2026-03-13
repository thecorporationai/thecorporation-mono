pub mod admin;
pub mod agent_executions;
pub mod agents;
pub mod auth;
pub mod branches;
pub mod compliance;
pub mod contacts;
pub mod equity;
pub mod execution;
pub mod formation;
pub mod governance;
pub mod governance_enforcement;
pub mod llm_proxy;
pub mod references;
pub mod secret_proxies;
pub mod secrets_proxy;
pub mod services;
pub mod treasury;
pub mod validation;
pub mod work_items;

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::FromRef;

use crate::domain::ids::{EntityId, WorkspaceId};
use crate::error::AppError;
use crate::git::signing::CommitSigner;
use crate::store::{RepoLayout, StorageBackendKind};

/// Query params requiring both workspace and entity identification.
///
/// Deprecated: use `EntityIdQuery` with a scoped auth extractor instead.
#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct WorkspaceEntityQuery {
    pub workspace_id: WorkspaceId,
    pub entity_id: EntityId,
}

/// Query param for entity identification (workspace_id comes from auth principal).
#[derive(serde::Deserialize, utoipa::IntoParams)]
pub struct EntityIdQuery {
    pub entity_id: EntityId,
}

/// Shared application state, passed to all route handlers via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub layout: Arc<RepoLayout>,
    /// Shared secret used to sign and verify JWTs.
    /// Loaded once from `JWT_SECRET` env var at startup.
    pub jwt_secret: Arc<[u8]>,
    /// Optional Ed25519 signer for cryptographic commit provenance.
    /// When present, all git commits are signed with this key.
    pub commit_signer: Option<Arc<CommitSigner>>,
    /// Optional Redis pool for agent execution queue + state.
    /// When absent, agent messaging works but jobs are not dispatched.
    pub redis: Option<deadpool_redis::Pool>,
    /// Fernet key for encrypting/decrypting secrets at rest in workspace repos.
    /// Loaded from `SECRETS_MASTER_KEY` env var. When absent, secret proxy
    /// operations that require encryption/decryption will fail.
    pub secrets_fernet: Option<Arc<fernet::Fernet>>,
    /// Maximum number of jobs allowed in the agent execution queue (0 = unlimited).
    pub max_queue_depth: u64,
    /// HTTP client for proxying LLM requests to upstream providers.
    pub http_client: reqwest::Client,
    /// Base URL for the upstream LLM provider (e.g. OpenRouter).
    pub llm_upstream_url: String,
    /// Model pricing table: model name -> pricing (cents per million tokens).
    pub model_pricing: HashMap<String, ModelPricing>,
    /// In-process throttle for bursty resource creation endpoints.
    pub creation_rate_limiter: Arc<CreationRateLimiter>,
    /// Which storage backend is active (git or valkey).
    pub storage_backend: StorageBackendKind,
    /// Sync Redis/Valkey client for store operations inside `spawn_blocking`.
    /// Required when `storage_backend` is `Valkey`, ignored for `Git`.
    pub valkey_client: Option<redis::Client>,
}

#[derive(Default)]
pub struct CreationRateLimiter {
    buckets: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl CreationRateLimiter {
    pub fn check(&self, key: String, limit: u32, window_seconds: u32) -> Result<(), AppError> {
        let mut buckets = self.buckets.lock().map_err(|_| {
            AppError::ServiceUnavailable("creation rate limiter unavailable".to_owned())
        })?;
        let window = Duration::from_secs(u64::from(window_seconds));
        let now = Instant::now();
        let entries = buckets.entry(key).or_default();
        while entries
            .front()
            .is_some_and(|instant| now.duration_since(*instant) >= window)
        {
            entries.pop_front();
        }
        if entries.len() >= limit as usize {
            return Err(AppError::RateLimited {
                limit,
                window_seconds,
            });
        }
        entries.push_back(now);
        Ok(())
    }
}

impl AppState {
    pub fn enforce_creation_rate_limit(
        &self,
        scope: &str,
        workspace_id: WorkspaceId,
        limit: u32,
        window_seconds: u32,
    ) -> Result<(), AppError> {
        self.creation_rate_limiter
            .check(format!("{workspace_id}:{scope}"), limit, window_seconds)
    }
}

/// Pricing for a single model: input/output costs in cents per million tokens.
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input: u64,
    pub output: u64,
}

/// Wrapper for extracting the optional Valkey client via `FromRef`.
#[derive(Clone)]
pub struct ValkeyClient(pub Option<redis::Client>);

impl FromRef<AppState> for Arc<RepoLayout> {
    fn from_ref(state: &AppState) -> Arc<RepoLayout> {
        state.layout.clone()
    }
}

impl FromRef<AppState> for Arc<[u8]> {
    fn from_ref(state: &AppState) -> Arc<[u8]> {
        state.jwt_secret.clone()
    }
}

impl FromRef<AppState> for ValkeyClient {
    fn from_ref(state: &AppState) -> ValkeyClient {
        ValkeyClient(state.valkey_client.clone())
    }
}
