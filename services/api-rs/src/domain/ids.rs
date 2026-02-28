//! Newtype IDs — prevent ID confusion at compile time.
//!
//! Every domain ID is a distinct type. `EntityId` cannot be passed where
//! `AccountId` is expected, even though both wrap `Uuid`.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Generate a newtype wrapper around `Uuid` with Display, FromStr, Serialize, etc.
macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Create a new random ID (UUID v4).
            #[inline]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Wrap an existing UUID.
            #[inline]
            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            /// Borrow the inner UUID.
            #[inline]
            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Consume and return the inner UUID.
            #[inline]
            pub fn into_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse()?))
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

// ── Formation ──────────────────────────────────────────────────────────
define_id!(EntityId);
define_id!(FormationId);
define_id!(DocumentId);
define_id!(SignatureId);
define_id!(TaxProfileId);
define_id!(FilingId);

// ── Treasury ───────────────────────────────────────────────────────────
define_id!(AccountId);
define_id!(JournalEntryId);
define_id!(InvoiceId);
define_id!(PaymentId);
define_id!(PayoutId);
define_id!(BankAccountId);
define_id!(KybPackageId);
define_id!(SpendingLimitId);
define_id!(LedgerLineId);

// ── Equity ─────────────────────────────────────────────────────────────
define_id!(CapTableId);
define_id!(ShareClassId);
define_id!(EquityGrantId);
define_id!(VestingScheduleId);
define_id!(VestingEventId);
define_id!(SafeNoteId);
define_id!(FundingRoundId);
define_id!(ValuationId);
define_id!(TransferId);
define_id!(RepurchaseRightId);
define_id!(InvestorLedgerEntryId);
define_id!(HolderId);
define_id!(LegalEntityId);
define_id!(ControlLinkId);
define_id!(InstrumentId);
define_id!(PositionId);
define_id!(EquityRoundId);
define_id!(EquityRuleSetId);
define_id!(ConversionExecutionId);
define_id!(TransferWorkflowId);
define_id!(FundraisingWorkflowId);

// ── Governance ─────────────────────────────────────────────────────────
define_id!(GovernanceBodyId);
define_id!(GovernanceSeatId);
define_id!(MeetingId);
define_id!(AgendaItemId);
define_id!(VoteId);
define_id!(ResolutionId);
define_id!(IncidentId);
define_id!(ScheduleAmendmentId);

// ── Contacts & Obligations ─────────────────────────────────────────────
define_id!(ContactId);
define_id!(ObligationId);
define_id!(DocumentRequestId);

// ── Execution ──────────────────────────────────────────────────────────
define_id!(IntentId);
define_id!(ApprovalArtifactId);
define_id!(PacketId);
define_id!(PacketSignatureId);
// ExecutionId is re-exported from agent_types above.
define_id!(ReceiptId);

// ── Contracts & Compliance ────────────────────────────────────────────
define_id!(ContractId);
define_id!(TaxFilingId);
define_id!(DeadlineId);
define_id!(ComplianceEscalationId);
define_id!(ComplianceEvidenceLinkId);
define_id!(ClassificationId);

// ── Treasury: Payments, Payroll, Distributions ───────────────────────
define_id!(PayrollRunId);
define_id!(DistributionId);
define_id!(ReconciliationId);

// ── Services (fulfillment marketplace) ─────────────────────────────────
define_id!(ServiceItemId);
define_id!(ServiceRequestId);

// ── Auth & Workspace ───────────────────────────────────────────────────
// AgentId, WorkspaceId, ExecutionId, MessageId come from the shared crate
// so that api-rs and agent-worker use the exact same types.
#[allow(unused_imports)] // ExecutionId not yet used in api-rs routes
pub use agent_types::{AgentId, ExecutionId, MessageId, WorkspaceId};

define_id!(ApiKeyId);
define_id!(SubscriptionId);
define_id!(StripeConnectionId);
define_id!(NotificationPrefsId);
define_id!(AuditEventId);
define_id!(CopyRequestId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_id_types_are_not_interchangeable() {
        let entity_id = EntityId::new();
        let account_id = AccountId::from_uuid(entity_id.into_uuid());
        // They wrap the same UUID but are different types.
        assert_eq!(entity_id.into_uuid(), account_id.into_uuid());
        // This would not compile: entity_id == account_id
    }

    #[test]
    fn roundtrip_serde() {
        let id = EntityId::new();
        let json = serde_json::to_string(&id).expect("serialize EntityId");
        let parsed: EntityId = serde_json::from_str(&json).expect("deserialize EntityId");
        assert_eq!(id, parsed);
    }

    #[test]
    fn roundtrip_serde_all_id_types() {
        // Verify a representative sample of ID types roundtrip correctly.
        let workspace = WorkspaceId::new();
        let json = serde_json::to_string(&workspace).expect("serialize WorkspaceId");
        let parsed: WorkspaceId = serde_json::from_str(&json).expect("deserialize WorkspaceId");
        assert_eq!(workspace, parsed);

        let grant = EquityGrantId::new();
        let json = serde_json::to_string(&grant).expect("serialize EquityGrantId");
        let parsed: EquityGrantId = serde_json::from_str(&json).expect("deserialize EquityGrantId");
        assert_eq!(grant, parsed);
    }

    #[test]
    fn from_str_roundtrip() {
        let id = EntityId::new();
        let s = id.to_string();
        let parsed: EntityId = s.parse().expect("parse EntityId from string");
        assert_eq!(id, parsed);
    }

    #[test]
    fn from_str_invalid() {
        let result = "not-a-uuid".parse::<EntityId>();
        assert!(result.is_err());
    }

    #[test]
    fn display_matches_inner_uuid() {
        let uuid = Uuid::new_v4();
        let id = MeetingId::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn from_and_into_uuid() {
        let uuid = Uuid::new_v4();
        let id: AccountId = uuid.into();
        let back: Uuid = id.into();
        assert_eq!(uuid, back);
    }
}
