//! Library entry point for `corp-server`.
//!
//! Exposes the application [`state`] and [`routes`] modules so that
//! integration tests (in `tests/`) can construct an in-process router without
//! going through environment variables or a real TCP listener.

// The OpenAPI spec builder uses a single large `serde_json::json!` macro which
// exceeds the default recursion depth of 128 during macro expansion.
#![recursion_limit = "512"]

pub mod error;
pub mod routes;
pub mod state;
