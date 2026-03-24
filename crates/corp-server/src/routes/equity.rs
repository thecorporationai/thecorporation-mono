//! Equity domain routes.
//!
//! Covers cap tables, share classes, equity grants, SAFE notes, valuations,
//! share transfers, funding rounds, and holders.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;

use corp_auth::{RequireEquityRead, RequireEquityWrite};
use corp_core::equity::{
    CapTable, ControlLink, ControlType, EquityGrant, FundingRound, Holder, HolderType, Instrument,
    InstrumentKind, InvestorLedgerEntry, LegalEntity, LegalEntityRole, Position, RepurchaseRight,
    SafeNote, ShareClass, ShareTransfer, Valuation, VestingEvent, VestingSchedule,
};
use corp_core::equity::types::{
    GrantType, InvestorLedgerEntryType, SafeType, ShareCount, StockType, TransferType,
    ValuationMethodology, ValuationType,
};
use corp_core::equity::vesting::materialize_vesting_events;
use corp_core::ids::{
    CapTableId, ContactId, EntityId, EquityGrantId, FundingRoundId, HolderId,
    InstrumentId, LegalEntityId, PositionId, RepurchaseRightId, SafeNoteId,
    ShareClassId, TransferId, ValuationId, VestingEventId, VestingScheduleId,
};
use crate::error::AppError;
use crate::state::AppState;

// ── Request body types ────────────────────────────────────────────────────────

/// Request body for `POST /entities/{entity_id}/cap-table`.
/// The cap table is a singleton aggregate per entity; no additional fields
/// are required to create one.
#[derive(Debug, Deserialize)]
pub struct CreateCapTableRequest {}

/// Request body for `POST /entities/{entity_id}/share-classes`.
#[derive(Debug, Deserialize)]
pub struct CreateShareClassRequest {
    pub cap_table_id: CapTableId,
    pub class_code: String,
    pub stock_type: StockType,
    /// Par value formatted string, e.g. `"0.00001"`.
    pub par_value: String,
    /// Authorized share count.
    pub authorized_shares: i64,
    /// Liquidation preference description; only relevant for preferred stock.
    pub liquidation_preference: Option<String>,
}

/// Request body for `POST /entities/{entity_id}/grants`.
#[derive(Debug, Deserialize)]
pub struct CreateGrantRequest {
    pub cap_table_id: CapTableId,
    pub share_class_id: ShareClassId,
    pub recipient_contact_id: ContactId,
    pub recipient_name: String,
    pub grant_type: GrantType,
    /// Number of shares to grant.
    pub shares: i64,
    /// Strike / exercise price in whole cents. `None` for outright grants.
    pub price_per_share: Option<i64>,
    pub vesting_start: Option<NaiveDate>,
    pub vesting_months: Option<u32>,
    pub cliff_months: Option<u32>,
}

/// Request body for `POST /entities/{entity_id}/safes`.
#[derive(Debug, Deserialize)]
pub struct IssueSafeRequest {
    pub cap_table_id: CapTableId,
    pub investor_contact_id: ContactId,
    pub investor_name: String,
    pub safe_type: SafeType,
    /// Principal investment in whole cents.
    pub investment_amount_cents: i64,
    pub valuation_cap_cents: Option<i64>,
    /// Discount percentage, e.g. `20` = 20%.
    pub discount_percent: Option<u32>,
}

/// Request body for `POST /entities/{entity_id}/safes/{safe_id}/convert`.
/// Kept as a struct for future extensibility.
#[derive(Debug, Deserialize)]
pub struct ConvertSafeRequest {}

/// Request body for `POST /entities/{entity_id}/valuations`.
#[derive(Debug, Deserialize)]
pub struct CreateValuationRequest {
    pub cap_table_id: CapTableId,
    pub valuation_type: ValuationType,
    pub methodology: ValuationMethodology,
    /// Total enterprise / FMV in whole cents.
    pub valuation_amount_cents: i64,
    pub effective_date: NaiveDate,
    pub prepared_by: Option<String>,
}

/// Request body for `POST /entities/{entity_id}/valuations/{valuation_id}/approve`.
#[derive(Debug, Deserialize)]
pub struct ApproveValuationRequest {
    pub approved_by: String,
}

/// Request body for `POST /entities/{entity_id}/transfers`.
#[derive(Debug, Deserialize)]
pub struct CreateTransferRequest {
    pub cap_table_id: CapTableId,
    pub from_holder_id: HolderId,
    pub to_holder_id: HolderId,
    pub share_class_id: ShareClassId,
    pub shares: i64,
    pub transfer_type: TransferType,
    pub price_per_share_cents: Option<i64>,
}

/// Request body for `POST /entities/{entity_id}/rounds`.
#[derive(Debug, Deserialize)]
pub struct CreateRoundRequest {
    pub cap_table_id: CapTableId,
    pub name: String,
    pub target_amount_cents: i64,
    pub price_per_share_cents: Option<i64>,
}

/// Request body for `POST /entities/{entity_id}/holders`.
#[derive(Debug, Deserialize)]
pub struct CreateHolderRequest {
    pub contact_id: Option<ContactId>,
    pub name: String,
    pub holder_type: HolderType,
}

/// Request body for `POST /entities/{entity_id}/vesting-schedules`.
#[derive(Debug, Deserialize)]
pub struct CreateVestingScheduleRequest {
    pub grant_id: EquityGrantId,
    pub total_shares: i64,
    pub vesting_start_date: NaiveDate,
    pub template: String,
    pub cliff_months: u32,
    pub total_months: u32,
    pub acceleration_single_trigger: bool,
    pub acceleration_double_trigger: bool,
    pub early_exercise_allowed: bool,
}

/// Request body for `POST /entities/{entity_id}/instruments`.
#[derive(Debug, Deserialize)]
pub struct CreateInstrumentRequest {
    pub symbol: String,
    pub kind: InstrumentKind,
    pub authorized_units: Option<i64>,
    pub issue_price_cents: Option<i64>,
    pub terms: Option<serde_json::Value>,
}

/// Request body for `POST /entities/{entity_id}/positions`.
#[derive(Debug, Deserialize)]
pub struct CreatePositionRequest {
    pub holder_id: HolderId,
    pub instrument_id: InstrumentId,
    pub quantity_units: i64,
    pub principal_cents: Option<i64>,
    pub source_reference: Option<String>,
}

/// Request body for `POST /entities/{entity_id}/positions/{position_id}/delta`.
#[derive(Debug, Deserialize)]
pub struct ApplyPositionDeltaRequest {
    pub quantity_delta: i64,
    pub principal_delta: Option<i64>,
    pub source_reference: Option<String>,
}

/// Request body for `POST /entities/{entity_id}/investor-ledger`.
#[derive(Debug, Deserialize)]
pub struct CreateLedgerEntryRequest {
    pub investor_id: ContactId,
    pub investor_name: String,
    pub entry_type: InvestorLedgerEntryType,
    pub amount_cents: i64,
    pub shares_received: Option<i64>,
    pub pro_rata_eligible: bool,
    pub memo: Option<String>,
    pub effective_date: NaiveDate,
    pub safe_note_id: Option<SafeNoteId>,
    pub funding_round_id: Option<FundingRoundId>,
}

/// Request body for `POST /entities/{entity_id}/legal-entities`.
#[derive(Debug, Deserialize)]
pub struct CreateLegalEntityRequest {
    pub name: String,
    pub role: LegalEntityRole,
    pub linked_entity_id: Option<EntityId>,
}

/// Request body for `POST /entities/{entity_id}/control-links`.
#[derive(Debug, Deserialize)]
pub struct CreateControlLinkRequest {
    pub parent_legal_entity_id: LegalEntityId,
    pub child_legal_entity_id: LegalEntityId,
    pub control_type: ControlType,
    pub voting_power_bps: Option<u32>,
    pub notes: Option<String>,
}

/// Request body for `POST /entities/{entity_id}/repurchase-rights`.
#[derive(Debug, Deserialize)]
pub struct CreateRepurchaseRightRequest {
    pub grant_id: EquityGrantId,
    pub share_count: i64,
    pub price_per_share_cents: i64,
    pub expiration_date: Option<NaiveDate>,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        // Cap table
        .route(
            "/entities/{entity_id}/cap-table",
            get(get_cap_table).post(create_cap_table),
        )
        // Share classes
        .route(
            "/entities/{entity_id}/share-classes",
            get(list_share_classes).post(create_share_class),
        )
        // Grants
        .route(
            "/entities/{entity_id}/grants",
            get(list_grants).post(create_grant),
        )
        .route(
            "/entities/{entity_id}/grants/{grant_id}",
            get(get_grant),
        )
        // SAFE notes
        .route(
            "/entities/{entity_id}/safes",
            get(list_safes).post(issue_safe),
        )
        .route(
            "/entities/{entity_id}/safes/{safe_id}",
            get(get_safe),
        )
        .route(
            "/entities/{entity_id}/safes/{safe_id}/convert",
            post(convert_safe),
        )
        .route(
            "/entities/{entity_id}/safes/{safe_id}/cancel",
            post(cancel_safe),
        )
        // Valuations
        .route(
            "/entities/{entity_id}/valuations",
            get(list_valuations).post(create_valuation),
        )
        .route(
            "/entities/{entity_id}/valuations/{valuation_id}",
            get(get_valuation),
        )
        .route(
            "/entities/{entity_id}/valuations/{valuation_id}/submit",
            post(submit_valuation),
        )
        .route(
            "/entities/{entity_id}/valuations/{valuation_id}/approve",
            post(approve_valuation),
        )
        .route(
            "/entities/{entity_id}/valuations/{valuation_id}/expire",
            post(expire_valuation),
        )
        .route(
            "/entities/{entity_id}/valuations/{valuation_id}/supersede",
            post(supersede_valuation),
        )
        // Transfers
        .route(
            "/entities/{entity_id}/transfers",
            get(list_transfers).post(create_transfer),
        )
        .route(
            "/entities/{entity_id}/transfers/{transfer_id}",
            get(get_transfer),
        )
        .route(
            "/entities/{entity_id}/transfers/{transfer_id}/approve",
            post(approve_transfer),
        )
        .route(
            "/entities/{entity_id}/transfers/{transfer_id}/deny",
            post(deny_transfer),
        )
        .route(
            "/entities/{entity_id}/transfers/{transfer_id}/cancel",
            post(cancel_transfer),
        )
        .route(
            "/entities/{entity_id}/transfers/{transfer_id}/execute",
            post(execute_transfer),
        )
        // Rounds
        .route(
            "/entities/{entity_id}/rounds",
            get(list_rounds).post(create_round),
        )
        .route(
            "/entities/{entity_id}/rounds/{round_id}",
            get(get_round),
        )
        .route(
            "/entities/{entity_id}/rounds/{round_id}/advance",
            post(advance_round),
        )
        .route(
            "/entities/{entity_id}/rounds/{round_id}/close",
            post(close_round),
        )
        // Holders
        .route(
            "/entities/{entity_id}/holders",
            get(list_holders).post(create_holder),
        )
        .route(
            "/entities/{entity_id}/holders/{holder_id}",
            get(get_holder),
        )
        // Vesting
        .route(
            "/entities/{entity_id}/vesting-schedules",
            get(list_vesting_schedules).post(create_vesting_schedule),
        )
        .route(
            "/entities/{entity_id}/vesting-schedules/{schedule_id}",
            get(get_vesting_schedule),
        )
        .route(
            "/entities/{entity_id}/vesting-schedules/{schedule_id}/terminate",
            post(terminate_vesting),
        )
        .route(
            "/entities/{entity_id}/vesting-schedules/{schedule_id}/materialize",
            post(materialize_events),
        )
        .route(
            "/entities/{entity_id}/vesting-events",
            get(list_vesting_events),
        )
        .route(
            "/entities/{entity_id}/vesting-events/{event_id}/vest",
            post(vest_event),
        )
        .route(
            "/entities/{entity_id}/vesting-events/{event_id}/forfeit",
            post(forfeit_event),
        )
        // Instruments
        .route(
            "/entities/{entity_id}/instruments",
            get(list_instruments).post(create_instrument),
        )
        .route(
            "/entities/{entity_id}/instruments/{instrument_id}",
            get(get_instrument),
        )
        // Positions
        .route(
            "/entities/{entity_id}/positions",
            get(list_positions).post(create_position),
        )
        .route(
            "/entities/{entity_id}/positions/{position_id}",
            get(get_position),
        )
        .route(
            "/entities/{entity_id}/positions/{position_id}/delta",
            post(apply_position_delta),
        )
        // Investor Ledger
        .route(
            "/entities/{entity_id}/investor-ledger",
            get(list_investor_ledger).post(create_ledger_entry),
        )
        // Legal Entities (corporate structure)
        .route(
            "/entities/{entity_id}/legal-entities",
            get(list_legal_entities).post(create_legal_entity),
        )
        .route(
            "/entities/{entity_id}/legal-entities/{le_id}",
            get(get_legal_entity),
        )
        // Control Links
        .route(
            "/entities/{entity_id}/control-links",
            get(list_control_links).post(create_control_link),
        )
        // Repurchase Rights
        .route(
            "/entities/{entity_id}/repurchase-rights",
            get(list_repurchase_rights).post(create_repurchase_right),
        )
        .route(
            "/entities/{entity_id}/repurchase-rights/{rr_id}/activate",
            post(activate_repurchase),
        )
        .route(
            "/entities/{entity_id}/repurchase-rights/{rr_id}/close",
            post(close_repurchase),
        )
        .route(
            "/entities/{entity_id}/repurchase-rights/{rr_id}/waive",
            post(waive_repurchase),
        )
}

// ── Cap table handlers ────────────────────────────────────────────────────────

async fn get_cap_table(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<CapTable>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let cap_tables = store.read_all::<CapTable>("main").await?;
    Ok(Json(cap_tables))
}

async fn create_cap_table(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(_body): Json<CreateCapTableRequest>,
) -> Result<Json<CapTable>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    // Cap table is a singleton per entity; return the existing one if it
    // already exists rather than creating a duplicate.
    let existing = store.read_all::<CapTable>("main").await?;
    if let Some(cap_table) = existing.into_iter().next() {
        return Ok(Json(cap_table));
    }
    let cap_table = CapTable::new(entity_id);
    store
        .write::<CapTable>(&cap_table, cap_table.cap_table_id, "main", "create cap table")
        .await?;
    Ok(Json(cap_table))
}

// ── Share class handlers ──────────────────────────────────────────────────────

async fn list_share_classes(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ShareClass>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let classes = store.read_all::<ShareClass>("main").await?;
    Ok(Json(classes))
}

async fn create_share_class(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateShareClassRequest>,
) -> Result<Json<ShareClass>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let share_class = ShareClass::new(
        entity_id,
        body.cap_table_id,
        body.class_code,
        body.stock_type,
        body.par_value,
        ShareCount::new(body.authorized_shares),
        body.liquidation_preference,
    );
    store
        .write::<ShareClass>(
            &share_class,
            share_class.share_class_id,
            "main",
            "create share class",
        )
        .await?;
    Ok(Json(share_class))
}

// ── Grant handlers ────────────────────────────────────────────────────────────

async fn list_grants(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<EquityGrant>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let grants = store.read_all::<EquityGrant>("main").await?;
    Ok(Json(grants))
}

async fn create_grant(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateGrantRequest>,
) -> Result<Json<EquityGrant>, AppError> {
    if body.shares <= 0 {
        return Err(AppError::BadRequest("shares must be greater than zero".into()));
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let share_class = store.read::<ShareClass>(body.share_class_id, "main").await?;
    if body.shares > share_class.authorized_shares.raw() {
        return Err(AppError::BadRequest(format!(
            "cannot issue {} shares: exceeds authorized {} for class {}",
            body.shares, share_class.authorized_shares.raw(), share_class.class_code
        )));
    }
    let grant = EquityGrant::new(
        entity_id,
        body.cap_table_id,
        body.share_class_id,
        body.recipient_contact_id,
        body.recipient_name,
        body.grant_type,
        ShareCount::new(body.shares),
        body.price_per_share,
        body.vesting_start,
        body.vesting_months,
        body.cliff_months,
    );
    store
        .write::<EquityGrant>(&grant, grant.grant_id, "main", "create equity grant")
        .await?;
    Ok(Json(grant))
}

async fn get_grant(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, grant_id)): Path<(EntityId, EquityGrantId)>,
) -> Result<Json<EquityGrant>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let grant = store.read::<EquityGrant>(grant_id, "main").await?;
    Ok(Json(grant))
}

// ── SAFE note handlers ────────────────────────────────────────────────────────

async fn list_safes(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<SafeNote>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let safes = store.read_all::<SafeNote>("main").await?;
    Ok(Json(safes))
}

async fn issue_safe(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<IssueSafeRequest>,
) -> Result<Json<SafeNote>, AppError> {
    if body.investment_amount_cents <= 0 {
        return Err(AppError::BadRequest("investment_amount_cents must be greater than zero".into()));
    }
    if let Some(cap) = body.valuation_cap_cents {
        if cap <= 0 {
            return Err(AppError::BadRequest("valuation_cap_cents must be greater than zero".into()));
        }
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let safe_note = SafeNote::new(
        entity_id,
        body.cap_table_id,
        body.investor_contact_id,
        body.investor_name,
        body.safe_type,
        body.investment_amount_cents,
        body.valuation_cap_cents,
        body.discount_percent,
    );
    store
        .write::<SafeNote>(&safe_note, safe_note.safe_note_id, "main", "issue safe note")
        .await?;
    Ok(Json(safe_note))
}

async fn get_safe(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, safe_id)): Path<(EntityId, SafeNoteId)>,
) -> Result<Json<SafeNote>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let safe_note = store.read::<SafeNote>(safe_id, "main").await?;
    Ok(Json(safe_note))
}

async fn convert_safe(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, safe_id)): Path<(EntityId, SafeNoteId)>,
    Json(_body): Json<ConvertSafeRequest>,
) -> Result<Json<SafeNote>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut safe_note = store.read::<SafeNote>(safe_id, "main").await?;
    safe_note
        .convert()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<SafeNote>(&safe_note, safe_id, "main", "convert safe note")
        .await?;
    Ok(Json(safe_note))
}

async fn cancel_safe(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, safe_id)): Path<(EntityId, SafeNoteId)>,
) -> Result<Json<SafeNote>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut safe_note = store.read::<SafeNote>(safe_id, "main").await?;
    safe_note
        .cancel()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<SafeNote>(&safe_note, safe_id, "main", "cancel safe note")
        .await?;
    Ok(Json(safe_note))
}

// ── Valuation handlers ────────────────────────────────────────────────────────

async fn list_valuations(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Valuation>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let valuations = store.read_all::<Valuation>("main").await?;
    Ok(Json(valuations))
}

async fn get_valuation(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, valuation_id)): Path<(EntityId, ValuationId)>,
) -> Result<Json<Valuation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let valuation = store.read::<Valuation>(valuation_id, "main").await?;
    Ok(Json(valuation))
}

async fn create_valuation(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateValuationRequest>,
) -> Result<Json<Valuation>, AppError> {
    if body.valuation_amount_cents <= 0 {
        return Err(AppError::BadRequest("valuation_amount_cents must be greater than zero".into()));
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let valuation = Valuation::new(
        entity_id,
        body.cap_table_id,
        body.valuation_type,
        body.methodology,
        body.valuation_amount_cents,
        body.effective_date,
        body.prepared_by,
    );
    store
        .write::<Valuation>(
            &valuation,
            valuation.valuation_id,
            "main",
            "create valuation",
        )
        .await?;
    Ok(Json(valuation))
}

async fn submit_valuation(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, valuation_id)): Path<(EntityId, ValuationId)>,
) -> Result<Json<Valuation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut valuation = store.read::<Valuation>(valuation_id, "main").await?;
    valuation
        .submit_for_approval()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Valuation>(
            &valuation,
            valuation_id,
            "main",
            "submit valuation for approval",
        )
        .await?;
    Ok(Json(valuation))
}

async fn approve_valuation(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, valuation_id)): Path<(EntityId, ValuationId)>,
    Json(body): Json<ApproveValuationRequest>,
) -> Result<Json<Valuation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut valuation = store.read::<Valuation>(valuation_id, "main").await?;
    valuation
        .approve(body.approved_by)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Valuation>(&valuation, valuation_id, "main", "approve valuation")
        .await?;
    Ok(Json(valuation))
}

async fn expire_valuation(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, valuation_id)): Path<(EntityId, ValuationId)>,
) -> Result<Json<Valuation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut valuation = store.read::<Valuation>(valuation_id, "main").await?;
    valuation
        .expire()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Valuation>(&valuation, valuation_id, "main", "expire valuation")
        .await?;
    Ok(Json(valuation))
}

async fn supersede_valuation(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, valuation_id)): Path<(EntityId, ValuationId)>,
) -> Result<Json<Valuation>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut valuation = store.read::<Valuation>(valuation_id, "main").await?;
    valuation
        .supersede()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Valuation>(&valuation, valuation_id, "main", "supersede valuation")
        .await?;
    Ok(Json(valuation))
}

// ── Transfer handlers ─────────────────────────────────────────────────────────

async fn list_transfers(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ShareTransfer>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let transfers = store.read_all::<ShareTransfer>("main").await?;
    Ok(Json(transfers))
}

async fn get_transfer(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, transfer_id)): Path<(EntityId, TransferId)>,
) -> Result<Json<ShareTransfer>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let transfer = store.read::<ShareTransfer>(transfer_id, "main").await?;
    Ok(Json(transfer))
}

async fn create_transfer(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateTransferRequest>,
) -> Result<Json<ShareTransfer>, AppError> {
    if body.shares <= 0 {
        return Err(AppError::BadRequest("shares must be greater than zero".into()));
    }
    if body.from_holder_id == body.to_holder_id {
        return Err(AppError::BadRequest("cannot transfer shares to the same holder".into()));
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let transfer = ShareTransfer::new(
        entity_id,
        body.cap_table_id,
        body.from_holder_id,
        body.to_holder_id,
        body.share_class_id,
        ShareCount::new(body.shares),
        body.transfer_type,
        body.price_per_share_cents,
    );
    store
        .write::<ShareTransfer>(&transfer, transfer.transfer_id, "main", "create transfer")
        .await?;
    Ok(Json(transfer))
}

/// `POST /entities/{entity_id}/transfers/{transfer_id}/approve`
///
/// Advances the transfer through its two-step board-approval pipeline.
/// Calling this endpoint twice moves the transfer through both steps:
///   1st call: `Draft` → `PendingBoardApproval`
///   2nd call: `PendingBoardApproval` → `Approved`
///
/// Once `Approved`, use the `/execute` endpoint to finalise the transfer.
async fn approve_transfer(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, transfer_id)): Path<(EntityId, TransferId)>,
) -> Result<Json<ShareTransfer>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut transfer = store.read::<ShareTransfer>(transfer_id, "main").await?;
    transfer
        .approve()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<ShareTransfer>(&transfer, transfer_id, "main", "approve transfer")
        .await?;
    Ok(Json(transfer))
}

async fn execute_transfer(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, transfer_id)): Path<(EntityId, TransferId)>,
) -> Result<Json<ShareTransfer>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut transfer = store.read::<ShareTransfer>(transfer_id, "main").await?;
    transfer
        .execute()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<ShareTransfer>(&transfer, transfer_id, "main", "execute transfer")
        .await?;
    Ok(Json(transfer))
}

async fn deny_transfer(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, transfer_id)): Path<(EntityId, TransferId)>,
) -> Result<Json<ShareTransfer>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut transfer = store.read::<ShareTransfer>(transfer_id, "main").await?;
    transfer
        .deny()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<ShareTransfer>(&transfer, transfer_id, "main", "deny transfer")
        .await?;
    Ok(Json(transfer))
}

async fn cancel_transfer(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, transfer_id)): Path<(EntityId, TransferId)>,
) -> Result<Json<ShareTransfer>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut transfer = store.read::<ShareTransfer>(transfer_id, "main").await?;
    transfer
        .cancel()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<ShareTransfer>(&transfer, transfer_id, "main", "cancel transfer")
        .await?;
    Ok(Json(transfer))
}

// ── Funding round handlers ────────────────────────────────────────────────────

async fn list_rounds(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<FundingRound>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let rounds = store.read_all::<FundingRound>("main").await?;
    Ok(Json(rounds))
}

async fn get_round(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, round_id)): Path<(EntityId, FundingRoundId)>,
) -> Result<Json<FundingRound>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let round = store.read::<FundingRound>(round_id, "main").await?;
    Ok(Json(round))
}

async fn create_round(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateRoundRequest>,
) -> Result<Json<FundingRound>, AppError> {
    if body.target_amount_cents <= 0 {
        return Err(AppError::BadRequest("target_amount_cents must be greater than zero".into()));
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let round = FundingRound::new(
        entity_id,
        body.cap_table_id,
        body.name,
        body.target_amount_cents,
        body.price_per_share_cents,
    );
    store
        .write::<FundingRound>(&round, round.round_id, "main", "create funding round")
        .await?;
    Ok(Json(round))
}

async fn close_round(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, round_id)): Path<(EntityId, FundingRoundId)>,
) -> Result<Json<FundingRound>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut round = store.read::<FundingRound>(round_id, "main").await?;
    round
        .close()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<FundingRound>(&round, round_id, "main", "close funding round")
        .await?;
    Ok(Json(round))
}

/// `POST /entities/{entity_id}/rounds/{round_id}/advance`
///
/// Advances a funding round through its pre-close pipeline:
/// `TermSheet` → `Diligence` → `Closing`.
///
/// Returns `400 Bad Request` if the round is already in `Closing` (use `/close`
/// instead) or already `Closed`.
async fn advance_round(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, round_id)): Path<(EntityId, FundingRoundId)>,
) -> Result<Json<FundingRound>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut round = store.read::<FundingRound>(round_id, "main").await?;
    round
        .advance_status()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<FundingRound>(&round, round_id, "main", "advance funding round")
        .await?;
    Ok(Json(round))
}

// ── Holder handlers ───────────────────────────────────────────────────────────

async fn list_holders(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Holder>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let holders = store.read_all::<Holder>("main").await?;
    Ok(Json(holders))
}

async fn create_holder(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateHolderRequest>,
) -> Result<Json<Holder>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let holder = Holder::new(entity_id, body.contact_id, body.name, body.holder_type);
    store
        .write::<Holder>(&holder, holder.holder_id, "main", "create holder")
        .await?;
    Ok(Json(holder))
}

async fn get_holder(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, holder_id)): Path<(EntityId, HolderId)>,
) -> Result<Json<Holder>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let holder = store.read::<Holder>(holder_id, "main").await?;
    Ok(Json(holder))
}

// ── Vesting schedule handlers ─────────────────────────────────────────────────

async fn list_vesting_schedules(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<VestingSchedule>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let schedules = store.read_all::<VestingSchedule>("main").await?;
    Ok(Json(schedules))
}

async fn create_vesting_schedule(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateVestingScheduleRequest>,
) -> Result<Json<VestingSchedule>, AppError> {
    if body.total_shares <= 0 {
        return Err(AppError::BadRequest("total_shares must be greater than zero".into()));
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let schedule = VestingSchedule::new(
        body.grant_id,
        entity_id,
        ShareCount::new(body.total_shares),
        body.vesting_start_date,
        body.template,
        body.cliff_months,
        body.total_months,
        body.acceleration_single_trigger,
        body.acceleration_double_trigger,
        body.early_exercise_allowed,
    );
    store
        .write::<VestingSchedule>(
            &schedule,
            schedule.schedule_id,
            "main",
            "create vesting schedule",
        )
        .await?;
    Ok(Json(schedule))
}

async fn get_vesting_schedule(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, schedule_id)): Path<(EntityId, VestingScheduleId)>,
) -> Result<Json<VestingSchedule>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let schedule = store.read::<VestingSchedule>(schedule_id, "main").await?;
    Ok(Json(schedule))
}

async fn terminate_vesting(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, schedule_id)): Path<(EntityId, VestingScheduleId)>,
) -> Result<Json<VestingSchedule>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut schedule = store.read::<VestingSchedule>(schedule_id, "main").await?;
    schedule.terminate(chrono::Utc::now().date_naive());
    store
        .write::<VestingSchedule>(&schedule, schedule_id, "main", "terminate vesting schedule")
        .await?;
    Ok(Json(schedule))
}

async fn materialize_events(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, schedule_id)): Path<(EntityId, VestingScheduleId)>,
) -> Result<Json<Vec<VestingEvent>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let schedule = store.read::<VestingSchedule>(schedule_id, "main").await?;
    let events = materialize_vesting_events(&schedule);
    for event in &events {
        store
            .write::<VestingEvent>(event, event.event_id, "main", "materialize vesting event")
            .await?;
    }
    Ok(Json(events))
}

async fn list_vesting_events(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<VestingEvent>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let events = store.read_all::<VestingEvent>("main").await?;
    Ok(Json(events))
}

async fn vest_event(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, event_id)): Path<(EntityId, VestingEventId)>,
) -> Result<Json<VestingEvent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut event = store.read::<VestingEvent>(event_id, "main").await?;
    event
        .vest()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<VestingEvent>(&event, event_id, "main", "vest event")
        .await?;
    Ok(Json(event))
}

async fn forfeit_event(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, event_id)): Path<(EntityId, VestingEventId)>,
) -> Result<Json<VestingEvent>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut event = store.read::<VestingEvent>(event_id, "main").await?;
    event
        .forfeit()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<VestingEvent>(&event, event_id, "main", "forfeit event")
        .await?;
    Ok(Json(event))
}

// ── Instrument handlers ───────────────────────────────────────────────────────

async fn list_instruments(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Instrument>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let instruments = store.read_all::<Instrument>("main").await?;
    Ok(Json(instruments))
}

async fn create_instrument(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateInstrumentRequest>,
) -> Result<Json<Instrument>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let instrument = Instrument::new(
        LegalEntityId::from_uuid(entity_id.as_uuid()),
        body.symbol,
        body.kind,
        body.authorized_units,
        body.issue_price_cents,
        body.terms.unwrap_or(serde_json::Value::Null),
    );
    store
        .write::<Instrument>(
            &instrument,
            instrument.instrument_id,
            "main",
            "create instrument",
        )
        .await?;
    Ok(Json(instrument))
}

async fn get_instrument(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, instrument_id)): Path<(EntityId, InstrumentId)>,
) -> Result<Json<Instrument>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let instrument = store.read::<Instrument>(instrument_id, "main").await?;
    Ok(Json(instrument))
}

// ── Position handlers ─────────────────────────────────────────────────────────

async fn list_positions(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Position>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let positions = store.read_all::<Position>("main").await?;
    Ok(Json(positions))
}

async fn create_position(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreatePositionRequest>,
) -> Result<Json<Position>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let position = Position::new(
        LegalEntityId::from_uuid(entity_id.as_uuid()),
        body.holder_id,
        body.instrument_id,
        body.quantity_units,
        body.principal_cents.unwrap_or(0),
        body.source_reference,
    )
    .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Position>(&position, position.position_id, "main", "create position")
        .await?;
    Ok(Json(position))
}

async fn get_position(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, position_id)): Path<(EntityId, PositionId)>,
) -> Result<Json<Position>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let position = store.read::<Position>(position_id, "main").await?;
    Ok(Json(position))
}

async fn apply_position_delta(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, position_id)): Path<(EntityId, PositionId)>,
    Json(body): Json<ApplyPositionDeltaRequest>,
) -> Result<Json<Position>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut position = store.read::<Position>(position_id, "main").await?;
    position
        .apply_delta(body.quantity_delta, body.principal_delta.unwrap_or(0), body.source_reference)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Position>(&position, position_id, "main", "apply position delta")
        .await?;
    Ok(Json(position))
}

// ── Investor ledger handlers ──────────────────────────────────────────────────

async fn list_investor_ledger(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<InvestorLedgerEntry>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let entries = store.read_all::<InvestorLedgerEntry>("main").await?;
    Ok(Json(entries))
}

async fn create_ledger_entry(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateLedgerEntryRequest>,
) -> Result<Json<InvestorLedgerEntry>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let entry = InvestorLedgerEntry::new(
        entity_id,
        body.investor_id,
        body.investor_name,
        body.safe_note_id,
        body.funding_round_id,
        body.entry_type,
        body.amount_cents,
        body.shares_received,
        body.pro_rata_eligible,
        body.memo,
        body.effective_date,
    );
    store
        .write::<InvestorLedgerEntry>(
            &entry,
            entry.entry_id,
            "main",
            "create investor ledger entry",
        )
        .await?;
    Ok(Json(entry))
}

// ── Legal entity handlers ─────────────────────────────────────────────────────

async fn list_legal_entities(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<LegalEntity>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let legal_entities = store.read_all::<LegalEntity>("main").await?;
    Ok(Json(legal_entities))
}

async fn create_legal_entity(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateLegalEntityRequest>,
) -> Result<Json<LegalEntity>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let legal_entity = LegalEntity::new(
        principal.workspace_id,
        body.linked_entity_id,
        body.name,
        body.role,
    );
    store
        .write::<LegalEntity>(
            &legal_entity,
            legal_entity.legal_entity_id,
            "main",
            "create legal entity",
        )
        .await?;
    Ok(Json(legal_entity))
}

async fn get_legal_entity(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path((entity_id, le_id)): Path<(EntityId, LegalEntityId)>,
) -> Result<Json<LegalEntity>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let legal_entity = store.read::<LegalEntity>(le_id, "main").await?;
    Ok(Json(legal_entity))
}

// ── Control link handlers ─────────────────────────────────────────────────────

async fn list_control_links(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ControlLink>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let links = store.read_all::<ControlLink>("main").await?;
    Ok(Json(links))
}

async fn create_control_link(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateControlLinkRequest>,
) -> Result<Json<ControlLink>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let link = ControlLink::new(
        body.parent_legal_entity_id,
        body.child_legal_entity_id,
        body.control_type,
        body.voting_power_bps,
        body.notes,
    );
    store
        .write::<ControlLink>(&link, link.control_link_id, "main", "create control link")
        .await?;
    Ok(Json(link))
}

// ── Repurchase right handlers ─────────────────────────────────────────────────

async fn list_repurchase_rights(
    RequireEquityRead(principal): RequireEquityRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<RepurchaseRight>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let rights = store.read_all::<RepurchaseRight>("main").await?;
    Ok(Json(rights))
}

async fn create_repurchase_right(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateRepurchaseRightRequest>,
) -> Result<Json<RepurchaseRight>, AppError> {
    if body.share_count <= 0 {
        return Err(AppError::BadRequest("share_count must be greater than zero".into()));
    }
    if body.price_per_share_cents < 0 {
        return Err(AppError::BadRequest("price_per_share_cents must be non-negative".into()));
    }
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let right = RepurchaseRight::new(
        entity_id,
        body.grant_id,
        ShareCount::new(body.share_count),
        body.price_per_share_cents,
        body.expiration_date,
    );
    store
        .write::<RepurchaseRight>(
            &right,
            right.repurchase_right_id,
            "main",
            "create repurchase right",
        )
        .await?;
    Ok(Json(right))
}

async fn activate_repurchase(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, rr_id)): Path<(EntityId, RepurchaseRightId)>,
) -> Result<Json<RepurchaseRight>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut right = store.read::<RepurchaseRight>(rr_id, "main").await?;
    right.activate();
    store
        .write::<RepurchaseRight>(&right, rr_id, "main", "activate repurchase right")
        .await?;
    Ok(Json(right))
}

async fn close_repurchase(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, rr_id)): Path<(EntityId, RepurchaseRightId)>,
) -> Result<Json<RepurchaseRight>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut right = store.read::<RepurchaseRight>(rr_id, "main").await?;
    right.close();
    store
        .write::<RepurchaseRight>(&right, rr_id, "main", "close repurchase right")
        .await?;
    Ok(Json(right))
}

async fn waive_repurchase(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, rr_id)): Path<(EntityId, RepurchaseRightId)>,
) -> Result<Json<RepurchaseRight>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let mut right = store.read::<RepurchaseRight>(rr_id, "main").await?;
    right.waive();
    store
        .write::<RepurchaseRight>(&right, rr_id, "main", "waive repurchase right")
        .await?;
    Ok(Json(right))
}
