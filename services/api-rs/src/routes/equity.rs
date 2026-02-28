//! Equity HTTP routes.
//!
//! Canonical cap-table operations for holders, legal entities, control links,
//! instruments, positions, rounds, and conversion previews/execution.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use super::AppState;
use crate::auth::{RequireEquityRead, RequireEquityWrite};
use crate::domain::equity::{
    control_link::{ControlLink, ControlType},
    conversion_execution::ConversionExecution,
    holder::{Holder, HolderType},
    instrument::{Instrument, InstrumentKind, InstrumentStatus},
    legal_entity::{LegalEntity, LegalEntityRole},
    position::{Position, PositionStatus},
    round::{EquityRound, EquityRoundStatus},
    rule_set::{AntiDilutionMethod, EquityRuleSet},
};
use crate::domain::execution::{intent::Intent, types::IntentStatus};
use crate::domain::governance::{
    body::GovernanceBody, meeting::Meeting, resolution::Resolution, types::BodyType,
};
use crate::domain::ids::{
    ContactId, ControlLinkId, ConversionExecutionId, EntityId, EquityRoundId, EquityRuleSetId,
    HolderId, InstrumentId, IntentId, LegalEntityId, MeetingId, PositionId, ResolutionId,
    WorkspaceId,
};
use crate::error::AppError;
use crate::git::commit::FileWrite;
use crate::store::entity_store::EntityStore;
use crate::store::stored_entity::StoredEntity;

// ── Queries ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CapTableBasis {
    #[default]
    Outstanding,
    AsConverted,
    FullyDiluted,
}

#[derive(Debug, Deserialize)]
pub struct CapTableQuery {
    #[serde(default)]
    pub basis: CapTableBasis,
    #[serde(default)]
    pub issuer_legal_entity_id: Option<LegalEntityId>,
}

#[derive(Debug, Deserialize)]
pub struct ControlMapQuery {
    pub entity_id: EntityId,
    pub root_entity_id: LegalEntityId,
}

#[derive(Debug, Deserialize)]
pub struct DilutionPreviewQuery {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
}

// ── Request types ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateHolderRequest {
    pub entity_id: EntityId,
    pub contact_id: ContactId,
    #[serde(default)]
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub holder_type: HolderType,
    #[serde(default)]
    pub external_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateLegalEntityRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub role: LegalEntityRole,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateControlLinkRequest {
    pub entity_id: EntityId,
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    #[serde(default)]
    pub voting_power_bps: Option<u32>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInstrumentRequest {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub symbol: String,
    pub kind: InstrumentKind,
    #[serde(default)]
    pub authorized_units: Option<i64>,
    #[serde(default)]
    pub issue_price_cents: Option<i64>,
    #[serde(default)]
    pub terms: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdjustPositionRequest {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub quantity_delta: i64,
    #[serde(default)]
    pub principal_delta_cents: i64,
    #[serde(default)]
    pub source_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRoundRequest {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub name: String,
    #[serde(default)]
    pub pre_money_cents: Option<i64>,
    #[serde(default)]
    pub round_price_cents: Option<i64>,
    #[serde(default)]
    pub target_raise_cents: Option<i64>,
    #[serde(default)]
    pub conversion_target_instrument_id: Option<InstrumentId>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyRoundTermsRequest {
    pub entity_id: EntityId,
    pub anti_dilution_method: AntiDilutionMethod,
    #[serde(default)]
    pub conversion_precedence: Vec<InstrumentKind>,
    #[serde(default)]
    pub protective_provisions: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardApproveRoundRequest {
    pub entity_id: EntityId,
    pub meeting_id: MeetingId,
    pub resolution_id: ResolutionId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptRoundRequest {
    pub entity_id: EntityId,
    pub intent_id: IntentId,
    #[serde(default)]
    pub accepted_by_contact_id: Option<ContactId>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewConversionRequest {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    #[serde(default)]
    pub source_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteConversionRequest {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    pub intent_id: IntentId,
    #[serde(default)]
    pub source_reference: Option<String>,
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct HolderResponse {
    pub holder_id: HolderId,
    pub contact_id: ContactId,
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub holder_type: HolderType,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct LegalEntityResponse {
    pub legal_entity_id: LegalEntityId,
    pub workspace_id: WorkspaceId,
    pub linked_entity_id: Option<EntityId>,
    pub name: String,
    pub role: LegalEntityRole,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ControlLinkResponse {
    pub control_link_id: ControlLinkId,
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    pub voting_power_bps: Option<u32>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct InstrumentResponse {
    pub instrument_id: InstrumentId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub symbol: String,
    pub kind: InstrumentKind,
    pub authorized_units: Option<i64>,
    pub issue_price_cents: Option<i64>,
    pub status: InstrumentStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub position_id: PositionId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub quantity_units: i64,
    pub principal_cents: i64,
    pub status: PositionStatus,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct RoundResponse {
    pub round_id: EquityRoundId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub name: String,
    pub pre_money_cents: Option<i64>,
    pub round_price_cents: Option<i64>,
    pub target_raise_cents: Option<i64>,
    pub conversion_target_instrument_id: Option<InstrumentId>,
    pub rule_set_id: Option<EquityRuleSetId>,
    pub board_approval_meeting_id: Option<MeetingId>,
    pub board_approval_resolution_id: Option<ResolutionId>,
    pub board_approved_at: Option<String>,
    pub accepted_by_contact_id: Option<ContactId>,
    pub accepted_at: Option<String>,
    pub status: EquityRoundStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct RuleSetResponse {
    pub rule_set_id: EquityRuleSetId,
    pub anti_dilution_method: AntiDilutionMethod,
    pub conversion_precedence: Vec<InstrumentKind>,
}

#[derive(Debug, Serialize)]
pub struct CapTableInstrumentSummary {
    pub instrument_id: InstrumentId,
    pub symbol: String,
    pub kind: InstrumentKind,
    pub authorized_units: Option<i64>,
    pub issued_units: i64,
    pub diluted_units: i64,
}

#[derive(Debug, Serialize)]
pub struct CapTableHolderSummary {
    pub holder_id: HolderId,
    pub name: String,
    pub outstanding_units: i64,
    pub as_converted_units: i64,
    pub fully_diluted_units: i64,
    pub outstanding_bps: u32,
    pub as_converted_bps: u32,
    pub fully_diluted_bps: u32,
}

#[derive(Debug, Serialize)]
pub struct CapTableResponse {
    pub entity_id: EntityId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub basis: CapTableBasis,
    pub total_units: i64,
    pub instruments: Vec<CapTableInstrumentSummary>,
    pub holders: Vec<CapTableHolderSummary>,
    pub generated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ConversionPreviewLine {
    pub source_position_id: PositionId,
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub principal_cents: i64,
    pub conversion_price_cents: i64,
    pub new_units: i64,
    pub basis: String,
}

#[derive(Debug, Serialize)]
pub struct ConversionPreviewResponse {
    pub entity_id: EntityId,
    pub round_id: EquityRoundId,
    pub target_instrument_id: InstrumentId,
    pub lines: Vec<ConversionPreviewLine>,
    pub anti_dilution_adjustment_units: i64,
    pub total_new_units: i64,
}

#[derive(Debug, Serialize)]
pub struct ConversionExecuteResponse {
    pub conversion_execution_id: ConversionExecutionId,
    pub round_id: EquityRoundId,
    pub converted_positions: usize,
    pub target_positions_touched: usize,
    pub total_new_units: i64,
}

#[derive(Debug, Serialize)]
pub struct ControlMapEdge {
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    pub voting_power_bps: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ControlMapResponse {
    pub root_entity_id: LegalEntityId,
    pub traversed_entities: Vec<LegalEntityId>,
    pub edges: Vec<ControlMapEdge>,
}

#[derive(Debug, Serialize)]
pub struct DilutionPreviewResponse {
    pub round_id: EquityRoundId,
    pub issuer_legal_entity_id: LegalEntityId,
    pub pre_round_outstanding_units: i64,
    pub projected_new_units: i64,
    pub projected_post_outstanding_units: i64,
    pub projected_dilution_bps: u32,
}

// ── Helpers ──────────────────────────────────────────────────────────

fn open_store<'a>(
    layout: &'a crate::store::RepoLayout,
    workspace_id: WorkspaceId,
    entity_id: EntityId,
) -> Result<EntityStore<'a>, AppError> {
    EntityStore::open(layout, workspace_id, entity_id).map_err(|e| match e {
        crate::git::error::GitStorageError::RepoNotFound(_) => {
            AppError::NotFound(format!("entity {} not found", entity_id))
        }
        other => AppError::Internal(other.to_string()),
    })
}

fn read_all<T: StoredEntity>(store: &EntityStore<'_>) -> Result<Vec<T>, AppError> {
    let ids = store
        .list_ids::<T>("main")
        .map_err(|e| AppError::Internal(format!("list {}: {e}", T::storage_dir())))?;

    let mut out = Vec::new();
    for id in ids {
        let rec = store
            .read::<T>("main", id)
            .map_err(|e| AppError::Internal(format!("read {} {}: {e}", T::storage_dir(), id)))?;
        out.push(rec);
    }
    Ok(out)
}

fn checked_bps(part: i64, total: i64) -> u32 {
    if part <= 0 || total <= 0 {
        return 0;
    }
    let p = i128::from(part) * 10_000_i128;
    let t = i128::from(total);
    let v = (p / t).clamp(0, i128::from(u32::MAX));
    u32::try_from(v).unwrap_or(0)
}

fn hash_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn infer_issuer(
    entity_id: EntityId,
    legal_entities: &[LegalEntity],
    explicit: Option<LegalEntityId>,
) -> Result<LegalEntityId, AppError> {
    if let Some(id) = explicit {
        return Ok(id);
    }

    legal_entities
        .iter()
        .find(|le| {
            le.linked_entity_id() == Some(entity_id) && le.role() == LegalEntityRole::Operating
        })
        .or_else(|| {
            legal_entities
                .iter()
                .find(|le| le.linked_entity_id() == Some(entity_id))
        })
        .map(|le| le.legal_entity_id())
        .ok_or_else(|| {
            AppError::NotFound(
                "no legal entity linked to this entity_id; create one via POST /v1/equity/entities"
                    .to_owned(),
            )
        })
}

fn units_for_basis(kind: InstrumentKind, qty: i64, basis: CapTableBasis) -> i64 {
    match basis {
        CapTableBasis::Outstanding => match kind {
            InstrumentKind::CommonEquity | InstrumentKind::PreferredEquity => qty,
            _ => 0,
        },
        CapTableBasis::AsConverted => match kind {
            InstrumentKind::CommonEquity
            | InstrumentKind::PreferredEquity
            | InstrumentKind::Safe
            | InstrumentKind::ConvertibleNote
            | InstrumentKind::Warrant => qty,
            InstrumentKind::OptionGrant => 0,
        },
        CapTableBasis::FullyDiluted => qty,
    }
}

fn compute_cap_table(
    entity_id: EntityId,
    issuer_legal_entity_id: LegalEntityId,
    basis: CapTableBasis,
    holders: &[Holder],
    instruments: &[Instrument],
    positions: &[Position],
) -> CapTableResponse {
    let issuer_instruments: Vec<&Instrument> = instruments
        .iter()
        .filter(|i| i.issuer_legal_entity_id() == issuer_legal_entity_id)
        .collect();

    let issuer_positions: Vec<&Position> = positions
        .iter()
        .filter(|p| p.issuer_legal_entity_id() == issuer_legal_entity_id)
        .collect();

    let mut holder_units_outstanding: HashMap<HolderId, i64> = HashMap::new();
    let mut holder_units_as_converted: HashMap<HolderId, i64> = HashMap::new();
    let mut holder_units_fully_diluted: HashMap<HolderId, i64> = HashMap::new();

    let instrument_map: HashMap<InstrumentId, &Instrument> = issuer_instruments
        .iter()
        .map(|i| (i.instrument_id(), *i))
        .collect();

    for p in &issuer_positions {
        let Some(inst) = instrument_map.get(&p.instrument_id()) else {
            continue;
        };
        let qty = p.quantity_units().max(0);

        *holder_units_outstanding.entry(p.holder_id()).or_insert(0) +=
            units_for_basis(inst.kind(), qty, CapTableBasis::Outstanding);
        *holder_units_as_converted.entry(p.holder_id()).or_insert(0) +=
            units_for_basis(inst.kind(), qty, CapTableBasis::AsConverted);
        *holder_units_fully_diluted.entry(p.holder_id()).or_insert(0) +=
            units_for_basis(inst.kind(), qty, CapTableBasis::FullyDiluted);
    }

    // Include unallocated option reserves in fully diluted denominator only.
    let mut unallocated_option_reserve: i64 = 0;
    for inst in &issuer_instruments {
        if inst.kind() == InstrumentKind::OptionGrant {
            let issued = issuer_positions
                .iter()
                .filter(|p| p.instrument_id() == inst.instrument_id())
                .map(|p| p.quantity_units().max(0))
                .sum::<i64>();
            if let Some(auth) = inst.authorized_units() {
                unallocated_option_reserve += (auth - issued).max(0);
            }
        }
    }

    let total_outstanding = holder_units_outstanding.values().copied().sum::<i64>();
    let total_as_converted = holder_units_as_converted.values().copied().sum::<i64>();
    let total_fully_diluted =
        holder_units_fully_diluted.values().copied().sum::<i64>() + unallocated_option_reserve;

    let total_units = match basis {
        CapTableBasis::Outstanding => total_outstanding,
        CapTableBasis::AsConverted => total_as_converted,
        CapTableBasis::FullyDiluted => total_fully_diluted,
    };

    let holder_name: HashMap<HolderId, String> = holders
        .iter()
        .map(|h| (h.holder_id(), h.name().to_owned()))
        .collect();

    let all_holder_ids: HashSet<HolderId> = holder_units_fully_diluted
        .keys()
        .chain(holder_units_as_converted.keys())
        .chain(holder_units_outstanding.keys())
        .copied()
        .collect();

    let mut holder_rows: Vec<CapTableHolderSummary> = all_holder_ids
        .into_iter()
        .map(|hid| {
            let outstanding_units = *holder_units_outstanding.get(&hid).unwrap_or(&0);
            let as_converted_units = *holder_units_as_converted.get(&hid).unwrap_or(&0);
            let fully_diluted_units = *holder_units_fully_diluted.get(&hid).unwrap_or(&0);

            CapTableHolderSummary {
                holder_id: hid,
                name: holder_name
                    .get(&hid)
                    .cloned()
                    .unwrap_or_else(|| "unknown holder".to_owned()),
                outstanding_units,
                as_converted_units,
                fully_diluted_units,
                outstanding_bps: checked_bps(outstanding_units, total_outstanding),
                as_converted_bps: checked_bps(as_converted_units, total_as_converted),
                fully_diluted_bps: checked_bps(fully_diluted_units, total_fully_diluted),
            }
        })
        .collect();
    holder_rows.sort_by(|a, b| a.name.cmp(&b.name));

    let mut instrument_rows = Vec::new();
    for inst in issuer_instruments {
        let issued_units = issuer_positions
            .iter()
            .filter(|p| p.instrument_id() == inst.instrument_id())
            .map(|p| p.quantity_units().max(0))
            .sum::<i64>();

        let diluted_units = match inst.kind() {
            InstrumentKind::OptionGrant => inst.authorized_units().unwrap_or(issued_units),
            _ => issued_units,
        };

        instrument_rows.push(CapTableInstrumentSummary {
            instrument_id: inst.instrument_id(),
            symbol: inst.symbol().to_owned(),
            kind: inst.kind(),
            authorized_units: inst.authorized_units(),
            issued_units,
            diluted_units,
        });
    }
    instrument_rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    CapTableResponse {
        entity_id,
        issuer_legal_entity_id,
        basis,
        total_units,
        instruments: instrument_rows,
        holders: holder_rows,
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn compute_conversion_preview(
    round: &EquityRound,
    rule_set: &EquityRuleSet,
    instruments: &[Instrument],
    positions: &[Position],
) -> Result<(Vec<ConversionPreviewLine>, i64), AppError> {
    let round_price = round.round_price_cents().ok_or_else(|| {
        AppError::BadRequest("round_price_cents is required before conversion".to_owned())
    })?;
    if round_price <= 0 {
        return Err(AppError::BadRequest(
            "round_price_cents must be positive".to_owned(),
        ));
    }

    let inst_map: HashMap<InstrumentId, &Instrument> =
        instruments.iter().map(|i| (i.instrument_id(), i)).collect();

    let precedence: Vec<InstrumentKind> = if rule_set.conversion_precedence().is_empty() {
        vec![
            InstrumentKind::Safe,
            InstrumentKind::ConvertibleNote,
            InstrumentKind::Warrant,
        ]
    } else {
        rule_set.conversion_precedence().to_vec()
    };

    let precedence_rank: HashMap<InstrumentKind, usize> = precedence
        .iter()
        .enumerate()
        .map(|(idx, k)| (*k, idx))
        .collect();

    let mut lines: Vec<ConversionPreviewLine> = positions
        .iter()
        .filter_map(|p| {
            let inst = inst_map.get(&p.instrument_id())?;
            if inst.issuer_legal_entity_id() != round.issuer_legal_entity_id() {
                return None;
            }
            match inst.kind() {
                InstrumentKind::Safe
                | InstrumentKind::ConvertibleNote
                | InstrumentKind::Warrant => {}
                _ => return None,
            }

            let terms = inst.terms();
            let discount_bps = terms
                .get("discount_bps")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(0)
                .min(10_000);
            let cap_price_cents = terms
                .get("cap_price_cents")
                .and_then(|v| v.as_i64())
                .filter(|v| *v > 0);

            let discounted_price = ((i128::from(round_price)
                * i128::from(10_000_u32 - discount_bps))
                / i128::from(10_000_u32)) as i64;
            let discounted_price = discounted_price.max(1);

            let mut conversion_price = round_price;
            let mut basis = "round_price".to_owned();
            if discount_bps > 0 && discounted_price < conversion_price {
                conversion_price = discounted_price;
                basis = "discount".to_owned();
            }
            if let Some(cap) = cap_price_cents {
                if cap < conversion_price {
                    conversion_price = cap;
                    basis = "cap_price".to_owned();
                }
            }

            let new_units = if p.principal_cents() > 0 {
                p.principal_cents() / conversion_price
            } else {
                p.quantity_units().max(0)
            }
            .max(0);

            Some(ConversionPreviewLine {
                source_position_id: p.position_id(),
                holder_id: p.holder_id(),
                instrument_id: p.instrument_id(),
                principal_cents: p.principal_cents(),
                conversion_price_cents: conversion_price,
                new_units,
                basis,
            })
        })
        .collect();

    lines.sort_by_key(|line| {
        let inst_kind = inst_map
            .get(&line.instrument_id)
            .map(|i| i.kind())
            .unwrap_or(InstrumentKind::Safe);
        *precedence_rank.get(&inst_kind).unwrap_or(&usize::MAX)
    });

    // Anti-dilution: compute additional preferred units under configured method.
    let anti_dilution_adjustment_units = match rule_set.anti_dilution_method() {
        AntiDilutionMethod::None => 0,
        AntiDilutionMethod::FullRatchet => {
            let mut adj = 0i64;
            for p in positions {
                let Some(inst) = inst_map.get(&p.instrument_id()) else {
                    continue;
                };
                if inst.issuer_legal_entity_id() != round.issuer_legal_entity_id()
                    || inst.kind() != InstrumentKind::PreferredEquity
                {
                    continue;
                }
                let Some(old_price) = inst.issue_price_cents() else {
                    continue;
                };
                if old_price <= round_price {
                    continue;
                }
                let qty = i128::from(p.quantity_units().max(0));
                let old = i128::from(old_price);
                let newp = i128::from(round_price);
                let adjusted_qty = (qty * old) / newp;
                let add = (adjusted_qty - qty).max(0);
                adj = adj.saturating_add(i64::try_from(add).unwrap_or(i64::MAX));
            }
            adj
        }
        AntiDilutionMethod::BroadBasedWeightedAverage
        | AntiDilutionMethod::NarrowBasedWeightedAverage => {
            // Simplified WA preview using aggregate shares.
            let mut existing = 0f64;
            let mut existing_broad = 0f64;
            for p in positions {
                let Some(inst) = inst_map.get(&p.instrument_id()) else {
                    continue;
                };
                if inst.issuer_legal_entity_id() != round.issuer_legal_entity_id() {
                    continue;
                }
                match inst.kind() {
                    InstrumentKind::CommonEquity | InstrumentKind::PreferredEquity => {
                        existing += p.quantity_units().max(0) as f64;
                        existing_broad += p.quantity_units().max(0) as f64;
                    }
                    InstrumentKind::OptionGrant => {
                        if let Some(auth) = inst.authorized_units() {
                            existing_broad += auth.max(0) as f64;
                        }
                    }
                    _ => {}
                }
            }
            let a = if rule_set.anti_dilution_method()
                == AntiDilutionMethod::BroadBasedWeightedAverage
            {
                existing_broad
            } else {
                existing
            };
            if a <= 0.0 {
                0
            } else {
                let c = round
                    .target_raise_cents()
                    .and_then(|raise| {
                        if round_price > 0 {
                            Some((raise as f64) / (round_price as f64))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0.0);

                // B approximates shares purchasable at prior preferred issue prices.
                let mut adjustment = 0f64;
                for p in positions {
                    let Some(inst) = inst_map.get(&p.instrument_id()) else {
                        continue;
                    };
                    if inst.issuer_legal_entity_id() != round.issuer_legal_entity_id()
                        || inst.kind() != InstrumentKind::PreferredEquity
                    {
                        continue;
                    }
                    let Some(cp1) = inst.issue_price_cents() else {
                        continue;
                    };
                    if cp1 <= 0 {
                        continue;
                    }
                    let b = (round.target_raise_cents().unwrap_or(0) as f64) / (cp1 as f64);
                    let cp2 = (cp1 as f64) * ((a + b) / (a + c).max(1.0));
                    if cp2 <= 0.0 || cp2 >= (cp1 as f64) {
                        continue;
                    }
                    let existing_qty = p.quantity_units().max(0) as f64;
                    let add = existing_qty * ((cp1 as f64 / cp2) - 1.0);
                    if add.is_finite() && add > 0.0 {
                        adjustment += add;
                    }
                }
                adjustment.floor() as i64
            }
        }
    };

    Ok((lines, anti_dilution_adjustment_units.max(0)))
}

fn ensure_authorized_round_intent(
    intent: &Intent,
    entity_id: EntityId,
    round_id: EquityRoundId,
    expected_intent_type: &str,
) -> Result<(), AppError> {
    if intent.entity_id() != entity_id {
        return Err(AppError::Forbidden(format!(
            "intent {} belongs to a different entity",
            intent.intent_id()
        )));
    }
    if intent.status() != IntentStatus::Authorized {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} must be authorized",
            intent.intent_id()
        )));
    }
    if intent.intent_type() != expected_intent_type {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} must have type {}",
            intent.intent_id(),
            expected_intent_type
        )));
    }
    let metadata_round_id = intent
        .metadata()
        .get("round_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::UnprocessableEntity(format!(
                "intent {} metadata must include round_id",
                intent.intent_id()
            ))
        })?;
    if metadata_round_id != round_id.to_string() {
        return Err(AppError::UnprocessableEntity(format!(
            "intent {} round_id mismatch",
            intent.intent_id()
        )));
    }
    Ok(())
}

fn validate_board_resolution_for_round(
    store: &EntityStore<'_>,
    entity_id: EntityId,
    meeting_id: MeetingId,
    resolution_id: ResolutionId,
) -> Result<(), AppError> {
    let meeting = store
        .read::<Meeting>("main", meeting_id)
        .map_err(|_| AppError::NotFound(format!("meeting {} not found", meeting_id)))?;

    let body = store
        .read::<GovernanceBody>("main", meeting.body_id())
        .map_err(|_| {
            AppError::NotFound(format!("governance body {} not found", meeting.body_id()))
        })?;

    if body.entity_id() != entity_id {
        return Err(AppError::BadRequest(format!(
            "meeting {} does not belong to entity {}",
            meeting_id, entity_id
        )));
    }
    if body.body_type() != BodyType::BoardOfDirectors {
        return Err(AppError::UnprocessableEntity(format!(
            "meeting {} is not associated with a board_of_directors body",
            meeting_id
        )));
    }

    let resolution: Resolution = store
        .read_resolution("main", meeting_id, resolution_id)
        .map_err(|_| AppError::NotFound(format!("resolution {} not found", resolution_id)))?;
    if !resolution.passed() {
        return Err(AppError::UnprocessableEntity(format!(
            "resolution {} did not pass",
            resolution_id
        )));
    }

    Ok(())
}

// ── Converters ───────────────────────────────────────────────────────

fn holder_to_response(h: &Holder) -> HolderResponse {
    HolderResponse {
        holder_id: h.holder_id(),
        contact_id: h.contact_id(),
        linked_entity_id: h.linked_entity_id(),
        name: h.name().to_owned(),
        holder_type: h.holder_type(),
        created_at: h.created_at().to_rfc3339(),
    }
}

fn legal_entity_to_response(le: &LegalEntity) -> LegalEntityResponse {
    LegalEntityResponse {
        legal_entity_id: le.legal_entity_id(),
        workspace_id: le.workspace_id(),
        linked_entity_id: le.linked_entity_id(),
        name: le.name().to_owned(),
        role: le.role(),
        created_at: le.created_at().to_rfc3339(),
    }
}

fn control_link_to_response(l: &ControlLink) -> ControlLinkResponse {
    ControlLinkResponse {
        control_link_id: l.control_link_id(),
        parent_legal_entity_id: l.parent_legal_entity_id(),
        child_legal_entity_id: l.child_legal_entity_id(),
        control_type: l.control_type(),
        voting_power_bps: l.voting_power_bps(),
        created_at: l.created_at().to_rfc3339(),
    }
}

fn instrument_to_response(i: &Instrument) -> InstrumentResponse {
    InstrumentResponse {
        instrument_id: i.instrument_id(),
        issuer_legal_entity_id: i.issuer_legal_entity_id(),
        symbol: i.symbol().to_owned(),
        kind: i.kind(),
        authorized_units: i.authorized_units(),
        issue_price_cents: i.issue_price_cents(),
        status: i.status(),
        created_at: i.created_at().to_rfc3339(),
    }
}

fn position_to_response(p: &Position) -> PositionResponse {
    PositionResponse {
        position_id: p.position_id(),
        issuer_legal_entity_id: p.issuer_legal_entity_id(),
        holder_id: p.holder_id(),
        instrument_id: p.instrument_id(),
        quantity_units: p.quantity_units(),
        principal_cents: p.principal_cents(),
        status: p.status(),
        updated_at: p.updated_at().to_rfc3339(),
    }
}

fn round_to_response(r: &EquityRound) -> RoundResponse {
    RoundResponse {
        round_id: r.equity_round_id(),
        issuer_legal_entity_id: r.issuer_legal_entity_id(),
        name: r.name().to_owned(),
        pre_money_cents: r.pre_money_cents(),
        round_price_cents: r.round_price_cents(),
        target_raise_cents: r.target_raise_cents(),
        conversion_target_instrument_id: r.conversion_target_instrument_id(),
        rule_set_id: r.rule_set_id(),
        board_approval_meeting_id: r.board_approval_meeting_id(),
        board_approval_resolution_id: r.board_approval_resolution_id(),
        board_approved_at: r.board_approved_at().map(|v| v.to_rfc3339()),
        accepted_by_contact_id: r.accepted_by_contact_id(),
        accepted_at: r.accepted_at().map(|v| v.to_rfc3339()),
        status: r.status(),
        created_at: r.created_at().to_rfc3339(),
    }
}

fn rule_set_to_response(r: &EquityRuleSet) -> RuleSetResponse {
    RuleSetResponse {
        rule_set_id: r.rule_set_id(),
        anti_dilution_method: r.anti_dilution_method(),
        conversion_precedence: r.conversion_precedence().to_vec(),
    }
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn create_holder(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateHolderRequest>,
) -> Result<Json<HolderResponse>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("holder name is required".to_owned()));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let holder = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let holder = Holder::new(
                HolderId::new(),
                req.contact_id,
                req.linked_entity_id,
                req.name,
                req.holder_type,
                req.external_reference,
            );
            let path = format!("cap-table/holders/{}.json", holder.holder_id());
            store
                .write_json(
                    "main",
                    &path,
                    &holder,
                    &format!("Create holder {}", holder.holder_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(holder)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(holder_to_response(&holder)))
}

async fn create_legal_entity(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateLegalEntityRequest>,
) -> Result<Json<LegalEntityResponse>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "legal entity name is required".to_owned(),
        ));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let legal_entity = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let le = LegalEntity::new(
                LegalEntityId::new(),
                workspace_id,
                req.linked_entity_id,
                req.name,
                req.role,
            );
            let path = format!("cap-table/entities/{}.json", le.legal_entity_id());
            store
                .write_json(
                    "main",
                    &path,
                    &le,
                    &format!("Create legal entity {}", le.legal_entity_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(le)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(legal_entity_to_response(&legal_entity)))
}

async fn create_control_link(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateControlLinkRequest>,
) -> Result<Json<ControlLinkResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let link = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let entities = read_all::<LegalEntity>(&store)?;
            let known: HashSet<LegalEntityId> =
                entities.iter().map(|e| e.legal_entity_id()).collect();
            if !known.contains(&req.parent_legal_entity_id)
                || !known.contains(&req.child_legal_entity_id)
            {
                return Err(AppError::BadRequest(
                    "parent_legal_entity_id and child_legal_entity_id must exist".to_owned(),
                ));
            }

            let link = ControlLink::new(
                ControlLinkId::new(),
                req.parent_legal_entity_id,
                req.child_legal_entity_id,
                req.control_type,
                req.voting_power_bps,
                req.notes,
            );
            let path = format!("cap-table/control-links/{}.json", link.control_link_id());
            store
                .write_json(
                    "main",
                    &path,
                    &link,
                    &format!("Create control link {}", link.control_link_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(link)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(control_link_to_response(&link)))
}

async fn create_instrument(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateInstrumentRequest>,
) -> Result<Json<InstrumentResponse>, AppError> {
    if req.symbol.trim().is_empty() {
        return Err(AppError::BadRequest(
            "instrument symbol is required".to_owned(),
        ));
    }
    if req.authorized_units.is_some_and(|v| v < 0) {
        return Err(AppError::BadRequest(
            "authorized_units cannot be negative".to_owned(),
        ));
    }

    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let instrument = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let entities = read_all::<LegalEntity>(&store)?;
            if !entities
                .iter()
                .any(|e| e.legal_entity_id() == req.issuer_legal_entity_id)
            {
                return Err(AppError::BadRequest(
                    "issuer_legal_entity_id does not exist".to_owned(),
                ));
            }

            let instrument = Instrument::new(
                InstrumentId::new(),
                req.issuer_legal_entity_id,
                req.symbol,
                req.kind,
                req.authorized_units,
                req.issue_price_cents,
                req.terms,
            );
            let path = format!("cap-table/instruments/{}.json", instrument.instrument_id());
            store
                .write_json(
                    "main",
                    &path,
                    &instrument,
                    &format!("Create instrument {}", instrument.instrument_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(instrument)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(instrument_to_response(&instrument)))
}

async fn adjust_position(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<AdjustPositionRequest>,
) -> Result<Json<PositionResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let position = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let holders = read_all::<Holder>(&store)?;
            if !holders.iter().any(|h| h.holder_id() == req.holder_id) {
                return Err(AppError::BadRequest("holder_id does not exist".to_owned()));
            }

            let instruments = read_all::<Instrument>(&store)?;
            let instrument = instruments
                .iter()
                .find(|i| i.instrument_id() == req.instrument_id)
                .ok_or_else(|| AppError::BadRequest("instrument_id does not exist".to_owned()))?;
            if instrument.issuer_legal_entity_id() != req.issuer_legal_entity_id {
                return Err(AppError::BadRequest(
                    "instrument issuer does not match issuer_legal_entity_id".to_owned(),
                ));
            }

            let all_positions = read_all::<Position>(&store)?;
            let existing = all_positions.into_iter().find(|p| {
                p.issuer_legal_entity_id() == req.issuer_legal_entity_id
                    && p.holder_id() == req.holder_id
                    && p.instrument_id() == req.instrument_id
            });

            let mut position = if let Some(mut p) = existing {
                p.apply_delta(
                    req.quantity_delta,
                    req.principal_delta_cents,
                    req.source_reference.clone(),
                    None,
                    None,
                )?;
                p
            } else {
                if req.quantity_delta < 0 || req.principal_delta_cents < 0 {
                    return Err(AppError::BadRequest(
                        "cannot create a new position with negative deltas".to_owned(),
                    ));
                }
                Position::new(
                    PositionId::new(),
                    req.issuer_legal_entity_id,
                    req.holder_id,
                    req.instrument_id,
                    req.quantity_delta,
                    req.principal_delta_cents,
                    req.source_reference,
                    None,
                    None,
                )?
            };

            // Keep a deterministic hash of current position values for traceability.
            let hash = hash_json(&serde_json::json!({
                "quantity_units": position.quantity_units(),
                "principal_cents": position.principal_cents(),
                "holder_id": position.holder_id(),
                "instrument_id": position.instrument_id(),
            }));
            position.apply_delta(
                0,
                0,
                position.source_reference().map(str::to_owned),
                None,
                Some(hash),
            )?;

            let path = format!("cap-table/positions/{}.json", position.position_id());
            store
                .write_json(
                    "main",
                    &path,
                    &position,
                    &format!("Adjust position {}", position.position_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(position)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(position_to_response(&position)))
}

async fn create_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateRoundRequest>,
) -> Result<Json<RoundResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let entities = read_all::<LegalEntity>(&store)?;
            if !entities
                .iter()
                .any(|e| e.legal_entity_id() == req.issuer_legal_entity_id)
            {
                return Err(AppError::BadRequest(
                    "issuer_legal_entity_id does not exist".to_owned(),
                ));
            }

            if let Some(target) = req.conversion_target_instrument_id {
                let instruments = read_all::<Instrument>(&store)?;
                if !instruments.iter().any(|i| i.instrument_id() == target) {
                    return Err(AppError::BadRequest(
                        "conversion_target_instrument_id does not exist".to_owned(),
                    ));
                }
            }

            let round = EquityRound::new(
                EquityRoundId::new(),
                req.issuer_legal_entity_id,
                req.name,
                req.pre_money_cents,
                req.round_price_cents,
                req.target_raise_cents,
                req.conversion_target_instrument_id,
                req.metadata,
            );
            let path = format!("cap-table/rounds/{}.json", round.equity_round_id());
            store
                .write_json(
                    "main",
                    &path,
                    &round,
                    &format!("Create equity round {}", round.equity_round_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(round)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(round_to_response(&round)))
}

async fn apply_round_terms(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(round_id): Path<EquityRoundId>,
    Json(req): Json<ApplyRoundTermsRequest>,
) -> Result<Json<RuleSetResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let rules = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut round = store
                .read::<EquityRound>("main", round_id)
                .map_err(|_| AppError::NotFound(format!("equity round {} not found", round_id)))?;

            let rules = EquityRuleSet::new(
                EquityRuleSetId::new(),
                req.anti_dilution_method,
                req.conversion_precedence,
                req.protective_provisions,
            );
            round.apply_terms(rules.rule_set_id())?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rules/{}.json", rules.rule_set_id()),
                    &rules,
                )
                .map_err(|e| AppError::Internal(format!("serialize rules: {e}")))?,
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Apply terms to round {}", round.equity_round_id()),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(rules)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(rule_set_to_response(&rules)))
}

async fn board_approve_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(round_id): Path<EquityRoundId>,
    Json(req): Json<BoardApproveRoundRequest>,
) -> Result<Json<RoundResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut round = store
                .read::<EquityRound>("main", round_id)
                .map_err(|_| AppError::NotFound(format!("equity round {} not found", round_id)))?;

            validate_board_resolution_for_round(
                &store,
                entity_id,
                req.meeting_id,
                req.resolution_id,
            )?;
            round.record_board_approval(req.meeting_id, req.resolution_id)?;

            let path = format!("cap-table/rounds/{}.json", round.equity_round_id());
            store
                .write_json(
                    "main",
                    &path,
                    &round,
                    &format!("Board approve round {}", round.equity_round_id()),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(round)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(round_to_response(&round)))
}

async fn accept_round(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Path(round_id): Path<EquityRoundId>,
    Json(req): Json<AcceptRoundRequest>,
) -> Result<Json<RoundResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut round = store
                .read::<EquityRound>("main", round_id)
                .map_err(|_| AppError::NotFound(format!("equity round {} not found", round_id)))?;
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;

            ensure_authorized_round_intent(&intent, entity_id, round_id, "equity.round.accept")?;
            round.accept(req.accepted_by_contact_id)?;
            intent.mark_executed()?;

            let files = vec![
                FileWrite::json(
                    format!("cap-table/rounds/{}.json", round.equity_round_id()),
                    &round,
                )
                .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            ];

            store
                .commit(
                    "main",
                    &format!("Accept round {}", round.equity_round_id()),
                    files,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(round)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(round_to_response(&round)))
}

async fn get_cap_table(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<CapTableQuery>,
) -> Result<Json<CapTableResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let holders = read_all::<Holder>(&store)?;
            let legal_entities = read_all::<LegalEntity>(&store)?;
            let instruments = read_all::<Instrument>(&store)?;
            let positions = read_all::<Position>(&store)?;

            let issuer_legal_entity_id =
                infer_issuer(entity_id, &legal_entities, query.issuer_legal_entity_id)?;

            Ok::<_, AppError>(compute_cap_table(
                entity_id,
                issuer_legal_entity_id,
                query.basis,
                &holders,
                &instruments,
                &positions,
            ))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

async fn preview_conversion(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Json(req): Json<PreviewConversionRequest>,
) -> Result<Json<ConversionPreviewResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let preview = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let round = store
                .read::<EquityRound>("main", req.round_id)
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", req.round_id))
                })?;
            let rule_set_id = round
                .rule_set_id()
                .ok_or_else(|| AppError::BadRequest("round terms are not applied".to_owned()))?;
            let rules = store
                .read::<EquityRuleSet>("main", rule_set_id)
                .map_err(|_| AppError::NotFound(format!("rule set {} not found", rule_set_id)))?;

            let instruments = read_all::<Instrument>(&store)?;
            let positions = read_all::<Position>(&store)?;

            let target_instrument_id =
                round.conversion_target_instrument_id().ok_or_else(|| {
                    AppError::BadRequest("conversion_target_instrument_id is required".to_owned())
                })?;

            let (lines, anti_dilution_adjustment_units) =
                compute_conversion_preview(&round, &rules, &instruments, &positions)?;

            let total_new_units = lines
                .iter()
                .map(|l| l.new_units)
                .sum::<i64>()
                .saturating_add(anti_dilution_adjustment_units);

            Ok::<_, AppError>(ConversionPreviewResponse {
                entity_id,
                round_id: req.round_id,
                target_instrument_id,
                lines,
                anti_dilution_adjustment_units,
                total_new_units,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(preview))
}

async fn execute_conversion(
    RequireEquityWrite(auth): RequireEquityWrite,
    State(state): State<AppState>,
    Json(req): Json<ExecuteConversionRequest>,
) -> Result<Json<ConversionExecuteResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_id = req.entity_id;
    if !auth.allows_entity(entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut round = store
                .read::<EquityRound>("main", req.round_id)
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", req.round_id))
                })?;
            let rule_set_id = round
                .rule_set_id()
                .ok_or_else(|| AppError::BadRequest("round terms are not applied".to_owned()))?;
            let rules = store
                .read::<EquityRuleSet>("main", rule_set_id)
                .map_err(|_| AppError::NotFound(format!("rule set {} not found", rule_set_id)))?;
            let mut intent = store
                .read::<Intent>("main", req.intent_id)
                .map_err(|_| AppError::NotFound(format!("intent {} not found", req.intent_id)))?;
            ensure_authorized_round_intent(
                &intent,
                entity_id,
                req.round_id,
                "equity.round.execute_conversion",
            )?;

            let target_instrument_id =
                round.conversion_target_instrument_id().ok_or_else(|| {
                    AppError::BadRequest("conversion_target_instrument_id is required".to_owned())
                })?;

            let instruments = read_all::<Instrument>(&store)?;
            let mut positions = read_all::<Position>(&store)?;
            let (lines, anti_dilution_adjustment_units) =
                compute_conversion_preview(&round, &rules, &instruments, &positions)?;

            let mut modified_paths = Vec::new();
            let mut touched_targets: HashSet<PositionId> = HashSet::new();

            for line in &lines {
                let Some(src_idx) = positions
                    .iter()
                    .position(|p| p.position_id() == line.source_position_id)
                else {
                    continue;
                };

                let src = &mut positions[src_idx];
                let close_hash = hash_json(&serde_json::json!({
                    "round_id": req.round_id,
                    "conversion_price_cents": line.conversion_price_cents,
                    "new_units": line.new_units,
                }));
                src.apply_delta(
                    -src.quantity_units(),
                    -src.principal_cents(),
                    req.source_reference.clone(),
                    None,
                    Some(close_hash),
                )?;
                modified_paths.push(
                    FileWrite::json(
                        format!("cap-table/positions/{}.json", src.position_id()),
                        src,
                    )
                    .map_err(|e| AppError::Internal(format!("serialize source position: {e}")))?,
                );

                // Find or create the target position.
                let existing_target_idx = positions.iter().position(|p| {
                    p.issuer_legal_entity_id() == round.issuer_legal_entity_id()
                        && p.holder_id() == line.holder_id
                        && p.instrument_id() == target_instrument_id
                });

                if let Some(idx) = existing_target_idx {
                    let target = &mut positions[idx];
                    let hash = hash_json(&serde_json::json!({
                        "round_id": req.round_id,
                        "source_position_id": line.source_position_id,
                        "new_units": line.new_units,
                    }));
                    target.apply_delta(
                        line.new_units,
                        0,
                        req.source_reference.clone(),
                        None,
                        Some(hash),
                    )?;
                    touched_targets.insert(target.position_id());
                    modified_paths.push(
                        FileWrite::json(
                            format!("cap-table/positions/{}.json", target.position_id()),
                            target,
                        )
                        .map_err(|e| {
                            AppError::Internal(format!("serialize target position: {e}"))
                        })?,
                    );
                } else {
                    let hash = hash_json(&serde_json::json!({
                        "round_id": req.round_id,
                        "source_position_id": line.source_position_id,
                        "new_units": line.new_units,
                    }));
                    let target = Position::new(
                        PositionId::new(),
                        round.issuer_legal_entity_id(),
                        line.holder_id,
                        target_instrument_id,
                        line.new_units,
                        0,
                        req.source_reference.clone(),
                        None,
                        Some(hash),
                    )?;
                    touched_targets.insert(target.position_id());
                    modified_paths.push(
                        FileWrite::json(
                            format!("cap-table/positions/{}.json", target.position_id()),
                            &target,
                        )
                        .map_err(|e| {
                            AppError::Internal(format!("serialize new target position: {e}"))
                        })?,
                    );
                    positions.push(target);
                }
            }

            // Anti-dilution adjustment units are added to a synthetic holder-neutral position only
            // if positive and target instrument exists.
            if anti_dilution_adjustment_units > 0 {
                let mut anti_holder = read_all::<Holder>(&store)?
                    .into_iter()
                    .find(|h| h.external_reference() == Some("anti_dilution_pool"));
                if anti_holder.is_none() {
                    let generated = Holder::new(
                        HolderId::new(),
                        ContactId::new(),
                        None,
                        "Anti-Dilution Pool".to_owned(),
                        HolderType::Other,
                        Some("anti_dilution_pool".to_owned()),
                    );
                    modified_paths.push(
                        FileWrite::json(
                            format!("cap-table/holders/{}.json", generated.holder_id()),
                            &generated,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize anti holder: {e}")))?,
                    );
                    anti_holder = Some(generated);
                }
                if let Some(holder) = anti_holder {
                    let target = Position::new(
                        PositionId::new(),
                        round.issuer_legal_entity_id(),
                        holder.holder_id(),
                        target_instrument_id,
                        anti_dilution_adjustment_units,
                        0,
                        Some("anti_dilution_adjustment".to_owned()),
                        None,
                        Some(hash_json(&serde_json::json!({
                            "round_id": req.round_id,
                            "anti_dilution_adjustment_units": anti_dilution_adjustment_units,
                        }))),
                    )?;
                    touched_targets.insert(target.position_id());
                    modified_paths.push(
                        FileWrite::json(
                            format!("cap-table/positions/{}.json", target.position_id()),
                            &target,
                        )
                        .map_err(|e| AppError::Internal(format!("serialize anti position: {e}")))?,
                    );
                }
            }

            round.close()?;
            intent.mark_executed()?;
            modified_paths.push(
                FileWrite::json(format!("cap-table/rounds/{}.json", req.round_id), &round)
                    .map_err(|e| AppError::Internal(format!("serialize round: {e}")))?,
            );
            modified_paths.push(
                FileWrite::json(
                    format!("execution/intents/{}.json", intent.intent_id()),
                    &intent,
                )
                .map_err(|e| AppError::Internal(format!("serialize intent: {e}")))?,
            );

            let total_new_units = lines
                .iter()
                .map(|l| l.new_units)
                .sum::<i64>()
                .saturating_add(anti_dilution_adjustment_units);

            let execution = ConversionExecution::new(
                ConversionExecutionId::new(),
                entity_id,
                req.round_id,
                serde_json::json!({
                    "line_count": lines.len(),
                    "anti_dilution_adjustment_units": anti_dilution_adjustment_units,
                    "total_new_units": total_new_units,
                    "rule_set_id": rule_set_id,
                    "target_instrument_id": target_instrument_id,
                }),
                req.source_reference,
            );

            modified_paths.push(
                FileWrite::json(
                    format!(
                        "cap-table/conversions/{}.json",
                        execution.conversion_execution_id()
                    ),
                    &execution,
                )
                .map_err(|e| AppError::Internal(format!("serialize conversion execution: {e}")))?,
            );

            store
                .commit(
                    "main",
                    &format!("Execute conversions for round {}", req.round_id),
                    modified_paths,
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(ConversionExecuteResponse {
                conversion_execution_id: execution.conversion_execution_id(),
                round_id: req.round_id,
                converted_positions: lines.len(),
                target_positions_touched: touched_targets.len(),
                total_new_units,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(result))
}

async fn get_control_map(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Query(query): Query<ControlMapQuery>,
) -> Result<Json<ControlMapResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(query.entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, query.entity_id)?;
            let links = read_all::<ControlLink>(&store)?;
            let entities = read_all::<LegalEntity>(&store)?;
            let entity_ids: HashSet<LegalEntityId> =
                entities.iter().map(|e| e.legal_entity_id()).collect();
            if !entity_ids.contains(&query.root_entity_id) {
                return Err(AppError::NotFound(format!(
                    "root_entity_id {} not found",
                    query.root_entity_id
                )));
            }

            let mut visited: HashSet<LegalEntityId> = HashSet::new();
            let mut stack = vec![query.root_entity_id];
            let mut edges = Vec::new();

            while let Some(node) = stack.pop() {
                if !visited.insert(node) {
                    continue;
                }
                for link in links.iter().filter(|l| l.parent_legal_entity_id() == node) {
                    edges.push(ControlMapEdge {
                        parent_legal_entity_id: link.parent_legal_entity_id(),
                        child_legal_entity_id: link.child_legal_entity_id(),
                        control_type: link.control_type(),
                        voting_power_bps: link.voting_power_bps(),
                    });
                    stack.push(link.child_legal_entity_id());
                }
            }

            let mut traversed_entities: Vec<LegalEntityId> = visited.into_iter().collect();
            traversed_entities.sort_by_key(|id| id.to_string());

            Ok::<_, AppError>(ControlMapResponse {
                root_entity_id: query.root_entity_id,
                traversed_entities,
                edges,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

async fn get_dilution_preview(
    RequireEquityRead(auth): RequireEquityRead,
    State(state): State<AppState>,
    Query(query): Query<DilutionPreviewQuery>,
) -> Result<Json<DilutionPreviewResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    if !auth.allows_entity(query.entity_id) {
        return Err(AppError::Forbidden("entity access denied".to_owned()));
    }

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, query.entity_id)?;
            let round = store
                .read::<EquityRound>("main", query.round_id)
                .map_err(|_| {
                    AppError::NotFound(format!("equity round {} not found", query.round_id))
                })?;

            let instruments = read_all::<Instrument>(&store)?;
            let positions = read_all::<Position>(&store)?;

            let pre_round_outstanding_units = positions
                .iter()
                .filter(|p| p.issuer_legal_entity_id() == round.issuer_legal_entity_id())
                .filter_map(|p| {
                    instruments
                        .iter()
                        .find(|i| i.instrument_id() == p.instrument_id())
                        .map(|i| (i, p))
                })
                .map(|(i, p)| {
                    units_for_basis(
                        i.kind(),
                        p.quantity_units().max(0),
                        CapTableBasis::Outstanding,
                    )
                })
                .sum::<i64>();

            let projected_new_units = round
                .target_raise_cents()
                .and_then(|raise| round.round_price_cents().map(|price| (raise, price)))
                .and_then(
                    |(raise, price)| {
                        if price > 0 { Some(raise / price) } else { None }
                    },
                )
                .unwrap_or(0)
                .max(0);

            let projected_post_outstanding_units =
                pre_round_outstanding_units.saturating_add(projected_new_units);
            let projected_dilution_bps =
                checked_bps(projected_new_units, projected_post_outstanding_units);

            Ok::<_, AppError>(DilutionPreviewResponse {
                round_id: query.round_id,
                issuer_legal_entity_id: round.issuer_legal_entity_id(),
                pre_round_outstanding_units,
                projected_new_units,
                projected_post_outstanding_units,
                projected_dilution_bps,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(response))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn equity_routes() -> Router<AppState> {
    Router::new()
        .route("/v1/equity/holders", post(create_holder))
        .route("/v1/equity/entities", post(create_legal_entity))
        .route("/v1/equity/control-links", post(create_control_link))
        .route("/v1/equity/instruments", post(create_instrument))
        .route("/v1/equity/positions/adjust", post(adjust_position))
        .route("/v1/equity/rounds", post(create_round))
        .route(
            "/v1/equity/rounds/{round_id}/apply-terms",
            post(apply_round_terms),
        )
        .route(
            "/v1/equity/rounds/{round_id}/board-approve",
            post(board_approve_round),
        )
        .route("/v1/equity/rounds/{round_id}/accept", post(accept_round))
        .route("/v1/equity/conversions/preview", post(preview_conversion))
        .route("/v1/equity/conversions/execute", post(execute_conversion))
        .route("/v1/entities/{entity_id}/cap-table", get(get_cap_table))
        .route("/v1/equity/control-map", get(get_control_map))
        .route("/v1/equity/dilution/preview", get(get_dilution_preview))
}
