//! Typed boundary for governance intent metadata.

use serde_json::Value;

use super::capability::GovernanceCapability;
use super::policy_engine::canonicalize_intent_type;

#[derive(Debug, Clone, Default)]
pub struct ParsedGovernanceMetadata {
    pub lane_id: Option<String>,
    pub template_approved: Option<bool>,
    pub is_reversible: Option<bool>,
    pub modifications: Vec<String>,
    pub context_rate_increase_percent: Option<f64>,
    pub context_price_increase_percent: Option<f64>,
    pub context_premium_increase_percent: Option<f64>,
    pub amount_cents: Option<i64>,
    pub decode_issues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TypedIntent<'a> {
    original_intent_type: &'a str,
    canonical_intent_type: String,
    capability: Option<GovernanceCapability>,
    metadata: ParsedGovernanceMetadata,
    raw_metadata: &'a Value,
}

impl<'a> TypedIntent<'a> {
    pub fn parse(intent_type: &'a str, raw_metadata: &'a Value) -> Self {
        let canonical_intent_type = canonicalize_intent_type(intent_type);
        let capability = canonical_intent_type.parse::<GovernanceCapability>().ok();
        let metadata = parse_metadata(raw_metadata);
        Self {
            original_intent_type: intent_type,
            canonical_intent_type,
            capability,
            metadata,
            raw_metadata,
        }
    }

    pub fn original_intent_type(&self) -> &str {
        self.original_intent_type
    }

    pub fn canonical_intent_type(&self) -> &str {
        &self.canonical_intent_type
    }

    pub fn capability(&self) -> Option<GovernanceCapability> {
        self.capability
    }

    pub fn metadata(&self) -> &ParsedGovernanceMetadata {
        &self.metadata
    }

    pub fn raw_metadata(&self) -> &Value {
        self.raw_metadata
    }
}

fn parse_metadata(value: &Value) -> ParsedGovernanceMetadata {
    let mut parsed = ParsedGovernanceMetadata {
        lane_id: get_lane_id(value),
        template_approved: get_optional_bool(value, "templateApproved"),
        is_reversible: get_optional_bool(value, "isReversible"),
        modifications: parse_modifications(value),
        context_rate_increase_percent: get_optional_context_number(value, "rateIncreasePercent"),
        context_price_increase_percent: get_optional_context_number(value, "priceIncreasePercent"),
        context_premium_increase_percent: get_optional_context_number(
            value,
            "premiumIncreasePercent",
        ),
        amount_cents: get_amount_cents(value),
        decode_issues: Vec::new(),
    };

    if value.get("templateApproved").is_some() && parsed.template_approved.is_none() {
        parsed
            .decode_issues
            .push("templateApproved must be a boolean".to_owned());
    }
    if value.get("isReversible").is_some() && parsed.is_reversible.is_none() {
        parsed
            .decode_issues
            .push("isReversible must be a boolean".to_owned());
    }
    if has_context_field(value, "rateIncreasePercent")
        && parsed.context_rate_increase_percent.is_none()
    {
        parsed
            .decode_issues
            .push("context.rateIncreasePercent must be numeric".to_owned());
    }
    if has_context_field(value, "priceIncreasePercent")
        && parsed.context_price_increase_percent.is_none()
    {
        parsed
            .decode_issues
            .push("context.priceIncreasePercent must be numeric".to_owned());
    }
    if has_context_field(value, "premiumIncreasePercent")
        && parsed.context_premium_increase_percent.is_none()
    {
        parsed
            .decode_issues
            .push("context.premiumIncreasePercent must be numeric".to_owned());
    }

    parsed
}

fn get_lane_id(value: &Value) -> Option<String> {
    value
        .get("laneId")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            value
                .get("context")
                .and_then(|ctx| ctx.get("laneId"))
                .and_then(Value::as_str)
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
        })
}

fn get_optional_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn get_optional_context_number(value: &Value, key: &str) -> Option<f64> {
    value
        .get("context")
        .and_then(|ctx| ctx.get(key))
        .and_then(Value::as_f64)
}

fn has_context_field(value: &Value, key: &str) -> bool {
    value.get("context").and_then(|ctx| ctx.get(key)).is_some()
}

fn parse_modifications(value: &Value) -> Vec<String> {
    match value.get("modifications") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase())
            .collect(),
        Some(Value::String(s)) => vec![s.trim().to_ascii_lowercase()],
        _ => Vec::new(),
    }
}

fn get_amount_cents(value: &Value) -> Option<i64> {
    value
        .get("amount_cents")
        .and_then(Value::as_i64)
        .or_else(|| {
            value
                .get("amount")
                .and_then(Value::as_i64)
                .filter(|amount| *amount > 0 && *amount < i64::MAX / 100)
                .map(|dollars| dollars.saturating_mul(100))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_known_capability() {
        let metadata = json!({});
        let typed = TypedIntent::parse("  pay_recurring_obligation  ", &metadata);
        assert_eq!(typed.canonical_intent_type(), "pay_recurring_obligation");
        assert_eq!(
            typed.capability(),
            Some(GovernanceCapability::PayRecurringObligation)
        );
    }

    #[test]
    fn captures_decode_issues_for_wrong_types() {
        let metadata = json!({
            "templateApproved": "yes",
            "context": { "priceIncreasePercent": "10" }
        });
        let typed = TypedIntent::parse("execute_standard_form_agreement", &metadata);
        assert!(!typed.metadata().decode_issues.is_empty());
    }
}
