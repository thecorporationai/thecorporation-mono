//! Governance markdown document generator and bundle metadata.

use anyhow::{Context, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use super::profile::GovernanceProfile;
use crate::domain::ids::{EntityId, GovernanceDocBundleId};

pub const GOVERNANCE_DOC_BUNDLES_ROOT: &str = "governance/doc-bundles";
pub const GOVERNANCE_DOC_BUNDLES_CURRENT_PATH: &str = "governance/doc-bundles/current.json";
pub const GOVERNANCE_DOC_BUNDLES_HISTORY_DIR: &str = "governance/doc-bundles/history";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceDocEntityType {
    Corporation,
    Llc,
}

impl GovernanceDocEntityType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Corporation => "corporation",
            Self::Llc => "llc",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedGovernanceDocument {
    pub path: String,
    pub source_path: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDocManifest {
    pub version: u32,
    pub entity_type: String,
    pub generated_at: String,
    pub source_root: String,
    pub documents: Vec<GeneratedGovernanceDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDocBundleManifest {
    pub bundle_id: GovernanceDocBundleId,
    pub entity_id: EntityId,
    pub entity_type: String,
    pub profile_version: u32,
    pub template_version: String,
    pub generated_at: String,
    pub source_root: String,
    pub documents: Vec<GeneratedGovernanceDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDocBundleCurrent {
    pub bundle_id: GovernanceDocBundleId,
    pub entity_id: EntityId,
    pub manifest_path: String,
    pub generated_at: String,
    pub template_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceDocBundleSummary {
    pub bundle_id: GovernanceDocBundleId,
    pub entity_id: EntityId,
    pub entity_type: String,
    pub profile_version: u32,
    pub template_version: String,
    pub generated_at: String,
    pub document_count: usize,
}

#[derive(Debug, Clone)]
pub struct RenderedGovernanceDocument {
    pub path: String,
    pub source_path: String,
    pub sha256: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RenderedGovernanceBundle {
    pub manifest: GovernanceDocBundleManifest,
    pub current: GovernanceDocBundleCurrent,
    pub summary: GovernanceDocBundleSummary,
    pub documents: Vec<RenderedGovernanceDocument>,
}

const COMMON_DOCS: &[&str] = &[
    "common/agent-delegation-schedule.md",
    "common/assumptions-and-decisions.md",
    "common/signing-and-records-standard.md",
    "common/agent-operator-service-agreement-template.md",
    "common/agent-operator-service-agreement-checklist.md",
];

const COMPLIANCE_DOCS: &[&str] = &[
    "compliance/formation-checklist.md",
    "compliance/annual-compliance-calendar.md",
];

const TRANSACTION_DOCS: &[&str] = &[
    "transactions/board-consent.md",
    "transactions/equity-issuance-approval.md",
    "transactions/investor-rights-agreement.md",
    "transactions/stock-transfer-agreement.md",
    "transactions/subscription-agreement.md",
    "transactions/transfer-board-consent.md",
];

const CORPORATION_DOCS: &[&str] = &[
    "corporation/articles-of-incorporation.md",
    "corporation/bylaws.md",
    "corporation/incorporator-action.md",
    "corporation/initial-board-consent.md",
    "corporation/stock-issuance-consent.md",
];

const LLC_DOCS: &[&str] = &[
    "llc/articles-of-organization.md",
    "llc/operating-agreement.md",
    "llc/initial-written-consent.md",
];

pub fn bundle_root(bundle_id: GovernanceDocBundleId) -> String {
    format!("{GOVERNANCE_DOC_BUNDLES_ROOT}/{bundle_id}")
}

pub fn bundle_manifest_path(bundle_id: GovernanceDocBundleId) -> String {
    format!("{}/manifest.json", bundle_root(bundle_id))
}

pub fn bundle_documents_prefix(bundle_id: GovernanceDocBundleId) -> String {
    format!("{}/documents", bundle_root(bundle_id))
}

pub fn bundle_history_path(bundle_id: GovernanceDocBundleId) -> String {
    format!("{GOVERNANCE_DOC_BUNDLES_HISTORY_DIR}/{bundle_id}.json")
}

pub fn relative_document_paths(entity_type: GovernanceDocEntityType) -> Vec<&'static str> {
    let mut docs = Vec::new();
    docs.extend_from_slice(COMMON_DOCS);
    docs.extend_from_slice(COMPLIANCE_DOCS);
    docs.extend_from_slice(TRANSACTION_DOCS);
    match entity_type {
        GovernanceDocEntityType::Corporation => docs.extend_from_slice(CORPORATION_DOCS),
        GovernanceDocEntityType::Llc => docs.extend_from_slice(LLC_DOCS),
    }
    docs
}

/// Legacy copy-style bundle generation to filesystem (CLI fallback).
pub fn generate_bundle(
    entity_type: GovernanceDocEntityType,
    out_dir: &Path,
) -> anyhow::Result<GovernanceDocManifest> {
    let repo_root = find_repo_root(&std::env::current_dir().context("read current directory")?)?;
    generate_bundle_from_repo_root(entity_type, &repo_root, out_dir)
}

/// Legacy copy-style bundle generation to filesystem (CLI fallback).
pub fn generate_bundle_from_repo_root(
    entity_type: GovernanceDocEntityType,
    repo_root: &Path,
    out_dir: &Path,
) -> anyhow::Result<GovernanceDocManifest> {
    let docs_root = repo_root.join("documents/governance");
    if !docs_root.is_dir() {
        bail!(
            "missing governance docs root at {}",
            docs_root.to_string_lossy()
        );
    }

    fs::create_dir_all(out_dir)
        .with_context(|| format!("create output dir {}", out_dir.to_string_lossy()))?;

    let mut generated = Vec::new();
    for rel in relative_document_paths(entity_type) {
        let source = docs_root.join(rel);
        if !source.is_file() {
            bail!(
                "missing governance source document: {}",
                source.to_string_lossy()
            );
        }
        let target = out_dir.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create output parent {}", parent.to_string_lossy()))?;
        }
        let bytes = fs::read(&source)
            .with_context(|| format!("read source document {}", source.to_string_lossy()))?;
        fs::write(&target, &bytes)
            .with_context(|| format!("write target document {}", target.to_string_lossy()))?;
        generated.push(GeneratedGovernanceDocument {
            path: rel.to_owned(),
            source_path: source
                .strip_prefix(repo_root)
                .unwrap_or(&source)
                .to_string_lossy()
                .into_owned(),
            sha256: sha256_hex(&bytes),
            bytes: bytes.len(),
        });
    }

    generated.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = GovernanceDocManifest {
        version: 1,
        entity_type: entity_type.as_str().to_owned(),
        generated_at: Utc::now().to_rfc3339(),
        source_root: docs_root.to_string_lossy().into_owned(),
        documents: generated,
    };
    fs::write(
        out_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).context("serialize governance doc manifest")?,
    )
    .with_context(|| {
        format!(
            "write manifest {}",
            out_dir.join("manifest.json").to_string_lossy()
        )
    })?;
    Ok(manifest)
}

pub fn render_bundle_from_profile(
    entity_type: GovernanceDocEntityType,
    entity_id: EntityId,
    profile: &GovernanceProfile,
    template_version: &str,
) -> anyhow::Result<RenderedGovernanceBundle> {
    let repo_root = find_repo_root(&std::env::current_dir().context("read current directory")?)?;
    render_bundle_from_profile_with_repo_root(
        entity_type,
        entity_id,
        profile,
        template_version,
        &repo_root,
    )
}

pub fn render_bundle_from_profile_with_repo_root(
    entity_type: GovernanceDocEntityType,
    entity_id: EntityId,
    profile: &GovernanceProfile,
    template_version: &str,
    repo_root: &Path,
) -> anyhow::Result<RenderedGovernanceBundle> {
    let docs_root = repo_root.join("documents/governance");
    if !docs_root.is_dir() {
        bail!(
            "missing governance docs root at {}",
            docs_root.to_string_lossy()
        );
    }

    let mut rendered_docs = Vec::new();
    for rel in relative_document_paths(entity_type) {
        let source = docs_root.join(rel);
        if !source.is_file() {
            bail!(
                "missing governance source document: {}",
                source.to_string_lossy()
            );
        }
        let source_path = source
            .strip_prefix(repo_root)
            .unwrap_or(&source)
            .to_string_lossy()
            .into_owned();

        let markdown = fs::read_to_string(&source)
            .with_context(|| format!("read source document {}", source.to_string_lossy()))?;
        let rendered = apply_profile_replacements(&markdown, entity_type, profile);
        let content = rendered.into_bytes();
        rendered_docs.push(RenderedGovernanceDocument {
            path: rel.to_owned(),
            source_path,
            sha256: sha256_hex(&content),
            content,
        });
    }
    rendered_docs.sort_by(|a, b| a.path.cmp(&b.path));

    let bundle_id = GovernanceDocBundleId::new();
    let generated_at = Utc::now().to_rfc3339();
    let manifest = GovernanceDocBundleManifest {
        bundle_id,
        entity_id,
        entity_type: entity_type.as_str().to_owned(),
        profile_version: profile.version(),
        template_version: template_version.to_owned(),
        generated_at: generated_at.clone(),
        source_root: docs_root.to_string_lossy().into_owned(),
        documents: rendered_docs
            .iter()
            .map(|d| GeneratedGovernanceDocument {
                path: d.path.clone(),
                source_path: d.source_path.clone(),
                sha256: d.sha256.clone(),
                bytes: d.content.len(),
            })
            .collect(),
    };
    let current = GovernanceDocBundleCurrent {
        bundle_id,
        entity_id,
        manifest_path: bundle_manifest_path(bundle_id),
        generated_at: generated_at.clone(),
        template_version: template_version.to_owned(),
    };
    let summary = GovernanceDocBundleSummary {
        bundle_id,
        entity_id,
        entity_type: entity_type.as_str().to_owned(),
        profile_version: profile.version(),
        template_version: template_version.to_owned(),
        generated_at,
        document_count: manifest.documents.len(),
    };
    Ok(RenderedGovernanceBundle {
        manifest,
        current,
        summary,
        documents: rendered_docs,
    })
}

fn apply_profile_replacements(
    source: &str,
    entity_type: GovernanceDocEntityType,
    profile: &GovernanceProfile,
) -> String {
    let mut out = source.to_owned();

    // Shared schedule/profile values.
    out = out.replace(
        "**Effective date**: `YYYY-MM-DD`",
        &format!("**Effective date**: `{}`", profile.effective_date()),
    );
    out = out.replace(
        "**Adopted by**: `TBD` (Initial Board Consent / Initial Member Consent)",
        &format!(
            "**Adopted by**: `{}` (Initial Board Consent / Initial Member Consent)",
            profile.adopted_by()
        ),
    );
    out = out.replace(
        "**Last reviewed**: `YYYY-MM-DD`",
        &format!("**Last reviewed**: `{}`", profile.last_reviewed()),
    );
    out = out.replace(
        "**Next mandatory review**: 12 months from effective date",
        &format!(
            "**Next mandatory review**: {}",
            profile.next_mandatory_review()
        ),
    );
    out = out.replace(
        "Effective Date: `YYYY-MM-DD`",
        &format!("Effective Date: `{}`", profile.effective_date()),
    );

    match entity_type {
        GovernanceDocEntityType::Corporation => {
            out = out.replace("TBD Corporation Name", profile.legal_name());
            out = out.replace(
                "Registered agent name: `TBD`",
                &format!(
                    "Registered agent name: `{}`",
                    profile.registered_agent_name().unwrap_or("TBD")
                ),
            );
            out = out.replace(
                "Registered office address (must be in Delaware): `TBD`",
                &format!(
                    "Registered office address (must be in Delaware): `{}`",
                    profile.registered_agent_address().unwrap_or("TBD")
                ),
            );
            if let Some(board_size) = profile.board_size() {
                out = out.replace(
                    "The Board shall consist of `TBD` director(s).",
                    &format!("The Board shall consist of `{board_size}` director(s)."),
                );
            }
            if let Some(inc_name) = profile.incorporator_name() {
                out = out.replace("- Name: `TBD`", &format!("- Name: `{inc_name}`"));
                out = out.replace(
                    "Incorporator Name: `TBD`",
                    &format!("Incorporator Name: `{inc_name}`"),
                );
            }
            if let Some(inc_addr) = profile.incorporator_address() {
                out = out.replace("- Address: `TBD`", &format!("- Address: `{inc_addr}`"));
            }
        }
        GovernanceDocEntityType::Llc => {
            out = out.replace("TBD LLC Name", profile.legal_name());
            if let Some(principal) = profile.principal_name() {
                out = out.replace("`TBD` (Principal)", &format!("`{principal}` (Principal)"));
            }
        }
    }

    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn find_repo_root(start: &Path) -> anyhow::Result<PathBuf> {
    let mut cursor = Some(start);
    while let Some(current) = cursor {
        if current.join("documents/governance").is_dir() {
            return Ok(current.to_path_buf());
        }
        cursor = current.parent();
    }
    bail!(
        "could not locate repository root containing documents/governance from {}",
        start.to_string_lossy()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::formation::{
        entity::Entity,
        types::{EntityType, Jurisdiction},
    };
    use crate::domain::ids::WorkspaceId;
    use tempfile::TempDir;

    fn make_entity(entity_type: EntityType) -> Entity {
        Entity::new(
            EntityId::new(),
            WorkspaceId::new(),
            "Acme Test Entity".to_owned(),
            entity_type,
            Jurisdiction::new("Delaware").expect("jurisdiction"),
            Some("Acme Registered Agent".to_owned()),
            Some("123 Main St".to_owned()),
        )
        .expect("entity")
    }

    #[test]
    fn path_set_contains_transactions_for_corp() {
        let docs = relative_document_paths(GovernanceDocEntityType::Corporation);
        assert!(docs.contains(&"transactions/board-consent.md"));
        assert!(docs.contains(&"transactions/stock-transfer-agreement.md"));
        assert!(docs.contains(&"corporation/bylaws.md"));
        assert!(!docs.contains(&"llc/operating-agreement.md"));
    }

    #[test]
    fn can_generate_bundle_from_repo_root() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = TempDir::new().expect("temp dir");
        let manifest = generate_bundle_from_repo_root(
            GovernanceDocEntityType::Corporation,
            &repo_root,
            out.path(),
        )
        .expect("generate bundle");
        assert!(!manifest.documents.is_empty());
        assert!(out.path().join("manifest.json").is_file());
        assert!(out.path().join("corporation/bylaws.md").is_file());
        assert!(out.path().join("transactions/board-consent.md").is_file());
    }

    #[test]
    fn render_bundle_with_profile_replacements() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entity = make_entity(EntityType::Corporation);
        let mut profile = GovernanceProfile::default_for_entity(&entity);
        profile.update(
            "Acme Holdings".to_owned(),
            "Delaware".to_owned(),
            profile.effective_date(),
            "Board".to_owned(),
            profile.last_reviewed(),
            profile.next_mandatory_review(),
            Some("Acme RA".to_owned()),
            Some("1 Center Plaza".to_owned()),
            Some(3),
            Some("Alice Founder".to_owned()),
            Some("1 Center Plaza".to_owned()),
            Some("Alice Founder".to_owned()),
            Some("CEO".to_owned()),
            Some(false),
        );

        let bundle = render_bundle_from_profile_with_repo_root(
            GovernanceDocEntityType::Corporation,
            entity.entity_id(),
            &profile,
            "v2",
            &repo_root,
        )
        .expect("render bundle");

        let bylaws = bundle
            .documents
            .iter()
            .find(|d| d.path == "corporation/bylaws.md")
            .expect("bylaws");
        let text = String::from_utf8(bylaws.content.clone()).expect("utf8");
        assert!(text.contains("Acme Holdings"));
        assert!(bundle.summary.document_count > 0);
        assert_eq!(bundle.manifest.template_version, "v2");
    }
}
