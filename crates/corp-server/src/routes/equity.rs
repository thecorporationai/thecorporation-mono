//! Equity domain routes.
//!
//! Covers cap tables, instruments, equity grants, SAFE notes, valuations,
//! share transfers, funding rounds, holders, vesting, positions, investor
//! ledger, legal entities, control links, and repurchase rights.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;
use corp_auth::{RequireEquityRead, RequireEquityWrite};
use corp_core::contacts::Contact;
use corp_core::equity::types::{
    GrantStatus, GrantType, InvestorLedgerEntryType, PositionStatus, SafeType, ShareCount,
    TransferType, ValuationMethodology, ValuationType,
};
use corp_core::equity::vesting::materialize_vesting_events;
use corp_core::equity::{
    CapTable, ControlLink, ControlType, EquityGrant, ExerciseType, FundingRound, Holder,
    HolderType, Instrument, InstrumentKind, InstrumentStatus, InvestorLedgerEntry, LegalEntity,
    LegalEntityRole, OptionExercise, Position, RepurchaseRight, SafeNote, ShareTransfer, Valuation,
    VestingEvent, VestingSchedule,
};
use corp_core::ids::{
    CapTableId, ContactId, EntityId, EquityGrantId, FundingRoundId, HolderId, InstrumentId,
    LegalEntityId, PositionId, RepurchaseRightId, SafeNoteId, TransferId,
    ValuationId, VestingEventId, VestingScheduleId,
};

// ── Request body types ────────────────────────────────────────────────────────

/// Request body for `POST /entities/{entity_id}/cap-table`.
#[derive(Debug, Deserialize)]
pub struct CreateCapTableRequest {}

/// Request body for `POST /entities/{entity_id}/instruments`.
#[derive(Debug, Deserialize)]
pub struct CreateInstrumentRequest {
    pub cap_table_id: CapTableId,
    pub symbol: String,
    pub kind: InstrumentKind,
    pub authorized_units: Option<i64>,
    /// Par value formatted string, e.g. `"0.00001"`.
    pub par_value: Option<String>,
    pub issue_price_cents: Option<i64>,
    /// Liquidation preference description; only relevant for preferred equity.
    pub liquidation_preference: Option<String>,
    pub terms: Option<serde_json::Value>,
}

/// Request body for `POST /entities/{entity_id}/grants`.
#[derive(Debug, Deserialize)]
pub struct CreateGrantRequest {
    pub cap_table_id: CapTableId,
    pub instrument_id: InstrumentId,
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
    /// Optional holder ID. If provided, a position is created for this holder.
    /// If omitted, no position is created (useful for option grants that haven't
    /// been exercised yet).
    pub holder_id: Option<HolderId>,
    /// Governance resolution that authorized this grant (e.g. board approval).
    pub resolution_id: Option<corp_core::ids::ResolutionId>,
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
#[derive(Debug, Deserialize)]
pub struct ConvertSafeRequest {
    /// The instrument to convert into (e.g. Series Seed Preferred).
    pub instrument_id: InstrumentId,
    /// Number of shares the investor receives from conversion.
    pub conversion_shares: i64,
    /// Holder record for the investor.
    pub holder_id: HolderId,
}

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
    pub instrument_id: InstrumentId,
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

/// Request body for `POST /entities/{entity_id}/grants/{grant_id}/exercise`.
#[derive(Debug, Deserialize)]
pub struct ExerciseOptionRequest {
    pub holder_id: HolderId,
    pub shares_to_exercise: i64,
    pub exercise_date: Option<NaiveDate>,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        // Cap table
        .route(
            "/entities/{entity_id}/cap-table",
            get(get_cap_table).post(create_cap_table),
        )
        // Instruments (replaces share-classes)
        .route(
            "/entities/{entity_id}/instruments",
            get(list_instruments).post(create_instrument),
        )
        .route(
            "/entities/{entity_id}/instruments/{instrument_id}",
            get(get_instrument),
        )
        // Grants
        .route(
            "/entities/{entity_id}/grants",
            get(list_grants).post(create_grant),
        )
        .route("/entities/{entity_id}/grants/{grant_id}", get(get_grant))
        .route(
            "/entities/{entity_id}/grants/{grant_id}/exercise",
            post(exercise_option),
        )
        // SAFE notes
        .route(
            "/entities/{entity_id}/safes",
            get(list_safes).post(issue_safe),
        )
        .route("/entities/{entity_id}/safes/{safe_id}", get(get_safe))
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
        .route("/entities/{entity_id}/rounds/{round_id}", get(get_round))
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
        .route("/entities/{entity_id}/holders/{holder_id}", get(get_holder))
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

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Verify the given `cap_table_id` belongs to the entity in the URL path.
async fn verify_cap_table_ownership(
    store: &corp_storage::entity_store::EntityStore,
    cap_table_id: CapTableId,
    entity_id: EntityId,
) -> Result<(), AppError> {
    let cap_table = store
        .read::<CapTable>(cap_table_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::BadRequest(format!("cap table {} not found", cap_table_id))
                }
                other => AppError::Storage(other),
            }
        })?;
    if cap_table.entity_id != entity_id {
        return Err(AppError::BadRequest(format!(
            "cap table {} does not belong to entity {}",
            cap_table_id, entity_id
        )));
    }
    Ok(())
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let existing = store.read_all::<CapTable>("main").await?;
    if let Some(cap_table) = existing.into_iter().next() {
        return Ok(Json(cap_table));
    }
    let cap_table = CapTable::new(entity_id);
    store
        .write::<CapTable>(
            &cap_table,
            cap_table.cap_table_id,
            "main",
            "create cap table",
        )
        .await?;
    Ok(Json(cap_table))
}

// ── Instrument handlers ──────────────────────────────────────────────────────

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
    use crate::routes::validation::validate_symbol;
    validate_symbol("symbol", &body.symbol)?;

    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;

    verify_cap_table_ownership(&store, body.cap_table_id, entity_id).await?;

    // Reject duplicate symbols within the same cap table.
    let existing: Vec<Instrument> = store
        .read_all::<Instrument>("main")
        .await
        .map_err(AppError::Storage)?;
    if existing.iter().any(|i| {
        i.cap_table_id == body.cap_table_id
            && i.symbol == body.symbol
            && i.status == InstrumentStatus::Active
    }) {
        return Err(AppError::Conflict(format!(
            "an active instrument with symbol '{}' already exists on this cap table",
            body.symbol
        )));
    }

    let instrument = Instrument::new(
        entity_id,
        body.cap_table_id,
        body.symbol,
        body.kind,
        body.authorized_units,
        body.par_value,
        body.issue_price_cents,
        body.liquidation_preference,
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
        return Err(AppError::BadRequest(
            "shares must be greater than zero".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    verify_cap_table_ownership(&store, body.cap_table_id, entity_id).await?;
    // Verify recipient contact exists
    store
        .read::<Contact>(body.recipient_contact_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => AppError::BadRequest(format!(
                    "recipient contact {} not found",
                    body.recipient_contact_id
                )),
                other => AppError::Storage(other),
            }
        })?;

    let instrument = store
        .read::<Instrument>(body.instrument_id, "main")
        .await?;

    // Cumulative over-issuance check: sum all active grants for this instrument.
    if let Some(authorized) = instrument.authorized_units {
        let existing_grants: Vec<EquityGrant> = store
            .read_all::<EquityGrant>("main")
            .await
            .map_err(AppError::Storage)?;
        let issued_shares: i64 = existing_grants
            .iter()
            .filter(|g| {
                g.instrument_id == body.instrument_id
                    && g.status != GrantStatus::Cancelled
                    && g.status != GrantStatus::Forfeited
            })
            .map(|g| g.shares.raw())
            .sum();
        let total_after = issued_shares + body.shares;
        if total_after > authorized {
            return Err(AppError::BadRequest(format!(
                "cannot issue {} shares: {} already issued + {} requested = {} total, \
                 exceeds authorized {} for instrument {}",
                body.shares, issued_shares, body.shares, total_after, authorized,
                instrument.symbol
            )));
        }
    }

    let grant = EquityGrant::new(
        entity_id,
        body.cap_table_id,
        body.instrument_id,
        body.recipient_contact_id,
        &body.recipient_name,
        body.grant_type,
        ShareCount::new(body.shares),
        body.price_per_share,
        body.vesting_start,
        body.vesting_months,
        body.cliff_months,
        body.resolution_id,
    );
    store
        .write::<EquityGrant>(&grant, grant.grant_id, "main", "create equity grant")
        .await?;

    // If a holder_id is provided, create a position for the grant recipient.
    // This connects the corporate action (grant) to the ledger (position).
    if let Some(holder_id) = body.holder_id {
        let principal_cents = body
            .price_per_share
            .map(|p| p * body.shares)
            .unwrap_or(0);
        let position = Position::new(
            entity_id,
            holder_id,
            body.instrument_id,
            body.shares,
            principal_cents,
            Some(format!("grant:{}", grant.grant_id)),
        )
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
        store
            .write::<Position>(
                &position,
                position.position_id,
                "main",
                "create position for grant",
            )
            .await?;
    }

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

// ── Option exercise handler ──────────────────────────────────────────────────

async fn exercise_option(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, grant_id)): Path<(EntityId, EquityGrantId)>,
    Json(body): Json<ExerciseOptionRequest>,
) -> Result<Json<OptionExercise>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;

    // 1. Load the grant and validate it's option-like
    let mut grant = store.read::<EquityGrant>(grant_id, "main").await?;
    match grant.grant_type {
        GrantType::StockOption | GrantType::Iso | GrantType::Nso => {}
        _ => {
            return Err(AppError::BadRequest(format!(
                "grant type {:?} is not exercisable — only stock_option, iso, nso grants can be exercised",
                grant.grant_type
            )));
        }
    }

    // 2. Validate strike price exists
    let strike_price = grant.price_per_share.ok_or_else(|| {
        AppError::BadRequest("grant has no price_per_share (strike price) set".into())
    })?;

    // 3. Validate grant status
    match grant.status {
        GrantStatus::Issued | GrantStatus::Vested => {}
        _ => {
            return Err(AppError::BadRequest(format!(
                "grant status is {:?} — only issued or vested grants can be exercised",
                grant.status
            )));
        }
    }

    // 4. Validate holder exists
    let _holder = store
        .read::<Holder>(body.holder_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::BadRequest(format!("holder {} not found", body.holder_id))
                }
                other => AppError::Storage(other),
            }
        })?;

    // 5. Validate shares > 0
    if body.shares_to_exercise <= 0 {
        return Err(AppError::BadRequest(
            "shares_to_exercise must be greater than zero".into(),
        ));
    }

    // 6. Determine exercisable shares (vesting check)
    let vesting_events: Vec<VestingEvent> = store
        .read_all::<VestingEvent>("main")
        .await
        .unwrap_or_default();
    let vested_shares: i64 = vesting_events
        .iter()
        .filter(|e| {
            e.grant_id == grant_id && e.status == corp_core::equity::types::VestingEventStatus::Vested
        })
        .map(|e| e.share_count.raw())
        .sum();

    // Check for early exercise
    let schedules: Vec<VestingSchedule> = store
        .read_all::<VestingSchedule>("main")
        .await
        .unwrap_or_default();
    let early_exercise_allowed = schedules
        .iter()
        .any(|s| s.grant_id == grant_id && s.early_exercise_allowed);

    let exercisable = if early_exercise_allowed {
        grant.shares.raw() // all shares, even unvested
    } else if vested_shares > 0 {
        vested_shares
    } else if vesting_events.iter().all(|e| e.grant_id != grant_id) {
        // No vesting events exist for this grant — treat all shares as exercisable
        // (grant without a vesting schedule = fully vested at issuance)
        grant.shares.raw()
    } else {
        vested_shares
    };

    // Sum previously exercised shares
    let prior_exercises: Vec<OptionExercise> = store
        .read_all::<OptionExercise>("main")
        .await
        .unwrap_or_default();
    let previously_exercised: i64 = prior_exercises
        .iter()
        .filter(|e| e.grant_id == grant_id)
        .map(|e| e.shares_exercised.raw())
        .sum();

    let available = exercisable - previously_exercised;
    if body.shares_to_exercise > available {
        return Err(AppError::BadRequest(format!(
            "cannot exercise {} shares: {} exercisable - {} previously exercised = {} available",
            body.shares_to_exercise, exercisable, previously_exercised, available
        )));
    }

    // 7. Determine exercise type
    let total_exercised_after = previously_exercised + body.shares_to_exercise;
    let exercise_type = if early_exercise_allowed && vested_shares < total_exercised_after {
        ExerciseType::Early
    } else if total_exercised_after == grant.shares.raw() {
        ExerciseType::Full
    } else {
        ExerciseType::Partial
    };

    let exercise_date = body
        .exercise_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    // 8. Update grant status if fully exercised
    if total_exercised_after >= grant.shares.raw() {
        grant.status = GrantStatus::Exercised;
        store
            .write::<EquityGrant>(&grant, grant_id, "main", "mark grant exercised")
            .await?;
    }

    // 9. Create or update position for the holder (before exercise record so we have position_id)
    let positions: Vec<Position> = store
        .read_all::<Position>("main")
        .await
        .unwrap_or_default();
    let existing_position = positions.iter().find(|p| {
        p.holder_id == body.holder_id
            && p.instrument_id == grant.instrument_id
            && p.status == PositionStatus::Active
    });

    let principal_delta = body.shares_to_exercise * strike_price;
    let exercise_id = corp_core::ids::OptionExerciseId::new();

    let result_position_id = if let Some(pos) = existing_position {
        let pid = pos.position_id;
        let mut pos = pos.clone();
        pos.apply_delta(
            body.shares_to_exercise,
            principal_delta,
            Some(format!("exercise:{}", exercise_id)),
        )
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
        store
            .write::<Position>(&pos, pos.position_id, "main", "update position for exercise")
            .await?;
        pid
    } else {
        let position = Position::new(
            entity_id,
            body.holder_id,
            grant.instrument_id,
            body.shares_to_exercise,
            principal_delta,
            Some(format!("exercise:{}", exercise_id)),
        )
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
        let pid = position.position_id;
        store
            .write::<Position>(
                &position,
                position.position_id,
                "main",
                "create position for exercise",
            )
            .await?;
        pid
    };

    // 10. Create the OptionExercise record (with position_id)
    let exercise = OptionExercise {
        exercise_id,
        entity_id,
        grant_id,
        holder_id: body.holder_id,
        shares_exercised: ShareCount::new(body.shares_to_exercise),
        strike_price_cents: strike_price,
        total_cost_cents: body.shares_to_exercise * strike_price,
        exercise_date,
        exercise_type: exercise_type.clone(),
        position_id: result_position_id,
        created_at: chrono::Utc::now(),
    };
    store
        .write::<OptionExercise>(
            &exercise,
            exercise.exercise_id,
            "main",
            "exercise option",
        )
        .await?;

    // 11. If early exercise with unvested shares, create repurchase right
    if exercise_type == ExerciseType::Early {
        let unvested_exercised = body.shares_to_exercise - (vested_shares - previously_exercised).max(0);
        if unvested_exercised > 0 {
            let mut rr = RepurchaseRight::new(
                entity_id,
                grant_id,
                ShareCount::new(unvested_exercised),
                strike_price,
                None,
            );
            rr.activate();
            store
                .write::<RepurchaseRight>(
                    &rr,
                    rr.repurchase_right_id,
                    "main",
                    "repurchase right for early exercise",
                )
                .await?;
        }
    }

    Ok(Json(exercise))
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
        return Err(AppError::BadRequest(
            "investment_amount_cents must be greater than zero".into(),
        ));
    }
    if let Some(cap) = body.valuation_cap_cents
        && cap <= 0
    {
        return Err(AppError::BadRequest(
            "valuation_cap_cents must be greater than zero".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    verify_cap_table_ownership(&store, body.cap_table_id, entity_id).await?;

    // Verify investor contact exists
    store
        .read::<Contact>(body.investor_contact_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => AppError::BadRequest(format!(
                    "investor contact {} not found",
                    body.investor_contact_id
                )),
                other => AppError::Storage(other),
            }
        })?;

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
        .write::<SafeNote>(
            &safe_note,
            safe_note.safe_note_id,
            "main",
            "issue safe note",
        )
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

/// Convert a SAFE note into equity.
///
/// This is a full orchestration endpoint that:
/// 1. Transitions the SafeNote status to `Converted`
/// 2. Creates an EquityGrant for the conversion shares
/// 3. Creates a Position for the investor's new holdings
async fn convert_safe(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, safe_id)): Path<(EntityId, SafeNoteId)>,
    Json(body): Json<ConvertSafeRequest>,
) -> Result<Json<SafeNote>, AppError> {
    if body.conversion_shares <= 0 {
        return Err(AppError::BadRequest(
            "conversion_shares must be greater than zero".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut safe_note = store.read::<SafeNote>(safe_id, "main").await?;

    // Verify target instrument exists
    let instrument = store
        .read::<Instrument>(body.instrument_id, "main")
        .await?;

    // Verify holder exists
    store
        .read::<Holder>(body.holder_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::BadRequest(format!("holder {} not found", body.holder_id))
                }
                other => AppError::Storage(other),
            }
        })?;

    // Over-issuance check: verify conversion won't exceed authorized units
    if let Some(authorized) = instrument.authorized_units {
        let existing_grants: Vec<EquityGrant> = store
            .read_all::<EquityGrant>("main")
            .await
            .map_err(AppError::Storage)?;
        let issued_shares: i64 = existing_grants
            .iter()
            .filter(|g| {
                g.instrument_id == body.instrument_id
                    && g.status != GrantStatus::Cancelled
                    && g.status != GrantStatus::Forfeited
            })
            .map(|g| g.shares.raw())
            .sum();
        let total_after = issued_shares + body.conversion_shares;
        if total_after > authorized {
            return Err(AppError::BadRequest(format!(
                "SAFE conversion would issue {} shares: {} already issued + {} conversion = {} total, \
                 exceeds authorized {} for instrument {}",
                body.conversion_shares, issued_shares, body.conversion_shares, total_after,
                authorized, instrument.symbol
            )));
        }
    }

    // 1. Transition SAFE status
    safe_note
        .convert()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<SafeNote>(&safe_note, safe_id, "main", "convert safe note")
        .await?;

    // 2. Create equity grant for the conversion
    let price_per_share = if body.conversion_shares > 0 {
        Some(safe_note.investment_amount_cents / body.conversion_shares)
    } else {
        None
    };
    let grant = EquityGrant::new(
        entity_id,
        instrument.cap_table_id,
        body.instrument_id,
        safe_note.investor_contact_id,
        &safe_note.investor_name,
        GrantType::PreferredStock,
        ShareCount::new(body.conversion_shares),
        price_per_share,
        None,
        None,
        None,
        None, // SAFE conversions don't require a separate resolution
    );
    store
        .write::<EquityGrant>(
            &grant,
            grant.grant_id,
            "main",
            "create grant from SAFE conversion",
        )
        .await?;

    // 3. Create position for the investor
    let position = Position::new(
        entity_id,
        body.holder_id,
        body.instrument_id,
        body.conversion_shares,
        safe_note.investment_amount_cents,
        Some(format!("safe_conversion:{}:{}", safe_id, grant.grant_id)),
    )
    .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Position>(
            &position,
            position.position_id,
            "main",
            "create position from SAFE conversion",
        )
        .await?;

    Ok(Json(safe_note))
}

async fn cancel_safe(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, safe_id)): Path<(EntityId, SafeNoteId)>,
) -> Result<Json<SafeNote>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        return Err(AppError::BadRequest(
            "valuation_amount_cents must be greater than zero".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    verify_cap_table_ownership(&store, body.cap_table_id, entity_id).await?;
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        return Err(AppError::BadRequest(
            "shares must be greater than zero".into(),
        ));
    }
    if body.from_holder_id == body.to_holder_id {
        return Err(AppError::BadRequest(
            "cannot transfer shares to the same holder".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    verify_cap_table_ownership(&store, body.cap_table_id, entity_id).await?;

    // Verify from_holder exists
    store
        .read::<Holder>(body.from_holder_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::BadRequest(format!("from_holder {} not found", body.from_holder_id))
                }
                other => AppError::Storage(other),
            }
        })?;

    // Verify to_holder exists
    store
        .read::<Holder>(body.to_holder_id, "main")
        .await
        .map_err(|e| {
            use corp_storage::error::StorageError;
            match e {
                StorageError::NotFound(_) => {
                    AppError::BadRequest(format!("to_holder {} not found", body.to_holder_id))
                }
                other => AppError::Storage(other),
            }
        })?;

    // Verify instrument exists
    store
        .read::<Instrument>(body.instrument_id, "main")
        .await?;

    // Validate sender has enough shares (sum active positions)
    let positions: Vec<Position> = store
        .read_all::<Position>("main")
        .await
        .map_err(AppError::Storage)?;
    let sender_balance: i64 = positions
        .iter()
        .filter(|p| {
            p.holder_id == body.from_holder_id
                && p.instrument_id == body.instrument_id
                && p.status == PositionStatus::Active
        })
        .map(|p| p.quantity_units)
        .sum();
    if sender_balance < body.shares {
        return Err(AppError::BadRequest(format!(
            "from_holder {} has {} shares of instrument {}, cannot transfer {}",
            body.from_holder_id, sender_balance, body.instrument_id, body.shares
        )));
    }

    let transfer = ShareTransfer::new(
        entity_id,
        body.cap_table_id,
        body.from_holder_id,
        body.to_holder_id,
        body.instrument_id,
        ShareCount::new(body.shares),
        body.transfer_type,
        body.price_per_share_cents,
    );
    store
        .write::<ShareTransfer>(&transfer, transfer.transfer_id, "main", "create transfer")
        .await?;
    Ok(Json(transfer))
}

async fn approve_transfer(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, transfer_id)): Path<(EntityId, TransferId)>,
) -> Result<Json<ShareTransfer>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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

/// Execute an approved transfer.
///
/// This is a full orchestration endpoint that:
/// 1. Transitions the transfer status to `Executed`
/// 2. Debits the sender's position (apply negative delta)
/// 3. Credits the receiver's position (find existing or create new)
async fn execute_transfer(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, transfer_id)): Path<(EntityId, TransferId)>,
) -> Result<Json<ShareTransfer>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut transfer = store.read::<ShareTransfer>(transfer_id, "main").await?;

    // 1. Transition status
    transfer
        .execute()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<ShareTransfer>(&transfer, transfer_id, "main", "execute transfer")
        .await?;

    let shares = transfer.shares.raw();
    let principal_cents = transfer
        .price_per_share_cents
        .map(|p| p * shares)
        .unwrap_or(0);
    let source = Some(format!("transfer:{}", transfer_id));

    // 2. Debit sender: find their active position for this instrument
    let positions: Vec<Position> = store
        .read_all::<Position>("main")
        .await
        .map_err(AppError::Storage)?;

    let sender_position = positions
        .iter()
        .find(|p| {
            p.holder_id == transfer.from_holder_id
                && p.instrument_id == transfer.instrument_id
                && p.status == PositionStatus::Active
        })
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "no active position found for sender {} on instrument {}",
                transfer.from_holder_id, transfer.instrument_id
            ))
        })?;

    let mut sender_pos = sender_position.clone();
    sender_pos
        .apply_delta(-shares, -principal_cents, source.clone())
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Position>(
            &sender_pos,
            sender_pos.position_id,
            "main",
            "debit sender position",
        )
        .await?;

    // 3. Credit receiver: find existing active position or create new one
    let receiver_position = positions.iter().find(|p| {
        p.holder_id == transfer.to_holder_id
            && p.instrument_id == transfer.instrument_id
            && p.status == PositionStatus::Active
    });

    if let Some(existing) = receiver_position {
        let mut receiver_pos = existing.clone();
        receiver_pos
            .apply_delta(shares, principal_cents, source)
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        store
            .write::<Position>(
                &receiver_pos,
                receiver_pos.position_id,
                "main",
                "credit receiver position",
            )
            .await?;
    } else {
        let new_pos = Position::new(
            entity_id,
            transfer.to_holder_id,
            transfer.instrument_id,
            shares,
            principal_cents,
            source,
        )
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
        store
            .write::<Position>(
                &new_pos,
                new_pos.position_id,
                "main",
                "create receiver position",
            )
            .await?;
    }

    Ok(Json(transfer))
}

async fn deny_transfer(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, transfer_id)): Path<(EntityId, TransferId)>,
) -> Result<Json<ShareTransfer>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        return Err(AppError::BadRequest(
            "target_amount_cents must be greater than zero".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    verify_cap_table_ownership(&store, body.cap_table_id, entity_id).await?;
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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

async fn advance_round(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, round_id)): Path<(EntityId, FundingRoundId)>,
) -> Result<Json<FundingRound>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
    use crate::routes::validation::validate_name;
    validate_name("name", &body.name)?;

    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        return Err(AppError::BadRequest(
            "total_shares must be greater than zero".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut event = store.read::<VestingEvent>(event_id, "main").await?;
    event
        .vest()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<VestingEvent>(&event, event_id, "main", "vest event")
        .await?;

    // Update the grant's vested_shares aggregate
    let mut grant = store
        .read::<EquityGrant>(event.grant_id, "main")
        .await?;
    grant.vested_shares = grant
        .vested_shares
        .checked_add(event.share_count)
        .unwrap_or(grant.vested_shares);
    store
        .write::<EquityGrant>(&grant, grant.grant_id, "main", "update vested_shares")
        .await?;

    Ok(Json(event))
}

async fn forfeit_event(
    RequireEquityWrite(principal): RequireEquityWrite,
    State(state): State<AppState>,
    Path((entity_id, event_id)): Path<(EntityId, VestingEventId)>,
) -> Result<Json<VestingEvent>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let position = Position::new(
        entity_id,
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut position = store.read::<Position>(position_id, "main").await?;
    position
        .apply_delta(
            body.quantity_delta,
            body.principal_delta.unwrap_or(0),
            body.source_reference,
        )
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        return Err(AppError::BadRequest(
            "share_count must be greater than zero".into(),
        ));
    }
    if body.price_per_share_cents < 0 {
        return Err(AppError::BadRequest(
            "price_per_share_cents must be non-negative".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
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
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut right = store.read::<RepurchaseRight>(rr_id, "main").await?;
    right.waive();
    store
        .write::<RepurchaseRight>(&right, rr_id, "main", "waive repurchase right")
        .await?;
    Ok(Json(right))
}
