//! Parse-don't-validate foundation types.
//!
//! These newtypes enforce invariants at construction/deserialization time.
//! Once you hold a `NonEmpty`, you *know* it's non-blank — no further
//! checks needed.  This is the core of "parse, don't validate": the return
//! type of the parser *is* the proof that validation succeeded.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// ── ParseError ──────────────────────────────────────────────────────

/// Error returned when a validated type rejects its input.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// A required string field was empty or whitespace-only.
    EmptyString,
    /// A cron expression didn't have the expected number of fields (≥ 5).
    InvalidCron(String),
    /// A numeric value was zero where a positive value is required.
    ZeroValue(&'static str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyString => f.write_str("value must be a non-empty string"),
            Self::InvalidCron(s) => write!(f, "invalid cron expression (need ≥5 fields): {s:?}"),
            Self::ZeroValue(field) => write!(f, "{field} must be positive (> 0)"),
        }
    }
}

impl std::error::Error for ParseError {}

// ── NonEmpty ────────────────────────────────────────────────────────

/// A `String` that is guaranteed non-empty and non-whitespace-only.
///
/// This is a *parsed* type: the only way to obtain one is through
/// `NonEmpty::parse()` or serde deserialization, both of which reject
/// blank strings.  Downstream code never needs to re-check for emptiness.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NonEmpty(String);

impl NonEmpty {
    /// Parse a string, rejecting empty / whitespace-only values.
    pub fn parse(s: impl Into<String>) -> Result<Self, ParseError> {
        let s = s.into();
        if s.trim().is_empty() {
            return Err(ParseError::EmptyString);
        }
        Ok(Self(s))
    }

    /// View the inner string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner `String`.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::ops::Deref for NonEmpty {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for NonEmpty {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NonEmpty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<NonEmpty> for String {
    fn from(ne: NonEmpty) -> Self {
        ne.0
    }
}

impl PartialEq<str> for NonEmpty {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for NonEmpty {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for NonEmpty {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl Serialize for NonEmpty {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NonEmpty {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        NonEmpty::parse(s).map_err(serde::de::Error::custom)
    }
}

// ── CronExpr ────────────────────────────────────────────────────────

/// A cron expression validated to have at least 5 whitespace-separated fields.
///
/// This is a lightweight parse — it doesn't validate each field's range,
/// but it rejects obviously malformed expressions at the system boundary.
/// The full matching logic lives in the worker's cron module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CronExpr(String);

impl CronExpr {
    /// Parse a cron expression, requiring at least 5 fields.
    pub fn parse(s: impl Into<String>) -> Result<Self, ParseError> {
        let s = s.into();
        let field_count = s.split_whitespace().count();
        if field_count < 5 {
            return Err(ParseError::InvalidCron(s));
        }
        Ok(Self(s))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::ops::Deref for CronExpr {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for CronExpr {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CronExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for CronExpr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CronExpr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        CronExpr::parse(s).map_err(serde::de::Error::custom)
    }
}

// ── Serde helpers for positive-number validation ─────────────────────

/// Deserialize a `u32` that must be > 0.
pub fn deserialize_positive_u32<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let v = u32::deserialize(d)?;
    if v == 0 {
        return Err(serde::de::Error::custom("value must be positive (> 0)"));
    }
    Ok(v)
}

/// Deserialize a `u64` that must be > 0.
pub fn deserialize_positive_u64<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let v = u64::deserialize(d)?;
    if v == 0 {
        return Err(serde::de::Error::custom("value must be positive (> 0)"));
    }
    Ok(v)
}

/// Deserialize an `f64` that must be > 0.0.
pub fn deserialize_positive_f64<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    let v = f64::deserialize(d)?;
    if v <= 0.0 {
        return Err(serde::de::Error::custom("value must be positive (> 0.0)"));
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── NonEmpty ─────────────────────────────────────────────────────

    #[test]
    fn nonempty_rejects_empty() {
        assert!(NonEmpty::parse("").is_err());
    }

    #[test]
    fn nonempty_rejects_whitespace_only() {
        assert!(NonEmpty::parse("   ").is_err());
        assert!(NonEmpty::parse("\t\n").is_err());
    }

    #[test]
    fn nonempty_accepts_valid() {
        let ne = NonEmpty::parse("hello").unwrap();
        assert_eq!(ne.as_str(), "hello");
        assert_eq!(&*ne, "hello"); // Deref
    }

    #[test]
    fn nonempty_preserves_whitespace() {
        // Leading/trailing whitespace is preserved (not trimmed)
        let ne = NonEmpty::parse("  hello  ").unwrap();
        assert_eq!(ne.as_str(), "  hello  ");
    }

    #[test]
    fn nonempty_serde_roundtrip() {
        let ne = NonEmpty::parse("test").unwrap();
        let json = serde_json::to_string(&ne).unwrap();
        assert_eq!(json, "\"test\"");
        let parsed: NonEmpty = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ne);
    }

    #[test]
    fn nonempty_serde_rejects_empty() {
        let result = serde_json::from_str::<NonEmpty>("\"\"");
        assert!(result.is_err());
    }

    #[test]
    fn nonempty_serde_rejects_blank() {
        let result = serde_json::from_str::<NonEmpty>("\"   \"");
        assert!(result.is_err());
    }

    #[test]
    fn nonempty_display() {
        let ne = NonEmpty::parse("world").unwrap();
        assert_eq!(format!("{ne}"), "world");
    }

    #[test]
    fn nonempty_into_string() {
        let ne = NonEmpty::parse("owned").unwrap();
        let s: String = ne.into();
        assert_eq!(s, "owned");
    }

    // ── CronExpr ────────────────────────────────────────────────────

    #[test]
    fn cronexpr_rejects_too_few_fields() {
        assert!(CronExpr::parse("* * *").is_err());
        assert!(CronExpr::parse("").is_err());
        assert!(CronExpr::parse("*/5 *").is_err());
    }

    #[test]
    fn cronexpr_accepts_five_fields() {
        let expr = CronExpr::parse("*/5 * * * *").unwrap();
        assert_eq!(expr.as_str(), "*/5 * * * *");
    }

    #[test]
    fn cronexpr_accepts_six_fields() {
        // Some cron impls support a seconds field
        let expr = CronExpr::parse("0 */5 * * * *").unwrap();
        assert_eq!(expr.as_str(), "0 */5 * * * *");
    }

    #[test]
    fn cronexpr_serde_roundtrip() {
        let expr = CronExpr::parse("30 9 * * 1-5").unwrap();
        let json = serde_json::to_string(&expr).unwrap();
        assert_eq!(json, "\"30 9 * * 1-5\"");
        let parsed: CronExpr = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, expr);
    }

    #[test]
    fn cronexpr_serde_rejects_invalid() {
        let result = serde_json::from_str::<CronExpr>("\"* *\"");
        assert!(result.is_err());
    }

    // ── Positive validators ─────────────────────────────────────────

    #[test]
    fn positive_u32_rejects_zero() {
        #[allow(dead_code)]
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "deserialize_positive_u32")]
            v: u32,
        }
        let result = serde_json::from_str::<T>(r#"{"v": 0}"#);
        assert!(result.is_err());
    }

    #[test]
    fn positive_u32_accepts_positive() {
        #[allow(dead_code)]
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "deserialize_positive_u32")]
            v: u32,
        }
        let t: T = serde_json::from_str(r#"{"v": 42}"#).unwrap();
        assert_eq!(t.v, 42);
    }

    #[test]
    fn positive_f64_rejects_zero_and_negative() {
        #[allow(dead_code)]
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "deserialize_positive_f64")]
            v: f64,
        }
        assert!(serde_json::from_str::<T>(r#"{"v": 0.0}"#).is_err());
        assert!(serde_json::from_str::<T>(r#"{"v": -1.0}"#).is_err());
        let t: T = serde_json::from_str(r#"{"v": 0.5}"#).unwrap();
        assert_eq!(t.v, 0.5);
    }
}
