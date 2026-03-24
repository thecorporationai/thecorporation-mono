//! Core storage traits.
//!
//! [`StoredEntity`] is the only trait that domain types need to implement in
//! order to be persisted via [`EntityStore`][crate::entity_store::EntityStore].
//! It deliberately carries no async methods — all I/O lives in the store, not
//! in the entity type.

use serde::{de::DeserializeOwned, Serialize};

/// Implemented by types that can be stored in an entity repository.
///
/// # Example
/// ```rust,ignore
/// use corp_storage::traits::StoredEntity;
/// use corp_core::ids::GovernanceBodyId;
///
/// impl StoredEntity for GovernanceBody {
///     type Id = GovernanceBodyId;
///     fn storage_dir() -> &'static str { "governance/bodies" }
/// }
/// ```
pub trait StoredEntity: DeserializeOwned + Serialize + Send + Sync {
    /// The typed ID used to address individual records.
    ///
    /// Must implement `Display` (for building the storage path), `FromStr`
    /// (for parsing paths back to IDs), `Copy`, and the send/sync marker
    /// traits required for async contexts.
    type Id: std::fmt::Display
        + std::str::FromStr
        + Copy
        + Send
        + Sync
        + 'static;

    /// Directory path within the entity repository (e.g. `"governance/bodies"`).
    ///
    /// The path must be relative and must not start or end with `/`.
    fn storage_dir() -> &'static str;

    /// Full storage path for a specific entity instance.
    ///
    /// Defaults to `"{storage_dir}/{id}.json"`.  Override only if the
    /// naming convention differs for a particular type.
    fn storage_path(id: Self::Id) -> String {
        format!("{}/{}.json", Self::storage_dir(), id)
    }
}
