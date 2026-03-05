//! Equity instruments (stored as `cap-table/instruments/{instrument_id}.json`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::ids::{InstrumentId, LegalEntityId};

/// Instrument kind in the ownership model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    CommonEquity,
    PreferredEquity,
    MembershipUnit,
    OptionGrant,
    Safe,
    ConvertibleNote,
    Warrant,
}

/// Lifecycle status of the instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentStatus {
    Active,
    Closed,
    Cancelled,
}

/// Canonical equity instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    instrument_id: InstrumentId,
    issuer_legal_entity_id: LegalEntityId,
    symbol: String,
    kind: InstrumentKind,
    authorized_units: Option<i64>,
    issue_price_cents: Option<i64>,
    /// Flexible legal/economic terms payload.
    terms: serde_json::Value,
    status: InstrumentStatus,
    created_at: DateTime<Utc>,
}

impl Instrument {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: InstrumentId,
        issuer_legal_entity_id: LegalEntityId,
        symbol: String,
        kind: InstrumentKind,
        authorized_units: Option<i64>,
        issue_price_cents: Option<i64>,
        terms: serde_json::Value,
    ) -> Self {
        Self {
            instrument_id,
            issuer_legal_entity_id,
            symbol,
            kind,
            authorized_units,
            issue_price_cents,
            terms,
            status: InstrumentStatus::Active,
            created_at: Utc::now(),
        }
    }

    pub fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    pub fn issuer_legal_entity_id(&self) -> LegalEntityId {
        self.issuer_legal_entity_id
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn kind(&self) -> InstrumentKind {
        self.kind
    }

    pub fn authorized_units(&self) -> Option<i64> {
        self.authorized_units
    }

    pub fn issue_price_cents(&self) -> Option<i64> {
        self.issue_price_cents
    }

    pub fn terms(&self) -> &serde_json::Value {
        &self.terms
    }

    pub fn status(&self) -> InstrumentStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn set_status(&mut self, status: InstrumentStatus) {
        self.status = status;
    }
}
