//! Reference resolution: `@last`, short IDs, name matching.
//!
//! Every command that accepts an entity/document/meeting/etc reference goes
//! through [`resolve`] which supports:
//!
//! - **Full UUID**: passed through as-is
//! - **Short ID prefix**: first 8+ hex chars of a UUID (e.g. `a1b2c3d4`)
//! - **`@last`**: the most recently used ID for the given resource kind
//! - **`@last:entity`**: typed variant — must match the expected kind
//! - **Name matching**: fuzzy match against resource names/labels
//!
//! Last-used IDs are persisted in `~/.corp/refs.json` so they survive across
//! CLI invocations.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Resource kinds ───────────────────────────────────────────────────────────

/// All resource kinds that support reference tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Entity,
    Document,
    Contact,
    Body,
    Seat,
    Meeting,
    AgendaItem,
    Vote,
    Resolution,
    ShareClass,
    Grant,
    SafeNote,
    Valuation,
    Transfer,
    Round,
    Holder,
    Account,
    JournalEntry,
    Invoice,
    Payment,
    BankAccount,
    PayrollRun,
    Intent,
    Obligation,
    Receipt,
    Agent,
    WorkItem,
    ServiceRequest,
    ApiKey,
}

impl RefKind {
    /// Human-readable label for error messages.
    pub fn label(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Document => "document",
            Self::Contact => "contact",
            Self::Body => "governance body",
            Self::Seat => "governance seat",
            Self::Meeting => "meeting",
            Self::AgendaItem => "agenda item",
            Self::Vote => "vote",
            Self::Resolution => "resolution",
            Self::ShareClass => "share class",
            Self::Grant => "equity grant",
            Self::SafeNote => "SAFE note",
            Self::Valuation => "valuation",
            Self::Transfer => "share transfer",
            Self::Round => "funding round",
            Self::Holder => "holder",
            Self::Account => "GL account",
            Self::JournalEntry => "journal entry",
            Self::Invoice => "invoice",
            Self::Payment => "payment",
            Self::BankAccount => "bank account",
            Self::PayrollRun => "payroll run",
            Self::Intent => "intent",
            Self::Obligation => "obligation",
            Self::Receipt => "receipt",
            Self::Agent => "agent",
            Self::WorkItem => "work item",
            Self::ServiceRequest => "service request",
            Self::ApiKey => "API key",
        }
    }

    /// The JSON field name that holds the primary ID for this kind.
    pub fn id_field(self) -> &'static str {
        match self {
            Self::Entity => "entity_id",
            Self::Document => "document_id",
            Self::Contact => "contact_id",
            Self::Body => "body_id",
            Self::Seat => "seat_id",
            Self::Meeting => "meeting_id",
            Self::AgendaItem => "item_id",
            Self::Vote => "vote_id",
            Self::Resolution => "resolution_id",
            Self::ShareClass => "share_class_id",
            Self::Grant => "grant_id",
            Self::SafeNote => "safe_note_id",
            Self::Valuation => "valuation_id",
            Self::Transfer => "transfer_id",
            Self::Round => "round_id",
            Self::Holder => "holder_id",
            Self::Account => "account_id",
            Self::JournalEntry => "entry_id",
            Self::Invoice => "invoice_id",
            Self::Payment => "payment_id",
            Self::BankAccount => "bank_account_id",
            Self::PayrollRun => "payroll_run_id",
            Self::Intent => "intent_id",
            Self::Obligation => "obligation_id",
            Self::Receipt => "receipt_id",
            Self::Agent => "agent_id",
            Self::WorkItem => "work_item_id",
            Self::ServiceRequest => "request_id",
            Self::ApiKey => "key_id",
        }
    }

    /// Field(s) to use for name/label matching (tried in order).
    pub fn label_fields(self) -> &'static [&'static str] {
        match self {
            Self::Entity => &["legal_name"],
            Self::Contact | Self::Holder => &["name"],
            Self::Body => &["name"],
            Self::Meeting | Self::AgendaItem => &["title"],
            Self::ShareClass => &["class_code"],
            Self::Grant => &["recipient_name"],
            Self::SafeNote => &["investor_name"],
            Self::Agent => &["name"],
            Self::WorkItem => &["title"],
            Self::Account => &["account_name"],
            Self::Invoice => &["customer_name"],
            Self::Round => &["name"],
            Self::ServiceRequest => &["service_slug"],
            _ => &[],
        }
    }
}

// ── Ref store (persisted) ────────────────────────────────────────────────────

/// Persisted map of last-used IDs per resource kind.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RefStore {
    /// kind → last ID (for global/non-entity-scoped resources)
    #[serde(default)]
    pub last: HashMap<String, String>,
    /// kind → entity_id → last ID (for entity-scoped resources)
    #[serde(default)]
    pub scoped: HashMap<String, HashMap<String, String>>,
}

impl RefStore {
    fn path() -> PathBuf {
        let dir = std::env::var("CORP_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".corp")
            });
        dir.join("refs.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            std::fs::write(path, json).ok();
        }
    }

    pub fn get_last(&self, kind: RefKind, entity_id: Option<&str>) -> Option<&str> {
        let key = serde_json::to_value(kind).ok()?.as_str()?.to_owned();
        // No — serde_json::to_value for a unit variant produces a string.
        // Let me just use the Display-like approach.
        if let Some(eid) = entity_id {
            self.scoped
                .get(&key)
                .and_then(|m| m.get(eid))
                .map(|s| s.as_str())
        } else {
            self.last.get(&key).map(|s| s.as_str())
        }
    }

    pub fn set_last(&mut self, kind: RefKind, id: &str, entity_id: Option<&str>) {
        let key = kind_key(kind);
        if let Some(eid) = entity_id {
            self.scoped
                .entry(key)
                .or_default()
                .insert(eid.to_owned(), id.to_owned());
        } else {
            self.last.insert(key, id.to_owned());
        }
    }
}

fn kind_key(kind: RefKind) -> String {
    // Serialize the enum variant as a snake_case string.
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{:?}", kind).to_lowercase())
}

// ── Resolution ───────────────────────────────────────────────────────────────

/// Resolve a user-provided reference to a canonical UUID.
///
/// Supports:
/// - Full UUID (36 chars with hyphens)
/// - `@last` or `@last:kind`
/// - Short ID prefix (≥4 hex chars, matched against known IDs)
/// - Name/label fuzzy match (when `candidates` is provided)
///
/// After resolution, the ID is remembered as `@last` for the given kind.
pub fn resolve(
    input: &str,
    kind: RefKind,
    candidates: Option<&[Value]>,
    entity_id: Option<&str>,
    refs: &mut RefStore,
) -> Result<String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        bail!("empty {} reference", kind.label());
    }

    // ── @last ────────────────────────────────────────────────────────────
    if trimmed.starts_with("@last") {
        // Parse @last or @last:kind
        let requested_kind = if trimmed == "@last" {
            kind
        } else if let Some(suffix) = trimmed.strip_prefix("@last:") {
            parse_kind(suffix)?
        } else {
            bail!("invalid reference syntax: {trimmed}");
        };

        if requested_kind != kind {
            bail!(
                "@last:{} cannot be used where a {} reference is expected",
                kind_key(requested_kind),
                kind.label()
            );
        }

        let key = kind_key(kind);
        let id = if let Some(eid) = entity_id {
            refs.scoped.get(&key).and_then(|m| m.get(eid)).cloned()
        } else {
            refs.last.get(&key).cloned()
        };

        match id {
            Some(id) => {
                refs.set_last(kind, &id, entity_id);
                return Ok(id);
            }
            None => bail!(
                "no {} recorded for @last — run a command that creates one first",
                kind.label()
            ),
        }
    }

    // ── Full UUID ────────────────────────────────────────────────────────
    if is_uuid(trimmed) {
        refs.set_last(kind, trimmed, entity_id);
        refs.save();
        return Ok(trimmed.to_owned());
    }

    // ── Short ID prefix ──────────────────────────────────────────────────
    if is_short_id(trimmed) {
        if let Some(candidates) = candidates {
            let matches = find_by_prefix(trimmed, kind, candidates);
            match matches.len() {
                0 => bail!("no {} found matching short ID '{}'", kind.label(), trimmed),
                1 => {
                    let id = matches[0].clone();
                    refs.set_last(kind, &id, entity_id);
                    refs.save();
                    return Ok(id);
                }
                _ => bail!(
                    "ambiguous short ID '{}' matches {} {}s: {}",
                    trimmed,
                    matches.len(),
                    kind.label(),
                    matches
                        .iter()
                        .take(5)
                        .map(|s| short_id(s))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        }
        // No candidates available — try it as a UUID prefix anyway
        // (the server will reject if invalid)
        return Ok(trimmed.to_owned());
    }

    // ── Name/label match ─────────────────────────────────────────────────
    if let Some(candidates) = candidates {
        let matches = find_by_name(trimmed, kind, candidates);
        match matches.len() {
            0 => {} // fall through
            1 => {
                let id = matches[0].clone();
                refs.set_last(kind, &id, entity_id);
                refs.save();
                return Ok(id);
            }
            _ => bail!(
                "ambiguous {} reference '{}' — matches: {}. Use a UUID or short ID instead.",
                kind.label(),
                trimmed,
                matches
                    .iter()
                    .take(5)
                    .map(|s| short_id(s))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    // Nothing matched — return as-is and let the server decide.
    Ok(trimmed.to_owned())
}

/// Remember an ID from a command response. Call after every write command.
pub fn remember_from_response(
    kind: RefKind,
    response: &Value,
    entity_id: Option<&str>,
    refs: &mut RefStore,
) {
    if let Some(id) = response.get(kind.id_field()).and_then(|v| v.as_str()) {
        refs.set_last(kind, id, entity_id);
        refs.save();
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// First 8 chars of a UUID for display.
pub fn short_id(uuid: &str) -> String {
    uuid.chars().filter(|c| *c != '-').take(8).collect()
}

fn is_uuid(s: &str) -> bool {
    // UUID v4 format: 8-4-4-4-12 hex chars with hyphens
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

fn is_short_id(s: &str) -> bool {
    s.len() >= 4 && s.len() <= 32 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn find_by_prefix(prefix: &str, kind: RefKind, candidates: &[Value]) -> Vec<String> {
    let prefix_lower = prefix.to_lowercase();
    let id_field = kind.id_field();
    candidates
        .iter()
        .filter_map(|v| {
            let id = v.get(id_field)?.as_str()?;
            let id_hex: String = id.chars().filter(|c| *c != '-').collect();
            if id_hex.to_lowercase().starts_with(&prefix_lower) {
                Some(id.to_owned())
            } else {
                None
            }
        })
        .collect()
}

fn find_by_name(query: &str, kind: RefKind, candidates: &[Value]) -> Vec<String> {
    let query_lower = query.to_lowercase();
    let id_field = kind.id_field();
    let label_fields = kind.label_fields();
    candidates
        .iter()
        .filter_map(|v| {
            let id = v.get(id_field)?.as_str()?;
            for field in label_fields {
                if let Some(label) = v.get(*field).and_then(|l| l.as_str()) {
                    if label.to_lowercase().contains(&query_lower) {
                        return Some(id.to_owned());
                    }
                }
            }
            None
        })
        .collect()
}

fn parse_kind(s: &str) -> Result<RefKind> {
    serde_json::from_value(Value::String(s.to_owned()))
        .with_context(|| format!("unknown resource kind: '{s}'"))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_uuid_valid() {
        assert!(is_uuid("a1b2c3d4-e5f6-7890-abcd-ef1234567890"));
    }

    #[test]
    fn is_uuid_invalid() {
        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid("a1b2c3d4"));
    }

    #[test]
    fn is_short_id_valid() {
        assert!(is_short_id("a1b2c3d4"));
        assert!(is_short_id("abcd"));
    }

    #[test]
    fn is_short_id_too_short() {
        assert!(!is_short_id("abc"));
    }

    #[test]
    fn short_id_display() {
        assert_eq!(short_id("a1b2c3d4-e5f6-7890-abcd-ef1234567890"), "a1b2c3d4");
    }

    #[test]
    fn resolve_full_uuid() {
        let mut refs = RefStore::default();
        let id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let result = resolve(id, RefKind::Entity, None, None, &mut refs).unwrap();
        assert_eq!(result, id);
        // Should be remembered
        assert_eq!(refs.last.get("entity").unwrap(), id);
    }

    #[test]
    fn resolve_at_last_after_remember() {
        let mut refs = RefStore::default();
        let id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        refs.set_last(RefKind::Entity, id, None);
        let result = resolve("@last", RefKind::Entity, None, None, &mut refs).unwrap();
        assert_eq!(result, id);
    }

    #[test]
    fn resolve_at_last_typed() {
        let mut refs = RefStore::default();
        let id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        refs.set_last(RefKind::Meeting, id, None);
        let result = resolve("@last:meeting", RefKind::Meeting, None, None, &mut refs).unwrap();
        assert_eq!(result, id);
    }

    #[test]
    fn resolve_at_last_wrong_kind_errors() {
        let mut refs = RefStore::default();
        refs.set_last(RefKind::Meeting, "some-id", None);
        let result = resolve("@last:meeting", RefKind::Entity, None, None, &mut refs);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_short_id_prefix() {
        let mut refs = RefStore::default();
        let candidates = vec![
            serde_json::json!({"entity_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890", "legal_name": "Acme"}),
            serde_json::json!({"entity_id": "ffffffff-0000-0000-0000-000000000000", "legal_name": "Other"}),
        ];
        let result = resolve(
            "a1b2c3d4",
            RefKind::Entity,
            Some(&candidates),
            None,
            &mut refs,
        )
        .unwrap();
        assert_eq!(result, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
    }

    #[test]
    fn resolve_by_name() {
        let mut refs = RefStore::default();
        let candidates = vec![
            serde_json::json!({"entity_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890", "legal_name": "Acme Corp"}),
            serde_json::json!({"entity_id": "ffffffff-0000-0000-0000-000000000000", "legal_name": "Other LLC"}),
        ];
        let result = resolve("Acme", RefKind::Entity, Some(&candidates), None, &mut refs).unwrap();
        assert_eq!(result, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
    }

    #[test]
    fn resolve_ambiguous_name_errors() {
        let mut refs = RefStore::default();
        let candidates = vec![
            serde_json::json!({"entity_id": "aaaa0000-0000-0000-0000-000000000000", "legal_name": "Acme East"}),
            serde_json::json!({"entity_id": "bbbb0000-0000-0000-0000-000000000000", "legal_name": "Acme West"}),
        ];
        let result = resolve("Acme", RefKind::Entity, Some(&candidates), None, &mut refs);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ambiguous"));
    }

    #[test]
    fn resolve_scoped_last() {
        let mut refs = RefStore::default();
        let eid = "entity-1";
        let contact_id = "c1c1c1c1-0000-0000-0000-000000000000";
        refs.set_last(RefKind::Contact, contact_id, Some(eid));
        let result = resolve("@last", RefKind::Contact, None, Some(eid), &mut refs).unwrap();
        assert_eq!(result, contact_id);
    }

    #[test]
    fn remember_from_response_extracts_id() {
        let mut refs = RefStore::default();
        let resp = serde_json::json!({"entity_id": "new-id-123", "legal_name": "Test"});
        remember_from_response(RefKind::Entity, &resp, None, &mut refs);
        assert_eq!(refs.last.get("entity").unwrap(), "new-id-123");
    }
}
