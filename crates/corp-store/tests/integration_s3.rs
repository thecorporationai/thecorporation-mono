//! Integration tests for the S3 durable backend.
//!
//! Requirements:
//! - DragonflyDB (or any Redis-compatible) — default `redis://127.0.0.1:16379/`
//! - RustFS (or any S3-compatible) — default `http://localhost:19000`
//!
//! Override via env vars: `REDIS_TEST_URL`, `S3_TEST_ENDPOINT`,
//! `S3_TEST_ACCESS_KEY`, `S3_TEST_SECRET_KEY`, `S3_TEST_BUCKET`.
//!
//! Run via:
//! ```sh
//! cargo test --test integration_s3 --features s3 -- --include-ignored
//! ```

#[cfg(feature = "s3")]
mod s3_tests {
    use chrono::Utc;
    use corp_store::durable::DurableBackend;
    use corp_store::entry::FileWrite;
    use corp_store::s3_backend::S3Backend;
    use corp_store::CorpStore;

    /// Build an S3Backend pointed at the S3-compatible instance.
    ///
    /// Uses env vars for configuration (defaults match docker-compose):
    /// - `S3_TEST_ENDPOINT` (default `http://localhost:19000`)
    /// - `S3_TEST_ACCESS_KEY` (default `minioadmin`)
    /// - `S3_TEST_SECRET_KEY` (default `minioadmin`)
    /// - `S3_TEST_BUCKET` (default `corp-store-test`)
    fn s3_backend() -> S3Backend {
        let endpoint = std::env::var("S3_TEST_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:19000".into());
        let access_key = std::env::var("S3_TEST_ACCESS_KEY")
            .unwrap_or_else(|_| "minioadmin".into());
        let secret_key = std::env::var("S3_TEST_SECRET_KEY")
            .unwrap_or_else(|_| "minioadmin".into());
        let bucket = std::env::var("S3_TEST_BUCKET")
            .unwrap_or_else(|_| "corp-store-test".into());

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");

        let client = rt.block_on(async {
            let creds = aws_sdk_s3::config::Credentials::new(
                &access_key,
                &secret_key,
                None,
                None,
                "test",
            );
            let config = aws_sdk_s3::Config::builder()
                .endpoint_url(&endpoint)
                .region(aws_sdk_s3::config::Region::new("us-east-1"))
                .credentials_provider(creds)
                .force_path_style(true)
                .behavior_version_latest()
                .build();
            aws_sdk_s3::Client::from_conf(config)
        });

        S3Backend::new(client, bucket, String::new())
    }

    /// Open a Redis connection to DragonflyDB.
    ///
    /// Uses `REDIS_TEST_URL` env var (default `redis://127.0.0.1:16379/`).
    fn dragonfly_connection() -> redis::Connection {
        let url = std::env::var("REDIS_TEST_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:16379/".into());
        let client = redis::Client::open(url.as_str())
            .expect("dragonfly client");
        client
            .get_connection()
            .expect("dragonfly connection")
    }

    /// Generate a unique workspace ID to avoid key collisions between tests.
    fn unique_ws() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("itest_{nanos:x}")
    }

    /// Flush all KV keys for an entity (simulates KV data loss).
    fn flush_entity_keys(con: &mut redis::Connection, ws: &str, ent: &str) {
        let keys_to_del = vec![
            format!("corp:log:{ws}:{ent}"),
            format!("corp:seq:{ws}:{ent}"),
            format!("corp:sha:{ws}:{ent}"),
            format!("corp:ref:{ws}:{ent}"),
            format!("corp:tree:{ws}:{ent}:main"),
        ];
        for key in &keys_to_del {
            let _: Result<(), _> = redis::cmd("DEL").arg(key).query(con);
        }
    }

    // ── Tests ────────────────────────────────────────────────────────

    #[test]
    #[ignore]
    fn commit_and_read_via_durable_backend() {
        let ws = unique_ws();
        let ent = "ent_s3_read";
        let mut con = dragonfly_connection();
        let backend = s3_backend();

        let mut store = CorpStore::with_durable(
            &mut con,
            &ws,
            ent,
            std::sync::Arc::new(backend),
        );

        // Commit two files through the durable path.
        let files = vec![
            FileWrite::new("alpha.txt", b"hello alpha".to_vec()),
            FileWrite::new("beta.json", br#"{"v":1}"#.to_vec()),
        ];
        let oid = store
            .commit_files("main", "initial commit", &files, None, Utc::now())
            .expect("durable commit");

        assert!(!oid.sha1_hex().is_empty(), "commit OID must be non-empty");

        // Verify readable via KV (the normal read path).
        let alpha = store.read_blob("main", "alpha.txt").expect("read alpha");
        assert_eq!(alpha, b"hello alpha");

        let beta = store.read_blob("main", "beta.json").expect("read beta");
        assert_eq!(beta, br#"{"v":1}"#);

        // Verify data actually landed in S3 by reading the blob directly
        // from S3 via the backend trait.
        let alpha_oid = corp_store::oid::hash_blob(b"hello alpha");
        let s3_backend = s3_backend();
        let s3_bytes = s3_backend
            .get_blob(&alpha_oid.sha256_hex())
            .expect("blob must exist in S3");
        assert_eq!(s3_bytes, b"hello alpha");
    }

    #[test]
    #[ignore]
    fn rebuild_from_s3_restores_full_state() {
        let ws = unique_ws();
        let ent = "ent_s3_rebuild";
        let mut con = dragonfly_connection();
        let backend = s3_backend();

        // Phase 1: commit files via durable store.
        {
            let mut store = CorpStore::with_durable(
                &mut con,
                &ws,
                ent,
                std::sync::Arc::new(backend),
            );

            let files = vec![
                FileWrite::new("doc.txt", b"document content".to_vec()),
                FileWrite::new("config.json", br#"{"key":"val"}"#.to_vec()),
            ];
            store
                .commit_files("main", "seed data", &files, None, Utc::now())
                .expect("durable commit");

            // Sanity check: data is readable before flush.
            let doc = store.read_blob("main", "doc.txt").expect("pre-flush read");
            assert_eq!(doc, b"document content");
        }

        // Phase 2: nuke all KV keys for this entity.
        flush_entity_keys(&mut con, &ws, ent);

        // Verify KV reads fail after flush.
        {
            let mut store = CorpStore::new(&mut con, &ws, ent);
            assert!(
                store.read_blob("main", "doc.txt").is_err(),
                "KV read must fail after flush"
            );
        }

        // Phase 3: rebuild from S3.
        let backend2 = s3_backend();
        {
            let mut store = CorpStore::with_durable(
                &mut con,
                &ws,
                ent,
                std::sync::Arc::new(backend2),
            );
            let rebuilt = store.rebuild_from_durable().expect("rebuild");
            assert_eq!(rebuilt, 1, "exactly one commit should be replayed");
        }

        // Phase 4: verify restored state.
        let backend3 = s3_backend();
        {
            let mut store = CorpStore::with_durable(
                &mut con,
                &ws,
                ent,
                std::sync::Arc::new(backend3),
            );

            // Blob reads should work again (via S3 fallback + rebuilt tree index).
            let doc = store.read_blob("main", "doc.txt").expect("post-rebuild read doc");
            assert_eq!(doc, b"document content");

            let cfg = store
                .read_blob("main", "config.json")
                .expect("post-rebuild read config");
            assert_eq!(cfg, br#"{"key":"val"}"#);

            // Commit log must be restored.
            let commits = store.all_commits().expect("all_commits");
            assert_eq!(commits.len(), 1);
            assert_eq!(commits[0].message, "seed data");

            // Branch field must survive the round-trip.
            assert_eq!(
                commits[0].branch.as_deref(),
                Some("main"),
                "branch field must be preserved"
            );
        }
    }

    #[test]
    #[ignore]
    fn multi_file_tree_objects_in_s3() {
        let ws = unique_ws();
        let ent = "ent_s3_trees";
        let mut con = dragonfly_connection();
        let backend = s3_backend();

        let mut store = CorpStore::with_durable(
            &mut con,
            &ws,
            ent,
            std::sync::Arc::new(backend),
        );

        // Commit nested files to force tree object creation.
        let files = vec![
            FileWrite::new("root.txt", b"root file".to_vec()),
            FileWrite::new("dir/child.txt", b"child file".to_vec()),
            FileWrite::new("dir/sub/deep.txt", b"deep file".to_vec()),
        ];
        store
            .commit_files("main", "nested commit", &files, None, Utc::now())
            .expect("durable commit with nesting");

        // Compute what the tree objects should be so we can verify they
        // exist in S3. The durable commit stores tree objects alongside
        // blobs using the same put_blob path.
        let s3 = s3_backend();

        // Verify the blob objects are in S3.
        let root_oid = corp_store::oid::hash_blob(b"root file");
        assert!(
            s3.blob_exists(&root_oid.sha256_hex()).expect("head root blob"),
            "root.txt blob must exist in S3"
        );

        let child_oid = corp_store::oid::hash_blob(b"child file");
        assert!(
            s3.blob_exists(&child_oid.sha256_hex()).expect("head child blob"),
            "dir/child.txt blob must exist in S3"
        );

        let deep_oid = corp_store::oid::hash_blob(b"deep file");
        assert!(
            s3.blob_exists(&deep_oid.sha256_hex()).expect("head deep blob"),
            "dir/sub/deep.txt blob must exist in S3"
        );

        // Compute the expected tree objects.
        use std::collections::BTreeMap;
        let mut file_map = BTreeMap::new();
        file_map.insert("root.txt".to_owned(), root_oid);
        file_map.insert("dir/child.txt".to_owned(), child_oid);
        file_map.insert("dir/sub/deep.txt".to_owned(), deep_oid);

        let (_root_tree, all_trees) = corp_store::oid::compute_root_tree(&file_map);

        // There should be 3 tree objects: dir/sub/, dir/, root.
        assert_eq!(all_trees.len(), 3, "expected 3 tree objects (dir/sub, dir, root)");

        // Every tree object must exist in S3.
        for (tree_oid, _raw) in &all_trees {
            let sha = tree_oid.sha256_hex();
            assert!(
                s3.blob_exists(&sha).expect("head tree object"),
                "tree object {sha} must exist in S3"
            );
        }
    }
}
