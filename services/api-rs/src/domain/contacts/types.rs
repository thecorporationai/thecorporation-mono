//! Contacts domain types — people and organizations that interact with the entity.

use serde::{Deserialize, Serialize};

// ── ContactType ────────────────────────────────────────────────────────

/// Whether a contact is a person or an organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactType {
    /// A natural person.
    Individual,
    /// A company, firm, or other organization.
    Organization,
}

// ── ContactCategory ────────────────────────────────────────────────────

/// The role or relationship a contact has with the entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactCategory {
    /// W-2 employee.
    Employee,
    /// Independent contractor (1099).
    Contractor,
    /// Member of the board of directors.
    BoardMember,
    /// External law firm.
    LawFirm,
    /// 409A or other valuation firm.
    ValuationFirm,
    /// CPA or accounting firm.
    AccountingFirm,
    /// Equity investor.
    Investor,
    /// Corporate officer (CEO, CFO, etc.).
    Officer,
    /// Other relationship.
    Other,
}

// ── ContactStatus ──────────────────────────────────────────────────────

/// Whether a contact record is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactStatus {
    /// Contact is active and current.
    Active,
    /// Contact is no longer active (departed, terminated, etc.).
    Inactive,
}

// ── CapTableAccess ─────────────────────────────────────────────────────

/// Level of cap table visibility granted to a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapTableAccess {
    /// No cap table access.
    #[serde(rename = "none")]
    None_,
    /// Can see summary totals only.
    Summary,
    /// Can see full cap table detail.
    Detailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contact_type_serde() {
        let ct = ContactType::Individual;
        let json = serde_json::to_string(&ct).expect("serialize ContactType");
        assert_eq!(json, "\"individual\"");
        let parsed: ContactType = serde_json::from_str(&json).expect("deserialize ContactType");
        assert_eq!(ct, parsed);
    }

    #[test]
    fn contact_category_serde() {
        let cc = ContactCategory::ValuationFirm;
        let json = serde_json::to_string(&cc).expect("serialize ContactCategory");
        assert_eq!(json, "\"valuation_firm\"");
        let parsed: ContactCategory =
            serde_json::from_str(&json).expect("deserialize ContactCategory");
        assert_eq!(cc, parsed);
    }

    #[test]
    fn cap_table_access_none_serde() {
        let access = CapTableAccess::None_;
        let json = serde_json::to_string(&access).expect("serialize CapTableAccess::None_");
        assert_eq!(json, "\"none\"");
        let parsed: CapTableAccess =
            serde_json::from_str("\"none\"").expect("deserialize CapTableAccess::None_");
        assert_eq!(parsed, CapTableAccess::None_);
    }

    #[test]
    fn cap_table_access_detailed_serde() {
        let access = CapTableAccess::Detailed;
        let json = serde_json::to_string(&access).expect("serialize CapTableAccess::Detailed");
        assert_eq!(json, "\"detailed\"");
        let parsed: CapTableAccess =
            serde_json::from_str(&json).expect("deserialize CapTableAccess::Detailed");
        assert_eq!(parsed, CapTableAccess::Detailed);
    }

    #[test]
    fn contact_status_serde() {
        let status = ContactStatus::Inactive;
        let json = serde_json::to_string(&status).expect("serialize ContactStatus");
        assert_eq!(json, "\"inactive\"");
    }

    #[test]
    fn all_categories_roundtrip() {
        let categories = [
            ContactCategory::Employee,
            ContactCategory::Contractor,
            ContactCategory::BoardMember,
            ContactCategory::LawFirm,
            ContactCategory::ValuationFirm,
            ContactCategory::AccountingFirm,
            ContactCategory::Investor,
            ContactCategory::Officer,
            ContactCategory::Other,
        ];
        for cat in &categories {
            let json = serde_json::to_string(cat).expect("serialize ContactCategory variant");
            let parsed: ContactCategory =
                serde_json::from_str(&json).expect("deserialize ContactCategory variant");
            assert_eq!(cat, &parsed);
        }
    }
}
