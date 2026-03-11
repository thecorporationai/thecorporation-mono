use chrono::{Datelike, Duration, NaiveDate, Utc};

use crate::error::AppError;

pub fn require_non_empty_trimmed(value: &str, field: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{field} cannot be empty")));
    }
    Ok(trimmed.to_owned())
}

pub fn reject_blank_optional(value: Option<&str>, field: &str) -> Result<(), AppError> {
    if value.is_some_and(|candidate| candidate.trim().is_empty()) {
        return Err(AppError::BadRequest(format!("{field} cannot be empty")));
    }
    Ok(())
}

pub fn require_non_empty_trimmed_max(
    value: &str,
    field: &str,
    max_len: usize,
) -> Result<String, AppError> {
    let trimmed = require_non_empty_trimmed(value, field)?;
    if trimmed.len() > max_len {
        return Err(AppError::BadRequest(format!(
            "{field} must be at most {max_len} characters"
        )));
    }
    Ok(trimmed)
}

pub fn normalize_slug(value: &str, field: &str, max_len: usize) -> Result<String, AppError> {
    let normalized = require_non_empty_trimmed_max(value, field, max_len)?.to_ascii_lowercase();
    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        return Err(AppError::BadRequest(format!(
            "{field} must be a non-empty slug"
        )));
    }
    Ok(normalized)
}

pub fn validate_not_too_far_past(
    field: &str,
    value: NaiveDate,
    max_days_past: i64,
) -> Result<(), AppError> {
    let min_date = Utc::now().date_naive() - Duration::days(max_days_past);
    if value < min_date {
        return Err(AppError::BadRequest(format!(
            "{field} cannot be more than {max_days_past} days in the past"
        )));
    }
    Ok(())
}

pub fn validate_not_too_far_future(
    field: &str,
    value: NaiveDate,
    max_days_future: i64,
) -> Result<(), AppError> {
    let max_date = Utc::now().date_naive() + Duration::days(max_days_future);
    if value > max_date {
        return Err(AppError::BadRequest(format!(
            "{field} cannot be more than {max_days_future} days in the future"
        )));
    }
    Ok(())
}

pub fn validate_date_order(
    start_field: &str,
    start: NaiveDate,
    end_field: &str,
    end: NaiveDate,
) -> Result<(), AppError> {
    if end < start {
        return Err(AppError::BadRequest(format!(
            "{end_field} must be on or after {start_field}"
        )));
    }
    Ok(())
}

pub fn validate_reasonable_year(
    field: &str,
    year: i32,
    min_year: i32,
    max_future_years: i32,
) -> Result<(), AppError> {
    let max_year = Utc::now().year() + max_future_years;
    if year < min_year || year > max_year {
        return Err(AppError::BadRequest(format!(
            "{field} must be between {min_year} and {max_year}"
        )));
    }
    Ok(())
}

/// Validate that an optional numeric JSON parameter is non-negative.
pub fn validate_non_negative_json_f64(
    params: &serde_json::Value,
    field: &str,
) -> Result<(), AppError> {
    if let Some(val) = params.get(field) {
        if let Some(n) = val.as_f64() {
            if n < 0.0 {
                return Err(AppError::BadRequest(format!(
                    "{field} cannot be negative"
                )));
            }
        }
    }
    Ok(())
}

/// Validate that a string field does not exceed a maximum length.
pub fn validate_max_len(value: &str, field: &str, max_len: usize) -> Result<(), AppError> {
    if value.len() > max_len {
        return Err(AppError::BadRequest(format!(
            "{field} must be at most {max_len} characters"
        )));
    }
    Ok(())
}

/// Reject JSON parameter keys that look like prototype pollution attempts.
pub fn reject_dangerous_param_keys(params: &serde_json::Value) -> Result<(), AppError> {
    if let Some(obj) = params.as_object() {
        for key in obj.keys() {
            if key.starts_with("__") || key.contains("constructor") || key.contains("prototype") {
                return Err(AppError::BadRequest(format!(
                    "parameter key '{key}' is not allowed"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_non_empty_trimmed_rejects_blank() {
        let err = require_non_empty_trimmed("   ", "title").expect_err("blank should fail");
        match err {
            AppError::BadRequest(message) => assert!(message.contains("title cannot be empty")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn date_window_helpers_reject_out_of_range_values() {
        let far_past = Utc::now().date_naive() - Duration::days(400);
        let far_future = Utc::now().date_naive() + Duration::days(400);
        assert!(validate_not_too_far_past("due_date", far_past, 365).is_err());
        assert!(validate_not_too_far_future("scheduled_date", far_future, 365).is_err());
    }

    #[test]
    fn slug_and_year_helpers_reject_bad_values() {
        assert!(normalize_slug("INVALID VALUE", "provider", 64).is_err());
        assert!(validate_reasonable_year("tax_year", 99999, 1900, 2).is_err());
    }

    #[test]
    fn validate_non_negative_json_f64_rejects_negative() {
        let params = serde_json::json!({"hourly_rate": -50.0});
        assert!(validate_non_negative_json_f64(&params, "hourly_rate").is_err());
        let params = serde_json::json!({"hourly_rate": 100.0});
        assert!(validate_non_negative_json_f64(&params, "hourly_rate").is_ok());
    }

    #[test]
    fn reject_dangerous_param_keys_blocks_proto() {
        let params = serde_json::json!({"__proto__": {"polluted": true}});
        assert!(reject_dangerous_param_keys(&params).is_err());
        let params = serde_json::json!({"name": "safe"});
        assert!(reject_dangerous_param_keys(&params).is_ok());
    }

    #[test]
    fn validate_max_len_rejects_long_values() {
        assert!(validate_max_len("short", "field", 100).is_ok());
        assert!(validate_max_len(&"x".repeat(101), "field", 100).is_err());
    }
}
