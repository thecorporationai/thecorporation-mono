//! Shared input validation helpers for route handlers.

use crate::error::AppError;

/// Maximum length for name/title/label fields.
const MAX_NAME_LEN: usize = 500;

/// Maximum length for short symbol/code fields.
const MAX_SYMBOL_LEN: usize = 50;

/// Validate a name field: must be non-empty after trimming, within length limit.
pub fn validate_name(field: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::BadRequest(format!("{field} must not be empty")));
    }
    if value.len() > MAX_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "{field} exceeds maximum length of {MAX_NAME_LEN} characters"
        )));
    }
    Ok(())
}

/// Validate a symbol/code field: must be non-empty, alphanumeric + hyphens,
/// within short length limit.
pub fn validate_symbol(field: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::BadRequest(format!("{field} must not be empty")));
    }
    if value.len() > MAX_SYMBOL_LEN {
        return Err(AppError::BadRequest(format!(
            "{field} exceeds maximum length of {MAX_SYMBOL_LEN} characters"
        )));
    }
    if !value
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
    {
        return Err(AppError::BadRequest(format!(
            "{field} contains invalid characters (allowed: alphanumeric, hyphens, underscores, spaces)"
        )));
    }
    Ok(())
}

/// Validate a jurisdiction code: must be a 2-letter uppercase string.
pub fn validate_jurisdiction(value: &str) -> Result<(), AppError> {
    if value.len() != 2 || !value.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(AppError::BadRequest(format!(
            "jurisdiction must be a 2-letter state code (e.g. 'DE'), got '{value}'"
        )));
    }
    Ok(())
}
