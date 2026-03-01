//! Validated branch name.
//!
//! `BranchName` wraps a `String` and ensures the branch name is safe for git.
//! It implements `Deref<Target=str>` so it can be passed anywhere `&str` is expected.

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// A validated git branch name.
///
/// Guarantees: non-empty, no `..`, no spaces, no leading `-`, no null bytes.
/// Implements `Deref<Target=str>` for transparent use with `&str` parameters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(try_from = "String")]
pub struct BranchName(String);

impl BranchName {
    /// Create a validated branch name.
    pub fn new(s: impl Into<String>) -> Result<Self, BranchNameError> {
        let s = s.into();
        validate_branch_name(&s)?;
        Ok(Self(s))
    }

    /// Returns the `"main"` branch name.
    pub fn main() -> Self {
        Self("main".to_owned())
    }

    /// Returns the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes and returns the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

fn validate_branch_name(s: &str) -> Result<(), BranchNameError> {
    if s.is_empty() {
        return Err(BranchNameError("branch name must not be empty".into()));
    }
    if s.starts_with('-') {
        return Err(BranchNameError(
            "branch name must not start with '-'".into(),
        ));
    }
    if s.contains("..") {
        return Err(BranchNameError("branch name must not contain '..'".into()));
    }
    if s.contains(' ') {
        return Err(BranchNameError(
            "branch name must not contain spaces".into(),
        ));
    }
    if s.contains('\0') {
        return Err(BranchNameError(
            "branch name must not contain null bytes".into(),
        ));
    }
    if s.ends_with('/') {
        return Err(BranchNameError("branch name must not end with '/'".into()));
    }
    if s.ends_with(".lock") {
        return Err(BranchNameError(
            "branch name must not end with '.lock'".into(),
        ));
    }
    Ok(())
}

/// Error returned when a branch name is invalid.
#[derive(Debug, Clone)]
pub struct BranchNameError(String);

impl fmt::Display for BranchNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid branch name: {}", self.0)
    }
}

impl std::error::Error for BranchNameError {}

impl TryFrom<String> for BranchName {
    type Error = BranchNameError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl<'de> Deserialize<'de> for BranchName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

impl Deref for BranchName {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for BranchName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BranchName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_branch_names() {
        assert!(BranchName::new("main").is_ok());
        assert!(BranchName::new("feature/equity-grants").is_ok());
        assert!(BranchName::new("fix-123").is_ok());
        assert!(BranchName::new("release/v1.0").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(BranchName::new("").is_err());
    }

    #[test]
    fn rejects_leading_dash() {
        assert!(BranchName::new("-feature").is_err());
    }

    #[test]
    fn rejects_double_dot() {
        assert!(BranchName::new("main..feature").is_err());
    }

    #[test]
    fn rejects_spaces() {
        assert!(BranchName::new("my branch").is_err());
    }

    #[test]
    fn rejects_trailing_slash() {
        assert!(BranchName::new("feature/").is_err());
    }

    #[test]
    fn rejects_dot_lock_suffix() {
        assert!(BranchName::new("branch.lock").is_err());
    }

    #[test]
    fn main_constant() {
        let m = BranchName::main();
        assert_eq!(m.as_str(), "main");
    }

    #[test]
    fn deref_to_str() {
        let b = BranchName::new("feature").unwrap();
        let s: &str = &b;
        assert_eq!(s, "feature");
    }

    #[test]
    fn serde_roundtrip() {
        let b = BranchName::new("dev").unwrap();
        let json = serde_json::to_string(&b).unwrap();
        let parsed: BranchName = serde_json::from_str(&json).unwrap();
        assert_eq!(b, parsed);
    }

    #[test]
    fn deserialize_rejects_invalid() {
        let json = serde_json::json!("my branch");
        let result: Result<BranchName, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }
}
