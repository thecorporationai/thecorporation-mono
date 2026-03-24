//! Integration tests for the Git backend of `corp-storage`.
//!
//! Each test spins up one or more temporary bare git repositories using
//! `tempfile::tempdir()` so no external infrastructure is required.  All
//! async tests use `#[tokio::test]`.
//!
//! Coverage:
//! 1.  `EntityStore::init` and `EntityStore::open`
//! 2.  Write and read a typed domain object (`Entity`, `Contact`)
//! 3.  `list_ids` — enumerate stored IDs
//! 4.  `read_all` — load every object of a type
//! 5.  `delete` — remove an object
//! 6.  `path_exists` — existence check
//! 7.  `write_json` / `read_json` — raw JSON round-trip
//! 8.  Read non-existent path returns `StorageError::NotFound`
//! 9.  Multiple entities stored in the same workspace repo
//! 10. Multiple independent workspaces (distinct repos)
//! 11. `WorkspaceStore::init`, `open`, and API key CRUD
//! 12. Concurrent reads — multiple tasks reading the same store in parallel

use std::path::PathBuf;
use std::sync::Arc;

use tempfile::TempDir;

use corp_core::contacts::{Contact, ContactCategory, ContactType};
use corp_core::formation::entity::{Entity, EntityType, Jurisdiction};
use corp_core::ids::{ContactId, EntityId, WorkspaceId};

use corp_storage::entity_store::{Backend as EntityBackend, EntityStore};
use corp_storage::error::StorageError;
use corp_storage::workspace_store::{ApiKeyRecord, Backend as WsBackend, WorkspaceStore};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns a `(TempDir, EntityBackend)` pair.
///
/// The `TempDir` must be kept alive for the duration of the test; dropping it
/// removes the directory.
fn git_entity_backend(dir: &TempDir) -> EntityBackend {
    EntityBackend::Git {
        repo_path: Arc::new(PathBuf::from(dir.path())),
    }
}

fn git_ws_backend(dir: &TempDir) -> WsBackend {
    WsBackend::Git {
        repo_path: Arc::new(PathBuf::from(dir.path())),
    }
}

fn make_entity(ws_id: WorkspaceId) -> Entity {
    Entity::new(
        ws_id,
        "Acme Corp",
        EntityType::CCorp,
        Jurisdiction::new("DE").unwrap(),
    )
    .unwrap()
}

fn make_contact(entity_id: EntityId, ws_id: WorkspaceId) -> Contact {
    Contact::new(
        entity_id,
        ws_id,
        ContactType::Individual,
        "Jane Founder",
        ContactCategory::Founder,
    )
    .unwrap()
}

// ── 1. EntityStore::init and EntityStore::open ────────────────────────────────

#[tokio::test]
async fn test_entity_store_init_and_open() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    // init creates the repo
    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .expect("init should succeed");

    assert_eq!(store.workspace_id(), ws_id);
    assert_eq!(store.entity_id(), ent_id);

    // HEAD file exists after init
    assert!(dir.path().join("HEAD").exists());

    // open succeeds on an already-initialised repo
    let _store2 = EntityStore::open(git_entity_backend(&dir), ws_id, ent_id)
        .await
        .expect("open of existing repo should succeed");
}

#[tokio::test]
async fn test_entity_store_open_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let result = EntityStore::open(git_entity_backend(&dir), ws_id, ent_id).await;
    assert!(
        matches!(result, Err(StorageError::NotFound(_))),
        "opening a repo that was never init'd should return NotFound"
    );
}

// ── 2. Write and read a typed domain object ───────────────────────────────────

#[tokio::test]
async fn test_write_and_read_entity() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    let entity = make_entity(ws_id);
    let stored_id = entity.entity_id;

    store
        .write::<Entity>(&entity, stored_id, "main", "add entity")
        .await
        .expect("write should succeed");

    let loaded: Entity = store
        .read::<Entity>(stored_id, "main")
        .await
        .expect("read should succeed");

    assert_eq!(loaded.entity_id, entity.entity_id);
    assert_eq!(loaded.legal_name, "Acme Corp");
}

#[tokio::test]
async fn test_write_and_read_contact() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    let contact = make_contact(ent_id, ws_id);
    let contact_id = contact.contact_id;

    store
        .write::<Contact>(&contact, contact_id, "main", "add contact")
        .await
        .unwrap();

    let loaded: Contact = store.read::<Contact>(contact_id, "main").await.unwrap();

    assert_eq!(loaded.contact_id, contact_id);
    assert_eq!(loaded.name, "Jane Founder");
    assert_eq!(loaded.category, ContactCategory::Founder);
}

// ── 3. list_ids ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_ids() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    // No contacts yet — list should be empty.
    let ids: Vec<ContactId> = store.list_ids::<Contact>("main").await.unwrap();
    assert!(ids.is_empty(), "expected no contacts initially");

    // Write three contacts.
    let c1 = make_contact(ent_id, ws_id);
    let c2 = make_contact(ent_id, ws_id);
    let c3 = make_contact(ent_id, ws_id);
    let id1 = c1.contact_id;
    let id2 = c2.contact_id;
    let id3 = c3.contact_id;

    for (c, id) in [(&c1, id1), (&c2, id2), (&c3, id3)] {
        store
            .write::<Contact>(c, id, "main", "add contact")
            .await
            .unwrap();
    }

    let mut ids: Vec<ContactId> = store.list_ids::<Contact>("main").await.unwrap();
    ids.sort();
    let mut expected = vec![id1, id2, id3];
    expected.sort();

    assert_eq!(ids, expected);
}

// ── 4. read_all ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_read_all() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    // Write two contacts.
    let c1 = make_contact(ent_id, ws_id);
    let c2 = make_contact(ent_id, ws_id);
    let id1 = c1.contact_id;
    let id2 = c2.contact_id;

    store
        .write::<Contact>(&c1, id1, "main", "add c1")
        .await
        .unwrap();
    store
        .write::<Contact>(&c2, id2, "main", "add c2")
        .await
        .unwrap();

    let all: Vec<Contact> = store.read_all::<Contact>("main").await.unwrap();
    assert_eq!(all.len(), 2);

    let mut loaded_ids: Vec<ContactId> = all.iter().map(|c| c.contact_id).collect();
    loaded_ids.sort();
    let mut expected = vec![id1, id2];
    expected.sort();
    assert_eq!(loaded_ids, expected);
}

// ── 5. delete ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_delete() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    let contact = make_contact(ent_id, ws_id);
    let contact_id = contact.contact_id;

    store
        .write::<Contact>(&contact, contact_id, "main", "add contact")
        .await
        .unwrap();

    // Confirm it exists.
    let loaded: Contact = store.read::<Contact>(contact_id, "main").await.unwrap();
    assert_eq!(loaded.contact_id, contact_id);

    // Delete it.
    store
        .delete::<Contact>(contact_id, "main", "remove contact")
        .await
        .expect("delete should succeed");

    // Now reading should return NotFound.
    let result = store.read::<Contact>(contact_id, "main").await;
    assert!(
        matches!(result, Err(StorageError::NotFound(_))),
        "read after delete should be NotFound, got: {:?}",
        result
    );
}

#[tokio::test]
async fn test_delete_nonexistent_returns_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    let ghost_id = ContactId::new();
    let result = store.delete::<Contact>(ghost_id, "main", "noop").await;
    assert!(
        matches!(result, Err(StorageError::NotFound(_))),
        "deleting a non-existent file should be NotFound, got: {:?}",
        result
    );
}

// ── 6. path_exists ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_path_exists() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    // The "init" file written during EntityStore::init should exist.
    let exists = store.path_exists("init", "main").await.unwrap();
    assert!(exists, "init path should exist after store init");

    // A path that was never written.
    let missing = store
        .path_exists("contacts/nonexistent.json", "main")
        .await
        .unwrap();
    assert!(!missing, "unwritten path should not exist");

    // Write a contact and check its path.
    let contact = make_contact(ent_id, ws_id);
    let contact_id = contact.contact_id;
    store
        .write::<Contact>(&contact, contact_id, "main", "add")
        .await
        .unwrap();

    let path = format!("contacts/{}.json", contact_id);
    let exists_now = store.path_exists(&path, "main").await.unwrap();
    assert!(exists_now, "contact path should exist after write");
}

// ── 7. write_json / read_json ─────────────────────────────────────────────────

#[tokio::test]
async fn test_write_and_read_raw_json() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Config {
        version: u32,
        label: String,
    }

    let cfg = Config {
        version: 3,
        label: "prod".into(),
    };

    store
        .write_json("config/settings.json", &cfg, "main", "add config")
        .await
        .expect("write_json should succeed");

    let loaded: Config = store
        .read_json("config/settings.json", "main")
        .await
        .expect("read_json should succeed");

    assert_eq!(loaded, cfg);
}

// ── 8. Read non-existent returns NotFound ─────────────────────────────────────

#[tokio::test]
async fn test_read_nonexistent_returns_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    let ghost_contact_id = ContactId::new();
    let result = store.read::<Contact>(ghost_contact_id, "main").await;

    assert!(
        matches!(result, Err(StorageError::NotFound(_))),
        "reading a path that was never written should return NotFound, got: {:?}",
        result
    );
}

// ── 9. Multiple entities in the same workspace repo ───────────────────────────

#[tokio::test]
async fn test_multiple_entity_types_same_store() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    let entity = make_entity(ws_id);
    let entity_id = entity.entity_id;

    // Write an Entity and a Contact into the same repo.
    store
        .write::<Entity>(&entity, entity_id, "main", "add entity")
        .await
        .unwrap();

    let c = make_contact(ent_id, ws_id);
    let c_id = c.contact_id;
    store
        .write::<Contact>(&c, c_id, "main", "add contact")
        .await
        .unwrap();

    // Both are independently readable and do not interfere.
    let loaded_entity: Entity = store.read::<Entity>(entity_id, "main").await.unwrap();
    let loaded_contact: Contact = store.read::<Contact>(c_id, "main").await.unwrap();

    assert_eq!(loaded_entity.entity_id, entity_id);
    assert_eq!(loaded_contact.contact_id, c_id);

    // list_ids for each type returns only the objects of that type.
    let entity_ids: Vec<EntityId> = store.list_ids::<Entity>("main").await.unwrap();
    assert_eq!(entity_ids.len(), 1);
    assert_eq!(entity_ids[0], entity_id);

    let contact_ids: Vec<ContactId> = store.list_ids::<Contact>("main").await.unwrap();
    assert_eq!(contact_ids.len(), 1);
    assert_eq!(contact_ids[0], c_id);
}

// ── 10. Multiple workspaces (distinct repos) ───────────────────────────────────

#[tokio::test]
async fn test_multiple_workspaces_are_isolated() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();

    let ws_a = WorkspaceId::new();
    let ws_b = WorkspaceId::new();
    let ent_a = EntityId::new();
    let ent_b = EntityId::new();

    let store_a = EntityStore::init(git_entity_backend(&dir_a), ws_a, ent_a, b"{}")
        .await
        .unwrap();
    let store_b = EntityStore::init(git_entity_backend(&dir_b), ws_b, ent_b, b"{}")
        .await
        .unwrap();

    // Write a contact only into workspace A.
    let ca = make_contact(ent_a, ws_a);
    let ca_id = ca.contact_id;
    store_a
        .write::<Contact>(&ca, ca_id, "main", "add ca")
        .await
        .unwrap();

    // Store B has no contacts.
    let b_contacts: Vec<ContactId> = store_b.list_ids::<Contact>("main").await.unwrap();
    assert!(b_contacts.is_empty(), "workspace B should have no contacts");

    // Store A can read back its contact; store B cannot.
    let loaded: Contact = store_a.read::<Contact>(ca_id, "main").await.unwrap();
    assert_eq!(loaded.contact_id, ca_id);

    let result = store_b.read::<Contact>(ca_id, "main").await;
    assert!(
        matches!(result, Err(StorageError::NotFound(_))),
        "workspace B should not see workspace A's contact"
    );
}

// ── 11. WorkspaceStore: init, open, API key CRUD ──────────────────────────────

#[tokio::test]
async fn test_workspace_store_init_and_open() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();

    let ws_store = WorkspaceStore::init(git_ws_backend(&dir), ws_id)
        .await
        .expect("WorkspaceStore::init should succeed");

    assert_eq!(ws_store.workspace_id(), ws_id);
    assert!(dir.path().join("HEAD").exists());

    // Open it again.
    let _ws_store2 = WorkspaceStore::open(git_ws_backend(&dir), ws_id)
        .await
        .expect("WorkspaceStore::open should succeed on existing repo");
}

#[tokio::test]
async fn test_workspace_store_open_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();

    let result = WorkspaceStore::open(git_ws_backend(&dir), ws_id).await;
    assert!(
        matches!(result, Err(StorageError::NotFound(_))),
        "opening a workspace that was never init'd should return NotFound"
    );
}

#[tokio::test]
async fn test_api_key_write_and_read() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();

    let ws_store = WorkspaceStore::init(git_ws_backend(&dir), ws_id)
        .await
        .unwrap();

    let record = ApiKeyRecord::new(
        "CI deploy key",
        "hash_of_secret",
        vec!["deploy".to_string()],
        None,
    );
    let key_id = record.key_id;

    ws_store
        .write_api_key(&record)
        .await
        .expect("write_api_key should succeed");

    let loaded = ws_store
        .read_api_key(key_id)
        .await
        .expect("read_api_key should succeed");

    assert_eq!(loaded.key_id, key_id);
    assert_eq!(loaded.name, "CI deploy key");
    assert_eq!(loaded.key_hash, "hash_of_secret");
    assert_eq!(loaded.scopes, vec!["deploy"]);
    assert!(!loaded.deleted);
}

#[tokio::test]
async fn test_api_key_list_ids() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();

    let ws_store = WorkspaceStore::init(git_ws_backend(&dir), ws_id)
        .await
        .unwrap();

    // No keys yet.
    let ids = ws_store.list_api_key_ids().await.unwrap();
    assert!(ids.is_empty());

    // Write two keys.
    let k1 = ApiKeyRecord::new("key-1", "hash1", vec![], None);
    let k2 = ApiKeyRecord::new("key-2", "hash2", vec!["read".to_string()], None);
    let id1 = k1.key_id;
    let id2 = k2.key_id;

    ws_store.write_api_key(&k1).await.unwrap();
    ws_store.write_api_key(&k2).await.unwrap();

    let mut ids = ws_store.list_api_key_ids().await.unwrap();
    ids.sort();
    let mut expected = vec![id1, id2];
    expected.sort();
    assert_eq!(ids, expected);
}

#[tokio::test]
async fn test_api_key_soft_delete() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();

    let ws_store = WorkspaceStore::init(git_ws_backend(&dir), ws_id)
        .await
        .unwrap();

    let record = ApiKeyRecord::new("temp key", "hsh", vec![], None);
    let key_id = record.key_id;

    ws_store.write_api_key(&record).await.unwrap();

    // Soft-delete.
    ws_store
        .delete_api_key(key_id)
        .await
        .expect("delete_api_key should succeed");

    // Record is still readable (retained for audit), but `deleted` is true.
    let loaded = ws_store.read_api_key(key_id).await.unwrap();
    assert!(loaded.deleted, "soft-deleted key should have deleted=true");

    // The key still appears in list_api_key_ids.
    let ids = ws_store.list_api_key_ids().await.unwrap();
    assert!(
        ids.contains(&key_id),
        "soft-deleted key should still be listed"
    );
}

#[tokio::test]
async fn test_api_key_read_nonexistent_returns_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();

    let ws_store = WorkspaceStore::init(git_ws_backend(&dir), ws_id)
        .await
        .unwrap();

    use corp_core::ids::ApiKeyId;
    let ghost_id = ApiKeyId::new();
    let result = ws_store.read_api_key(ghost_id).await;

    assert!(
        matches!(result, Err(StorageError::NotFound(_))),
        "reading a non-existent API key should return NotFound"
    );
}

// ── 12. Concurrent reads ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_concurrent_reads() {
    let dir = tempfile::tempdir().unwrap();
    let ws_id = WorkspaceId::new();
    let ent_id = EntityId::new();

    let store = EntityStore::init(git_entity_backend(&dir), ws_id, ent_id, b"{}")
        .await
        .unwrap();

    // Pre-populate five contacts.
    let contacts: Vec<Contact> = (0..5).map(|_| make_contact(ent_id, ws_id)).collect();
    let ids: Vec<_> = contacts.iter().map(|c| c.contact_id).collect();

    for (c, id) in contacts.iter().zip(ids.iter()) {
        store
            .write::<Contact>(c, *id, "main", "seed")
            .await
            .unwrap();
    }

    // Share the repo path so we can open independent store handles per task.
    let repo_path = Arc::new(PathBuf::from(dir.path()));

    let tasks: Vec<_> = ids
        .iter()
        .map(|&contact_id| {
            let rp = Arc::clone(&repo_path);
            tokio::spawn(async move {
                let backend = EntityBackend::Git { repo_path: rp };
                let store = EntityStore::open(backend, ws_id, ent_id).await.unwrap();
                let loaded: Contact = store.read::<Contact>(contact_id, "main").await.unwrap();
                assert_eq!(loaded.contact_id, contact_id);
            })
        })
        .collect();

    for task in tasks {
        task.await.expect("concurrent read task should not panic");
    }
}
