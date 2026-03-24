//! Authentication and authorization for the corporate governance platform.
//!
//! # Modules
//!
//! - [`error`]      — [`AuthError`] enum and its HTTP response mapping.
//! - [`jwt`]        — HS256 JWT encoding and decoding via [`JwtConfig`].
//! - [`api_key`]    — Argon2id-hashed API key generation and verification via
//!                    [`ApiKeyManager`].
//! - [`principal`]  — Resolved request identity ([`Principal`]) built from
//!                    validated JWT claims.
//! - [`extractors`] — Axum extractors, scoped newtype extractors, and
//!                    [`extractors::RateLimiter`].

pub mod error;
pub mod jwt;
pub mod api_key;
pub mod principal;
pub mod extractors;

// ── Flat re-exports ────────────────────────────────────────────────────────

pub use error::AuthError;
pub use jwt::JwtConfig;
pub use api_key::ApiKeyManager;
pub use principal::Principal;
pub use extractors::{
    ApiKeyResolver,
    RateLimiter,
    // Formation
    RequireFormationCreate,
    RequireFormationRead,
    RequireFormationSign,
    // Equity
    RequireEquityRead,
    RequireEquityWrite,
    // Governance
    RequireGovernanceRead,
    RequireGovernanceWrite,
    RequireGovernanceVote,
    // Treasury
    RequireTreasuryRead,
    RequireTreasuryWrite,
    // Contacts
    RequireContactsRead,
    RequireContactsWrite,
    // Execution
    RequireExecutionRead,
    RequireExecutionWrite,
    // Agents
    RequireAgentsRead,
    RequireAgentsWrite,
    // Work items
    RequireWorkItemsRead,
    RequireWorkItemsWrite,
    // Services
    RequireServicesRead,
    RequireServicesWrite,
    // Platform
    RequireAdmin,
};
