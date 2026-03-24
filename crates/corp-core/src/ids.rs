//! Typed UUID newtypes for all domain entities.
//!
//! Every ID is a zero-cost newtype around `uuid::Uuid`. Use `define_id!` to add new
//! ID types; the macro derives every trait needed for use as map keys, JSON fields,
//! and URL path segments.

// std and serde are referenced inside macro bodies via fully-qualified paths
// so the macro remains hygienic when used in external crates or doctests.

// ── Macro ────────────────────────────────────────────────────────────────────

/// Generate a typed UUID newtype with the full set of standard trait impls.
///
/// ```
/// use corp_core::define_id;
/// // uuid must be in scope because the macro references `uuid::Uuid` by path.
/// use uuid;
/// define_id!(MyId);
/// let id = MyId::new();
/// let roundtrip: MyId = id.to_string().parse().unwrap();
/// assert_eq!(id, roundtrip);
/// ```
#[macro_export]
macro_rules! define_id {
    // Named alias: `define_id!(Foo as Bar)` emits `pub type Bar = Foo;`
    ($existing:ident as $alias:ident) => {
        pub type $alias = $existing;
    };

    // Plain form: `define_id!(Foo)` emits the full newtype.
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
        pub struct $name(uuid::Uuid);

        impl $name {
            /// Generate a new random (v4) ID.
            #[inline]
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4())
            }

            /// Wrap an existing `Uuid`.
            #[inline]
            pub fn from_uuid(u: uuid::Uuid) -> Self {
                Self(u)
            }

            /// Return the inner `Uuid`.
            #[inline]
            pub fn as_uuid(&self) -> uuid::Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse()?))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0.to_string())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = <String as serde::Deserialize>::deserialize(deserializer)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

// Re-export so callers inside this crate can use the macro without the
// crate-level path prefix.
pub use define_id;

// ── Formation ────────────────────────────────────────────────────────────────

define_id!(EntityId);
define_id!(DocumentId);
define_id!(SignatureId);
define_id!(TaxProfileId);
define_id!(FilingId);
define_id!(ContractId);

// ── Treasury ─────────────────────────────────────────────────────────────────

define_id!(AccountId);
define_id!(JournalEntryId);
define_id!(InvoiceId);
define_id!(PaymentId);
define_id!(PayoutId);
define_id!(BankAccountId);
define_id!(SpendingLimitId);
define_id!(PayrollRunId);
define_id!(DistributionId);
define_id!(ReconciliationId);

// ── Equity ───────────────────────────────────────────────────────────────────

define_id!(CapTableId);
define_id!(ShareClassId);
define_id!(EquityGrantId);
define_id!(VestingScheduleId);
define_id!(SafeNoteId);
define_id!(FundingRoundId);
define_id!(ValuationId);
define_id!(TransferId);
define_id!(HolderId);
define_id!(InstrumentId);
define_id!(PositionId);
define_id!(EquityRoundId);
define_id!(EquityRuleSetId);
define_id!(ConversionExecutionId);
define_id!(TransferWorkflowId);
define_id!(FundraisingWorkflowId);
define_id!(VestingEventId);
define_id!(InvestorLedgerEntryId);
define_id!(LegalEntityId);
define_id!(ControlLinkId);
define_id!(RepurchaseRightId);

// ── Governance ───────────────────────────────────────────────────────────────

define_id!(GovernanceBodyId);
define_id!(GovernanceSeatId);
define_id!(MeetingId);
define_id!(AgendaItemId);
define_id!(VoteId);
define_id!(ResolutionId);
define_id!(IncidentId);
define_id!(ScheduleAmendmentId);
define_id!(GovernanceDocBundleId);
define_id!(GovernanceAuditEntryId);
define_id!(GovernanceAuditCheckpointId);

// ── Contacts ─────────────────────────────────────────────────────────────────

define_id!(ContactId);

// ── Execution ────────────────────────────────────────────────────────────────

define_id!(IntentId);
define_id!(ObligationId);
define_id!(ApprovalArtifactId);
define_id!(PacketId);
define_id!(ReceiptId);
define_id!(DocumentRequestId);

// ── Work Items ───────────────────────────────────────────────────────────────

define_id!(WorkItemId);

// ── Services ─────────────────────────────────────────────────────────────────

define_id!(ServiceItemId);
define_id!(ServiceRequestId);

// ── Agents ───────────────────────────────────────────────────────────────────

define_id!(AgentId);
define_id!(AgentExecutionId);
define_id!(AgentMessageId);

// ── Auth ─────────────────────────────────────────────────────────────────────

define_id!(WorkspaceId);
define_id!(ApiKeyId);
define_id!(SshKeyId);

// ── Compliance ───────────────────────────────────────────────────────────────

define_id!(TaxFilingId);
define_id!(DeadlineId);
define_id!(ComplianceEscalationId);
define_id!(ClassificationId);

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_roundtrip_display_parse() {
        let id = EntityId::new();
        let s = id.to_string();
        let parsed: EntityId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn entity_id_roundtrip_json() {
        let id = EntityId::new();
        let json = serde_json::to_string(&id).unwrap();
        // Should be a quoted UUID string, not an object.
        assert!(json.starts_with('"'));
        let de: EntityId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, de);
    }

    #[test]
    fn from_uuid_as_uuid() {
        let raw = uuid::Uuid::new_v4();
        let id = WorkspaceId::from_uuid(raw);
        assert_eq!(id.as_uuid(), raw);
    }

    #[test]
    fn different_id_types_are_distinct() {
        // Type-system check: EntityId and DocumentId are different types.
        // This test merely confirms they compile and produce distinct values.
        let a = EntityId::new();
        let b = DocumentId::new();
        assert_ne!(a.to_string(), b.to_string()); // overwhelmingly true for v4
    }

    #[test]
    fn ordering_and_hash() {
        use std::collections::BTreeSet;
        let ids: BTreeSet<_> = (0..5).map(|_| AgentId::new()).collect();
        assert_eq!(ids.len(), 5);
    }

    // ── Additional define_id! macro coverage ──────────────────────────────────

    #[test]
    fn workspace_id_roundtrip_display_parse() {
        let id = WorkspaceId::new();
        let s = id.to_string();
        let parsed: WorkspaceId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn document_id_roundtrip_display_parse() {
        let id = DocumentId::new();
        let s = id.to_string();
        let parsed: DocumentId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn account_id_roundtrip_json() {
        let id = AccountId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
        let de: AccountId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, de);
    }

    #[test]
    fn invoice_id_roundtrip_json() {
        let id = InvoiceId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
        let de: InvoiceId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, de);
    }

    #[test]
    fn contact_id_roundtrip_json() {
        let id = ContactId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
        let de: ContactId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, de);
    }

    #[test]
    fn intent_id_roundtrip_display_parse() {
        let id = IntentId::new();
        let s = id.to_string();
        let parsed: IntentId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn work_item_id_roundtrip_json() {
        let id = WorkItemId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
        let de: WorkItemId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, de);
    }

    #[test]
    fn service_request_id_roundtrip_json() {
        let id = ServiceRequestId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
        let de: ServiceRequestId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, de);
    }

    #[test]
    fn new_generates_unique_ids() {
        // Each call to new() should produce a different UUID
        let a = EntityId::new();
        let b = EntityId::new();
        let c = EntityId::new();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    #[test]
    fn from_uuid_roundtrip_as_uuid() {
        let raw = uuid::Uuid::new_v4();
        let entity = EntityId::from_uuid(raw);
        assert_eq!(entity.as_uuid(), raw);

        let contact = ContactId::from_uuid(raw);
        assert_eq!(contact.as_uuid(), raw);
    }

    #[test]
    fn display_format_is_uuid_string() {
        let id = EntityId::new();
        let s = id.to_string();
        // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx (36 chars with dashes)
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn json_serialization_is_string_not_object() {
        // Verify that ID is serialized as a plain JSON string, not {"0": ...}
        let id = AgentId::new();
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.starts_with('"'));
        assert!(json.ends_with('"'));
        assert!(!json.contains('{'));
        assert!(!json.contains('}'));
    }

    #[test]
    fn entity_id_and_document_id_are_distinct_types() {
        // Type-level check: cannot assign one to the other.
        // Runtime: they are separate newtypes around Uuid.
        let entity_id = EntityId::new();
        let doc_id = DocumentId::new();
        // Both display as UUIDs but are different types
        assert_ne!(entity_id.to_string(), doc_id.to_string()); // very likely different v4 UUIDs
        // Confirm same underlying UUID wraps identically
        let raw = uuid::Uuid::new_v4();
        let e = EntityId::from_uuid(raw);
        let d = DocumentId::from_uuid(raw);
        assert_eq!(e.as_uuid(), d.as_uuid()); // same raw UUID
        assert_eq!(e.to_string(), d.to_string()); // same display
    }

    #[test]
    fn ids_are_hashable() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for _ in 0..10 {
            set.insert(WorkspaceId::new());
        }
        assert_eq!(set.len(), 10);
    }

    #[test]
    fn ids_default_generates_new_id() {
        let a = EntityId::default();
        let b = EntityId::default();
        // default() calls new() which generates random UUID
        assert_ne!(a, b);
    }

    #[test]
    fn parse_invalid_uuid_returns_error() {
        let result: Result<EntityId, _> = "not-a-uuid".parse();
        assert!(result.is_err());
    }
}
