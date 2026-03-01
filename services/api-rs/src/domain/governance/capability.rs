//! Typed governance capability identifiers.
//!
//! This enum is the Rust-side typed boundary for governance policy capabilities.
//! Runtime policy remains defined by `governance/ast/v1/governance-ast.json`,
//! and tests here enforce parity with AST capability keys.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GovernanceCapability {
    MaintainBooksRecords,
    PrepareComplianceDocs,
    PayRecurringObligation,
    AuthorizeExpenditure,
    RoutineCorrespondence,
    InformationGathering,
    ComplianceDeadlineTracking,
    ExecuteStandardFormAgreement,
    InternalAccountTransfer,
    PayrollExecution,
    TaxPaymentPerFiling,
    RegisteredAgentRenewal,
    FinancialCommitmentAboveLimits,
    NewContract,
    MaterialAmendment,
    HireEmployee,
    EngageContractor,
    TaxElection,
    AccountingMethodChange,
    EquityCommunication,
    BankAccountOpenClose,
    AmbiguousNovelAction,
    LegalClaimResponse,
    FranchiseTaxMethodChoice,
    AmendCharter,
    AmendGovernanceDocs,
    IssueEquity,
    ModifyAgentFramework,
    DissolveEntity,
    MergeConsolidate,
    SellSubstantiallyAllAssets,
    PersonalGuarantee,
    RemoveReplaceAgent,
    InitiateSettleLitigation,
    DeclareDividends,
    AdmitNewMembers,
    EquityRoundAccept,
    EquityRoundExecuteConversion,
    EquityTransferPrepare,
    EquityTransferExecute,
    EquityFundraisingPrepare,
    EquityFundraisingAccept,
    EquityFundraisingClose,
}

impl GovernanceCapability {
    pub const ALL: [Self; 43] = [
        Self::MaintainBooksRecords,
        Self::PrepareComplianceDocs,
        Self::PayRecurringObligation,
        Self::AuthorizeExpenditure,
        Self::RoutineCorrespondence,
        Self::InformationGathering,
        Self::ComplianceDeadlineTracking,
        Self::ExecuteStandardFormAgreement,
        Self::InternalAccountTransfer,
        Self::PayrollExecution,
        Self::TaxPaymentPerFiling,
        Self::RegisteredAgentRenewal,
        Self::FinancialCommitmentAboveLimits,
        Self::NewContract,
        Self::MaterialAmendment,
        Self::HireEmployee,
        Self::EngageContractor,
        Self::TaxElection,
        Self::AccountingMethodChange,
        Self::EquityCommunication,
        Self::BankAccountOpenClose,
        Self::AmbiguousNovelAction,
        Self::LegalClaimResponse,
        Self::FranchiseTaxMethodChoice,
        Self::AmendCharter,
        Self::AmendGovernanceDocs,
        Self::IssueEquity,
        Self::ModifyAgentFramework,
        Self::DissolveEntity,
        Self::MergeConsolidate,
        Self::SellSubstantiallyAllAssets,
        Self::PersonalGuarantee,
        Self::RemoveReplaceAgent,
        Self::InitiateSettleLitigation,
        Self::DeclareDividends,
        Self::AdmitNewMembers,
        Self::EquityRoundAccept,
        Self::EquityRoundExecuteConversion,
        Self::EquityTransferPrepare,
        Self::EquityTransferExecute,
        Self::EquityFundraisingPrepare,
        Self::EquityFundraisingAccept,
        Self::EquityFundraisingClose,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::MaintainBooksRecords => "maintain_books_records",
            Self::PrepareComplianceDocs => "prepare_compliance_docs",
            Self::PayRecurringObligation => "pay_recurring_obligation",
            Self::AuthorizeExpenditure => "authorize_expenditure",
            Self::RoutineCorrespondence => "routine_correspondence",
            Self::InformationGathering => "information_gathering",
            Self::ComplianceDeadlineTracking => "compliance_deadline_tracking",
            Self::ExecuteStandardFormAgreement => "execute_standard_form_agreement",
            Self::InternalAccountTransfer => "internal_account_transfer",
            Self::PayrollExecution => "payroll_execution",
            Self::TaxPaymentPerFiling => "tax_payment_per_filing",
            Self::RegisteredAgentRenewal => "registered_agent_renewal",
            Self::FinancialCommitmentAboveLimits => "financial_commitment_above_limits",
            Self::NewContract => "new_contract",
            Self::MaterialAmendment => "material_amendment",
            Self::HireEmployee => "hire_employee",
            Self::EngageContractor => "engage_contractor",
            Self::TaxElection => "tax_election",
            Self::AccountingMethodChange => "accounting_method_change",
            Self::EquityCommunication => "equity_communication",
            Self::BankAccountOpenClose => "bank_account_open_close",
            Self::AmbiguousNovelAction => "ambiguous_novel_action",
            Self::LegalClaimResponse => "legal_claim_response",
            Self::FranchiseTaxMethodChoice => "franchise_tax_method_choice",
            Self::AmendCharter => "amend_charter",
            Self::AmendGovernanceDocs => "amend_governance_docs",
            Self::IssueEquity => "issue_equity",
            Self::ModifyAgentFramework => "modify_agent_framework",
            Self::DissolveEntity => "dissolve_entity",
            Self::MergeConsolidate => "merge_consolidate",
            Self::SellSubstantiallyAllAssets => "sell_substantially_all_assets",
            Self::PersonalGuarantee => "personal_guarantee",
            Self::RemoveReplaceAgent => "remove_replace_agent",
            Self::InitiateSettleLitigation => "initiate_settle_litigation",
            Self::DeclareDividends => "declare_dividends",
            Self::AdmitNewMembers => "admit_new_members",
            Self::EquityRoundAccept => "equity.round.accept",
            Self::EquityRoundExecuteConversion => "equity.round.execute_conversion",
            Self::EquityTransferPrepare => "equity.transfer.prepare",
            Self::EquityTransferExecute => "equity.transfer.execute",
            Self::EquityFundraisingPrepare => "equity.fundraising.prepare",
            Self::EquityFundraisingAccept => "equity.fundraising.accept",
            Self::EquityFundraisingClose => "equity.fundraising.close",
        }
    }
}

impl fmt::Display for GovernanceCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GovernanceCapability {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "maintain_books_records" => Ok(Self::MaintainBooksRecords),
            "prepare_compliance_docs" => Ok(Self::PrepareComplianceDocs),
            "pay_recurring_obligation" => Ok(Self::PayRecurringObligation),
            "authorize_expenditure" => Ok(Self::AuthorizeExpenditure),
            "routine_correspondence" => Ok(Self::RoutineCorrespondence),
            "information_gathering" => Ok(Self::InformationGathering),
            "compliance_deadline_tracking" => Ok(Self::ComplianceDeadlineTracking),
            "execute_standard_form_agreement" => Ok(Self::ExecuteStandardFormAgreement),
            "internal_account_transfer" => Ok(Self::InternalAccountTransfer),
            "payroll_execution" => Ok(Self::PayrollExecution),
            "tax_payment_per_filing" => Ok(Self::TaxPaymentPerFiling),
            "registered_agent_renewal" => Ok(Self::RegisteredAgentRenewal),
            "financial_commitment_above_limits" => Ok(Self::FinancialCommitmentAboveLimits),
            "new_contract" => Ok(Self::NewContract),
            "material_amendment" => Ok(Self::MaterialAmendment),
            "hire_employee" => Ok(Self::HireEmployee),
            "engage_contractor" => Ok(Self::EngageContractor),
            "tax_election" => Ok(Self::TaxElection),
            "accounting_method_change" => Ok(Self::AccountingMethodChange),
            "equity_communication" => Ok(Self::EquityCommunication),
            "bank_account_open_close" => Ok(Self::BankAccountOpenClose),
            "ambiguous_novel_action" => Ok(Self::AmbiguousNovelAction),
            "legal_claim_response" => Ok(Self::LegalClaimResponse),
            "franchise_tax_method_choice" => Ok(Self::FranchiseTaxMethodChoice),
            "amend_charter" => Ok(Self::AmendCharter),
            "amend_governance_docs" => Ok(Self::AmendGovernanceDocs),
            "issue_equity" => Ok(Self::IssueEquity),
            "modify_agent_framework" => Ok(Self::ModifyAgentFramework),
            "dissolve_entity" => Ok(Self::DissolveEntity),
            "merge_consolidate" => Ok(Self::MergeConsolidate),
            "sell_substantially_all_assets" => Ok(Self::SellSubstantiallyAllAssets),
            "personal_guarantee" => Ok(Self::PersonalGuarantee),
            "remove_replace_agent" => Ok(Self::RemoveReplaceAgent),
            "initiate_settle_litigation" => Ok(Self::InitiateSettleLitigation),
            "declare_dividends" => Ok(Self::DeclareDividends),
            "admit_new_members" => Ok(Self::AdmitNewMembers),
            "equity.round.accept" => Ok(Self::EquityRoundAccept),
            "equity.round.execute_conversion" => Ok(Self::EquityRoundExecuteConversion),
            "equity.transfer.prepare" => Ok(Self::EquityTransferPrepare),
            "equity.transfer.execute" => Ok(Self::EquityTransferExecute),
            "equity.fundraising.prepare" => Ok(Self::EquityFundraisingPrepare),
            "equity.fundraising.accept" => Ok(Self::EquityFundraisingAccept),
            "equity.fundraising.close" => Ok(Self::EquityFundraisingClose),
            _ => Err(format!("unknown governance capability: {value}")),
        }
    }
}

impl Serialize for GovernanceCapability {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for GovernanceCapability {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse::<GovernanceCapability>()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::domain::governance::policy_ast::default_governance_ast;

    #[test]
    fn roundtrip_all_capabilities() {
        for capability in GovernanceCapability::ALL {
            let serialized = capability.as_str();
            let parsed = serialized
                .parse::<GovernanceCapability>()
                .expect("capability should parse");
            assert_eq!(parsed, capability);
            assert_eq!(parsed.to_string(), serialized);
        }
    }

    #[test]
    fn ast_tier_defaults_match_rust_capability_set() {
        let ast = default_governance_ast();
        let ast_caps = ast
            .rules
            .tier_defaults
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        let rust_caps = GovernanceCapability::ALL
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            ast_caps, rust_caps,
            "AST tier_defaults keys and Rust capability enum must stay in sync"
        );
    }

    #[test]
    fn ast_non_delegable_are_known_capabilities() {
        // With typed AST fields, non_delegable entries are already
        // GovernanceCapability values — deserialization would fail on
        // unknown capabilities. This test confirms the structural invariant.
        let ast = default_governance_ast();
        for capability in &ast.rules.non_delegable {
            assert!(
                GovernanceCapability::ALL.contains(capability),
                "non_delegable capability {capability} is not in ALL"
            );
        }
    }
}
