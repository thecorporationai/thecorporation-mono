//! Agent capability taxonomy — what actions an agent may perform and at what tier.

use serde::{Deserialize, Serialize};

// ── AuthorityTier ─────────────────────────────────────────────────────────────

/// The authority level required to authorise a governance action.
///
/// Tiers are ordered: Tier1 < Tier2 < Tier3. Higher tiers require greater
/// oversight (board approval, stockholder vote, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityTier {
    /// Routine operational actions an authorised agent may take autonomously.
    Tier1,
    /// Actions that require officer or designated-approver review.
    Tier2,
    /// Actions that require board, member, or stockholder approval.
    Tier3,
}

impl AuthorityTier {
    /// Numeric level (1, 2, or 3).
    pub fn level(self) -> u8 {
        match self {
            AuthorityTier::Tier1 => 1,
            AuthorityTier::Tier2 => 2,
            AuthorityTier::Tier3 => 3,
        }
    }

    /// Construct from a numeric level. Returns `None` for out-of-range values.
    pub fn from_level(level: u8) -> Option<Self> {
        match level {
            1 => Some(AuthorityTier::Tier1),
            2 => Some(AuthorityTier::Tier2),
            3 => Some(AuthorityTier::Tier3),
            _ => None,
        }
    }
}

// ── GovernanceCapability ──────────────────────────────────────────────────────

/// An atomic action an agent may be authorised to perform.
///
/// Capabilities are grouped by tier. The [`default_tier`] function returns the
/// canonical tier for each capability; individual policy overrides may deviate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceCapability {
    // ── Tier 1 ────────────────────────────────────────────────────────────────
    /// Maintain and update corporate books and records.
    MaintainBooksRecords,
    /// Prepare standard compliance documents (filings, reports, certificates).
    PrepareComplianceDocs,
    /// Pay a recurring obligation (rent, subscriptions, utilities).
    PayRecurringObligation,
    /// Authorise an expenditure within pre-approved spending limits.
    AuthorizeExpenditure,
    /// Send routine correspondence on behalf of the entity.
    RoutineCorrespondence,
    /// Gather information (data queries, records requests, due-diligence).
    InformationGathering,
    /// Track and surface upcoming compliance deadlines.
    ComplianceDeadlineTracking,
    /// Execute a standard form agreement with no material deviations.
    ExecuteStandardFormAgreement,
    /// Transfer funds between internal accounts within limits.
    InternalAccountTransfer,
    /// Execute a payroll run per an approved payroll schedule.
    PayrollExecution,
    /// Remit a tax payment per an existing filing.
    TaxPaymentPerFiling,
    /// Renew the entity's registered agent.
    RegisteredAgentRenewal,

    // ── Tier 2 ────────────────────────────────────────────────────────────────
    /// Make a financial commitment above pre-approved limits.
    FinancialCommitmentAboveLimits,
    /// Enter into a new contract not covered by a standard form.
    NewContract,
    /// Amend a material term of an existing contract or agreement.
    MaterialAmendment,
    /// Hire a new employee.
    HireEmployee,
    /// Engage a contractor outside standard rate cards.
    EngageContractor,
    /// Make or change a tax election.
    TaxElection,
    /// Change the entity's accounting method.
    AccountingMethodChange,
    /// Communicate with equity holders about their holdings.
    EquityCommunication,
    /// Open or close a bank or financial account.
    BankAccountOpenClose,
    /// Take an action that is ambiguous or novel with no clear precedent.
    AmbiguousNovelAction,
    /// Respond to a legal claim, demand letter, or regulatory inquiry.
    LegalClaimResponse,
    /// Choose an alternative franchise tax calculation method.
    FranchiseTaxMethodChoice,

    // ── Tier 3 ────────────────────────────────────────────────────────────────
    /// Amend the certificate of incorporation, articles, or charter.
    AmendCharter,
    /// Amend bylaws, operating agreement, or other governance documents.
    AmendGovernanceDocs,
    /// Issue new equity securities.
    IssueEquity,
    /// Modify the agent framework or add/remove agent capabilities.
    ModifyAgentFramework,
    /// Dissolve or wind up the entity.
    DissolveEntity,
    /// Merge with or consolidate into another entity.
    MergeConsolidate,
    /// Sell substantially all of the entity's assets.
    SellSubstantiallyAllAssets,
    /// Provide a personal guarantee on behalf of the entity.
    PersonalGuarantee,
    /// Remove or replace an authorised agent.
    RemoveReplaceAgent,
    /// Initiate or settle litigation.
    InitiateSettleLitigation,
    /// Declare a dividend or distribution.
    DeclareDividends,
    /// Admit new members to an LLC or partnership.
    AdmitNewMembers,
}

/// Return the default [`AuthorityTier`] for a given capability.
pub fn default_tier(cap: &GovernanceCapability) -> AuthorityTier {
    match cap {
        // Tier 1
        GovernanceCapability::MaintainBooksRecords
        | GovernanceCapability::PrepareComplianceDocs
        | GovernanceCapability::PayRecurringObligation
        | GovernanceCapability::AuthorizeExpenditure
        | GovernanceCapability::RoutineCorrespondence
        | GovernanceCapability::InformationGathering
        | GovernanceCapability::ComplianceDeadlineTracking
        | GovernanceCapability::ExecuteStandardFormAgreement
        | GovernanceCapability::InternalAccountTransfer
        | GovernanceCapability::PayrollExecution
        | GovernanceCapability::TaxPaymentPerFiling
        | GovernanceCapability::RegisteredAgentRenewal => AuthorityTier::Tier1,

        // Tier 2
        GovernanceCapability::FinancialCommitmentAboveLimits
        | GovernanceCapability::NewContract
        | GovernanceCapability::MaterialAmendment
        | GovernanceCapability::HireEmployee
        | GovernanceCapability::EngageContractor
        | GovernanceCapability::TaxElection
        | GovernanceCapability::AccountingMethodChange
        | GovernanceCapability::EquityCommunication
        | GovernanceCapability::BankAccountOpenClose
        | GovernanceCapability::AmbiguousNovelAction
        | GovernanceCapability::LegalClaimResponse
        | GovernanceCapability::FranchiseTaxMethodChoice => AuthorityTier::Tier2,

        // Tier 3
        GovernanceCapability::AmendCharter
        | GovernanceCapability::AmendGovernanceDocs
        | GovernanceCapability::IssueEquity
        | GovernanceCapability::ModifyAgentFramework
        | GovernanceCapability::DissolveEntity
        | GovernanceCapability::MergeConsolidate
        | GovernanceCapability::SellSubstantiallyAllAssets
        | GovernanceCapability::PersonalGuarantee
        | GovernanceCapability::RemoveReplaceAgent
        | GovernanceCapability::InitiateSettleLitigation
        | GovernanceCapability::DeclareDividends
        | GovernanceCapability::AdmitNewMembers => AuthorityTier::Tier3,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── AuthorityTier::level() ────────────────────────────────────────────────

    #[test]
    fn tier_levels() {
        assert_eq!(AuthorityTier::Tier1.level(), 1);
        assert_eq!(AuthorityTier::Tier2.level(), 2);
        assert_eq!(AuthorityTier::Tier3.level(), 3);
    }

    // ── AuthorityTier::from_level() ───────────────────────────────────────────

    #[test]
    fn from_level_valid() {
        assert_eq!(AuthorityTier::from_level(1), Some(AuthorityTier::Tier1));
        assert_eq!(AuthorityTier::from_level(2), Some(AuthorityTier::Tier2));
        assert_eq!(AuthorityTier::from_level(3), Some(AuthorityTier::Tier3));
    }

    #[test]
    fn from_level_zero_returns_none() {
        assert_eq!(AuthorityTier::from_level(0), None);
    }

    #[test]
    fn from_level_four_returns_none() {
        assert_eq!(AuthorityTier::from_level(4), None);
    }

    #[test]
    fn from_level_max_returns_none() {
        assert_eq!(AuthorityTier::from_level(u8::MAX), None);
    }

    #[test]
    fn tier_level_roundtrip() {
        for tier in [AuthorityTier::Tier1, AuthorityTier::Tier2, AuthorityTier::Tier3] {
            let level = tier.level();
            assert_eq!(AuthorityTier::from_level(level), Some(tier));
        }
        assert_eq!(AuthorityTier::from_level(0), None);
        assert_eq!(AuthorityTier::from_level(4), None);
    }

    // ── Tier ordering ─────────────────────────────────────────────────────────

    #[test]
    fn ordering() {
        assert!(AuthorityTier::Tier1 < AuthorityTier::Tier2);
        assert!(AuthorityTier::Tier2 < AuthorityTier::Tier3);
        assert!(AuthorityTier::Tier1 < AuthorityTier::Tier3);
    }

    #[test]
    fn tier_equality() {
        assert_eq!(AuthorityTier::Tier1, AuthorityTier::Tier1);
        assert_ne!(AuthorityTier::Tier1, AuthorityTier::Tier2);
    }

    #[test]
    fn tier_serde_roundtrip() {
        for tier in [AuthorityTier::Tier1, AuthorityTier::Tier2, AuthorityTier::Tier3] {
            let json = serde_json::to_string(&tier).unwrap();
            let back: AuthorityTier = serde_json::from_str(&json).unwrap();
            assert_eq!(tier, back);
        }
    }

    // ── default_tier(): Tier1 capabilities ───────────────────────────────────

    #[test]
    fn tier1_capabilities() {
        assert_eq!(
            default_tier(&GovernanceCapability::MaintainBooksRecords),
            AuthorityTier::Tier1
        );
        assert_eq!(
            default_tier(&GovernanceCapability::PayrollExecution),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_prepare_compliance_docs() {
        assert_eq!(
            default_tier(&GovernanceCapability::PrepareComplianceDocs),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_pay_recurring_obligation() {
        assert_eq!(
            default_tier(&GovernanceCapability::PayRecurringObligation),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_authorize_expenditure() {
        assert_eq!(
            default_tier(&GovernanceCapability::AuthorizeExpenditure),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_routine_correspondence() {
        assert_eq!(
            default_tier(&GovernanceCapability::RoutineCorrespondence),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_information_gathering() {
        assert_eq!(
            default_tier(&GovernanceCapability::InformationGathering),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_compliance_deadline_tracking() {
        assert_eq!(
            default_tier(&GovernanceCapability::ComplianceDeadlineTracking),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_execute_standard_form_agreement() {
        assert_eq!(
            default_tier(&GovernanceCapability::ExecuteStandardFormAgreement),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_internal_account_transfer() {
        assert_eq!(
            default_tier(&GovernanceCapability::InternalAccountTransfer),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_tax_payment_per_filing() {
        assert_eq!(
            default_tier(&GovernanceCapability::TaxPaymentPerFiling),
            AuthorityTier::Tier1
        );
    }

    #[test]
    fn tier1_registered_agent_renewal() {
        assert_eq!(
            default_tier(&GovernanceCapability::RegisteredAgentRenewal),
            AuthorityTier::Tier1
        );
    }

    // ── default_tier(): Tier2 capabilities ───────────────────────────────────

    #[test]
    fn tier2_capabilities() {
        assert_eq!(
            default_tier(&GovernanceCapability::NewContract),
            AuthorityTier::Tier2
        );
        assert_eq!(
            default_tier(&GovernanceCapability::HireEmployee),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_financial_commitment_above_limits() {
        assert_eq!(
            default_tier(&GovernanceCapability::FinancialCommitmentAboveLimits),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_material_amendment() {
        assert_eq!(
            default_tier(&GovernanceCapability::MaterialAmendment),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_engage_contractor() {
        assert_eq!(
            default_tier(&GovernanceCapability::EngageContractor),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_tax_election() {
        assert_eq!(
            default_tier(&GovernanceCapability::TaxElection),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_accounting_method_change() {
        assert_eq!(
            default_tier(&GovernanceCapability::AccountingMethodChange),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_equity_communication() {
        assert_eq!(
            default_tier(&GovernanceCapability::EquityCommunication),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_bank_account_open_close() {
        assert_eq!(
            default_tier(&GovernanceCapability::BankAccountOpenClose),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_ambiguous_novel_action() {
        assert_eq!(
            default_tier(&GovernanceCapability::AmbiguousNovelAction),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_legal_claim_response() {
        assert_eq!(
            default_tier(&GovernanceCapability::LegalClaimResponse),
            AuthorityTier::Tier2
        );
    }

    #[test]
    fn tier2_franchise_tax_method_choice() {
        assert_eq!(
            default_tier(&GovernanceCapability::FranchiseTaxMethodChoice),
            AuthorityTier::Tier2
        );
    }

    // ── default_tier(): Tier3 capabilities ───────────────────────────────────

    #[test]
    fn tier3_capabilities() {
        assert_eq!(
            default_tier(&GovernanceCapability::IssueEquity),
            AuthorityTier::Tier3
        );
        assert_eq!(
            default_tier(&GovernanceCapability::DissolveEntity),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_amend_charter() {
        assert_eq!(
            default_tier(&GovernanceCapability::AmendCharter),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_amend_governance_docs() {
        assert_eq!(
            default_tier(&GovernanceCapability::AmendGovernanceDocs),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_modify_agent_framework() {
        assert_eq!(
            default_tier(&GovernanceCapability::ModifyAgentFramework),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_merge_consolidate() {
        assert_eq!(
            default_tier(&GovernanceCapability::MergeConsolidate),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_sell_substantially_all_assets() {
        assert_eq!(
            default_tier(&GovernanceCapability::SellSubstantiallyAllAssets),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_personal_guarantee() {
        assert_eq!(
            default_tier(&GovernanceCapability::PersonalGuarantee),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_remove_replace_agent() {
        assert_eq!(
            default_tier(&GovernanceCapability::RemoveReplaceAgent),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_initiate_settle_litigation() {
        assert_eq!(
            default_tier(&GovernanceCapability::InitiateSettleLitigation),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_declare_dividends() {
        assert_eq!(
            default_tier(&GovernanceCapability::DeclareDividends),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn tier3_admit_new_members() {
        assert_eq!(
            default_tier(&GovernanceCapability::AdmitNewMembers),
            AuthorityTier::Tier3
        );
    }

    #[test]
    fn all_variants_have_a_tier() {
        // Exhaustive coverage: every defined variant must map to a tier without panicking.
        let caps = [
            GovernanceCapability::MaintainBooksRecords,
            GovernanceCapability::PrepareComplianceDocs,
            GovernanceCapability::PayRecurringObligation,
            GovernanceCapability::AuthorizeExpenditure,
            GovernanceCapability::RoutineCorrespondence,
            GovernanceCapability::InformationGathering,
            GovernanceCapability::ComplianceDeadlineTracking,
            GovernanceCapability::ExecuteStandardFormAgreement,
            GovernanceCapability::InternalAccountTransfer,
            GovernanceCapability::PayrollExecution,
            GovernanceCapability::TaxPaymentPerFiling,
            GovernanceCapability::RegisteredAgentRenewal,
            GovernanceCapability::FinancialCommitmentAboveLimits,
            GovernanceCapability::NewContract,
            GovernanceCapability::MaterialAmendment,
            GovernanceCapability::HireEmployee,
            GovernanceCapability::EngageContractor,
            GovernanceCapability::TaxElection,
            GovernanceCapability::AccountingMethodChange,
            GovernanceCapability::EquityCommunication,
            GovernanceCapability::BankAccountOpenClose,
            GovernanceCapability::AmbiguousNovelAction,
            GovernanceCapability::LegalClaimResponse,
            GovernanceCapability::FranchiseTaxMethodChoice,
            GovernanceCapability::AmendCharter,
            GovernanceCapability::AmendGovernanceDocs,
            GovernanceCapability::IssueEquity,
            GovernanceCapability::ModifyAgentFramework,
            GovernanceCapability::DissolveEntity,
            GovernanceCapability::MergeConsolidate,
            GovernanceCapability::SellSubstantiallyAllAssets,
            GovernanceCapability::PersonalGuarantee,
            GovernanceCapability::RemoveReplaceAgent,
            GovernanceCapability::InitiateSettleLitigation,
            GovernanceCapability::DeclareDividends,
            GovernanceCapability::AdmitNewMembers,
        ];
        // 12 Tier1 + 12 Tier2 + 12 Tier3 = 36 total
        assert_eq!(caps.len(), 36);
        for cap in &caps {
            let _ = default_tier(cap); // must not panic
        }
    }

    #[test]
    fn all_tier1_caps_count_12() {
        let caps = [
            GovernanceCapability::MaintainBooksRecords,
            GovernanceCapability::PrepareComplianceDocs,
            GovernanceCapability::PayRecurringObligation,
            GovernanceCapability::AuthorizeExpenditure,
            GovernanceCapability::RoutineCorrespondence,
            GovernanceCapability::InformationGathering,
            GovernanceCapability::ComplianceDeadlineTracking,
            GovernanceCapability::ExecuteStandardFormAgreement,
            GovernanceCapability::InternalAccountTransfer,
            GovernanceCapability::PayrollExecution,
            GovernanceCapability::TaxPaymentPerFiling,
            GovernanceCapability::RegisteredAgentRenewal,
        ];
        for cap in &caps {
            assert_eq!(default_tier(cap), AuthorityTier::Tier1, "{cap:?} should be Tier1");
        }
        assert_eq!(caps.len(), 12);
    }

    #[test]
    fn all_tier2_caps_count_12() {
        let caps = [
            GovernanceCapability::FinancialCommitmentAboveLimits,
            GovernanceCapability::NewContract,
            GovernanceCapability::MaterialAmendment,
            GovernanceCapability::HireEmployee,
            GovernanceCapability::EngageContractor,
            GovernanceCapability::TaxElection,
            GovernanceCapability::AccountingMethodChange,
            GovernanceCapability::EquityCommunication,
            GovernanceCapability::BankAccountOpenClose,
            GovernanceCapability::AmbiguousNovelAction,
            GovernanceCapability::LegalClaimResponse,
            GovernanceCapability::FranchiseTaxMethodChoice,
        ];
        for cap in &caps {
            assert_eq!(default_tier(cap), AuthorityTier::Tier2, "{cap:?} should be Tier2");
        }
        assert_eq!(caps.len(), 12);
    }

    #[test]
    fn all_tier3_caps_count_12() {
        let caps = [
            GovernanceCapability::AmendCharter,
            GovernanceCapability::AmendGovernanceDocs,
            GovernanceCapability::IssueEquity,
            GovernanceCapability::ModifyAgentFramework,
            GovernanceCapability::DissolveEntity,
            GovernanceCapability::MergeConsolidate,
            GovernanceCapability::SellSubstantiallyAllAssets,
            GovernanceCapability::PersonalGuarantee,
            GovernanceCapability::RemoveReplaceAgent,
            GovernanceCapability::InitiateSettleLitigation,
            GovernanceCapability::DeclareDividends,
            GovernanceCapability::AdmitNewMembers,
        ];
        for cap in &caps {
            assert_eq!(default_tier(cap), AuthorityTier::Tier3, "{cap:?} should be Tier3");
        }
        assert_eq!(caps.len(), 12);
    }

    #[test]
    fn capability_serde_roundtrip() {
        let cap = GovernanceCapability::IssueEquity;
        let json = serde_json::to_string(&cap).unwrap();
        let back: GovernanceCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, back);
    }
}
