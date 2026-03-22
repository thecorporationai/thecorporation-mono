//! S3-backed durable storage implementation.
//!
//! Implements `DurableBackend` using AWS S3 (or S3-compatible services
//! like MinIO, R2, etc.).
//!
//! # Layout
//!
//! ```text
//! s3://{bucket}/blobs/{sha256_hex}
//! s3://{bucket}/commits/{ws}/{ent}/{seq:010}.json
//! s3://{bucket}/refs/{ws}/{ent}/{branch}.json
//! ```
//!
//! # Configuration
//!
//! - `S3_BUCKET`  — bucket name (required)
//! - `S3_PREFIX`  — optional key prefix (e.g. `corp/` → `corp/blobs/...`)
//! - `AWS_REGION` — AWS region (or `auto` for S3-compatible endpoints)
//! - Standard AWS credential chain (env, profile, IMDS, etc.)

use crate::durable::DurableBackend;
use crate::error::StoreError;

use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;

/// S3 durable backend.
///
/// All operations are synchronous (blocking the calling thread) because
/// `DurableBackend` is called from `spawn_blocking` contexts. Internally
/// uses a single-threaded Tokio runtime to drive the async S3 SDK.
pub struct S3Backend {
    client: Client,
    bucket: String,
    prefix: String,
    rt: tokio::runtime::Runtime,
}

impl S3Backend {
    /// Create a new S3 backend from environment configuration.
    ///
    /// Reads `S3_BUCKET` (required), `S3_PREFIX` (optional, default ""),
    /// and standard AWS SDK config (`AWS_REGION`, credentials, etc.).
    ///
    /// **Note**: This creates an internal tokio runtime and must NOT be
    /// called from within an existing tokio runtime. Use `from_env_async`
    /// instead when calling from async code.
    pub fn from_env() -> Result<Self, StoreError> {
        let bucket = std::env::var("S3_BUCKET")
            .map_err(|_| StoreError::Config("S3_BUCKET env var is required".into()))?;
        let prefix = std::env::var("S3_PREFIX").unwrap_or_default();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| StoreError::Internal(format!("tokio runtime: {e}")))?;

        let client = rt.block_on(async {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            Client::new(&config)
        });

        tracing::info!(bucket = %bucket, prefix = %prefix, "S3 durable backend initialized");

        Ok(Self {
            client,
            bucket,
            prefix,
            rt,
        })
    }

    /// Async version of `from_env` — safe to call from within a tokio runtime.
    ///
    /// Loads AWS config using the caller's async runtime, then creates a
    /// dedicated single-threaded runtime for the sync `DurableBackend` methods.
    pub async fn from_env_async() -> Result<Self, StoreError> {
        let bucket = std::env::var("S3_BUCKET")
            .map_err(|_| StoreError::Config("S3_BUCKET env var is required".into()))?;
        let prefix = std::env::var("S3_PREFIX").unwrap_or_default();

        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| StoreError::Internal(format!("tokio runtime: {e}")))?;

        tracing::info!(bucket = %bucket, prefix = %prefix, "S3 durable backend initialized");

        Ok(Self {
            client,
            bucket,
            prefix,
            rt,
        })
    }

    /// Create from explicit config (for testing with custom endpoints).
    pub fn new(client: Client, bucket: String, prefix: String) -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        Self { client, bucket, prefix, rt }
    }

    fn key(&self, suffix: &str) -> String {
        if self.prefix.is_empty() {
            suffix.to_owned()
        } else {
            format!("{}/{}", self.prefix.trim_end_matches('/'), suffix)
        }
    }

    fn put_object(&self, key: &str, body: &[u8]) -> Result<(), StoreError> {
        self.rt.block_on(async {
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .body(ByteStream::from(body.to_vec()))
                .send()
                .await
                .map_err(|e| StoreError::Internal(format!("S3 PUT {key}: {e}")))?;
            Ok(())
        })
    }

    fn get_object(&self, key: &str) -> Result<Vec<u8>, StoreError> {
        self.rt.block_on(async {
            let resp = self.client
                .get_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| StoreError::NotFound(format!("S3 GET {key}: {e}")))?;
            let bytes = resp.body.collect().await
                .map_err(|e| StoreError::Internal(format!("S3 read body {key}: {e}")))?;
            Ok(bytes.into_bytes().to_vec())
        })
    }

    fn head_object(&self, key: &str) -> Result<bool, StoreError> {
        self.rt.block_on(async {
            match self.client
                .head_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
            {
                Ok(_) => Ok(true),
                Err(e) => {
                    let svc_err = e.into_service_error();
                    if svc_err.is_not_found() {
                        Ok(false)
                    } else {
                        Err(StoreError::Internal(format!("S3 HEAD {key}: {svc_err}")))
                    }
                }
            }
        })
    }

    fn list_objects(&self, prefix: &str) -> Result<Vec<String>, StoreError> {
        self.rt.block_on(async {
            let mut keys = Vec::new();
            let mut continuation_token: Option<String> = None;

            loop {
                let mut req = self.client
                    .list_objects_v2()
                    .bucket(&self.bucket)
                    .prefix(prefix);
                if let Some(token) = continuation_token.take() {
                    req = req.continuation_token(token);
                }
                let resp = req.send().await
                    .map_err(|e| StoreError::Internal(format!("S3 LIST {prefix}: {e}")))?;

                if let Some(contents) = resp.contents {
                    for obj in contents {
                        if let Some(key) = obj.key {
                            keys.push(key);
                        }
                    }
                }

                if resp.is_truncated.unwrap_or(false) {
                    continuation_token = resp.next_continuation_token;
                } else {
                    break;
                }
            }

            Ok(keys)
        })
    }
}

impl DurableBackend for S3Backend {
    fn put_blob(&self, sha256_hex: &str, content: &[u8]) -> Result<(), StoreError> {
        let key = self.key(&format!("blobs/{sha256_hex}"));
        self.put_object(&key, content)
    }

    fn blob_exists(&self, sha256_hex: &str) -> Result<bool, StoreError> {
        let key = self.key(&format!("blobs/{sha256_hex}"));
        self.head_object(&key)
    }

    fn get_blob(&self, sha256_hex: &str) -> Result<Vec<u8>, StoreError> {
        let key = self.key(&format!("blobs/{sha256_hex}"));
        self.get_object(&key)
    }

    fn put_commit(
        &self,
        ws: &str,
        ent: &str,
        seq: u64,
        entry_json: &[u8],
    ) -> Result<(), StoreError> {
        let key = self.key(&format!("commits/{ws}/{ent}/{seq:010}.json"));
        self.put_object(&key, entry_json)
    }

    fn put_ref(
        &self,
        ws: &str,
        ent: &str,
        branch: &str,
        ref_json: &[u8],
    ) -> Result<(), StoreError> {
        let key = self.key(&format!("refs/{ws}/{ent}/{branch}.json"));
        self.put_object(&key, ref_json)
    }

    fn list_commits(&self, ws: &str, ent: &str) -> Result<Vec<Vec<u8>>, StoreError> {
        let prefix = self.key(&format!("commits/{ws}/{ent}/"));
        let keys = self.list_objects(&prefix)?;
        let mut commits = Vec::new();
        for key in keys {
            if key.ends_with(".json") {
                commits.push(self.get_object(&key)?);
            }
        }
        Ok(commits)
    }

    fn list_blobs(&self) -> Result<Vec<String>, StoreError> {
        let prefix = self.key("blobs/");
        let keys = self.list_objects(&prefix)?;
        Ok(keys
            .into_iter()
            .filter_map(|k| k.rsplit('/').next().map(|s| s.to_owned()))
            .collect())
    }
}
