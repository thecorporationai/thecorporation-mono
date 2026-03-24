//! Authentication and authorization for the corporate governance platform.
//!
//! # Modules
//!
//! - [`error`]      — [`AuthError`] enum and its HTTP response mapping.
//! - [`jwt`]        — HS256 JWT encoding and decoding via [`JwtConfig`].
//! - [`api_key`] — Argon2id-hashed API key generation and verification via [`ApiKeyManager`].
//! - [`principal`] — Resolved request identity ([`Principal`]) built from validated JWT claims.
//! - [`extractors`] — Axum extractors, scoped newtype extractors, and [`extractors::RateLimiter`].

pub mod api_key;
pub mod error;
pub mod extractors;
pub mod jwt;
pub mod principal;

// ── Flat re-exports ────────────────────────────────────────────────────────

pub use api_key::ApiKeyManager;
pub use error::AuthError;
pub use extractors::{
    ApiKeyResolver,
    RateLimiter,
    // Platform
    RequireAdmin,
    // Agents
    RequireAgentsRead,
    RequireAgentsWrite,
    // Contacts
    RequireContactsRead,
    RequireContactsWrite,
    // Equity
    RequireEquityRead,
    RequireEquityWrite,
    // Execution
    RequireExecutionRead,
    RequireExecutionWrite,
    // Formation
    RequireFormationCreate,
    RequireFormationRead,
    RequireFormationSign,
    // Governance
    RequireGovernanceRead,
    RequireGovernanceVote,
    RequireGovernanceWrite,
    // Services
    RequireServicesRead,
    RequireServicesWrite,
    // Treasury
    RequireTreasuryRead,
    RequireTreasuryWrite,
    // Work items
    RequireWorkItemsRead,
    RequireWorkItemsWrite,
};
pub use jwt::JwtConfig;
pub use principal::Principal;
