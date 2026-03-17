//! Governance markdown document generator and bundle metadata.

use anyhow::Context;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use super::doc_ast::{
    ContentNode, DocumentCategory, DocumentDefinition, EntityTypeKey, GovernanceDocAst,
};
use super::profile::{CompanyAddress, FiscalYearEnd, GovernanceProfile};
use crate::domain::ids::{EntityId, GovernanceDocBundleId};

pub const GOVERNANCE_DOC_BUNDLES_ROOT: &str = "governance/doc-bundles";
pub const GOVERNANCE_DOC_BUNDLES_CURRENT_PATH: &str = "governance/doc-bundles/current.json";
pub const GOVERNANCE_DOC_BUNDLES_HISTORY_DIR: &str = "governance/doc-bundles/history";
pub const GOVERNANCE_DOC_AST_SOURCE_PATH: &str = "governance/ast/governance-ast.json";

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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GeneratedGovernanceDocument {
    pub path: String,
    pub source_path: String,
    pub sha256: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GovernanceDocManifest {
    pub version: u32,
    pub entity_type: String,
    pub generated_at: String,
    pub source_root: String,
    pub documents: Vec<GeneratedGovernanceDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GovernanceDocBundleCurrent {
    pub bundle_id: GovernanceDocBundleId,
    pub entity_id: EntityId,
    pub manifest_path: String,
    pub generated_at: String,
    pub template_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
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

pub fn relative_document_paths(entity_type: GovernanceDocEntityType) -> Vec<String> {
    let ast = super::doc_ast::default_doc_ast();
    let key = match entity_type {
        GovernanceDocEntityType::Corporation => EntityTypeKey::Corporation,
        GovernanceDocEntityType::Llc => EntityTypeKey::Llc,
    };
    ast.documents
        .iter()
        .filter(|d| include_in_production_bundle(d))
        .filter(|d| d.entity_scope.matches(key))
        .map(|d| d.path.clone())
        .collect()
}

/// Export canonical governance templates from the compiled AST.
pub fn generate_bundle(
    entity_type: GovernanceDocEntityType,
    out_dir: &Path,
) -> anyhow::Result<GovernanceDocManifest> {
    generate_bundle_from_repo_root(entity_type, Path::new("."), out_dir)
}

/// Backward-compatible entry point. `repo_root` is ignored because the AST is
/// the sole source of truth for canonical document content.
pub fn generate_bundle_from_repo_root(
    entity_type: GovernanceDocEntityType,
    _repo_root: &Path,
    out_dir: &Path,
) -> anyhow::Result<GovernanceDocManifest> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("create output dir {}", out_dir.to_string_lossy()))?;

    let ast = super::doc_ast::default_doc_ast();
    let entity_key = match entity_type {
        GovernanceDocEntityType::Corporation => EntityTypeKey::Corporation,
        GovernanceDocEntityType::Llc => EntityTypeKey::Llc,
    };
    let mut generated = Vec::new();
    for doc in ast
        .documents
        .iter()
        .filter(|doc| include_in_production_bundle(doc))
        .filter(|doc| doc.entity_scope.matches(entity_key))
    {
        let target = out_dir.join(&doc.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create output parent {}", parent.to_string_lossy()))?;
        }
        let bytes = render_document_template_from_ast(doc, ast, entity_key).into_bytes();
        fs::write(&target, &bytes)
            .with_context(|| format!("write target document {}", target.to_string_lossy()))?;
        generated.push(GeneratedGovernanceDocument {
            path: doc.path.clone(),
            source_path: governance_ast_source_path(doc),
            sha256: sha256_hex(&bytes),
            bytes: bytes.len(),
        });
    }

    generated.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = GovernanceDocManifest {
        version: 2,
        entity_type: entity_type.as_str().to_owned(),
        generated_at: Utc::now().to_rfc3339(),
        source_root: GOVERNANCE_DOC_AST_SOURCE_PATH.to_owned(),
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
    render_bundle_from_profile_with_repo_root(
        entity_type,
        entity_id,
        profile,
        template_version,
        Path::new("."),
    )
}

pub fn render_bundle_from_profile_with_repo_root(
    entity_type: GovernanceDocEntityType,
    entity_id: EntityId,
    profile: &GovernanceProfile,
    template_version: &str,
    _repo_root: &Path,
) -> anyhow::Result<RenderedGovernanceBundle> {
    let ast = super::doc_ast::default_doc_ast();
    let entity_key = match entity_type {
        GovernanceDocEntityType::Corporation => EntityTypeKey::Corporation,
        GovernanceDocEntityType::Llc => EntityTypeKey::Llc,
    };
    let mut rendered_docs = Vec::new();
    for doc in ast
        .documents
        .iter()
        .filter(|doc| include_in_production_bundle(doc))
        .filter(|doc| doc.entity_scope.matches(entity_key))
    {
        let rendered = render_document_from_ast(doc, ast, entity_key, profile);
        let content = rendered.into_bytes();
        rendered_docs.push(RenderedGovernanceDocument {
            path: doc.path.clone(),
            source_path: governance_ast_source_path(doc),
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
        source_root: GOVERNANCE_DOC_AST_SOURCE_PATH.to_owned(),
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

fn include_in_production_bundle(doc: &super::doc_ast::DocumentDefinition) -> bool {
    !doc.id.ends_with("_template") && doc.category != DocumentCategory::Transactions
}

fn governance_ast_source_path(doc: &DocumentDefinition) -> String {
    format!("{GOVERNANCE_DOC_AST_SOURCE_PATH}#{}", doc.id)
}

/// Placeholder patterns that indicate unfinished document generation.
const PLACEHOLDER_PATTERNS: &[&str] = &["`TBD`", "`TBD ", "TBD`", "{{"];

fn missing_field_marker(key: &str) -> String {
    format!("[MISSING: {key}]")
}

fn template_field_marker(key: &str) -> String {
    format!("{{{{{key}}}}}")
}

fn looks_like_legacy_placeholder(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.contains("TBD") || trimmed.contains("YYYY-MM-DD")
}

fn extract_missing_field_keys(content: &str) -> Vec<String> {
    let mut keys = std::collections::BTreeSet::new();
    let mut remainder = content;
    while let Some(start) = remainder.find("[MISSING:") {
        let after = &remainder[start + "[MISSING:".len()..];
        let Some(end) = after.find(']') else {
            break;
        };
        let key = after[..end].trim();
        if !key.is_empty() {
            keys.insert(key.to_owned());
        }
        remainder = &after[end + 1..];
    }
    keys.into_iter().collect()
}

pub fn detect_placeholder_warnings_for_text(path: &str, content: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let missing_fields = extract_missing_field_keys(content);
    if !missing_fields.is_empty() {
        warnings.push(format!(
            "{path}: missing required fields: {}",
            missing_fields.join(", ")
        ));
    }
    for pattern in PLACEHOLDER_PATTERNS {
        let count = content.matches(pattern).count();
        if count > 0 {
            warnings.push(format!(
                "{path}: {count} occurrence(s) of placeholder \"{pattern}\""
            ));
        }
    }
    warnings
}

/// Scan rendered documents for residual placeholder markers.
fn detect_placeholders(docs: &[RenderedGovernanceDocument]) -> Vec<String> {
    let mut warnings = Vec::new();
    for doc in docs {
        let content = match std::str::from_utf8(&doc.content) {
            Ok(s) => s,
            Err(_) => continue,
        };
        warnings.extend(detect_placeholder_warnings_for_text(&doc.path, content));
    }
    warnings
}

fn substitute_template(text: &str, ast: &GovernanceDocAst) -> String {
    let mut result = text.to_owned();
    if result.contains("{{spending_defaults.per_vendor_annual_cap}}") {
        result = result.replace(
            "{{spending_defaults.per_vendor_annual_cap}}",
            &format_usd(ast.spending_defaults.per_vendor_annual_cap_cents),
        );
    }
    result
}

fn render_template_node(
    out: &mut String,
    node: &ContentNode,
    ast: &GovernanceDocAst,
    entity_type: EntityTypeKey,
) {
    match node {
        ContentNode::Heading { level, text } => {
            out.push('\n');
            for _ in 0..*level {
                out.push('#');
            }
            out.push(' ');
            out.push_str(&substitute_template(text, ast));
            out.push('\n');
        }
        ContentNode::Paragraph { text } => {
            out.push('\n');
            out.push_str(&substitute_template(text, ast));
            out.push('\n');
        }
        ContentNode::OrderedList { items } => {
            out.push('\n');
            for (i, item) in items.iter().enumerate() {
                out.push_str(&format!("{}. {}\n", i + 1, substitute_template(item, ast)));
            }
        }
        ContentNode::UnorderedList { items } => {
            out.push('\n');
            for item in items {
                out.push_str(&format!("- {}\n", substitute_template(item, ast)));
            }
        }
        ContentNode::Table { headers, rows } => {
            out.push('\n');
            out.push('|');
            for header in headers {
                out.push_str(&format!(" {} |", header));
            }
            out.push('\n');
            out.push('|');
            for _ in headers {
                out.push_str("---|");
            }
            out.push('\n');
            for row in rows {
                out.push('|');
                for cell in row {
                    out.push_str(&format!(" {} |", substitute_template(cell, ast)));
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
                    render_template_node(out, child, ast, entity_type);
                }
            }
        }
        ContentNode::SignatureBlock { role, fields } => {
            out.push('\n');
            out.push_str(&format!("{role}: ____________________"));
            for field in fields {
                match field.as_str() {
                    "date" => out.push_str("\nDate: ____________________"),
                    "name" => out.push_str("\nName: ____________________"),
                    "title" => out.push_str("\nTitle: ____________________"),
                    _ => out.push_str(&format!("\n{field}: ____________________")),
                }
            }
            out.push('\n');
        }
        ContentNode::Placeholder { key, label } => {
            out.push_str(&format!("**{label}**: {}\n", template_field_marker(key)));
        }
        ContentNode::Note { text } => {
            out.push('\n');
            out.push_str(&format!("> **Note:** {}\n", substitute_template(text, ast)));
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
            out.push_str(&substitute_template(text, ast));
        }
        ContentNode::HorizontalRule => {
            out.push_str("\n---\n");
        }
    }
}

pub fn render_document_template_from_ast(
    doc: &DocumentDefinition,
    ast: &GovernanceDocAst,
    entity_type: EntityTypeKey,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n", doc.title));

    if let Some(preamble) = &doc.preamble {
        out.push('\n');
        out.push_str(&format!("> {}\n", substitute_template(preamble, ast)));
    }

    if !doc.metadata_fields.is_empty() {
        out.push('\n');
        for field in &doc.metadata_fields {
            let value = field
                .default
                .as_deref()
                .filter(|value| !looks_like_legacy_placeholder(value))
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| template_field_marker(&field.key));
            out.push_str(&format!("**{}**: `{}`\n", field.label, value));
        }
    }

    for node in &doc.content {
        render_template_node(&mut out, node, ast, entity_type);
    }

    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

// ── AST-based rendering ──────────────────────────────────────────────

/// Format cents as a USD string (e.g. 1000000 → "$10,000").
pub fn format_usd(cents: i64) -> String {
    let negative = cents < 0;
    let abs = cents.unsigned_abs();
    let dollars = abs / 100;
    let remainder = abs % 100;
    let s = dollars.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    if remainder == 0 {
        if negative {
            format!("-${result}")
        } else {
            format!("${result}")
        }
    } else if negative {
        format!("-${result}.{remainder:02}")
    } else {
        format!("${result}.{remainder:02}")
    }
}

/// Render a single document definition from the v2 AST to markdown.
pub fn render_document_from_ast(
    doc: &DocumentDefinition,
    ast: &GovernanceDocAst,
    entity_type: EntityTypeKey,
    profile: &GovernanceProfile,
) -> String {
    render_document_from_ast_with_context(doc, ast, entity_type, profile, &Value::Null)
}

pub fn render_document_from_ast_with_context(
    doc: &DocumentDefinition,
    ast: &GovernanceDocAst,
    entity_type: EntityTypeKey,
    profile: &GovernanceProfile,
    context: &Value,
) -> String {
    let mut out = String::new();

    // Title
    out.push_str(&format!("# {}\n", doc.title));

    // Preamble (as blockquote)
    if let Some(preamble) = &doc.preamble {
        out.push('\n');
        let rendered_preamble = substitute(preamble, ast, profile, context);
        out.push_str(&format!("> {rendered_preamble}\n"));
    }

    // Metadata fields
    if !doc.metadata_fields.is_empty() {
        out.push('\n');
        for field in &doc.metadata_fields {
            let value = resolve_context_field(&field.key, profile, context)
                .or_else(|| field.default.clone())
                .unwrap_or_else(|| missing_field_marker(&field.key));
            out.push_str(&format!("**{}**: `{}`\n", field.label, value));
        }
    }

    // Content nodes
    for node in &doc.content {
        render_node(&mut out, node, ast, entity_type, profile, context);
    }

    out
}

pub(super) fn resolve_profile_field(key: &str, profile: &GovernanceProfile) -> Option<String> {
    match key {
        "effective_date" => Some(profile.effective_date().to_string()),
        "adopted_by" => Some(profile.adopted_by().to_owned()),
        "last_reviewed" => Some(profile.last_reviewed().to_string()),
        "next_mandatory_review" => Some(profile.next_mandatory_review().to_string()),
        "legal_name" => Some(profile.legal_name().to_owned()),
        "company_address" => profile.company_address().map(format_company_address),
        "registered_agent_name" => profile.registered_agent_name().map(ToOwned::to_owned),
        "registered_agent_address" => profile.registered_agent_address().map(ToOwned::to_owned),
        "board_size" => profile.board_size().map(|n| n.to_string()),
        "incorporator_name" => profile.incorporator_name().map(ToOwned::to_owned),
        "incorporator_address" => profile.incorporator_address().map(ToOwned::to_owned),
        "principal_name" => profile.principal_name().map(ToOwned::to_owned),
        "fiscal_year_end" => profile.fiscal_year_end().map(format_fiscal_year_end),
        "jurisdiction" => Some(profile.jurisdiction().to_owned()),
        _ => None,
    }
}

fn resolve_json_path<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in key.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn json_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => Some(format_json_value(other)),
    }
}

pub(super) fn resolve_context_field(
    key: &str,
    profile: &GovernanceProfile,
    context: &Value,
) -> Option<String> {
    resolve_profile_field(key, profile)
        .or_else(|| resolve_json_path(context, key).and_then(json_value_to_string))
        .or_else(|| {
            resolve_json_path(context, &format!("fields.{key}")).and_then(json_value_to_string)
        })
}

fn render_node(
    out: &mut String,
    node: &ContentNode,
    ast: &GovernanceDocAst,
    entity_type: EntityTypeKey,
    profile: &GovernanceProfile,
    context: &Value,
) {
    match node {
        ContentNode::Heading { level, text } => {
            out.push('\n');
            for _ in 0..*level {
                out.push('#');
            }
            out.push(' ');
            out.push_str(&substitute(text, ast, profile, context));
            out.push('\n');
        }
        ContentNode::Paragraph { text } => {
            out.push('\n');
            out.push_str(&substitute(text, ast, profile, context));
            out.push('\n');
        }
        ContentNode::OrderedList { items } => {
            out.push('\n');
            for (i, item) in items.iter().enumerate() {
                out.push_str(&format!(
                    "{}. {}\n",
                    i + 1,
                    substitute(item, ast, profile, context)
                ));
            }
        }
        ContentNode::UnorderedList { items } => {
            out.push('\n');
            for item in items {
                out.push_str(&format!("- {}\n", substitute(item, ast, profile, context)));
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
                    out.push_str(&format!(" {} |", substitute(cell, ast, profile, context)));
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
                    render_node(out, child, ast, entity_type, profile, context);
                }
            }
        }
        ContentNode::SignatureBlock { role, fields } => {
            out.push('\n');
            out.push_str(&format!("{role}: ____________________"));
            for field in fields {
                match field.as_str() {
                    "date" => out.push_str("\nDate: ____________________"),
                    "name" => out.push_str("\nName: ____________________"),
                    "title" => out.push_str("\nTitle: ____________________"),
                    _ => out.push_str(&format!("\n{field}: ____________________")),
                }
            }
            out.push('\n');
        }
        ContentNode::Placeholder { key, label } => {
            let value = resolve_context_field(key, profile, context)
                .unwrap_or_else(|| missing_field_marker(key));
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
            out.push_str(&substitute(text, ast, profile, context));
        }
        ContentNode::HorizontalRule => {
            out.push_str("\n---\n");
        }
    }
}

pub(super) fn format_company_address(addr: &CompanyAddress) -> String {
    let mut parts = vec![addr.street.clone(), addr.city.clone()];
    if let Some(county) = &addr.county {
        parts.push(county.clone());
    }
    parts.push(addr.state.clone());
    parts.push(addr.zip.clone());
    parts.join(", ")
}

pub(super) fn format_fiscal_year_end(fy: &FiscalYearEnd) -> String {
    let month_name = match fy.month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };
    format!("{} {}", month_name, fy.day)
}

pub(super) fn format_par_value(units: u64) -> String {
    if units == 0 {
        return "0".to_owned();
    }
    let dollars = units / 10_000;
    let remainder = units % 10_000;
    if remainder == 0 {
        format!("{dollars}")
    } else {
        // Stored as ten-thousandths of a dollar so startup-standard par values
        // like $0.0001 round-trip without collapsing to a cent.
        let value = units as f64 / 10_000.0;
        let s = format!("{value:.4}");
        s.trim_end_matches('0').to_owned()
    }
}

fn par_value_units_to_total_cents(shares: u64, par_value_units: u64) -> i64 {
    ((shares as u128 * par_value_units as u128) / 100) as i64
}

pub(super) fn format_number_with_commas(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

pub(super) fn format_json_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "N/A".to_owned(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| item.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        serde_json::Value::Object(obj) => obj
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                format!("{}: {}", k, val)
            })
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

pub(super) fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn render_data_table(
    out: &mut String,
    source: &str,
    columns: &[super::doc_ast::DataTableColumn],
    ast: &GovernanceDocAst,
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
        "structured_data.autonomy_lanes" => {
            if let Some(sd) = &ast.structured_data {
                for lane in &sd.autonomy_lanes {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => lane.label.clone(),
                            "conditions_text" => {
                                if lane.conditions.is_empty() {
                                    "None".to_owned()
                                } else {
                                    lane.conditions
                                        .iter()
                                        .filter_map(|c| c.get("label").and_then(|l| l.as_str()))
                                        .collect::<Vec<_>>()
                                        .join("; ")
                                }
                            }
                            "authority_text" => lane.authority_action.clone(),
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        "structured_data.approval_validity" => {
            if let Some(sd) = &ast.structured_data {
                if let Some(av) = &sd.approval_validity {
                    for req in &av.required_elements {
                        out.push('|');
                        for col in columns {
                            let val = match col.key.as_str() {
                                "label" => req.label.clone(),
                                "description" => req.rule.clone(),
                                _ => String::new(),
                            };
                            out.push_str(&format!(" {} |", val));
                        }
                        out.push('\n');
                    }
                }
            }
        }
        "structured_data.credential_custody" => {
            if let Some(sd) = &ast.structured_data {
                for cred in &sd.credential_custody {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => cred.label.clone(),
                            "custodian" => cred.custodian.clone(),
                            "agent_access" => cred.agent_access.clone(),
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        "structured_data.emergency_modes" => {
            if let Some(sd) = &ast.structured_data {
                for mode in &sd.emergency_modes {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => mode.label.clone(),
                            "description" => {
                                let mut parts = Vec::new();
                                if mode.tier1_allowed {
                                    parts.push("Tier 1 allowed");
                                }
                                if mode.tier2_allowed {
                                    parts.push("Tier 2 allowed");
                                }
                                if mode.reversible_only {
                                    parts.push("Reversible only");
                                }
                                if parts.is_empty() {
                                    "All actions suspended".to_owned()
                                } else {
                                    parts.join(", ")
                                }
                            }
                            "activated_by_text" => format_json_value(&mode.activated_by),
                            "deactivated_by_text" => format_json_value(&mode.deactivated_by),
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        "structured_data.auto_suspension_triggers" => {
            if let Some(sd) = &ast.structured_data {
                for trigger in &sd.auto_suspension_triggers {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => trigger.label.clone(),
                            "description" => trigger.description.clone(),
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        "structured_data.report_schedule" => {
            if let Some(sd) = &ast.structured_data {
                for report in &sd.report_schedule {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => report.label.clone(),
                            "frequency" => report.frequency.clone(),
                            "content_summary" => report.content_keys.join(", "),
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        "structured_data.adjustment_rules" => {
            if let Some(sd) = &ast.structured_data {
                for rule in &sd.adjustment_rules {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "description" => {
                                format!(
                                    "{} {}",
                                    capitalize(&rule.action),
                                    rule.target.replace('_', " ")
                                )
                            }
                            "permitted_text" => {
                                if rule.requires_board_resolution {
                                    "No, requires Board/Member resolution".to_owned()
                                } else {
                                    "Yes".to_owned()
                                }
                            }
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        "structured_data.change_control_rules" => {
            if let Some(sd) = &ast.structured_data {
                for rule in &sd.change_control_rules {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => rule.label.clone(),
                            "tier" => format!("Tier {}", rule.tier),
                            "notes" => {
                                let mut notes = Vec::new();
                                if rule.requires_impact_assessment {
                                    notes.push("Requires impact assessment");
                                }
                                if rule.requires_governance_amendment {
                                    notes.push("Requires governance amendment");
                                }
                                notes.join("; ")
                            }
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        "structured_data.retention_schedule" => {
            if let Some(sd) = &ast.structured_data {
                for record in &sd.retention_schedule {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => record.label.clone(),
                            "retention_text" => {
                                if record.permanent {
                                    "Permanent".to_owned()
                                } else if let Some(years) = record.retention_years {
                                    format!("{years} years")
                                } else {
                                    "N/A".to_owned()
                                }
                            }
                            "governing_requirement" => {
                                record.governing_requirement.clone().unwrap_or_default()
                            }
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        "structured_data.severity_classification" => {
            if let Some(sd) = &ast.structured_data {
                for level in &sd.severity_classification {
                    out.push('|');
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => level.label.clone(),
                            "response_sla_text" => {
                                if let Some(hours) = level.response_sla_hours {
                                    if hours == 0 {
                                        "Immediate".to_owned()
                                    } else {
                                        format!("{hours} hours")
                                    }
                                } else {
                                    "N/A".to_owned()
                                }
                            }
                            "auto_lockdown_text" => {
                                if level.auto_lockdown { "Yes" } else { "No" }.to_owned()
                            }
                            _ => String::new(),
                        };
                        out.push_str(&format!(" {} |", val));
                    }
                    out.push('\n');
                }
            }
        }
        _ => {
            out.push_str(&format!("<!-- unknown data source: {} -->\n", source));
        }
    }
}

/// Substitute `{{key}}` placeholders in text with values from the AST or profile.
pub(super) fn substitute(
    text: &str,
    ast: &GovernanceDocAst,
    profile: &GovernanceProfile,
    context: &Value,
) -> String {
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

    // Company address
    if result.contains("{{company_address}}") {
        let val = profile
            .company_address()
            .map(format_company_address)
            .unwrap_or_else(|| missing_field_marker("company_address"));
        result = result.replace("{{company_address}}", &val);
    }

    // Registered agent
    if result.contains("{{registered_agent_name}}") {
        result = result.replace(
            "{{registered_agent_name}}",
            profile
                .registered_agent_name()
                .unwrap_or("[MISSING: registered_agent_name]"),
        );
    }
    if result.contains("{{registered_agent_address}}") {
        result = result.replace(
            "{{registered_agent_address}}",
            profile
                .registered_agent_address()
                .unwrap_or("[MISSING: registered_agent_address]"),
        );
    }

    // Founders
    if result.contains("{{founders_list}}") {
        let val = if profile.founders().is_empty() {
            missing_field_marker("founders_list")
        } else {
            profile
                .founders()
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        result = result.replace("{{founders_list}}", &val);
    }

    // Fiscal year end
    if result.contains("{{fiscal_year_end}}") {
        let val = profile
            .fiscal_year_end()
            .map(format_fiscal_year_end)
            .unwrap_or_else(|| missing_field_marker("fiscal_year_end"));
        result = result.replace("{{fiscal_year_end}}", &val);
    }

    // Board
    if result.contains("{{board_size}}") {
        let val = profile
            .board_size()
            .map(|n| n.to_string())
            .unwrap_or_else(|| missing_field_marker("board_size"));
        result = result.replace("{{board_size}}", &val);
    }
    if result.contains("{{directors_list}}") {
        let val = if profile.directors().is_empty() {
            missing_field_marker("directors_list")
        } else {
            profile
                .directors()
                .iter()
                .map(|d| d.name.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        };
        result = result.replace("{{directors_list}}", &val);
    }
    if result.contains("{{officers_list}}") {
        let val = if profile.officers().is_empty() {
            missing_field_marker("officers_list")
        } else {
            profile
                .officers()
                .iter()
                .map(|o| format!("{}: {}", o.title, o.name))
                .collect::<Vec<_>>()
                .join("\n")
        };
        result = result.replace("{{officers_list}}", &val);
    }

    // Stock details
    if result.contains("{{authorized_shares}}") {
        let val = profile
            .stock_details()
            .map(|s| format_number_with_commas(s.authorized_shares))
            .unwrap_or_else(|| missing_field_marker("authorized_shares"));
        result = result.replace("{{authorized_shares}}", &val);
    }
    if result.contains("{{par_value}}") {
        let val = profile
            .stock_details()
            .map(|s| format_par_value(s.par_value_cents))
            .unwrap_or_else(|| missing_field_marker("par_value"));
        result = result.replace("{{par_value}}", &val);
    }

    // Incorporator
    if result.contains("{{incorporator_name}}") {
        result = result.replace(
            "{{incorporator_name}}",
            profile
                .incorporator_name()
                .unwrap_or("[MISSING: incorporator_name]"),
        );
    }
    if result.contains("{{incorporator_address}}") {
        result = result.replace(
            "{{incorporator_address}}",
            profile
                .incorporator_address()
                .unwrap_or("[MISSING: incorporator_address]"),
        );
    }

    // Principal
    if result.contains("{{principal_name}}") {
        result = result.replace(
            "{{principal_name}}",
            profile
                .principal_name()
                .unwrap_or("[MISSING: principal_name]"),
        );
    }

    // Provider legal name (hardcoded for now)
    if result.contains("{{provider_legal_name}}") {
        result = result.replace("{{provider_legal_name}}", "The Corporation, Inc.");
    }

    // Jurisdiction
    if result.contains("{{jurisdiction}}") {
        result = result.replace("{{jurisdiction}}", profile.jurisdiction());
    }

    // Founders stock table
    if result.contains("{{founders_stock_table}}") {
        let val = if profile.founders().is_empty() {
            missing_field_marker("founders_stock_table")
        } else {
            let mut table = String::from("| Name | Shares |\n|---|---|\n");
            for f in profile.founders() {
                let shares = f
                    .shares
                    .map(|s| format_number_with_commas(s))
                    .unwrap_or_else(|| missing_field_marker("founder.shares"));
                table.push_str(&format!("| {} | {} |\n", f.name, shares));
            }
            table
        };
        result = result.replace("{{founders_stock_table}}", &val);
    }

    // Founders table (membership %)
    if result.contains("{{founders_table}}") {
        let val = if profile.founders().is_empty() {
            missing_field_marker("founders_table")
        } else {
            let total: u64 = profile.founders().iter().filter_map(|f| f.shares).sum();
            let mut table = String::from("| Name | Membership % |\n|---|---|\n");
            for f in profile.founders() {
                let pct = if total > 0 {
                    f.shares
                        .map(|s| format!("{:.1}%", (s as f64 / total as f64) * 100.0))
                        .unwrap_or_else(|| missing_field_marker("founder.membership_pct"))
                } else {
                    missing_field_marker("founder.membership_pct")
                };
                table.push_str(&format!("| {} | {} |\n", f.name, pct));
            }
            table
        };
        result = result.replace("{{founders_table}}", &val);
    }

    // RSPA / CIIA context — first founder
    let first_founder = profile.founders().first();
    if result.contains("{{purchaser_name}}") {
        let val = first_founder
            .map(|f| f.name.as_str())
            .unwrap_or("[MISSING: purchaser_name]");
        result = result.replace("{{purchaser_name}}", val);
    }
    if result.contains("{{purchase_shares}}") {
        let val = first_founder
            .and_then(|f| f.shares)
            .map(|s| format_number_with_commas(s))
            .unwrap_or_else(|| missing_field_marker("purchase_shares"));
        result = result.replace("{{purchase_shares}}", &val);
    }
    if result.contains("{{total_purchase_price}}") {
        let val = first_founder
            .and_then(|f| f.shares)
            .and_then(|shares| {
                profile.stock_details().map(|sd| {
                    format_usd(par_value_units_to_total_cents(shares, sd.par_value_cents))
                })
            })
            .unwrap_or_else(|| missing_field_marker("total_purchase_price"));
        result = result.replace("{{total_purchase_price}}", &val);
    }
    if result.contains("{{vesting_months}}") {
        let val = first_founder
            .and_then(|f| f.vesting.as_ref())
            .map(|v| v.total_months.to_string())
            .unwrap_or_else(|| "48".to_owned());
        result = result.replace("{{vesting_months}}", &val);
    }
    if result.contains("{{cliff_months}}") {
        let val = first_founder
            .and_then(|f| f.vesting.as_ref())
            .map(|v| v.cliff_months.to_string())
            .unwrap_or_else(|| "12".to_owned());
        result = result.replace("{{cliff_months}}", &val);
    }
    if result.contains("{{ip_description}}") {
        let val = first_founder
            .and_then(|f| f.ip_contribution.as_deref())
            .unwrap_or("[MISSING: ip_description]");
        result = result.replace("{{ip_description}}", val);
    }
    if result.contains("{{assignor_name}}") {
        let val = first_founder
            .map(|f| f.name.as_str())
            .unwrap_or("[MISSING: assignor_name]");
        result = result.replace("{{assignor_name}}", val);
    }

    while let Some(start) = result.find("{{") {
        let Some(end_rel) = result[start + 2..].find("}}") else {
            break;
        };
        let end = start + 2 + end_rel;
        let key = result[start + 2..end].trim().to_owned();
        let replacement = resolve_context_field(&key, profile, context)
            .unwrap_or_else(|| missing_field_marker(&key));
        result.replace_range(start..end + 2, &replacement);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::formation::{
        entity::Entity,
        types::{EntityType, Jurisdiction},
    };
    use crate::domain::ids::WorkspaceId;
    use std::fs;
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
    fn path_set_contains_expected_docs_for_corp() {
        let docs = relative_document_paths(GovernanceDocEntityType::Corporation);
        let has = |s: &str| docs.iter().any(|d| d == s);
        assert!(has("common/agent-delegation-schedule.md"));
        assert!(has("corporation/bylaws.md"));
        assert!(has("corporation/certificate-of-incorporation.md"));
        assert!(!has("common/agent-operator-service-agreement-template.md"));
        assert!(!has("transactions/board-consent.md"));
        assert!(!has("llc/operating-agreement.md"));
        assert!(!has("llc/articles-of-organization.md"));
    }

    #[test]
    fn can_generate_bundle_from_repo_root() {
        let repo_root = TempDir::new().expect("temp repo root");
        let out = TempDir::new().expect("temp dir");
        let manifest = generate_bundle_from_repo_root(
            GovernanceDocEntityType::Corporation,
            repo_root.path(),
            out.path(),
        )
        .expect("generate bundle");
        assert!(!manifest.documents.is_empty());
        assert_eq!(manifest.version, 2);
        assert_eq!(manifest.source_root, GOVERNANCE_DOC_AST_SOURCE_PATH);
        assert!(out.path().join("manifest.json").is_file());
        assert!(out.path().join("corporation/bylaws.md").is_file());
        assert!(!out.path().join("transactions/board-consent.md").exists());
        let bylaws = fs::read_to_string(out.path().join("corporation/bylaws.md")).expect("bylaws");
        assert!(bylaws.contains("{{legal_name}}"));
        assert!(!bylaws.contains("`TBD`"));
    }

    #[test]
    fn delegation_schedule_renders_from_ast() {
        let ast = super::super::doc_ast::default_doc_ast();
        let entity = make_entity(EntityType::CCorp);
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
        assert!(
            rendered.contains("$50,000"),
            "should contain per-vendor cap $50,000"
        );
        assert!(rendered.contains("# Agent Delegation Schedule"));
        assert!(rendered.contains("Authority precedence"));
    }

    #[test]
    fn signing_standard_renders_from_ast() {
        let ast = super::super::doc_ast::default_doc_ast();
        let entity = make_entity(EntityType::CCorp);
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
        let repo_root = TempDir::new().expect("temp repo root");
        let entity = make_entity(EntityType::CCorp);
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
            repo_root.path(),
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
        assert_eq!(bundle.manifest.source_root, GOVERNANCE_DOC_AST_SOURCE_PATH);
        assert!(
            bylaws.source_path.ends_with("#bylaws"),
            "unexpected source path: {}",
            bylaws.source_path
        );
        assert!(
            !bundle
                .documents
                .iter()
                .any(|d| d.path == "transactions/board-consent.md")
        );
        assert!(
            !bundle
                .documents
                .iter()
                .any(|d| d.path == "common/agent-operator-service-agreement-template.md")
        );
    }

    fn make_complete_profile() -> GovernanceProfile {
        let entity = make_entity(EntityType::CCorp);
        let mut profile = GovernanceProfile::default_for_entity(&entity);
        profile.update(
            "Acme Holdings, Inc.".to_owned(),
            "Delaware".to_owned(),
            profile.effective_date(),
            "Board of Directors".to_owned(),
            profile.last_reviewed(),
            profile.next_mandatory_review(),
            Some("Delaware Registered Agent Co.".to_owned()),
            Some("1209 Orange St, Wilmington, DE 19801".to_owned()),
            Some(1),
            Some("Alice Founder".to_owned()),
            Some("123 Main St, San Francisco, CA 94105".to_owned()),
            Some("Alice Founder".to_owned()),
            Some("CEO".to_owned()),
            Some(false),
        );
        profile.set_company_address(super::super::profile::CompanyAddress {
            street: "123 Main St".to_owned(),
            city: "San Francisco".to_owned(),
            county: None,
            state: "CA".to_owned(),
            zip: "94105".to_owned(),
        });
        profile.set_founders(vec![super::super::profile::FounderInfo {
            name: "Alice Founder".to_owned(),
            shares: Some(8_000_000),
            vesting: Some(super::super::profile::VestingSchedule {
                total_months: 48,
                cliff_months: 12,
                acceleration_on_termination: false,
            }),
            ip_contribution: Some("Initial software platform".to_owned()),
            email: Some("alice@acme.com".to_owned()),
            address: None,
        }]);
        profile.set_directors(vec![super::super::profile::DirectorInfo {
            name: "Alice Founder".to_owned(),
            address: None,
        }]);
        profile.set_officers(vec![super::super::profile::OfficerInfo {
            name: "Alice Founder".to_owned(),
            title: "CEO".to_owned(),
        }]);
        profile.set_stock_details(super::super::profile::StockDetails {
            authorized_shares: 10_000_000,
            par_value_cents: 1,
            share_class: "Common Stock".to_owned(),
        });
        profile.set_fiscal_year_end(super::super::profile::FiscalYearEnd { month: 12, day: 31 });
        profile
    }

    #[test]
    fn delegation_schedule_no_unknown_sources_or_placeholders() {
        let ast = super::super::doc_ast::default_doc_ast();
        let profile = make_complete_profile();
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
        assert!(
            !rendered.contains("<!-- unknown data source"),
            "found unknown data source marker in delegation schedule:\n{}",
            rendered
        );
        assert!(
            !rendered.contains("{{"),
            "found unresolved placeholder in delegation schedule:\n{}",
            rendered
        );
    }

    #[test]
    fn formation_docs_no_unknown_placeholders() {
        let ast = super::super::doc_ast::default_doc_ast();
        let profile = make_complete_profile();
        let formation_ids = [
            "certificate_of_incorporation",
            "bylaws",
            "incorporator_action",
            "initial_board_consent",
        ];
        for doc_id in &formation_ids {
            let doc = ast.documents.iter().find(|d| d.id == *doc_id);
            if let Some(doc) = doc {
                let rendered = render_document_from_ast(
                    doc,
                    ast,
                    super::super::doc_ast::EntityTypeKey::Corporation,
                    &profile,
                );
                assert!(
                    !rendered.contains("{{"),
                    "found unresolved placeholder in {doc_id}:\n{rendered}"
                );
            }
        }
    }
}
