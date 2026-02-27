//! Execution domain types — intents, obligations, and receipts.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── IntentStatus ───────────────────────────────────────────────────────

/// Lifecycle status of an execution intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    /// Intent has been submitted.
    Pending,
    /// Intent has been evaluated for feasibility.
    Evaluated,
    /// Intent has been authorized for execution.
    Authorized,
    /// Intent has been successfully executed.
    Executed,
    /// Intent execution failed.
    Failed,
}

impl fmt::Display for IntentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Evaluated => write!(f, "evaluated"),
            Self::Authorized => write!(f, "authorized"),
            Self::Executed => write!(f, "executed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

// ── ObligationStatus ───────────────────────────────────────────────────

/// Lifecycle status of a compliance or operational obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationStatus {
    /// Obligation is required but not yet started.
    Required,
    /// Work is in progress to fulfill the obligation.
    InProgress,
    /// Obligation has been fulfilled.
    Fulfilled,
    /// Obligation has been waived.
    Waived,
    /// Obligation has expired without being fulfilled.
    Expired,
}

impl fmt::Display for ObligationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Required => write!(f, "required"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Fulfilled => write!(f, "fulfilled"),
            Self::Waived => write!(f, "waived"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

// ── ObligationType ─────────────────────────────────────────────────────

/// An extensible obligation type represented as a string.
///
/// Obligation types are not a fixed enum because they vary by jurisdiction,
/// entity type, and operational context.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObligationType(String);

impl ObligationType {
    /// Create a new obligation type.
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Return the string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ObligationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ObligationType {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ObligationType {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

// ── AssigneeType ───────────────────────────────────────────────────────

/// Who is responsible for fulfilling an obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssigneeType {
    /// Handled internally by the platform or entity.
    Internal,
    /// Delegated to an external third party.
    ThirdParty,
    /// Requires action from a human stakeholder.
    Human,
}

// ── AuthorityTier ──────────────────────────────────────────────────────

/// The authority level required to approve an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthorityTier {
    /// Lowest authority level — routine operations.
    #[serde(rename = "tier_1")]
    Tier1,
    /// Mid-level authority — significant decisions.
    #[serde(rename = "tier_2")]
    Tier2,
    /// Highest authority — major corporate actions.
    #[serde(rename = "tier_3")]
    Tier3,
}

impl fmt::Display for AuthorityTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tier1 => write!(f, "tier_1"),
            Self::Tier2 => write!(f, "tier_2"),
            Self::Tier3 => write!(f, "tier_3"),
        }
    }
}

// ── ReceiptStatus ──────────────────────────────────────────────────────

/// Status of an execution receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    /// Receipt is pending confirmation.
    Pending,
    /// Execution was confirmed successful.
    Executed,
    /// Execution failed.
    Failed,
}

// ── DocumentRequestStatus ──────────────────────────────────────────────

/// Status of a request for a document from a stakeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentRequestStatus {
    /// Document has been requested.
    Requested,
    /// Document has been provided.
    Provided,
    /// Document is not applicable.
    NotApplicable,
    /// Document requirement has been waived.
    Waived,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_status_serde() {
        let status = IntentStatus::Authorized;
        let json = serde_json::to_string(&status).expect("serialize IntentStatus");
        assert_eq!(json, "\"authorized\"");
        let parsed: IntentStatus =
            serde_json::from_str(&json).expect("deserialize IntentStatus");
        assert_eq!(status, parsed);
    }

    #[test]
    fn authority_tier_display() {
        assert_eq!(AuthorityTier::Tier1.to_string(), "tier_1");
        assert_eq!(AuthorityTier::Tier2.to_string(), "tier_2");
        assert_eq!(AuthorityTier::Tier3.to_string(), "tier_3");
    }

    #[test]
    fn authority_tier_serde() {
        let tier = AuthorityTier::Tier2;
        let json = serde_json::to_string(&tier).expect("serialize AuthorityTier");
        assert_eq!(json, "\"tier_2\"");
        let parsed: AuthorityTier =
            serde_json::from_str(&json).expect("deserialize AuthorityTier");
        assert_eq!(tier, parsed);
    }

    #[test]
    fn obligation_type_roundtrip() {
        let ot = ObligationType::new("annual_report".to_owned());
        let json = serde_json::to_string(&ot).expect("serialize ObligationType");
        assert_eq!(json, "\"annual_report\"");
        let parsed: ObligationType =
            serde_json::from_str(&json).expect("deserialize ObligationType");
        assert_eq!(ot, parsed);
    }

    #[test]
    fn obligation_type_from_str() {
        let ot: ObligationType = "tax_filing".into();
        assert_eq!(ot.as_str(), "tax_filing");
        assert_eq!(ot.to_string(), "tax_filing");
    }

    #[test]
    fn receipt_status_serde() {
        let status = ReceiptStatus::Failed;
        let json = serde_json::to_string(&status).expect("serialize ReceiptStatus");
        assert_eq!(json, "\"failed\"");
    }

    #[test]
    fn document_request_status_serde() {
        let status = DocumentRequestStatus::NotApplicable;
        let json = serde_json::to_string(&status).expect("serialize DocumentRequestStatus");
        assert_eq!(json, "\"not_applicable\"");
    }
}
