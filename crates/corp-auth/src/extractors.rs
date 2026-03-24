//! Axum extractors for authentication and authorization.
//!
//! # Token resolution order
//!
//! 1. `Authorization: Bearer corp_…` → API key path.
//! 2. `X-Api-Key: corp_…`           → API key path.
//! 3. `Authorization: Bearer <jwt>`     → JWT path.
//!
//! # Scoped extractors
//!
//! Use the generated newtypes (e.g. [`RequireFormationRead`]) as handler
//! parameters to enforce scope checks at the type level.  The inner
//! [`Principal`] is accessible via the tuple field (`.0`).
//!
//! # Rate limiter
//!
//! [`RateLimiter`] is a simple in-process sliding-window limiter keyed on an
//! arbitrary string (typically the workspace ID or raw API key).

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use corp_core::auth::Scope;

use crate::error::AuthError;
use crate::jwt::JwtConfig;
use crate::principal::Principal;

// ── ApiKeyResolver ─────────────────────────────────────────────────────────

/// Application-level trait for resolving a raw API key to a [`Principal`].
///
/// Implement this trait for your storage backend (database, cache, etc.) and
/// register it in your Axum application state.
///
/// The implementation must perform its own hash verification (typically via
/// [`crate::api_key::ApiKeyManager::verify`]) against stored key hashes.
#[async_trait]
pub trait ApiKeyResolver: Send + Sync + 'static {
    /// Resolve `raw_key` to a [`Principal`], or return an auth error.
    ///
    /// Return [`AuthError::InvalidApiKey`] when the key does not exist or
    /// does not pass verification.
    async fn resolve(&self, raw_key: &str) -> Result<Principal, AuthError>;
}

// ── Principal extractor ────────────────────────────────────────────────────

/// Bearer-token / API-key prefix used to identify live secret keys.
const API_KEY_PREFIX: &str = "corp_";

impl<S> FromRequestParts<S> for Principal
where
    S: Send + Sync,
    JwtConfig: FromRef<S>,
    Arc<dyn ApiKeyResolver>: FromRef<S>,
{
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        // Collect what we need from parts before the async move.
        let x_api_key = parts
            .headers
            .get("X-Api-Key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        let jwt_config = JwtConfig::from_ref(state);
        let resolver = Arc::<dyn ApiKeyResolver>::from_ref(state);

        async move {
            // 1. Check X-Api-Key header first.
            if let Some(raw) = x_api_key {
                return resolver.resolve(&raw).await;
            }

            // 2. Parse the Authorization header.
            let header = auth_header.ok_or(AuthError::MissingToken)?;

            let bearer = header
                .strip_prefix("Bearer ")
                .ok_or(AuthError::InvalidToken)?
                .to_owned();

            // 3. Dispatch on key prefix vs. JWT.
            if bearer.starts_with(API_KEY_PREFIX) {
                resolver.resolve(&bearer).await
            } else {
                let claims = jwt_config.decode(&bearer)?;
                Ok(Principal::from_claims(claims))
            }
        }
    }
}

// ── Scoped extractors (macro-generated) ────────────────────────────────────

/// Generate a newtype extractor that resolves a [`Principal`] and then
/// enforces a required scope, rejecting the request with
/// [`AuthError::InsufficientScope`] if the scope is absent.
///
/// The generated type derives `Debug` and `Clone` and exposes the inner
/// `Principal` as the `.0` tuple field.
#[macro_export]
macro_rules! require_scope {
    ($name:ident, $scope:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name(pub $crate::principal::Principal);

        impl<S> axum::extract::FromRequestParts<S> for $name
        where
            S: Send + Sync,
            $crate::principal::Principal:
                axum::extract::FromRequestParts<S, Rejection = $crate::error::AuthError>,
        {
            type Rejection = $crate::error::AuthError;

            fn from_request_parts(
                parts: &mut axum::http::request::Parts,
                state: &S,
            ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
                let principal_fut =
                    <$crate::principal::Principal as axum::extract::FromRequestParts<S>>::from_request_parts(parts, state);
                async move {
                    let principal = principal_fut.await?;
                    principal.require_scope(&$scope)?;
                    Ok(Self(principal))
                }
            }
        }
    };
}

// Formation
require_scope!(RequireFormationCreate, Scope::FormationCreate);
require_scope!(RequireFormationRead, Scope::FormationRead);
require_scope!(RequireFormationSign, Scope::FormationSign);

// Equity
require_scope!(RequireEquityRead, Scope::EquityRead);
require_scope!(RequireEquityWrite, Scope::EquityWrite);

// Governance
require_scope!(RequireGovernanceRead, Scope::GovernanceRead);
require_scope!(RequireGovernanceWrite, Scope::GovernanceWrite);
require_scope!(RequireGovernanceVote, Scope::GovernanceVote);

// Treasury
require_scope!(RequireTreasuryRead, Scope::TreasuryRead);
require_scope!(RequireTreasuryWrite, Scope::TreasuryWrite);

// Contacts
require_scope!(RequireContactsRead, Scope::ContactsRead);
require_scope!(RequireContactsWrite, Scope::ContactsWrite);

// Execution
require_scope!(RequireExecutionRead, Scope::ExecutionRead);
require_scope!(RequireExecutionWrite, Scope::ExecutionWrite);

// Agents
require_scope!(RequireAgentsRead, Scope::AgentsRead);
require_scope!(RequireAgentsWrite, Scope::AgentsWrite);

// Work items
require_scope!(RequireWorkItemsRead, Scope::WorkItemsRead);
require_scope!(RequireWorkItemsWrite, Scope::WorkItemsWrite);

// Services
require_scope!(RequireServicesRead, Scope::ServicesRead);
require_scope!(RequireServicesWrite, Scope::ServicesWrite);

// Platform
require_scope!(RequireAdmin, Scope::Admin);

// ── RateLimiter ───────────────────────────────────────────────────────────

/// A simple in-process sliding-window rate limiter.
///
/// Keys are arbitrary strings (workspace IDs, IP addresses, API key prefixes,
/// etc.).  All state is held in memory and is not shared across processes.
///
/// For production use with multiple server instances, replace this with a
/// Redis-backed implementation.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    limits: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    /// Create a new `RateLimiter` allowing `max_requests` within `window`.
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            limits: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// Check the rate limit for `key`.
    ///
    /// Prunes timestamps older than the sliding window, then records the
    /// current request.  Returns [`AuthError::RateLimited`] when the request
    /// count for the window has been exceeded.
    pub fn check(&self, key: &str) -> Result<(), AuthError> {
        let mut map = self
            .limits
            .lock()
            .map_err(|e| AuthError::InternalError(format!("rate limiter lock poisoned: {e}")))?;

        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or_else(Instant::now);

        let timestamps = map.entry(key.to_owned()).or_default();

        // Evict entries that have fallen outside the sliding window.
        timestamps.retain(|&t| t >= cutoff);

        if timestamps.len() >= self.max_requests {
            return Err(AuthError::RateLimited);
        }

        timestamps.push(now);
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn rate_limiter_allows_under_limit() {
        let rl = RateLimiter::new(5, Duration::from_secs(60));
        for _ in 0..5 {
            rl.check("workspace-1").unwrap();
        }
    }

    #[test]
    fn rate_limiter_blocks_over_limit() {
        let rl = RateLimiter::new(3, Duration::from_secs(60));
        for _ in 0..3 {
            rl.check("workspace-2").unwrap();
        }
        match rl.check("workspace-2") {
            Err(AuthError::RateLimited) => {}
            other => panic!("expected RateLimited, got {:?}", other),
        }
    }

    #[test]
    fn rate_limiter_different_keys_are_independent() {
        let rl = RateLimiter::new(1, Duration::from_secs(60));
        rl.check("key-a").unwrap();
        rl.check("key-b").unwrap(); // separate bucket — should succeed
        match rl.check("key-a") {
            Err(AuthError::RateLimited) => {}
            other => panic!("expected RateLimited for key-a, got {:?}", other),
        }
    }

    #[test]
    fn rate_limiter_window_expires() {
        // Use a 1-ns window so timestamps immediately become stale.
        let rl = RateLimiter::new(1, Duration::from_nanos(1));
        rl.check("key-c").unwrap();
        // Spin briefly to ensure the 1-ns window elapses.
        std::thread::sleep(Duration::from_millis(1));
        // Window has passed — the previous entry is pruned, so this succeeds.
        rl.check("key-c").unwrap();
    }
}
