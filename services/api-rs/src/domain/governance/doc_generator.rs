//! Governance markdown document generator and bundle metadata.

use anyhow::{Context, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use super::doc_ast::{
    ContentNode, DocumentDefinition, EntityTypeKey, GovernanceDocAstV2,
};
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
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
    let placeholder_warnings = detect_placeholders(&rendered_docs);
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
        warnings: placeholder_warnings,
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

/// Placeholder patterns that indicate unfinished document generation.
const PLACEHOLDER_PATTERNS: &[&str] = &["`TBD`", "YYYY-MM-DD", "`TBD ", "TBD`"];

/// Scan rendered documents for residual placeholder markers.
fn detect_placeholders(docs: &[RenderedGovernanceDocument]) -> Vec<String> {
    let mut warnings = Vec::new();
    for doc in docs {
        let content = match std::str::from_utf8(&doc.content) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for pattern in PLACEHOLDER_PATTERNS {
            let count = content.matches(pattern).count();
            if count > 0 {
                warnings.push(format!(
                    "{}: {} occurrence(s) of placeholder \"{}\"",
                    doc.path, count, pattern
                ));
            }
        }
    }
    warnings
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

// ── AST-based rendering ──────────────────────────────────────────────

/// Format cents as a USD string (e.g. 1000000 → "$10,000").
pub fn format_usd(cents: i64) -> String {
    let dollars = cents / 100;
    if dollars == 0 {
        return "$0".to_owned();
    }
    let negative = dollars < 0;
    let abs = dollars.unsigned_abs();
    let s = abs.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    if negative {
        format!("-${result}")
    } else {
        format!("${result}")
    }
}

/// Render a single document definition from the v2 AST to markdown.
pub fn render_document_from_ast(
    doc: &DocumentDefinition,
    ast: &GovernanceDocAstV2,
    entity_type: EntityTypeKey,
    profile: &GovernanceProfile,
) -> String {
    let mut out = String::new();

    // Title
    out.push_str(&format!("# {}\n", doc.title));

    // Preamble (as blockquote)
    if let Some(preamble) = &doc.preamble {
        out.push('\n');
        out.push_str(&format!("> {preamble}\n"));
    }

    // Metadata fields
    if !doc.metadata_fields.is_empty() {
        out.push('\n');
        for field in &doc.metadata_fields {
            let value = resolve_profile_field(&field.key, profile)
                .or_else(|| field.default.clone())
                .unwrap_or_else(|| {
                    field
                        .placeholder
                        .clone()
                        .unwrap_or_else(|| "`TBD`".to_owned())
                });
            out.push_str(&format!("**{}**: `{}`\n", field.label, value));
        }
    }

    // Content nodes
    for node in &doc.content {
        render_node(&mut out, node, ast, entity_type, profile);
    }

    out
}

fn resolve_profile_field(key: &str, profile: &GovernanceProfile) -> Option<String> {
    match key {
        "effective_date" => Some(profile.effective_date().to_string()),
        "adopted_by" => Some(profile.adopted_by().to_owned()),
        "last_reviewed" => Some(profile.last_reviewed().to_string()),
        "next_mandatory_review" => Some(profile.next_mandatory_review().to_string()),
        "legal_name" => Some(profile.legal_name().to_owned()),
        _ => None,
    }
}

fn render_node(
    out: &mut String,
    node: &ContentNode,
    ast: &GovernanceDocAstV2,
    entity_type: EntityTypeKey,
    profile: &GovernanceProfile,
) {
    match node {
        ContentNode::Heading { level, text } => {
            out.push('\n');
            for _ in 0..*level {
                out.push('#');
            }
            out.push(' ');
            out.push_str(&substitute(text, ast, profile));
            out.push('\n');
        }
        ContentNode::Paragraph { text } => {
            out.push('\n');
            out.push_str(&substitute(text, ast, profile));
            out.push('\n');
        }
        ContentNode::OrderedList { items } => {
            out.push('\n');
            for (i, item) in items.iter().enumerate() {
                out.push_str(&format!("{}. {}\n", i + 1, substitute(item, ast, profile)));
            }
        }
        ContentNode::UnorderedList { items } => {
            out.push('\n');
            for item in items {
                out.push_str(&format!("- {}\n", substitute(item, ast, profile)));
            }
        }
        ContentNode::Table { headers, rows } => {
            out.push('\n');
            // Header row
            out.push('|');
            for h in headers {
                out.push_str(&format!(" {} |", h));
            }
            out.push('\n');
            // Separator
            out.push('|');
            for _ in headers {
                out.push_str("---|");
            }
            out.push('\n');
            // Data rows
            for row in rows {
                out.push('|');
                for cell in row {
                    out.push_str(&format!(" {} |", substitute(cell, ast, profile)));
                }
                out.push('\n');
            }
        }
        ContentNode::DataTable { source, columns } => {
            render_data_table(out, source, columns, ast, entity_type);
        }
        ContentNode::Conditional {
            when_entity,
            content,
        } => {
            if *when_entity == entity_type {
                for child in content {
                    render_node(out, child, ast, entity_type, profile);
                }
            }
        }
        ContentNode::SignatureBlock { role, fields } => {
            out.push('\n');
            out.push_str(&format!("{role}: ____________________"));
            for field in fields {
                match field.as_str() {
                    "date" => out.push_str("  Date: `YYYY-MM-DD`"),
                    "name" => out.push_str(&format!("\nName / Title: `TBD`")),
                    "title" => {} // included in name line
                    _ => out.push_str(&format!("\n{field}: `TBD`")),
                }
            }
            out.push('\n');
        }
        ContentNode::Placeholder { key, label } => {
            let value = resolve_profile_field(key, profile)
                .unwrap_or_else(|| "`TBD`".to_owned());
            out.push_str(&format!("**{label}**: {value}\n"));
        }
        ContentNode::Note { text } => {
            out.push('\n');
            out.push_str(&format!("> **Note:** {text}\n"));
        }
        ContentNode::CodeBlock { language, lines } => {
            out.push('\n');
            out.push_str("```");
            if let Some(lang) = language {
                out.push_str(lang);
            }
            out.push('\n');
            for line in lines {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("```\n");
        }
        ContentNode::DocumentRef { text, .. } => {
            out.push_str(&substitute(text, ast, profile));
        }
        ContentNode::HorizontalRule => {
            out.push_str("\n---\n");
        }
    }
}

fn render_data_table(
    out: &mut String,
    source: &str,
    columns: &[super::doc_ast::DataTableColumn],
    ast: &GovernanceDocAstV2,
    _entity_type: EntityTypeKey,
) {
    out.push('\n');
    // Header
    out.push('|');
    for col in columns {
        out.push_str(&format!(" {} |", col.header));
    }
    out.push('\n');
    // Separator — right-align USD columns
    out.push('|');
    for col in columns {
        if col.format.as_deref() == Some("usd") {
            out.push_str("---:|");
        } else {
            out.push_str("---|");
        }
    }
    out.push('\n');

    match source {
        "authority_precedence" => {
            for entry in &ast.authority_precedence {
                out.push('|');
                for col in columns {
                    let val = match col.key.as_str() {
                        "rank" => entry.rank.to_string(),
                        "source" => entry.source.clone(),
                        "label" => entry.label.clone(),
                        _ => String::new(),
                    };
                    out.push_str(&format!(" {} |", val));
                }
                out.push('\n');
            }
        }
        "spending_defaults.categories" => {
            for cat in &ast.spending_defaults.categories {
                out.push('|');
                for col in columns {
                    let val = match col.key.as_str() {
                        "id" => cat.id.clone(),
                        "label" => cat.label.clone(),
                        "per_transaction_cents" => {
                            if col.format.as_deref() == Some("usd") {
                                format_usd(cat.per_transaction_cents)
                            } else {
                                cat.per_transaction_cents.to_string()
                            }
                        }
                        "monthly_aggregate_cents" => {
                            if col.format.as_deref() == Some("usd") {
                                format_usd(cat.monthly_aggregate_cents)
                            } else {
                                cat.monthly_aggregate_cents.to_string()
                            }
                        }
                        _ => String::new(),
                    };
                    out.push_str(&format!(" {} |", val));
                }
                out.push('\n');
            }
        }
        _ => {
            out.push_str(&format!("<!-- unknown data source: {} -->\n", source));
        }
    }
}

/// Substitute `{{key}}` placeholders in text with values from the AST or profile.
fn substitute(text: &str, ast: &GovernanceDocAstV2, profile: &GovernanceProfile) -> String {
    let mut result = text.to_owned();

    // AST-derived substitutions
    if result.contains("{{spending_defaults.per_vendor_annual_cap}}") {
        result = result.replace(
            "{{spending_defaults.per_vendor_annual_cap}}",
            &format_usd(ast.spending_defaults.per_vendor_annual_cap_cents),
        );
    }

    // Profile-derived substitutions
    if result.contains("{{effective_date}}") {
        result = result.replace("{{effective_date}}", &profile.effective_date().to_string());
    }
    if result.contains("{{legal_name}}") {
        result = result.replace("{{legal_name}}", profile.legal_name());
    }
    if result.contains("{{entity_legal_name}}") {
        result = result.replace("{{entity_legal_name}}", profile.legal_name());
    }
    if result.contains("{{adopted_by}}") {
        result = result.replace("{{adopted_by}}", profile.adopted_by());
    }

    result
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
    fn delegation_schedule_renders_from_ast() {
        let ast = super::super::doc_ast::default_doc_ast();
        let entity = make_entity(EntityType::Corporation);
        let profile = GovernanceProfile::default_for_entity(&entity);
        let doc = ast
            .documents
            .iter()
            .find(|d| d.id == "agent_delegation_schedule")
            .expect("delegation schedule");
        let rendered = render_document_from_ast(
            doc,
            ast,
            super::super::doc_ast::EntityTypeKey::Corporation,
            &profile,
        );
        // Key spending amounts from AST
        assert!(rendered.contains("$10,000"), "should contain $10,000");
        assert!(rendered.contains("$7,500"), "should contain $7,500");
        assert!(rendered.contains("$5,000"), "should contain $5,000");
        assert!(rendered.contains("$2,500"), "should contain $2,500");
        assert!(rendered.contains("$500"), "should contain $500");
        assert!(rendered.contains("$50,000"), "should contain per-vendor cap $50,000");
        assert!(rendered.contains("# Agent Delegation Schedule"));
        assert!(rendered.contains("Authority precedence"));
    }

    #[test]
    fn signing_standard_renders_from_ast() {
        let ast = super::super::doc_ast::default_doc_ast();
        let entity = make_entity(EntityType::Corporation);
        let profile = GovernanceProfile::default_for_entity(&entity);
        let doc = ast
            .documents
            .iter()
            .find(|d| d.id == "signing_and_records_standard")
            .expect("signing standard");
        let rendered = render_document_from_ast(
            doc,
            ast,
            super::super::doc_ast::EntityTypeKey::Corporation,
            &profile,
        );
        assert!(rendered.contains("# Signing and Records Standard"));
        assert!(rendered.contains("Hash-chain integrity"));
        assert!(rendered.contains("Incident report format"));
        assert!(rendered.contains("SHA-256"));
    }

    #[test]
    fn format_usd_basic() {
        assert_eq!(format_usd(1_000_000), "$10,000");
        assert_eq!(format_usd(750_000), "$7,500");
        assert_eq!(format_usd(50_000), "$500");
        assert_eq!(format_usd(5_000_000), "$50,000");
        assert_eq!(format_usd(100), "$1");
        assert_eq!(format_usd(0), "$0");
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
