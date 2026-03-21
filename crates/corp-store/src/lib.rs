//! Valkey-backed git-compatible content-addressed store.
//!
//! Replaces per-entity bare git repositories with a single Valkey instance.
//! Objects are content-addressed with dual hashing: SHA-1 (git compatible)
//! and SHA-256 (future-proof internal key). A bidirectional lookup table
//! bridges the two hash spaces.
//!
//! # Data model
//!
//! ```text
//! corp:blob           HASH   {sha256} → {bytes}           Blob content (primary key = SHA-256)
//! corp:oid:1to256     HASH   {sha1}   → {sha256}          SHA-1 → SHA-256 lookup
//! corp:oid:256to1     HASH   {sha256} → {sha1}            SHA-256 → SHA-1 lookup
//! corp:log:{ws}:{ent} ZSET   score=seq, member={json}     Commit log per entity
//! corp:seq:{ws}:{ent} STRING monotonic counter             Sequence generator
//! corp:sha:{ws}:{ent} HASH   {sha1} → {seq}               Commit SHA → sequence
//! corp:ref:{ws}:{ent} HASH   {branch} → {sha1}            Branch heads
//! corp:tree:{ws}:{ent}:{br}  HASH   {path} → {sha1}       Current file tree per branch
//! corp:file:{ws}:{ent}:{path} ZSET  score=seq, member={sha1}  File history
//! corp:actor:{ws}     ZSET   score=seq, member={ws}:{ent}:{sha1}  Actor audit trail
//! corp:workspaces     SET    {ws_id}                       All workspaces
//! corp:entities:{ws}  SET    {ent_id}                      Entities per workspace
//! ```
//!
//! # SHA-1 compatibility
//!
//! Hash computation replicates git's exact object format (`blob {len}\0{content}`,
//! etc.) so that SHA-1 OIDs match real git. External repos can push changes
//! via the application layer and the OIDs will be identical.
//!
//! # Usage
//!
//! ```no_run
//! use corp_store::{store, entry::FileWrite};
//! use chrono::Utc;
//!
//! let client = redis::Client::open("redis://127.0.0.1/").unwrap();
//! let mut con = client.get_connection().unwrap();
//!
//! // Write files.
//! let files = vec![FileWrite::new("bylaws.json", b"{}".to_vec())];
//! let oid = store::commit_files(&mut con, "ws_1", "ent_1", "main", "add bylaws", &files, None, Utc::now()).unwrap();
//!
//! // Read back.
//! let content = store::read_blob(&mut con, "ws_1", "ent_1", "main", "bylaws.json").unwrap();
//! ```

pub mod branch;
pub mod durable;
pub mod entry;
pub mod error;
pub mod git_protocol;
pub mod keys;
pub mod merge;
pub mod oid;
pub mod store;

pub use error::StoreError;
pub use oid::DualOid;
pub use store::GitObjectType;
