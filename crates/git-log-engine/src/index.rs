//! SHA-to-offset index for O(1) entry lookup.
//!
//! Maintained in-memory, rebuilt from the log file on open.
//! Maps commit SHA hex -> (byte offset in log file, sequence number).

use std::collections::HashMap;

/// In-memory index from commit SHA to log file position.
#[derive(Debug, Default)]
pub struct ShaIndex {
    /// SHA hex -> (byte offset of the NDJSON line, sequence number).
    entries: HashMap<String, IndexEntry>,
    /// Highest sequence number seen.
    max_sequence: u64,
    /// SHA of the most recent commit in the log.
    tip: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct IndexEntry {
    pub offset: u64,
    pub sequence: u64,
}

impl ShaIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a commit in the index.
    pub fn insert(&mut self, sha: String, offset: u64, sequence: u64) {
        if sequence >= self.max_sequence {
            self.max_sequence = sequence;
            self.tip = Some(sha.clone());
        }
        self.entries.insert(sha, IndexEntry { offset, sequence });
    }

    /// Look up a commit by SHA.
    pub fn get(&self, sha: &str) -> Option<&IndexEntry> {
        self.entries.get(sha)
    }

    /// The highest sequence number in the log.
    pub fn max_sequence(&self) -> u64 {
        self.max_sequence
    }

    /// SHA of the most recent commit, if any.
    pub fn tip(&self) -> Option<&str> {
        self.tip.as_deref()
    }

    /// Number of indexed entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Find the checkpoint with the highest sequence <= target_sequence.
    /// Checkpoints are stored with a `ckpt:` prefix key.
    pub fn latest_checkpoint_before(&self, target_sequence: u64) -> Option<(&str, &IndexEntry)> {
        self.entries
            .iter()
            .filter(|(k, e)| k.starts_with("ckpt:") && e.sequence <= target_sequence)
            .max_by_key(|(_, e)| e.sequence)
            .map(|(k, e)| (k.as_str(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_lookup() {
        let mut idx = ShaIndex::new();
        idx.insert("abc123".into(), 100, 1);
        idx.insert("def456".into(), 250, 2);

        assert_eq!(idx.get("abc123").unwrap().offset, 100);
        assert_eq!(idx.get("def456").unwrap().sequence, 2);
        assert!(idx.get("missing").is_none());
    }

    #[test]
    fn tracks_tip() {
        let mut idx = ShaIndex::new();
        idx.insert("first".into(), 0, 1);
        idx.insert("second".into(), 100, 2);
        assert_eq!(idx.tip(), Some("second"));
        assert_eq!(idx.max_sequence(), 2);
    }
}
