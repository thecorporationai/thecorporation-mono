//! Core execution enumerations.

use serde::{Deserialize, Serialize};

// ── IntentStatus ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    Pending,
    Evaluated,
    Authorized,
    Executed,
    Failed,
    Cancelled,
}

// ── ObligationStatus ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationStatus {
    Required,
    InProgress,
    Fulfilled,
    Waived,
    Expired,
}

// ── AssigneeType ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssigneeType {
    Internal,
    ThirdParty,
    Human,
}

// ── ReceiptStatus ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    Pending,
    Executed,
    Failed,
}

// ── DocumentRequestStatus ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentRequestStatus {
    Requested,
    Provided,
    NotApplicable,
    Waived,
}
