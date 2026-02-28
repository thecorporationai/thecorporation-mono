//! Auth middleware — Axum extractor that resolves a `Principal` from the
//! `Authorization` header (Bearer JWT or raw API key).

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::domain::auth::claims::decode_token;
use crate::domain::auth::error::AuthError;
use crate::domain::auth::scopes::{Scope, ScopeSet};
use crate::domain::ids::{EntityId, WorkspaceId};

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
            pub fn entity_id(&self) -> Option<EntityId> {
                self.0.entity_id()
            }

            #[allow(dead_code)]
            pub fn scopes(&self) -> &ScopeSet {
                self.0.scopes()
            }
        }

        impl<S: Send + Sync> FromRequestParts<S> for $name {
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
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthRejection(AuthError::Unauthorized))?;

        if let Some(token) = auth_header.strip_prefix("Bearer ") {
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
            let claims =
                decode_token(token, secret.as_bytes()).map_err(AuthRejection)?;

            return Ok(Principal {
                workspace_id: claims.workspace_id(),
                entity_id: claims.entity_id(),
                scopes: ScopeSet::from_vec(claims.scopes().to_vec()),
            });
        }

        if auth_header.starts_with("sk_") {
            // API key path — raw API keys cannot be verified in the middleware
            // because keys are stored per-workspace and the middleware does not
            // know which workspace to search. Callers must first exchange their
            // API key for a JWT via POST /v1/auth/token-exchange, then use the
            // returned Bearer token for subsequent requests.
            return Err(AuthRejection(AuthError::InvalidApiKey));
        }

        Err(AuthRejection(AuthError::Unauthorized))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::auth::claims::{encode_token, Claims};
    use crate::domain::auth::scopes::Scope;
    use axum::http::Request;
    use chrono::Utc;

    /// Helper: extract Principal from a request with the given Authorization header.
    async fn extract_principal(auth_value: &str) -> Result<Principal, AuthRejection> {
        let req = Request::builder()
            .header("authorization", auth_value)
            .body(())
            .expect("build request");
        let (mut parts, _body) = req.into_parts();
        Principal::from_request_parts(&mut parts, &()).await
    }

    #[tokio::test]
    async fn missing_header_returns_unauthorized() {
        let req = Request::builder().body(()).expect("build request");
        let (mut parts, _body) = req.into_parts();
        let result = Principal::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn valid_bearer_token_extracts_principal() {
        let secret = "dev-secret-do-not-use-in-production";
        // SAFETY: test-only; tests run single-threaded via cargo test default.
        unsafe { std::env::set_var("JWT_SECRET", secret) };

        let ws = WorkspaceId::new();
        let now = Utc::now().timestamp();
        let claims = Claims::new(ws, None, vec![Scope::Admin], now, now + 3600);
        let token = encode_token(&claims, secret.as_bytes()).expect("encode");

        let principal = extract_principal(&format!("Bearer {token}"))
            .await
            .expect("extract principal");

        assert_eq!(principal.workspace_id(), ws);
        assert!(principal.scopes().has(Scope::Admin));
        assert!(principal.entity_id().is_none());
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
}
