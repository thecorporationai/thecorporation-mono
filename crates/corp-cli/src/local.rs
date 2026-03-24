//! Local-mode backend: operates directly on a git repo, no server needed.
//!
//! When `--local` or `--data-dir` is set, the CLI embeds `corp-storage` and
//! `corp-core` to perform all operations in-process against bare git repos.
//! This means `corp form create --local` works without any running server.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use sha2::Digest;

use corp_core::contacts::{Contact, ContactCategory, ContactType};
use corp_core::formation::{
    Document, DocumentType, Entity, EntityType, Filing, FilingType, FormationStatus,
    IrsTaxClassification, Jurisdiction, Signature, TaxProfile,
};
use corp_core::ids::*;
use corp_storage::entity_store::{Backend as EntityBackend, EntityStore};
use corp_storage::workspace_store::{Backend as WsBackend, WorkspaceStore};

/// A handle to a local data directory that can open stores without HTTP.
#[derive(Clone)]
pub struct LocalBackend {
    pub data_dir: PathBuf,
    pub workspace_id: WorkspaceId,
}

impl LocalBackend {
    /// Open (or create) a local backend at `data_dir` for the given workspace.
    pub fn new(data_dir: impl Into<PathBuf>, workspace_id: WorkspaceId) -> Self {
        Self {
            data_dir: data_dir.into(),
            workspace_id,
        }
    }

    /// Derive a default workspace ID from the data dir (deterministic).
    pub fn default_workspace(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        // Use a deterministic UUID based on the canonical path.
        let canonical = data_dir.canonicalize().unwrap_or_else(|_| data_dir.clone());
        let bytes = canonical.to_string_lossy().as_bytes().to_vec();
        let hash = sha2::Sha256::digest(&bytes);
        let ws_id = WorkspaceId::from_uuid(
            uuid::Uuid::from_slice(&hash[..16]).unwrap_or_else(|_| uuid::Uuid::new_v4()),
        );
        Self {
            data_dir,
            workspace_id: ws_id,
        }
    }

    fn entity_path(&self, entity_id: EntityId) -> PathBuf {
        self.data_dir
            .join(self.workspace_id.to_string())
            .join("entities")
            .join(entity_id.to_string())
    }

    fn workspace_path(&self) -> PathBuf {
        self.data_dir
            .join(self.workspace_id.to_string())
            .join("workspace")
    }

    /// Open an existing entity store.
    pub async fn open_entity(&self, entity_id: EntityId) -> Result<EntityStore> {
        let path = self.entity_path(entity_id);
        let backend = EntityBackend::Git {
            repo_path: Arc::new(path),
        };
        EntityStore::open(backend, self.workspace_id, entity_id)
            .await
            .with_context(|| format!("open entity {entity_id}"))
    }

    /// Init a new entity store (creates bare git repo).
    pub async fn init_entity(&self, entity_id: EntityId) -> Result<EntityStore> {
        let path = self.entity_path(entity_id);
        tokio::fs::create_dir_all(&path).await?;
        let backend = EntityBackend::Git {
            repo_path: Arc::new(path),
        };
        EntityStore::init(backend, self.workspace_id, entity_id, b"{}")
            .await
            .with_context(|| format!("init entity {entity_id}"))
    }

    /// Open or init the workspace store.
    pub async fn workspace_store(&self) -> Result<WorkspaceStore> {
        let path = self.workspace_path();
        let backend = WsBackend::Git {
            repo_path: Arc::new(path.clone()),
        };
        match WorkspaceStore::open(backend, self.workspace_id).await {
            Ok(ws) => Ok(ws),
            Err(_) => {
                tokio::fs::create_dir_all(&path).await?;
                let backend = WsBackend::Git {
                    repo_path: Arc::new(path),
                };
                WorkspaceStore::init(backend, self.workspace_id)
                    .await
                    .context("init workspace store")
            }
        }
    }

    // ── High-level operations ────────────────────────────────────────────────

    /// Create a new entity with filing and tax profile. Returns JSON.
    pub async fn create_entity(
        &self,
        legal_name: &str,
        entity_type: EntityType,
        jurisdiction: &str,
    ) -> Result<serde_json::Value> {
        let jur = Jurisdiction::new(jurisdiction).map_err(|e| anyhow::anyhow!("{e}"))?;
        let entity = Entity::new(self.workspace_id, legal_name, entity_type, jur.clone())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let entity_id = entity.entity_id;

        // Init entity store.
        let store = self.init_entity(entity_id).await?;

        // Register in workspace index.
        let ws = self.workspace_store().await?;
        ws.register_entity(entity_id)
            .await
            .map_err(|e| anyhow::anyhow!("register entity: {e}"))?;

        // Write entity.
        store
            .write::<Entity>(&entity, entity_id, "main", "create entity")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Create filing.
        let filing_type = match entity_type {
            EntityType::CCorp => FilingType::CertificateOfIncorporation,
            EntityType::Llc => FilingType::CertificateOfFormation,
        };
        let filing = Filing::new(entity_id, self.workspace_id, filing_type, jur.as_str());
        store
            .write::<Filing>(&filing, filing.filing_id, "main", "create filing")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Create tax profile.
        let classification = match entity_type {
            EntityType::CCorp => IrsTaxClassification::CCorporation,
            EntityType::Llc => IrsTaxClassification::DisregardedEntity,
        };
        let tax = TaxProfile::new(entity_id, self.workspace_id, classification);
        store
            .write::<TaxProfile>(&tax, tax.tax_profile_id, "main", "create tax profile")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(serde_json::to_value(&entity)?)
    }

    /// List all entities in the workspace. Returns JSON array.
    pub async fn list_entities(&self) -> Result<serde_json::Value> {
        let ws = self.workspace_store().await?;
        let ids = ws.list_entity_ids().await.unwrap_or_default();
        let mut entities = Vec::new();
        for id in ids {
            if let Ok(store) = self.open_entity(id).await
                && let Ok(e) = store.read::<Entity>(id, "main").await
            {
                entities.push(serde_json::to_value(&e)?);
            }
        }
        Ok(serde_json::Value::Array(entities))
    }

    /// Get a single entity. Returns JSON.
    pub async fn get_entity(&self, entity_id: EntityId) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let entity: Entity = store
            .read(entity_id, "main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(serde_json::to_value(&entity)?)
    }

    /// Advance formation status. Returns JSON.
    pub async fn advance_formation(&self, entity_id: EntityId) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let mut entity: Entity = store
            .read(entity_id, "main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let prev = entity.formation_status;
        entity
            .advance_status()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Auto-create documents on Pending → DocumentsGenerated.
        if prev == FormationStatus::Pending {
            let doc_types = match entity.entity_type {
                EntityType::CCorp => vec![
                    (
                        DocumentType::CertificateOfIncorporation,
                        "Certificate of Incorporation",
                    ),
                    (DocumentType::Bylaws, "Bylaws"),
                    (DocumentType::IncorporatorAction, "Action of Incorporator"),
                ],
                EntityType::Llc => vec![
                    (
                        DocumentType::ArticlesOfOrganization,
                        "Articles of Organization",
                    ),
                    (DocumentType::OperatingAgreement, "Operating Agreement"),
                ],
            };
            for (dt, title) in doc_types {
                let content = serde_json::json!({
                    "entity_name": entity.legal_name,
                    "entity_type": format!("{:?}", entity.entity_type),
                    "jurisdiction": entity.jurisdiction.as_str(),
                    "document_type": title,
                });
                let bytes = serde_json::to_vec(&content)?;
                let hash = format!("{:x}", sha2::Sha256::digest(&bytes));
                let doc = Document::new(entity_id, self.workspace_id, dt, title, content, hash);
                store
                    .write::<Document>(&doc, doc.document_id, "main", &format!("create {title}"))
                    .await
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
        }

        store
            .write::<Entity>(&entity, entity_id, "main", "advance formation")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(serde_json::to_value(&entity)?)
    }

    /// List formation documents. Returns JSON array.
    pub async fn list_documents(&self, entity_id: EntityId) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let docs: Vec<Document> = store
            .read_all("main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(serde_json::to_value(&docs)?)
    }

    /// Sign a document. Finds the owning entity by scanning workspace. Returns JSON.
    pub async fn sign_document(
        &self,
        document_id: DocumentId,
        signer_name: &str,
        signer_role: &str,
        signer_email: &str,
        signature_text: &str,
        _consent_text: &str,
    ) -> Result<serde_json::Value> {
        let ws = self.workspace_store().await?;
        let ids = ws.list_entity_ids().await.unwrap_or_default();

        for eid in ids {
            let Ok(store) = self.open_entity(eid).await else {
                continue;
            };
            let Ok(mut doc) = store.read::<Document>(document_id, "main").await else {
                continue;
            };

            let sig = Signature::new(
                document_id,
                signer_name,
                signer_role,
                signer_email,
                signature_text,
                None,
                doc.content_hash.clone(),
            );
            doc.sign(sig, &[]).map_err(|e| anyhow::anyhow!("{e}"))?;

            store
                .write::<Document>(&doc, document_id, "main", "sign document")
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;

            return Ok(serde_json::to_value(&doc)?);
        }

        anyhow::bail!("document {document_id} not found in any entity")
    }

    /// Get filing for an entity. Returns JSON.
    pub async fn get_filing(&self, entity_id: EntityId) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let ids = store
            .list_ids::<Filing>("main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let fid = ids
            .first()
            .ok_or_else(|| anyhow::anyhow!("no filing found"))?;
        let filing: Filing = store
            .read(*fid, "main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(serde_json::to_value(&filing)?)
    }

    /// Confirm filing. Returns JSON.
    pub async fn confirm_filing(
        &self,
        entity_id: EntityId,
        confirmation_number: Option<&str>,
    ) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let ids = store
            .list_ids::<Filing>("main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let fid = ids
            .first()
            .ok_or_else(|| anyhow::anyhow!("no filing found"))?;
        let mut filing: Filing = store
            .read(*fid, "main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let conf = confirmation_number
            .map(|s| s.to_owned())
            .unwrap_or_else(|| format!("CONF-{}", Utc::now().timestamp()));
        filing
            .confirm(conf, Utc::now())
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        store
            .write::<Filing>(&filing, *fid, "main", "confirm filing")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(serde_json::to_value(&filing)?)
    }

    /// Get tax profile. Returns JSON.
    pub async fn get_tax(&self, entity_id: EntityId) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let ids = store
            .list_ids::<TaxProfile>("main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let tid = ids
            .first()
            .ok_or_else(|| anyhow::anyhow!("no tax profile found"))?;
        let tax: TaxProfile = store
            .read(*tid, "main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(serde_json::to_value(&tax)?)
    }

    /// Confirm EIN. Returns JSON.
    pub async fn confirm_ein(&self, entity_id: EntityId, ein: &str) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let ids = store
            .list_ids::<TaxProfile>("main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let tid = ids
            .first()
            .ok_or_else(|| anyhow::anyhow!("no tax profile found"))?;
        let mut tax: TaxProfile = store
            .read(*tid, "main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        tax.assign_ein(ein).map_err(|e| anyhow::anyhow!("{e}"))?;

        store
            .write::<TaxProfile>(&tax, *tid, "main", "confirm EIN")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(serde_json::to_value(&tax)?)
    }

    /// Dissolve an entity. Returns JSON.
    pub async fn dissolve_entity(&self, entity_id: EntityId) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let mut entity: Entity = store
            .read(entity_id, "main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        entity
            .dissolve(Utc::now().date_naive())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        store
            .write::<Entity>(&entity, entity_id, "main", "dissolve entity")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(serde_json::to_value(&entity)?)
    }

    /// Create a contact. Returns JSON.
    pub async fn create_contact(
        &self,
        entity_id: EntityId,
        name: &str,
        _email: Option<&str>,
        category: Option<&str>,
    ) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let cat = category
            .and_then(|c| serde_json::from_value(serde_json::Value::String(c.to_owned())).ok())
            .unwrap_or(ContactCategory::Other);
        let contact = Contact::new(
            entity_id,
            self.workspace_id,
            ContactType::Individual,
            name,
            cat,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        let cid = contact.contact_id;
        store
            .write::<Contact>(&contact, cid, "main", "create contact")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(serde_json::to_value(&contact)?)
    }

    /// List contacts. Returns JSON array.
    pub async fn list_contacts(&self, entity_id: EntityId) -> Result<serde_json::Value> {
        let store = self.open_entity(entity_id).await?;
        let contacts: Vec<Contact> = store
            .read_all("main")
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(serde_json::to_value(&contacts)?)
    }
}
