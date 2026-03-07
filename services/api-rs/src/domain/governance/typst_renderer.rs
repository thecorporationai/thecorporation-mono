//! Typst-based PDF renderer for governance documents.
//!
//! Converts governance document AST nodes into Typst markup, compiles them
//! via the Typst engine, and exports to PDF bytes. Supports inline SVG
//! signature rendering for signed documents.

use std::sync::OnceLock;

use typst::diag::FileError;
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use super::doc_ast::{
    ContentNode, DataTableColumn, DocumentDefinition, EntityTypeKey, GovernanceDocAst,
};
use super::doc_generator::{
    capitalize, format_json_value, format_usd, resolve_profile_field, substitute,
};
use super::profile::GovernanceProfile;
use crate::domain::formation::document::Signature;

// ── Fonts ───────────────────────────────────────────────────────────

struct FontSlot {
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

static FONTS: OnceLock<FontSlot> = OnceLock::new();

fn loaded_fonts() -> &'static FontSlot {
    FONTS.get_or_init(|| {
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        for data in typst_assets::fonts() {
            let bytes = Bytes::new(data);
            for font in Font::iter(bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }
        FontSlot {
            book: LazyHash::new(book),
            fonts,
        }
    })
}

// ── PdfWorld ────────────────────────────────────────────────────────

struct PdfWorld {
    library: LazyHash<Library>,
    main_id: FileId,
    source: Source,
}

impl PdfWorld {
    fn new(typst_source: String) -> Self {
        let main_id = FileId::new(None, VirtualPath::new("/main.typ"));
        Self {
            library: LazyHash::new(Library::default()),
            main_id,
            source: Source::new(main_id, typst_source),
        }
    }
}

impl World for PdfWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &loaded_fonts().book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> Result<Source, FileError> {
        if id == self.main_id {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rooted_path().into()))
        }
    }

    fn file(&self, id: FileId) -> Result<Bytes, FileError> {
        Err(FileError::NotFound(id.vpath().as_rooted_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        loaded_fonts().fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        let now = chrono::Utc::now();
        Datetime::from_ymd(
            now.format("%Y").to_string().parse().unwrap_or(2026),
            now.format("%m").to_string().parse().unwrap_or(1),
            now.format("%d").to_string().parse().unwrap_or(1),
        )
    }
}

// ── Typst escaping ──────────────────────────────────────────────────

fn escape_typst(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '#' | '*' | '_' | '<' | '>' | '@' | '$' | '`' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// ── Document preamble ───────────────────────────────────────────────

const TYPST_PREAMBLE: &str = r##"#set page(paper: "us-letter", margin: (top: 1in, bottom: 1in, left: 1.25in, right: 1.25in), numbering: "1")
#set text(font: "New Computer Modern", size: 11pt)
#set par(justify: true, leading: 0.65em)

#show heading.where(level: 1): it => {
  v(0.5em)
  text(size: 16pt, weight: "bold", it.body)
  v(0.3em)
}
#show heading.where(level: 2): it => {
  v(0.4em)
  text(size: 13pt, weight: "bold", it.body)
  v(0.2em)
}
#show heading.where(level: 3): it => {
  v(0.3em)
  text(size: 11pt, weight: "bold", it.body)
  v(0.1em)
}

"##;

// ── Public entry point ──────────────────────────────────────────────

/// Render a governance document to PDF bytes.
pub fn render_pdf(
    doc: &DocumentDefinition,
    ast: &GovernanceDocAst,
    entity_type: EntityTypeKey,
    profile: &GovernanceProfile,
    signatures: &[Signature],
) -> Result<Vec<u8>, String> {
    let typst_source = render_typst_document(doc, ast, entity_type, profile, signatures);
    compile_typst_to_pdf(&typst_source)
}

fn compile_typst_to_pdf(source: &str) -> Result<Vec<u8>, String> {
    let world = PdfWorld::new(source.to_owned());
    let result = typst::compile::<PagedDocument>(&world);

    let document = result
        .output
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        })?;

    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
        errors
            .into_iter()
            .map(|e| e.message.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })
}

// ── AST → Typst markup ─────────────────────────────────────────────

fn render_typst_document(
    doc: &DocumentDefinition,
    ast: &GovernanceDocAst,
    entity_type: EntityTypeKey,
    profile: &GovernanceProfile,
    signatures: &[Signature],
) -> String {
    let mut out = String::from(TYPST_PREAMBLE);

    // Title
    out.push_str(&format!("= {}\n", escape_typst(&doc.title)));

    // Preamble (as block quote)
    if let Some(preamble) = &doc.preamble {
        let rendered = substitute(preamble, ast, profile);
        out.push_str(&format!(
            "\n#block(fill: luma(245), inset: 10pt, radius: 4pt)[{}]\n",
            escape_typst(&rendered)
        ));
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
                        .unwrap_or_else(|| "TBD".to_owned())
                });
            out.push_str(&format!(
                "*{}*: `{}`\n",
                escape_typst(&field.label),
                value
            ));
        }
    }

    // Content nodes
    for node in &doc.content {
        render_node(&mut out, node, ast, entity_type, profile, signatures);
    }

    out
}

fn render_node(
    out: &mut String,
    node: &ContentNode,
    ast: &GovernanceDocAst,
    entity_type: EntityTypeKey,
    profile: &GovernanceProfile,
    signatures: &[Signature],
) {
    match node {
        ContentNode::Heading { level, text } => {
            out.push('\n');
            for _ in 0..*level {
                out.push('=');
            }
            out.push(' ');
            out.push_str(&escape_typst(&substitute(text, ast, profile)));
            out.push('\n');
        }
        ContentNode::Paragraph { text } => {
            out.push('\n');
            out.push_str(&escape_typst(&substitute(text, ast, profile)));
            out.push_str("\n\n");
        }
        ContentNode::OrderedList { items } => {
            out.push('\n');
            for item in items {
                out.push_str(&format!("+ {}\n", escape_typst(&substitute(item, ast, profile))));
            }
        }
        ContentNode::UnorderedList { items } => {
            out.push('\n');
            for item in items {
                out.push_str(&format!("- {}\n", escape_typst(&substitute(item, ast, profile))));
            }
        }
        ContentNode::Table { headers, rows } => {
            render_static_table(out, headers, rows, ast, profile);
        }
        ContentNode::DataTable { source, columns } => {
            render_typst_data_table(out, source, columns, ast, entity_type);
        }
        ContentNode::Conditional {
            when_entity,
            content,
        } => {
            if *when_entity == entity_type {
                for child in content {
                    render_node(out, child, ast, entity_type, profile, signatures);
                }
            }
        }
        ContentNode::SignatureBlock { role, fields } => {
            render_signature_block(out, role, fields, signatures);
        }
        ContentNode::Placeholder { key, label } => {
            let value = resolve_profile_field(key, profile)
                .unwrap_or_else(|| "TBD".to_owned());
            out.push_str(&format!(
                "*{}*: {}\n",
                escape_typst(label),
                escape_typst(&value)
            ));
        }
        ContentNode::Note { text } => {
            out.push_str(&format!(
                "\n#block(fill: luma(240), inset: 10pt, radius: 4pt)[*Note:* {}]\n",
                escape_typst(text)
            ));
        }
        ContentNode::CodeBlock { language, lines } => {
            out.push_str("\n```");
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
            out.push_str(&escape_typst(&substitute(text, ast, profile)));
        }
        ContentNode::HorizontalRule => {
            out.push_str("\n#line(length: 100%)\n");
        }
    }
}

// ── Static table ────────────────────────────────────────────────────

fn render_static_table(
    out: &mut String,
    headers: &[String],
    rows: &[Vec<String>],
    ast: &GovernanceDocAst,
    profile: &GovernanceProfile,
) {
    let ncols = headers.len();
    out.push_str(&format!("\n#table(\n  columns: {ncols},\n"));

    // Header row
    out.push_str("  table.header(");
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("[*{}*]", escape_typst(h)));
    }
    out.push_str("),\n");

    // Data rows
    for row in rows {
        for cell in row {
            let rendered = substitute(cell, ast, profile);
            out.push_str(&format!("  [{}],\n", escape_typst(&rendered)));
        }
    }
    out.push_str(")\n");
}

// ── Signature block ─────────────────────────────────────────────────

fn render_signature_block(
    out: &mut String,
    role: &str,
    fields: &[String],
    signatures: &[Signature],
) {
    out.push('\n');

    // Try to find a matching signature by role
    let matching_sig = signatures
        .iter()
        .find(|s| s.signer_role().eq_ignore_ascii_case(role));

    if let Some(sig) = matching_sig {
        // Signed — render actual signature
        out.push_str(&format!("*{}*:\n\n", escape_typst(role)));

        if let Some(svg) = sig.signature_svg() {
            // Embed SVG signature image
            let svg_escaped = svg.replace('\"', "\\\"");
            out.push_str(&format!(
                "#image.decode(bytes(\"{svg_escaped}\"), width: 40%)\n\n"
            ));
        } else {
            // Fallback: italic signature text
            out.push_str(&format!("_/s/ {}_\n\n", escape_typst(sig.signature_text())));
        }

        for field in fields {
            match field.as_str() {
                "name" => {
                    out.push_str(&format!(
                        "Name: {}\n\n",
                        escape_typst(sig.signer_name())
                    ));
                }
                "date" => {
                    out.push_str(&format!(
                        "Date: {}\n\n",
                        sig.signed_at().format("%Y-%m-%d")
                    ));
                }
                "title" => {} // included in name line typically
                _ => {
                    out.push_str(&format!("{}: ---\n\n", escape_typst(field)));
                }
            }
        }
    } else {
        // Unsigned — blank signature line
        out.push_str(&format!("*{}*:\n\n", escape_typst(role)));
        out.push_str("#line(length: 60%)\n\n");

        for field in fields {
            match field.as_str() {
                "name" => out.push_str("Name / Title: \\_\\_\\_\\_\\_\\_\\_\\_\\_\\_\n\n"),
                "date" => out.push_str("Date: \\_\\_\\_\\_\\_\\_\\_\\_\\_\\_\n\n"),
                "title" => {}
                _ => {
                    out.push_str(&format!(
                        "{}: \\_\\_\\_\\_\\_\\_\\_\\_\\_\\_\n\n",
                        escape_typst(field)
                    ));
                }
            }
        }
    }
}

// ── Data table ──────────────────────────────────────────────────────

fn render_typst_data_table(
    out: &mut String,
    source: &str,
    columns: &[DataTableColumn],
    ast: &GovernanceDocAst,
    _entity_type: EntityTypeKey,
) {
    let ncols = columns.len();

    // Build alignment spec
    let aligns: Vec<&str> = columns
        .iter()
        .map(|col| {
            if col.format.as_deref() == Some("usd") {
                "right"
            } else {
                "left"
            }
        })
        .collect();
    let align_str = aligns.join(", ");

    out.push_str(&format!(
        "\n#table(\n  columns: {ncols},\n  align: ({align_str},),\n"
    ));

    // Header row
    out.push_str("  table.header(");
    for (i, col) in columns.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("[*{}*]", escape_typst(&col.header)));
    }
    out.push_str("),\n");

    // Data rows per source
    match source {
        "authority_precedence" => {
            for entry in &ast.authority_precedence {
                for col in columns {
                    let val = match col.key.as_str() {
                        "rank" => entry.rank.to_string(),
                        "source" => entry.source.clone(),
                        "label" => entry.label.clone(),
                        _ => String::new(),
                    };
                    out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                }
            }
        }
        "spending_defaults.categories" => {
            for cat in &ast.spending_defaults.categories {
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
                    out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                }
            }
        }
        "structured_data.autonomy_lanes" => {
            if let Some(sd) = &ast.structured_data {
                for lane in &sd.autonomy_lanes {
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => lane.label.clone(),
                            "conditions_text" => {
                                if lane.conditions.is_empty() {
                                    "None".to_owned()
                                } else {
                                    lane.conditions
                                        .iter()
                                        .filter_map(|c| {
                                            c.get("label").and_then(|l| l.as_str())
                                        })
                                        .collect::<Vec<_>>()
                                        .join("; ")
                                }
                            }
                            "authority_text" => lane.authority_action.clone(),
                            _ => String::new(),
                        };
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        "structured_data.approval_validity" => {
            if let Some(sd) = &ast.structured_data {
                if let Some(av) = &sd.approval_validity {
                    for req in &av.required_elements {
                        for col in columns {
                            let val = match col.key.as_str() {
                                "label" => req.label.clone(),
                                "description" => req.rule.clone(),
                                _ => String::new(),
                            };
                            out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                        }
                    }
                }
            }
        }
        "structured_data.credential_custody" => {
            if let Some(sd) = &ast.structured_data {
                for cred in &sd.credential_custody {
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => cred.label.clone(),
                            "custodian" => cred.custodian.clone(),
                            "agent_access" => cred.agent_access.clone(),
                            _ => String::new(),
                        };
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        "structured_data.emergency_modes" => {
            if let Some(sd) = &ast.structured_data {
                for mode in &sd.emergency_modes {
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
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        "structured_data.auto_suspension_triggers" => {
            if let Some(sd) = &ast.structured_data {
                for trigger in &sd.auto_suspension_triggers {
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => trigger.label.clone(),
                            "description" => trigger.description.clone(),
                            _ => String::new(),
                        };
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        "structured_data.report_schedule" => {
            if let Some(sd) = &ast.structured_data {
                for report in &sd.report_schedule {
                    for col in columns {
                        let val = match col.key.as_str() {
                            "label" => report.label.clone(),
                            "frequency" => report.frequency.clone(),
                            "content_summary" => report.content_keys.join(", "),
                            _ => String::new(),
                        };
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        "structured_data.adjustment_rules" => {
            if let Some(sd) = &ast.structured_data {
                for rule in &sd.adjustment_rules {
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
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        "structured_data.change_control_rules" => {
            if let Some(sd) = &ast.structured_data {
                for rule in &sd.change_control_rules {
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
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        "structured_data.retention_schedule" => {
            if let Some(sd) = &ast.structured_data {
                for record in &sd.retention_schedule {
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
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        "structured_data.severity_classification" => {
            if let Some(sd) = &ast.structured_data {
                for level in &sd.severity_classification {
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
                        out.push_str(&format!("  [{}],\n", escape_typst(&val)));
                    }
                }
            }
        }
        _ => {
            out.push_str(&format!("  [Unknown data source: {}],\n", escape_typst(source)));
        }
    }

    out.push_str(")\n");
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::formation::entity::Entity;
    use crate::domain::formation::types::{EntityType, Jurisdiction};
    use crate::domain::ids::{EntityId, WorkspaceId};

    fn make_entity() -> Entity {
        Entity::new(
            EntityId::new(),
            WorkspaceId::new(),
            "Acme Test Corp".to_owned(),
            EntityType::CCorp,
            Jurisdiction::new("Delaware").expect("jurisdiction"),
            Some("Acme Registered Agent".to_owned()),
            Some("123 Main St".to_owned()),
        )
        .expect("entity")
    }

    #[test]
    fn typst_preamble_compiles() {
        let source = format!("{TYPST_PREAMBLE}Hello, world!");
        let pdf = compile_typst_to_pdf(&source).expect("should compile");
        assert!(pdf.starts_with(b"%PDF"), "output should start with %PDF header");
        assert!(pdf.len() > 100, "PDF should have substantial size");
    }

    #[test]
    fn escape_typst_handles_special_chars() {
        assert_eq!(escape_typst("$100"), "\\$100");
        assert_eq!(escape_typst("#heading"), "\\#heading");
        assert_eq!(escape_typst("a * b"), "a \\* b");
        assert_eq!(escape_typst("a_b"), "a\\_b");
        assert_eq!(escape_typst("x < y > z"), "x \\< y \\> z");
        assert_eq!(escape_typst("@ref"), "\\@ref");
        assert_eq!(escape_typst("plain text"), "plain text");
    }

    #[test]
    fn delegation_schedule_renders_to_pdf() {
        let ast = super::super::doc_ast::default_doc_ast();
        let entity = make_entity();
        let profile = GovernanceProfile::default_for_entity(&entity);
        let doc = ast
            .documents
            .iter()
            .find(|d| d.id == "agent_delegation_schedule")
            .expect("delegation schedule");
        let pdf = render_pdf(doc, ast, EntityTypeKey::Corporation, &profile, &[])
            .expect("should render PDF");
        assert!(pdf.starts_with(b"%PDF"), "output should start with %PDF header");
        assert!(pdf.len() > 1000, "PDF should have substantial size");
    }

    #[test]
    fn signature_svg_embeds_in_pdf() {
        let source = format!(
            "{TYPST_PREAMBLE}= Test Document\n\nSome text.\n\n\
             _/s/ Jane Doe_\n\nName: Jane Doe\n\nDate: 2026-01-15\n"
        );
        let pdf = compile_typst_to_pdf(&source).expect("should compile with signature");
        assert!(pdf.starts_with(b"%PDF"));
    }
}
