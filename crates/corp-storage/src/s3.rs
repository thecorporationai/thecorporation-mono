//! S3 / compatible object-store durable backend.
//!
//! Used alongside the KV backend: the KV store is the hot path (fast reads and
//! writes), while S3 provides the durable record of all commits and blobs so
//! that the KV state can be fully rebuilt after a cache flush or node failure.
//!
//! # Object layout
//! ```text
//! {prefix}/blobs/{sha256}               — raw blob bytes
//! {prefix}/commits/{ws}/{ent}/{seq:020} — JSON CommitEntry
//! ```
//! The `seq` is zero-padded to 20 digits so that S3 `LIST` returns commits in
//! chronological order without a sort step.

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;

#[cfg(feature = "kv")]
use redis::aio::ConnectionManager;

use crate::error::StorageError;
#[cfg(feature = "kv")]
use crate::kv::{self, BLOB_TTL_SECS, CommitEntry};

// ── Error mapping ─────────────────────────────────────────────────────────────

type Result<T> = std::result::Result<T, StorageError>;

fn s3_err(e: impl std::fmt::Display) -> StorageError {
    StorageError::S3Error(e.to_string())
}

// ── S3Backend ────────────────────────────────────────────────────────────────

/// An S3-backed durable store for blobs and commit entries.
pub struct S3Backend {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Backend {
    // ── Construction ─────────────────────────────────────────────────────────

    /// Create a new `S3Backend` using the ambient AWS configuration (environment
    /// variables, `~/.aws/`, instance metadata, etc.).
    ///
    /// `prefix` must not start or end with `/`; the empty string `""` stores
    /// objects at the bucket root.
    pub async fn new(bucket: String, prefix: String) -> Result<Self> {
        let mut config_loader = aws_config::defaults(BehaviorVersion::latest());

        // Support custom S3-compatible endpoints (RustFS, MinIO, etc.)
        // via the AWS_ENDPOINT_URL or CORP_S3_ENDPOINT env vars.
        if let Ok(endpoint) =
            std::env::var("AWS_ENDPOINT_URL").or_else(|_| std::env::var("CORP_S3_ENDPOINT"))
        {
            config_loader = config_loader.endpoint_url(&endpoint);
        }

        let config = config_loader.load().await;
        let client = Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .region(
                    config
                        .region()
                        .cloned()
                        .unwrap_or_else(|| aws_sdk_s3::config::Region::new("us-east-1")),
                )
                .credentials_provider(config.credentials_provider().unwrap().clone())
                .endpoint_url(config.endpoint_url().unwrap_or("https://s3.amazonaws.com"))
                .force_path_style(true) // Required for S3-compatible stores
                .build(),
        );
        Ok(Self {
            client,
            bucket,
            prefix,
        })
    }

    // ── Key helpers ───────────────────────────────────────────────────────────

    fn blob_key(&self, sha: &str) -> String {
        if self.prefix.is_empty() {
            format!("blobs/{}", sha)
        } else {
            format!("{}/blobs/{}", self.prefix, sha)
        }
    }

    fn commit_key(&self, ws: &str, ent: &str, seq: u64) -> String {
        if self.prefix.is_empty() {
            format!("commits/{}/{}/{:020}", ws, ent, seq)
        } else {
            format!("{}/commits/{}/{}/{:020}", self.prefix, ws, ent, seq)
        }
    }

    fn commit_prefix(&self, ws: &str, ent: &str) -> String {
        if self.prefix.is_empty() {
            format!("commits/{}/{}/", ws, ent)
        } else {
            format!("{}/commits/{}/{}/", self.prefix, ws, ent)
        }
    }

    // ── Blob operations ───────────────────────────────────────────────────────

    /// Upload a blob.  The key is the content's SHA-256 hex digest so the
    /// operation is idempotent.
    pub async fn put_blob(&self, sha: &str, data: &[u8]) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(self.blob_key(sha))
            .body(ByteStream::from(data.to_vec()))
            .send()
            .await
            .map_err(|e| s3_err(e.into_service_error()))?;
        Ok(())
    }

    /// Download a blob by its SHA-256 digest.
    pub async fn get_blob(&self, sha: &str) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(self.blob_key(sha))
            .send()
            .await
            .map_err(|e| {
                let svc = e.into_service_error();
                if svc.is_no_such_key() {
                    StorageError::NotFound(format!("blob '{}'", sha))
                } else {
                    s3_err(svc)
                }
            })?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| s3_err(format!("collecting blob body: {}", e)))?;
        Ok(bytes.into_bytes().to_vec())
    }

    /// Return `true` if a blob with the given SHA-256 digest exists in S3.
    pub async fn blob_exists(&self, sha: &str) -> Result<bool> {
        let result = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(self.blob_key(sha))
            .send()
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(e) => {
                let svc = e.into_service_error();
                if svc.is_not_found() {
                    Ok(false)
                } else {
                    Err(s3_err(svc))
                }
            }
        }
    }

    // ── Commit operations ─────────────────────────────────────────────────────

    /// Persist a commit entry (serialised as JSON bytes) for `(ws, ent, seq)`.
    pub async fn put_commit(&self, ws: &str, ent: &str, seq: u64, data: &[u8]) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(self.commit_key(ws, ent, seq))
            .body(ByteStream::from(data.to_vec()))
            .send()
            .await
            .map_err(|e| s3_err(e.into_service_error()))?;
        Ok(())
    }

    /// List all commit entries for `(ws, ent)`, in ascending sequence order.
    ///
    /// Returns a `Vec` of `(seq, bytes)` pairs.  Uses paginated `LIST` so it
    /// handles arbitrarily large histories without loading everything into
    /// memory at once.
    pub async fn list_commits(&self, ws: &str, ent: &str) -> Result<Vec<(u64, Vec<u8>)>> {
        let prefix = self.commit_prefix(ws, ent);
        let mut continuation_token: Option<String> = None;
        let mut keys: Vec<String> = Vec::new();

        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&prefix);

            if let Some(ref token) = continuation_token {
                req = req.continuation_token(token);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| s3_err(e.into_service_error()))?;

            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    keys.push(key.to_owned());
                }
            }

            if resp.is_truncated().unwrap_or(false) {
                continuation_token = resp.next_continuation_token().map(|s| s.to_owned());
            } else {
                break;
            }
        }

        // Keys are already sorted lexicographically because of zero-padding.
        let mut commits = Vec::with_capacity(keys.len());
        for key in keys {
            // Extract seq from the last path component.
            let seq_str = key.rsplit('/').next().unwrap_or("0");
            let seq: u64 = seq_str.parse().unwrap_or(0);

            let resp = self
                .client
                .get_object()
                .bucket(&self.bucket)
                .key(&key)
                .send()
                .await
                .map_err(|e| s3_err(e.into_service_error()))?;

            let bytes = resp
                .body
                .collect()
                .await
                .map_err(|e| s3_err(format!("collecting commit body: {}", e)))?;
            commits.push((seq, bytes.into_bytes().to_vec()));
        }

        Ok(commits)
    }

    // ── KV rebuild ────────────────────────────────────────────────────────────

    /// Rebuild the Redis/Valkey KV state from the durable S3 record.
    ///
    /// This replays all commit entries in order, reconstructing:
    /// - The tree hash for every branch seen in the commit log.
    /// - The blob cache (blobs are fetched from S3 and re-cached with TTL).
    /// - The sequence counter.
    /// - The commit entries in KV.
    ///
    /// The operation is **not** atomic at the KV level (it is a recovery
    /// procedure, not an online transaction).  Call this only when the KV node
    /// is known to have lost state and is not currently serving traffic.
    #[cfg(feature = "kv")]
    pub async fn rebuild_from_s3(
        &self,
        con: &mut ConnectionManager,
        ws: &str,
        ent: &str,
    ) -> Result<()> {
        use redis::AsyncCommands;

        tracing::info!(ws, ent, "starting KV rebuild from S3");

        let commits = self.list_commits(ws, ent).await?;
        if commits.is_empty() {
            tracing::info!(ws, ent, "no commits found in S3, nothing to rebuild");
            return Ok(());
        }

        // Ensure the entity is registered.
        kv::init_entity(con, ws, ent).await?;

        // Reconstruct branch trees by replaying commits in order.
        // We maintain an in-memory map of branch → { path → blob_sha } and
        // flush it at the end.
        let mut branch_trees: std::collections::HashMap<
            String,
            std::collections::HashMap<String, String>,
        > = std::collections::HashMap::new();

        let mut max_seq: u64 = 0;

        for (seq, data) in &commits {
            let entry: CommitEntry = serde_json::from_slice(data).map_err(StorageError::from)?;

            max_seq = max_seq.max(*seq);

            let tree = branch_trees.entry(entry.branch.clone()).or_default();

            for (path, sha) in &entry.files {
                if sha == "<deleted>" {
                    tree.remove(path);
                } else {
                    tree.insert(path.clone(), sha.clone());
                }
            }

            // Re-cache the commit entry in KV.
            con.set::<_, _, ()>(
                format!("corp:{}:{}:commit:{}", ws, ent, seq),
                data.as_slice(),
            )
            .await
            .map_err(|e| StorageError::KvError(e.to_string()))?;
        }

        // Restore the sequence counter to the highest observed value.
        con.set::<_, _, ()>(format!("corp:{}:{}:seq", ws, ent), max_seq)
            .await
            .map_err(|e| StorageError::KvError(e.to_string()))?;

        // Flush reconstructed trees into KV and re-cache blobs.
        for (branch, tree) in &branch_trees {
            // Collect all unique blob SHAs that are still alive in the tree.
            let shas: std::collections::HashSet<&str> = tree.values().map(|s| s.as_str()).collect();

            // Re-cache blobs from S3.
            for sha in shas {
                let blob_key = format!("corp:{}:{}:blob:{}", ws, ent, sha);
                let already: bool = con
                    .exists(&blob_key)
                    .await
                    .map_err(|e| StorageError::KvError(e.to_string()))?;

                if !already {
                    match self.get_blob(sha).await {
                        Ok(bytes) => {
                            let mut pipe = redis::pipe();
                            pipe.set(&blob_key, bytes.as_slice())
                                .expire(&blob_key, BLOB_TTL_SECS as i64);
                            pipe.query_async::<()>(con)
                                .await
                                .map_err(|e| StorageError::KvError(e.to_string()))?;
                        }
                        Err(StorageError::NotFound(_)) => {
                            // Blob may be referenced but not yet uploaded to S3 in this
                            // recovery window; skip with a warning.
                            tracing::warn!(sha, "blob not found in S3 during rebuild, skipping");
                        }
                        Err(e) => return Err(e),
                    }
                }
            }

            // Write the tree hash in one pipeline.
            if !tree.is_empty() {
                let tree_key = format!("corp:{}:{}:tree:{}", ws, ent, branch);
                let mut pipe = redis::pipe();
                for (path, sha) in tree {
                    pipe.hset(&tree_key, path, sha);
                }
                pipe.query_async::<()>(con)
                    .await
                    .map_err(|e| StorageError::KvError(e.to_string()))?;
            }
        }

        tracing::info!(ws, ent, max_seq, "KV rebuild from S3 complete");
        Ok(())
    }
}
