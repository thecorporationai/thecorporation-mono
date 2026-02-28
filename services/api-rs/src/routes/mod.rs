pub mod admin;
pub mod agent_executions;
pub mod agents;
pub mod auth;
pub mod billing;
pub mod branches;
pub mod compliance;
pub mod contacts;
pub mod equity;
pub mod execution;
pub mod formation;
pub mod governance;
pub mod llm_proxy;
pub mod secret_proxies;
pub mod secrets_proxy;
pub mod services;
pub mod treasury;
pub mod webhooks;

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::ids::{EntityId, WorkspaceId};
use crate::git::signing::CommitSigner;
use crate::store::RepoLayout;

/// Query params requiring both workspace and entity identification.
///
/// Deprecated: use `EntityIdQuery` with a scoped auth extractor instead.
#[derive(serde::Deserialize)]
pub struct WorkspaceEntityQuery {
    pub workspace_id: WorkspaceId,
    pub entity_id: EntityId,
}

/// Query param for entity identification (workspace_id comes from auth principal).
#[derive(serde::Deserialize)]
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
}

/// Pricing for a single model: input/output costs in cents per million tokens.
#[derive(Debug, Clone)]
pub struct ModelPricing {
    pub input: u64,
    pub output: u64,
}
