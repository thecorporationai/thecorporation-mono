//! Equity domain: cap tables, share classes, grants, SAFEs, valuations,
//! transfers, funding rounds, holders, vesting, instruments, positions,
//! legal entities, control links, investor ledger, rule sets, and repurchase
//! rights.

pub mod cap_table;
pub mod control_link;
pub mod grant;
pub mod holder;
pub mod instrument;
pub mod investor_ledger;
pub mod legal_entity;
pub mod position;
pub mod repurchase;
pub mod round;
pub mod rule_set;
pub mod safe_note;
pub mod share_class;
pub mod transfer;
pub mod types;
pub mod valuation;
pub mod vesting;

// Convenience re-exports so callers can write `equity::CapTable` rather than
// `equity::cap_table::CapTable`.
pub use cap_table::CapTable;
pub use control_link::{ControlLink, ControlType};
pub use grant::EquityGrant;
pub use holder::{Holder, HolderType};
pub use instrument::{Instrument, InstrumentKind, InstrumentStatus};
pub use investor_ledger::InvestorLedgerEntry;
pub use legal_entity::{LegalEntity, LegalEntityRole};
pub use position::Position;
pub use repurchase::RepurchaseRight;
pub use round::FundingRound;
pub use rule_set::{AntiDilutionMethod, EquityRuleSet};
pub use safe_note::SafeNote;
pub use share_class::ShareClass;
pub use transfer::ShareTransfer;
pub use types::{
    CapTableStatus, FundingRoundStatus, GrantStatus, GrantType, InvestorLedgerEntryType,
    Percentage, PositionStatus, PricePerShare, RecipientType, RepurchaseStatus, SafeStatus,
    SafeType, ShareCount, StockType, TransferStatus, TransferType, ValuationCap,
    ValuationMethodology, ValuationStatus, ValuationType, VestingEventStatus, VestingEventType,
    VestingStatus, VotingRights,
};
pub use valuation::Valuation;
pub use vesting::{VestingEvent, VestingSchedule, materialize_vesting_events};
