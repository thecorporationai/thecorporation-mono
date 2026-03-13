//! Integration tests — require a running Redis/Valkey at localhost:6379.
//!
//! Each test uses a unique namespace prefix to avoid collisions.
//! Tests are skipped if Redis is unavailable.

use chrono::Utc;
use corp_store::{branch, entry::*, merge, store};

fn get_connection() -> Option<redis::Connection> {
    let client = redis::Client::open("redis://127.0.0.1/").ok()?;
    client.get_connection().ok()
}

fn unique_ns() -> (String, String) {
    let id = uuid_v4_hex();
    (format!("test_ws_{id}"), format!("test_ent_{id}"))
}

fn uuid_v4_hex() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{nanos:x}")
}

fn cleanup(con: &mut redis::Connection, ws: &str, ent: &str) {
    // Best-effort cleanup of test keys.
    let patterns = [
        format!("corp:log:{ws}:{ent}"),
        format!("corp:seq:{ws}:{ent}"),
        format!("corp:sha:{ws}:{ent}"),
        format!("corp:ref:{ws}:{ent}"),
        format!("corp:tree:{ws}:{ent}:*"),
    ];
    for pat in &patterns {
        let _: Result<(), _> = redis::cmd("DEL").arg(pat).query(con);
    }
}

#[test]
fn commit_and_read_back() {
    let Some(mut con) = get_connection() else {
        eprintln!("SKIP: no redis");
        return;
    };
    let (ws, ent) = unique_ns();

    // Initialize with an empty commit so refs exist.
    let files = vec![FileWrite::new("corp.json", b"{}".to_vec())];
    let oid = store::commit_files(&mut con, &ws, &ent, "main", "init", &files, None, Utc::now())
        .unwrap();

    assert!(!oid.sha1_hex().is_empty());

    // Read back.
    let content = store::read_blob(&mut con, &ws, &ent, "main", "corp.json").unwrap();
    assert_eq!(content, b"{}");

    // List dir.
    let entries = store::list_dir(&mut con, &ws, &ent, "main", "").unwrap();
    assert_eq!(entries, vec![("corp.json".to_owned(), false)]);

    cleanup(&mut con, &ws, &ent);
}

#[test]
fn branch_create_and_list() {
    let Some(mut con) = get_connection() else {
        eprintln!("SKIP: no redis");
        return;
    };
    let (ws, ent) = unique_ns();

    // Init main.
    let files = vec![FileWrite::new("a.txt", b"hello".to_vec())];
    store::commit_files(&mut con, &ws, &ent, "main", "init", &files, None, Utc::now()).unwrap();

    // Create branch.
    let info = branch::create_branch(&mut con, &ws, &ent, "feature", "main").unwrap();
    assert_eq!(info.name, "feature");

    // List branches.
    let branches = branch::list_branches(&mut con, &ws, &ent).unwrap();
    let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"main"));
    assert!(names.contains(&"feature"));

    // Feature branch should have same file.
    let content = store::read_blob(&mut con, &ws, &ent, "feature", "a.txt").unwrap();
    assert_eq!(content, b"hello");

    // Commit to feature — should not affect main.
    let files = vec![FileWrite::new("b.txt", b"world".to_vec())];
    store::commit_files(&mut con, &ws, &ent, "feature", "add b", &files, None, Utc::now()).unwrap();
    assert!(store::path_exists(&mut con, &ws, &ent, "feature", "b.txt").unwrap());
    assert!(!store::path_exists(&mut con, &ws, &ent, "main", "b.txt").unwrap());

    // Delete branch.
    branch::delete_branch(&mut con, &ws, &ent, "feature").unwrap();
    let branches = branch::list_branches(&mut con, &ws, &ent).unwrap();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].name, "main");

    // Cannot delete main.
    assert!(branch::delete_branch(&mut con, &ws, &ent, "main").is_err());

    cleanup(&mut con, &ws, &ent);
}

#[test]
fn fast_forward_merge() {
    let Some(mut con) = get_connection() else {
        eprintln!("SKIP: no redis");
        return;
    };
    let (ws, ent) = unique_ns();

    // Init main.
    let files = vec![FileWrite::new("a.txt", b"hello".to_vec())];
    store::commit_files(&mut con, &ws, &ent, "main", "init", &files, None, Utc::now()).unwrap();

    // Branch and commit.
    branch::create_branch(&mut con, &ws, &ent, "feature", "main").unwrap();
    let files = vec![FileWrite::new("b.txt", b"new".to_vec())];
    store::commit_files(&mut con, &ws, &ent, "feature", "add b", &files, None, Utc::now()).unwrap();

    // Merge feature into main — should fast-forward.
    let result = merge::merge_branch(&mut con, &ws, &ent, "feature", "main", None).unwrap();
    assert!(matches!(result, merge::MergeResult::FastForward { .. }));

    // Main should now have b.txt.
    assert!(store::path_exists(&mut con, &ws, &ent, "main", "b.txt").unwrap());

    cleanup(&mut con, &ws, &ent);
}

#[test]
fn three_way_merge_json_conflict_resolution() {
    let Some(mut con) = get_connection() else {
        eprintln!("SKIP: no redis");
        return;
    };
    let (ws, ent) = unique_ns();

    // Base state.
    let base = serde_json::to_vec_pretty(&serde_json::json!({
        "legal_name": "Acme Inc",
        "jurisdiction": "Delaware",
        "status": "active"
    }))
    .unwrap();
    store::commit_files(
        &mut con, &ws, &ent, "main", "base",
        &[FileWrite::new("corp.json", base)], None, Utc::now(),
    ).unwrap();

    // Create branch.
    branch::create_branch(&mut con, &ws, &ent, "feature", "main").unwrap();

    // Main changes legal_name.
    let main_update = serde_json::to_vec_pretty(&serde_json::json!({
        "legal_name": "Acme Corp",
        "jurisdiction": "Delaware",
        "status": "active"
    }))
    .unwrap();
    store::commit_files(
        &mut con, &ws, &ent, "main", "update name",
        &[FileWrite::new("corp.json", main_update)], None, Utc::now(),
    ).unwrap();

    // Feature changes jurisdiction.
    let feature_update = serde_json::to_vec_pretty(&serde_json::json!({
        "legal_name": "Acme Inc",
        "jurisdiction": "California",
        "status": "active"
    }))
    .unwrap();
    store::commit_files(
        &mut con, &ws, &ent, "feature", "update jurisdiction",
        &[FileWrite::new("corp.json", feature_update)], None, Utc::now(),
    ).unwrap();

    // Merge.
    let result = merge::merge_branch(&mut con, &ws, &ent, "feature", "main", None).unwrap();
    assert!(matches!(result, merge::MergeResult::ThreeWayMerge { .. }));

    // Read merged result.
    let merged: serde_json::Value =
        store::read_json(&mut con, &ws, &ent, "main", "corp.json").unwrap();
    assert_eq!(merged["legal_name"], "Acme Corp"); // from main
    assert_eq!(merged["jurisdiction"], "California"); // from feature
    assert_eq!(merged["status"], "active"); // unchanged

    cleanup(&mut con, &ws, &ent);
}

#[test]
fn squash_merge() {
    let Some(mut con) = get_connection() else {
        eprintln!("SKIP: no redis");
        return;
    };
    let (ws, ent) = unique_ns();

    // Init main.
    let files = vec![FileWrite::new("a.txt", b"a".to_vec())];
    store::commit_files(&mut con, &ws, &ent, "main", "init", &files, None, Utc::now()).unwrap();

    // Branch and make multiple commits.
    branch::create_branch(&mut con, &ws, &ent, "feature", "main").unwrap();
    store::commit_files(
        &mut con, &ws, &ent, "feature", "add b",
        &[FileWrite::new("b.txt", b"b".to_vec())], None, Utc::now(),
    ).unwrap();
    store::commit_files(
        &mut con, &ws, &ent, "feature", "add c",
        &[FileWrite::new("c.txt", b"c".to_vec())], None, Utc::now(),
    ).unwrap();

    // Squash merge.
    let result = merge::merge_branch_squash(&mut con, &ws, &ent, "feature", "main", None).unwrap();
    assert!(matches!(result, merge::MergeResult::Squash { .. }));

    // Main should have all files.
    assert!(store::path_exists(&mut con, &ws, &ent, "main", "b.txt").unwrap());
    assert!(store::path_exists(&mut con, &ws, &ent, "main", "c.txt").unwrap());

    cleanup(&mut con, &ws, &ent);
}

#[test]
fn file_history_across_branches() {
    let Some(mut con) = get_connection() else {
        eprintln!("SKIP: no redis");
        return;
    };
    let (ws, ent) = unique_ns();

    // Create and modify a file across commits.
    store::commit_files(
        &mut con, &ws, &ent, "main", "v1",
        &[FileWrite::new("doc.json", b"{\"v\": 1}".to_vec())], None, Utc::now(),
    ).unwrap();
    store::commit_files(
        &mut con, &ws, &ent, "main", "v2",
        &[FileWrite::new("doc.json", b"{\"v\": 2}".to_vec())], None, Utc::now(),
    ).unwrap();
    store::commit_files(
        &mut con, &ws, &ent, "main", "unrelated",
        &[FileWrite::new("other.txt", b"x".to_vec())], None, Utc::now(),
    ).unwrap();
    store::commit_files(
        &mut con, &ws, &ent, "main", "v3",
        &[FileWrite::new("doc.json", b"{\"v\": 3}".to_vec())], None, Utc::now(),
    ).unwrap();

    let history = store::file_history(&mut con, &ws, &ent, "doc.json").unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].message, "v1");
    assert_eq!(history[1].message, "v2");
    assert_eq!(history[2].message, "v3");

    cleanup(&mut con, &ws, &ent);
}
