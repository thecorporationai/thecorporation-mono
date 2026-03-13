//! Auth middleware — Axum extractor that resolves a `Principal` from the
//! `Authorization` header (Bearer JWT or raw API key).

use std::sync::Arc;

use axum::Json;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::domain::auth::claims::PrincipalType;
use crate::domain::auth::claims::decode_token;
use crate::domain::auth::error::AuthError;
use crate::domain::auth::scopes::{Scope, ScopeSet};
use crate::domain::ids::{ContactId, EntityId, WorkspaceId};
use crate::routes::ValkeyClient;
use crate::store::RepoLayout;
use crate::store::workspace_store::WorkspaceStore;

// ── Scoped extractors ──────────────────────────────────────────────────
//
// Each scoped extractor wraps a `Principal` and checks that the principal
// has the required scope. Adding a scoped extractor to a handler signature
// enforces auth + authorization at compile time — if the extractor is
// missing, the handler cannot access `workspace_id`.

macro_rules! define_scoped_extractor {
    ($name:ident, $scope:expr) => {
        #[derive(Debug)]
        pub struct $name(pub Principal);

        impl $name {
            pub fn workspace_id(&self) -> WorkspaceId {
                self.0.workspace_id()
            }

            #[allow(dead_code)]
            pub fn contact_id(&self) -> Option<ContactId> {
                self.0.contact_id()
            }

            #[allow(dead_code)]
            pub fn entity_id(&self) -> Option<EntityId> {
                self.0.entity_id()
            }

            #[allow(dead_code)]
            pub fn entity_ids(&self) -> Option<&[EntityId]> {
                self.0.entity_ids()
            }

            #[allow(dead_code)]
            pub fn allows_entity(&self, entity_id: EntityId) -> bool {
                self.0.allows_entity(entity_id)
            }

            #[allow(dead_code)]
            pub fn scopes(&self) -> &ScopeSet {
                self.0.scopes()
            }
        }

        impl<S> FromRequestParts<S> for $name
        where
            S: Send + Sync + 'static,
            Arc<RepoLayout>: FromRef<S>,
            ValkeyClient: FromRef<S>,
        {
            type Rejection = AuthRejection;

            async fn from_request_parts(
                parts: &mut Parts,
                state: &S,
            ) -> Result<Self, Self::Rejection> {
                let principal = Principal::from_request_parts(parts, state).await?;
                if !principal.scopes().has($scope) {
                    return Err(AuthRejection(AuthError::InsufficientScopes(format!(
                        "required: {}",
                        $scope
                    ))));
                }
                Ok(Self(principal))
            }
        }
    };
}

// Formation
define_scoped_extractor!(RequireFormationCreate, Scope::FormationCreate);
define_scoped_extractor!(RequireFormationRead, Scope::FormationRead);
define_scoped_extractor!(RequireFormationSign, Scope::FormationSign);

// Equity
define_scoped_extractor!(RequireEquityRead, Scope::EquityRead);
define_scoped_extractor!(RequireEquityWrite, Scope::EquityWrite);
define_scoped_extractor!(RequireEquityTransfer, Scope::EquityTransfer);

// Governance
define_scoped_extractor!(RequireGovernanceRead, Scope::GovernanceRead);
define_scoped_extractor!(RequireGovernanceWrite, Scope::GovernanceWrite);
define_scoped_extractor!(RequireGovernanceVote, Scope::GovernanceVote);

// Treasury
define_scoped_extractor!(RequireTreasuryRead, Scope::TreasuryRead);
define_scoped_extractor!(RequireTreasuryWrite, Scope::TreasuryWrite);
define_scoped_extractor!(RequireTreasuryApprove, Scope::TreasuryApprove);

// Contacts
define_scoped_extractor!(RequireContactsRead, Scope::ContactsRead);
define_scoped_extractor!(RequireContactsWrite, Scope::ContactsWrite);

// Execution
define_scoped_extractor!(RequireExecutionRead, Scope::ExecutionRead);
define_scoped_extractor!(RequireExecutionWrite, Scope::ExecutionWrite);

// Services
define_scoped_extractor!(RequireServicesRead, Scope::ServicesRead);
define_scoped_extractor!(RequireServicesWrite, Scope::ServicesWrite);

// Branches
define_scoped_extractor!(RequireBranchCreate, Scope::BranchCreate);
define_scoped_extractor!(RequireBranchMerge, Scope::BranchMerge);
define_scoped_extractor!(RequireBranchDelete, Scope::BranchDelete);

// Admin
define_scoped_extractor!(RequireAdmin, Scope::Admin);

/// The resolved identity of a request.
///
/// Extracted from the `Authorization` header. Routes that need auth should
/// include `Principal` as an extractor parameter.
#[derive(Debug, Clone)]
pub struct Principal {
    workspace_id: WorkspaceId,
    entity_id: Option<EntityId>,
    contact_id: Option<ContactId>,
    entity_ids: Option<Vec<EntityId>>,
    principal_type: PrincipalType,
    scopes: ScopeSet,
}

impl Principal {
    /// The workspace this principal is acting within.
    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    /// Optional entity the principal represents (officer, member, etc.).
    pub fn entity_id(&self) -> Option<EntityId> {
        self.entity_id
    }

    /// Optional contact represented by the principal.
    pub fn contact_id(&self) -> Option<ContactId> {
        self.contact_id
    }

    /// Optional explicit entity scope for this principal.
    pub fn entity_ids(&self) -> Option<&[EntityId]> {
        self.entity_ids.as_deref()
    }

    /// Returns true if this principal can access `entity_id`.
    pub fn allows_entity(&self, entity_id: EntityId) -> bool {
        match self.entity_ids() {
            Some(ids) => ids.contains(&entity_id),
            None => true,
        }
    }

    pub fn principal_type(&self) -> PrincipalType {
        self.principal_type
    }

    /// The scopes granted to this principal.
    pub fn scopes(&self) -> &ScopeSet {
        &self.scopes
    }
}

/// Rejection type when auth extraction fails.
#[derive(Debug)]
pub struct AuthRejection(AuthError);

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let (status, msg) = match &self.0 {
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AuthError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "invalid api key"),
            AuthError::ExpiredApiKey => (StatusCode::UNAUTHORIZED, "expired api key"),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "token expired"),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "invalid token"),
            AuthError::InsufficientScopes(_) => (StatusCode::FORBIDDEN, "insufficient scopes"),
        };
        let body = json!({ "error": { "code": "auth_error", "detail": msg } });
        (status, Json(body)).into_response()
    }
}

impl<S> FromRequestParts<S> for Principal
where
    S: Send + Sync + 'static,
    Arc<RepoLayout>: FromRef<S>,
    ValkeyClient: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthRejection(AuthError::Unauthorized))?;

        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if token.starts_with("sk_") {
                // Direct API key path (Bearer sk_...) for integration parity.
                let layout = Arc::<RepoLayout>::from_ref(state);
                let ValkeyClient(vc) = ValkeyClient::from_ref(state);
                return principal_from_api_key(&layout, token, vc.as_ref())
                    .map_err(AuthRejection);
            }

            // JWT path — decode the token using the shared secret from env.
            // In production JWT_SECRET must be set; in debug builds we fall
            // back to an insecure dev secret (matching main.rs startup).
            let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                if cfg!(debug_assertions) {
                    "dev-secret-do-not-use-in-production".into()
                } else {
                    // In release, if the env var is missing, reject all tokens.
                    String::new()
                }
            });
            if secret.is_empty() {
                return Err(AuthRejection(AuthError::Unauthorized));
            }
            let claims = decode_token(token, secret.as_bytes()).map_err(AuthRejection)?;

            return Ok(Principal {
                workspace_id: claims.workspace_id(),
                entity_id: claims.entity_id(),
                contact_id: claims.contact_id(),
                entity_ids: claims.entity_ids().map(|ids| ids.to_vec()),
                principal_type: claims.principal_type(),
                scopes: ScopeSet::from_vec(claims.scopes().to_vec()),
            });
        }

        if auth_header.starts_with("sk_") {
            // Also accept raw API key header (without Bearer prefix).
            let layout = Arc::<RepoLayout>::from_ref(state);
            let ValkeyClient(vc) = ValkeyClient::from_ref(state);
            return principal_from_api_key(&layout, auth_header, vc.as_ref())
                .map_err(AuthRejection);
        }

        Err(AuthRejection(AuthError::Unauthorized))
    }
}

fn principal_from_api_key(
    layout: &RepoLayout,
    api_key: &str,
    valkey_client: Option<&redis::Client>,
) -> Result<Principal, AuthError> {
    let (workspace_ids, shared_con) = WorkspaceStore::list_and_prepare(layout, valkey_client)
        .map_err(|_| AuthError::Unauthorized)?;

    for workspace_id in workspace_ids {
        let ws_store = match WorkspaceStore::open_shared(layout, workspace_id, shared_con.clone()) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let key_ids = match ws_store.list_api_key_ids() {
            Ok(ids) => ids,
            Err(_) => continue,
        };

        for key_id in key_ids {
            let Ok(record) = ws_store.read_api_key(key_id) else {
                continue;
            };
            if !record.is_valid() {
                continue;
            }
            let Ok(matches) =
                crate::domain::auth::api_key::verify_api_key(api_key, record.key_hash())
            else {
                continue;
            };
            if !matches {
                continue;
            }

            let entity_ids = record.entity_ids().map(|ids| ids.to_vec());
            let entity_id = entity_ids.as_ref().and_then(|ids| ids.first()).copied();
            return Ok(Principal {
                workspace_id: record.workspace_id(),
                entity_id,
                contact_id: record.contact_id(),
                entity_ids,
                principal_type: PrincipalType::User,
                scopes: ScopeSet::from_vec(record.scopes().to_vec()),
            });
        }
    }

    Err(AuthError::InvalidApiKey)
}

/// Extractor for internal worker traffic authenticated with a static bearer token.
#[derive(Debug, Clone, Copy)]
pub struct RequireInternalWorker;

impl<S> FromRequestParts<S> for RequireInternalWorker
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthRejection(AuthError::Unauthorized))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthRejection(AuthError::Unauthorized))?;

        let expected = std::env::var("INTERNAL_WORKER_TOKEN").unwrap_or_default();
        if expected.is_empty() || token != expected {
            return Err(AuthRejection(AuthError::Unauthorized));
        }

        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth::claims::{Claims, encode_token};
    use crate::domain::auth::scopes::Scope;
    use axum::http::Request;
    use chrono::Utc;

    /// Minimal test state that satisfies the `FromRef` bounds for auth extractors.
    #[derive(Clone)]
    struct TestState {
        layout: Arc<RepoLayout>,
    }

    impl TestState {
        fn new() -> Self {
            Self {
                layout: Arc::new(RepoLayout::new(std::path::PathBuf::from("/tmp/test-repos"))),
            }
        }
    }

    impl FromRef<TestState> for Arc<RepoLayout> {
        fn from_ref(state: &TestState) -> Arc<RepoLayout> {
            state.layout.clone()
        }
    }

    impl FromRef<TestState> for ValkeyClient {
        fn from_ref(_state: &TestState) -> ValkeyClient {
            ValkeyClient(None)
        }
    }

    /// Helper: extract Principal from a request with the given Authorization header.
    async fn extract_principal(auth_value: &str) -> Result<Principal, AuthRejection> {
        let state = TestState::new();
        let req = Request::builder()
            .header("authorization", auth_value)
            .body(())
            .expect("build request");
        let (mut parts, _body) = req.into_parts();
        Principal::from_request_parts(&mut parts, &state).await
    }

    #[tokio::test]
    async fn missing_header_returns_unauthorized() {
        let state = TestState::new();
        let req = Request::builder().body(()).expect("build request");
        let (mut parts, _body) = req.into_parts();
        let result = Principal::from_request_parts(&mut parts, &state).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn valid_bearer_token_extracts_principal() {
        let secret = "dev-secret-do-not-use-in-production";
        // SAFETY: test-only; tests run single-threaded via cargo test default.
        unsafe { std::env::set_var("JWT_SECRET", secret) };

        let ws = WorkspaceId::new();
        let now = Utc::now().timestamp();
        let claims = Claims::new(
            ws,
            None,
            None,
            None,
            PrincipalType::User,
            vec![Scope::Admin],
            now,
            now + 3600,
        );
        let token = encode_token(&claims, secret.as_bytes()).expect("encode");

        let principal = extract_principal(&format!("Bearer {token}"))
            .await
            .expect("extract principal");

        assert_eq!(principal.workspace_id(), ws);
        assert!(principal.scopes().has(Scope::Admin));
        assert!(principal.entity_id().is_none());
        assert!(principal.contact_id().is_none());
        assert!(principal.entity_ids().is_none());
    }

    #[tokio::test]
    async fn invalid_bearer_token_rejected() {
        // SAFETY: test-only; tests run single-threaded via cargo test default.
        unsafe { std::env::set_var("JWT_SECRET", "dev-secret-do-not-use-in-production") };
        let result = extract_principal("Bearer not.a.valid.jwt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn api_key_stub_returns_invalid() {
        let result = extract_principal("sk_abc123").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn garbage_header_returns_unauthorized() {
        let result = extract_principal("Basic dXNlcjpwYXNz").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn internal_worker_token_extracts() {
        // SAFETY: test-only env mutation.
        unsafe { std::env::set_var("INTERNAL_WORKER_TOKEN", "worker-token-test") };
        let state = TestState::new();
        let req = Request::builder()
            .header("authorization", "Bearer worker-token-test")
            .body(())
            .expect("build request");
        let (mut parts, _body) = req.into_parts();
        let extracted = RequireInternalWorker::from_request_parts(&mut parts, &state).await;
        assert!(extracted.is_ok());
    }
}
