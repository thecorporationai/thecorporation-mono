//! Output formatting helpers for the `corp` CLI.
//!
//! Three rendering modes:
//! - **Default**: human-readable tables via `comfy_table`.
//! - **JSON** (`--json`): pretty-printed raw JSON from the API.
//! - **Quiet** (`--quiet`): single `id` field per line (machine-friendly).

use colored::Colorize;
use comfy_table::{
    Attribute, Cell, CellAlignment, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL,
};
use serde_json::Value;

// ── OutputMode ────────────────────────────────────────────────────────────────

/// Resolved output mode derived from CLI flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Human-readable tables.
    Human,
    /// Raw pretty JSON.
    Json,
    /// ID-per-line (for scripting).
    Quiet,
}

impl OutputMode {
    pub fn from_flags(json: bool, quiet: bool) -> Self {
        if json {
            Self::Json
        } else if quiet {
            Self::Quiet
        } else {
            Self::Human
        }
    }
}

// ── Top-level print helpers ───────────────────────────────────────────────────

/// Print the API response according to the selected mode.
///
/// - JSON mode: pretty-prints the whole value.
/// - Quiet mode: extracts `id` (or a list of IDs from an array).
/// - Human mode: delegates to `print_human`.
pub fn print_value(value: &Value, mode: OutputMode) {
    match mode {
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_default()
            );
        }
        OutputMode::Quiet => print_ids(value),
        OutputMode::Human => print_human(value),
    }
}

/// Print a success message (suppressed in JSON and quiet modes).
pub fn print_success(msg: &str, mode: OutputMode) {
    if mode == OutputMode::Human {
        println!("{} {}", "✓".green(), msg);
    }
}

/// Print a warning (always shown).
pub fn print_warn(msg: &str) {
    eprintln!("{} {}", "!".yellow(), msg);
}

/// Print an error message to stderr (always shown).
pub fn print_error(msg: &str) {
    eprintln!("{} {}", "✗".red(), msg);
}

/// Print a section header (suppressed in quiet / JSON modes).
pub fn print_header(title: &str, mode: OutputMode) {
    if mode == OutputMode::Human {
        println!("\n{}", title.bold().underline());
    }
}

// ── Human-readable rendering ──────────────────────────────────────────────────

/// Render `value` in a human-readable way.
///
/// - Arrays of objects → table (keys become column headers).
/// - Single object → two-column key/value table.
/// - Primitives → printed directly.
pub fn print_human(value: &Value) {
    match value {
        Value::Array(items) => {
            if items.is_empty() {
                println!("{}", "(no items)".dimmed());
                return;
            }
            // Sniff columns from the first element.
            if let Some(Value::Object(first)) = items.first() {
                let cols: Vec<String> = first.keys().cloned().collect();
                print_json_objects(&cols, items);
            } else {
                for item in items {
                    println!("{}", item);
                }
            }
        }
        Value::Object(map) => {
            let mut tbl = Table::new();
            tbl.set_content_arrangement(ContentArrangement::Dynamic);
            tbl.set_header(vec!["Field", "Value"]);
            for (k, v) in map {
                tbl.add_row(vec![k.as_str(), &format_value(v)]);
            }
            println!("{tbl}");
        }
        Value::Null => println!("{}", "(null)".dimmed()),
        other => println!("{other}"),
    }
}

/// Render a list of JSON objects as a table with the given column headers.
/// Internal helper used by [`print_human`] and [`print_json_table`].
fn print_json_objects(cols: &[String], rows: &[Value]) {
    let mut tbl = Table::new();
    tbl.set_content_arrangement(ContentArrangement::Dynamic);
    tbl.set_header(cols.iter().map(|c| c.as_str()).collect::<Vec<_>>());

    for row in rows {
        if let Value::Object(map) = row {
            let cells: Vec<String> = cols
                .iter()
                .map(|c| format_value(map.get(c).unwrap_or(&Value::Null)))
                .collect();
            tbl.add_row(cells);
        }
    }
    println!("{tbl}");
}

/// Format a JSON value into a compact, display-friendly string.
pub fn format_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::from("—"),
        Value::Array(a) => format!("[{} items]", a.len()),
        Value::Object(_) => String::from("{…}"),
    }
}

// ── Quiet / ID extraction ─────────────────────────────────────────────────────

fn print_ids(value: &Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                if let Some(id) = extract_id(item) {
                    println!("{id}");
                }
            }
        }
        other => {
            if let Some(id) = extract_id(other) {
                println!("{id}");
            }
        }
    }
}

/// Extract the primary ID field from a JSON object.
///
/// Checks, in order: `id`, any field ending with `_id`.
fn extract_id(value: &Value) -> Option<String> {
    let obj = value.as_object()?;
    if let Some(v) = obj.get("id") {
        return Some(format_value(v));
    }
    // Look for a field ending with `_id` — take the first one.
    obj.iter()
        .find(|(k, _)| k.ends_with("_id"))
        .map(|(_, v)| format_value(v))
}

// ── Key/value display ─────────────────────────────────────────────────────────

/// Print a simple key=value line (suppressed in quiet mode).
pub fn kv(label: &str, value: &str, mode: OutputMode) {
    if mode == OutputMode::Human {
        println!("{}: {}", label.bold(), value);
    }
}

// ── Rich titled tables ────────────────────────────────────────────────────────

/// Print a titled table with explicit string headers and pre-formatted rows.
///
/// Uses UTF-8 box-drawing with round corners and dynamic column sizing.
///
/// ```text
///  Entities
/// ╭──────────┬──────────────┬────────────────╮
/// │ ID       │ Name         │ Status         │
/// ╞══════════╪══════════════╪════════════════╡
/// │ abc-123  │ Acme Corp    │ active         │
/// ╰──────────┴──────────────┴────────────────╯
/// ```
pub fn print_table(title: &str, headers: &[&str], rows: &[Vec<String>]) {
    if !title.is_empty() {
        println!("\n{}", title.bold());
    }
    let mut tbl = rich_table();
    tbl.set_header(headers.iter().map(|h| {
        Cell::new(h)
            .add_attribute(Attribute::Bold)
            .fg(comfy_table::Color::Cyan)
    }));
    for row in rows {
        tbl.add_row(row);
    }
    println!("{tbl}");
}

/// Print a single domain object as a labelled key-value panel.
///
/// ```text
///  Entity
/// ╭───────────────┬───────────────────╮
/// │ Field         │ Value             │
/// ╞═══════════════╪═══════════════════╡
/// │ ID            │ abc-123           │
/// │ Name          │ Acme Corp         │
/// ╰───────────────┴───────────────────╯
/// ```
pub fn print_object(title: &str, fields: &[(&str, String)]) {
    if !title.is_empty() {
        println!("\n{}", title.bold());
    }
    let mut tbl = rich_table();
    tbl.set_header(vec![
        Cell::new("Field")
            .add_attribute(Attribute::Bold)
            .fg(comfy_table::Color::Cyan),
        Cell::new("Value")
            .add_attribute(Attribute::Bold)
            .fg(comfy_table::Color::Cyan),
    ]);
    for (key, val) in fields {
        tbl.add_row(vec![
            Cell::new(key).add_attribute(Attribute::Bold),
            Cell::new(val),
        ]);
    }
    println!("{tbl}");
}

/// Print a JSON array (or a single object) as a table, auto-detecting columns
/// from the keys of the first object encountered.
///
/// Arrays of objects produce one row per element.  A bare object is treated as
/// a single-row table.  Any other JSON shape falls back to [`print_json`].
pub fn print_json_table(value: &Value) {
    let objects: Vec<&Value> = match value {
        Value::Array(arr) => arr.iter().collect(),
        Value::Object(_) => vec![value],
        _ => {
            print_json(value);
            return;
        }
    };

    if objects.is_empty() {
        println!("{}", "  (no results)".dimmed());
        return;
    }

    // Union of keys across all objects (insertion-ordered from first object).
    let mut headers: Vec<String> = Vec::new();
    for obj in &objects {
        if let Value::Object(map) = obj {
            for key in map.keys() {
                if !headers.contains(key) {
                    headers.push(key.clone());
                }
            }
        }
    }

    if headers.is_empty() {
        print_json(value);
        return;
    }

    let rows: Vec<Vec<String>> = objects
        .iter()
        .filter_map(|obj| {
            if let Value::Object(map) = obj {
                Some(
                    headers
                        .iter()
                        .map(|h| format_value(map.get(h).unwrap_or(&Value::Null)))
                        .collect(),
                )
            } else {
                None
            }
        })
        .collect();

    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    print_table("", &header_refs, &rows);
}

// ── Raw / plain output ────────────────────────────────────────────────────────

/// Print pretty-printed JSON (for `--json` mode).
pub fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    );
}

/// Print a single bare ID string (for `--quiet` mode).
pub fn print_id(id: &str) {
    println!("{id}");
}

// ── Status panel ─────────────────────────────────────────────────────────────

/// Print a colour-highlighted status panel.
///
/// Each entry is `(label, value, color)` where `color` is one of:
/// `"green"`, `"yellow"`, `"red"`, `"blue"`, `"cyan"`, `"magenta"`,
/// `"dark_green"`, `"dark_red"` (or any unrecognised string → white).
///
/// ```text
///  API Status
/// ╭──────────────┬──────────────────╮
/// │ Field        │ Value            │
/// ╞══════════════╪══════════════════╡
/// │ Health       │ healthy          │  ← green
/// │ Environment  │ production       │  ← cyan
/// │ Alerts       │ 2                │  ← red
/// ╰──────────────┴──────────────────╯
/// ```
pub fn print_status(title: &str, fields: &[(&str, String, &str)]) {
    if !title.is_empty() {
        println!("\n{}", title.bold());
    }
    let mut tbl = rich_table();
    tbl.set_header(vec![
        Cell::new("Field")
            .add_attribute(Attribute::Bold)
            .fg(comfy_table::Color::Cyan),
        Cell::new("Value")
            .add_attribute(Attribute::Bold)
            .fg(comfy_table::Color::Cyan),
    ]);
    for (label, value, color) in fields {
        tbl.add_row(vec![
            Cell::new(label).add_attribute(Attribute::Bold),
            color_cell(value, color),
        ]);
    }
    println!("{tbl}");
}

// ── Datetime / money formatters ───────────────────────────────────────────────

/// Format an ISO-8601 / RFC-3339 datetime string for display.
///
/// `"2025-12-01T14:30:00Z"` → `"2025-12-01 14:30 UTC"`.
/// Returns the raw string unchanged when parsing fails.
pub fn format_datetime(dt: &str) -> String {
    use chrono::{DateTime, Utc};
    dt.parse::<DateTime<Utc>>()
        .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|_| dt.to_owned())
}

/// Format an integer number of cents as a dollar amount string.
///
/// `1234` → `"$12.34"`,  `-50` → `"-$0.50"`,  `1_000_000_00` → `"$1,000,000.00"`.
pub fn format_cents(cents: i64) -> String {
    let negative = cents < 0;
    let abs = cents.unsigned_abs();
    let dollars = abs / 100;
    let remainder = abs % 100;
    let amount = format!("${}.{remainder:02}", thousands(dollars));
    if negative {
        format!("-{amount}")
    } else {
        amount
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Construct a consistently styled [`Table`] with UTF-8 round corners.
fn rich_table() -> Table {
    let mut tbl = Table::new();
    tbl.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    tbl
}

/// Build a [`Cell`] tinted with the named color string.
fn color_cell(value: &str, color: &str) -> Cell {
    let tc = match color {
        "green" => comfy_table::Color::Green,
        "yellow" => comfy_table::Color::Yellow,
        "red" => comfy_table::Color::Red,
        "blue" => comfy_table::Color::Blue,
        "cyan" => comfy_table::Color::Cyan,
        "magenta" => comfy_table::Color::Magenta,
        "dark_green" => comfy_table::Color::DarkGreen,
        "dark_red" => comfy_table::Color::DarkRed,
        _ => comfy_table::Color::White,
    };
    Cell::new(value).fg(tc).set_alignment(CellAlignment::Left)
}

/// Insert comma thousands separators into an integer (e.g. `1000000` → `"1,000,000"`).
fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── format_cents ─────────────────────────────────────────────────────────

    #[test]
    fn cents_zero() {
        assert_eq!(format_cents(0), "$0.00");
    }

    #[test]
    fn cents_positive() {
        assert_eq!(format_cents(1234), "$12.34");
    }

    #[test]
    fn cents_negative() {
        assert_eq!(format_cents(-50), "-$0.50");
    }

    #[test]
    fn cents_large_with_commas() {
        assert_eq!(format_cents(1_000_000_00), "$1,000,000.00");
    }

    #[test]
    fn cents_exact_dollars() {
        assert_eq!(format_cents(10_00), "$10.00");
    }

    // ── format_datetime ───────────────────────────────────────────────────────

    #[test]
    fn datetime_valid_iso() {
        assert_eq!(
            format_datetime("2025-12-01T14:30:00Z"),
            "2025-12-01 14:30 UTC"
        );
    }

    #[test]
    fn datetime_invalid_falls_back() {
        let raw = "not-a-date";
        assert_eq!(format_datetime(raw), raw);
    }

    // ── thousands ─────────────────────────────────────────────────────────────

    #[test]
    fn thousands_below_1000() {
        assert_eq!(thousands(999), "999");
    }

    #[test]
    fn thousands_exactly_1000() {
        assert_eq!(thousands(1_000), "1,000");
    }

    #[test]
    fn thousands_million() {
        assert_eq!(thousands(1_000_000), "1,000,000");
    }

    // ── format_value (cell) ───────────────────────────────────────────────────

    #[test]
    fn cell_null() {
        assert_eq!(format_value(&Value::Null), "—");
    }

    #[test]
    fn cell_string() {
        assert_eq!(format_value(&json!("hello")), "hello");
    }

    #[test]
    fn cell_empty_array() {
        assert_eq!(format_value(&json!([])), "[0 items]");
    }

    #[test]
    fn cell_array_items() {
        assert_eq!(format_value(&json!([1, 2, 3])), "[3 items]");
    }

    #[test]
    fn cell_object() {
        assert_eq!(format_value(&json!({"a": 1})), "{…}");
    }

    // ── print_json_table smoke ────────────────────────────────────────────────

    #[test]
    fn json_table_empty_array() {
        // Must not panic.
        print_json_table(&json!([]));
    }

    #[test]
    fn json_table_primitive_falls_back() {
        // Must not panic.
        print_json_table(&json!(42));
    }

    // ── OutputMode ────────────────────────────────────────────────────────────

    #[test]
    fn output_mode_flags() {
        assert_eq!(OutputMode::from_flags(true, false), OutputMode::Json);
        assert_eq!(OutputMode::from_flags(false, true), OutputMode::Quiet);
        assert_eq!(OutputMode::from_flags(false, false), OutputMode::Human);
        // --json takes precedence
        assert_eq!(OutputMode::from_flags(true, true), OutputMode::Json);
    }
}
