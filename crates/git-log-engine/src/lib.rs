//! Per-repo JSON-LD log engine for fast git history replay.
//!
//! Materializes git commit history into an append-only NDJSON-LD log
//! file alongside each bare git repository. The log is indexed in
//! memory for O(1) commit lookup and supports fast sequential replay
//! to reconstruct file tree state at any point in history.
//!
//! # JSON-LD format
//!
//! Each log file is a stream of newline-delimited JSON-LD objects:
//!
//! - **Line 0**: `@context` — vocabulary mapping for all terms
//! - **Lines 1..N**: `LogEntry` — either `Commit` or `Checkpoint`
//!
//! Every entry is a self-describing linked data node. The `@context`
//! maps short field names (like `sha`, `changes`, `timestamp`) to
//! full IRIs from the `git:`, `prov:`, and `xsd:` vocabularies.
//!
//! # Performance model
//!
//! | Operation | Without engine | With engine |
//! |---|---|---|
//! | Read commit metadata | O(revwalk) | O(1) seek |
//! | File history | O(n * diff) | O(n) scan |
//! | State at commit | O(revwalk + tree) | O(replay from checkpoint) |
//! | Audit by actor | O(n * parse) | O(n) scan, pre-parsed |
//!
//! Checkpoints are written every 100 commits, bounding replay cost.
//!
//! # Usage
//!
//! ```no_run
//! use git_log_engine::LogEngine;
//! use std::path::Path;
//!
//! let mut engine = LogEngine::open(Path::new("/data/ws_1/ent_1.git")).unwrap();
//!
//! // Materialize all new commits from the main branch.
//! engine.materialize("main").unwrap();
//!
//! // Replay to the latest state.
//! let tree = engine.replay_latest().unwrap();
//! for (path, blob_sha) in &tree {
//!     println!("{path} -> {blob_sha}");
//! }
//!
//! // Query file history.
//! let history = engine.history_of("governance/bylaws.json").unwrap();
//! ```

pub mod context;
pub mod engine;
pub mod entry;
pub mod error;
pub mod index;

pub use engine::LogEngine;
pub use entry::{
    ChangeAction, CheckpointEntry, CommitEntry, FileChange, LogEntry, PersonIdent, TreeFile,
};
pub use error::LogEngineError;
