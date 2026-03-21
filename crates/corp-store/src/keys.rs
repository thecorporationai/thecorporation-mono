//! Valkey key patterns.
//!
//! All keys are prefixed with `corp:` to namespace within a shared Valkey instance.

/// Blob content by SHA-256: `HSET corp:blob {sha256_hex} {bytes}`
pub fn blob_key() -> &'static str {
    "corp:blob"
}

/// SHA-1 → SHA-256 mapping: `HSET corp:oid:1to256 {sha1_hex} {sha256_hex}`
pub fn oid_1to256_key() -> &'static str {
    "corp:oid:1to256"
}

/// SHA-256 → SHA-1 mapping: `HSET corp:oid:256to1 {sha256_hex} {sha1_hex}`
pub fn oid_256to1_key() -> &'static str {
    "corp:oid:256to1"
}

/// Commit log sorted set: `ZADD corp:log:{ws}:{ent} {seq} {json}`
pub fn log_key(ws: &str, ent: &str) -> String {
    format!("corp:log:{ws}:{ent}")
}

/// Sequence counter: `INCR corp:seq:{ws}:{ent}`
pub fn seq_key(ws: &str, ent: &str) -> String {
    format!("corp:seq:{ws}:{ent}")
}

/// SHA-1 → sequence lookup: `HSET corp:sha:{ws}:{ent} {sha1_hex} {seq}`
pub fn sha_index_key(ws: &str, ent: &str) -> String {
    format!("corp:sha:{ws}:{ent}")
}

/// Branch refs: `HSET corp:ref:{ws}:{ent} {branch} {sha1_hex}`
pub fn ref_key(ws: &str, ent: &str) -> String {
    format!("corp:ref:{ws}:{ent}")
}

/// Current tree state per branch: `HSET corp:tree:{ws}:{ent}:{branch} {path} {blob_sha1_hex}`
pub fn tree_key(ws: &str, ent: &str, branch: &str) -> String {
    format!("corp:tree:{ws}:{ent}:{branch}")
}

/// File history index: `ZADD corp:file:{ws}:{ent}:{path} {seq} {sha1_hex}`
pub fn file_history_key(ws: &str, ent: &str, path: &str) -> String {
    format!("corp:file:{ws}:{ent}:{path}")
}

/// Actor index (cross-entity): `ZADD corp:actor:{actor_ws} {seq} {ws}:{ent}:{sha1_hex}`
pub fn actor_key(actor_ws: &str) -> String {
    format!("corp:actor:{actor_ws}")
}

/// List of all entities in a workspace: `SADD corp:entities:{ws} {ent}`
pub fn entities_key(ws: &str) -> String {
    format!("corp:entities:{ws}")
}

/// Set of all workspace IDs: `SADD corp:workspaces {ws}`
pub fn workspaces_key() -> &'static str {
    "corp:workspaces"
}

/// Git object type by SHA-1: `HSET corp:git-obj-type {sha1_hex} "blob"|"tree"|"commit"`
pub fn git_obj_type_key() -> &'static str {
    "corp:git-obj-type"
}
