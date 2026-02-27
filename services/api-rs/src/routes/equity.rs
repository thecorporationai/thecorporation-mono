//! Equity HTTP routes.
//!
//! Endpoints for cap tables, equity grants, SAFE notes, valuations,
//! share transfers, and funding rounds.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::domain::equity::{
    funding_round::FundingRound,
    grant::EquityGrant,
    safe_note::SafeNote,
    transfer::ShareTransfer,
    types::*,
    valuation::Valuation,
};
use crate::domain::ids::{
    ContactId, EntityId, EquityGrantId, FundingRoundId, SafeNoteId, ShareClassId, TransferId,
    ValuationId, WorkspaceId,
};
use crate::domain::treasury::types::Cents;
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Query types ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct EntityQuery {
    pub workspace_id: WorkspaceId,
}

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateGrantRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub grant_type: Option<GrantType>,
    pub shares: i64,
    pub recipient_name: String,
    #[serde(default)]
    pub recipient_type: Option<RecipientType>,
    #[serde(default)]
    pub share_class_id: Option<ShareClassId>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Deserialize)]
pub struct CreateSafeNoteRequest {
    pub entity_id: EntityId,
    pub investor_name: String,
    pub principal_amount_cents: i64,
    pub safe_type: SafeType,
    #[serde(default)]
    pub valuation_cap_cents: Option<i64>,
    #[serde(default)]
    pub discount_rate: Option<f64>,
    #[serde(default)]
    pub pro_rata_rights: Option<bool>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Deserialize)]
pub struct CreateValuationRequest {
    pub entity_id: EntityId,
    pub valuation_type: ValuationType,
    pub methodology: ValuationMethodology,
    #[serde(default)]
    pub fmv_per_share_cents: Option<i64>,
    #[serde(default)]
    pub enterprise_value_cents: Option<i64>,
    pub effective_date: NaiveDate,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Deserialize)]
pub struct CreateTransferRequest {
    pub entity_id: EntityId,
    pub share_class_id: ShareClassId,
    pub from_contact_id: ContactId,
    pub to_contact_id: ContactId,
    pub transfer_type: TransferType,
    pub shares: i64,
    #[serde(default)]
    pub price_per_share_cents: Option<i64>,
    #[serde(default)]
    pub governing_doc_type: Option<GoverningDocType>,
    #[serde(default)]
    pub transferee_rights: Option<TransfereeRights>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Deserialize)]
pub struct CreateFundingRoundRequest {
    pub entity_id: EntityId,
    pub round_name: String,
    #[serde(default)]
    pub pre_money_valuation_cents: Option<i64>,
    #[serde(default)]
    pub lead_investor_id: Option<ContactId>,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Deserialize)]
pub struct BylawsReviewRequest {
    pub approved: bool,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub reviewer: Option<String>,
}

#[derive(Deserialize)]
pub struct RofrDecisionRequest {
    #[serde(default)]
    pub offered: bool,
    #[serde(default)]
    pub waived: bool,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ShareClassSummary {
    pub share_class_id: ShareClassId,
    pub class_code: String,
    pub stock_type: StockType,
    pub authorized: i64,
    pub outstanding: i64,
}

#[derive(Serialize)]
pub struct HolderSummary {
    pub name: String,
    pub recipient_type: RecipientType,
    pub shares: i64,
    pub share_class_id: ShareClassId,
}

#[derive(Serialize)]
pub struct CapTableResponse {
    pub entity_id: EntityId,
    pub share_classes: Vec<ShareClassSummary>,
    pub total_authorized: i64,
    pub total_outstanding: i64,
    pub holders: Vec<HolderSummary>,
    pub outstanding_safes: usize,
}

#[derive(Serialize)]
pub struct GrantResponse {
    pub grant_id: EquityGrantId,
    pub entity_id: EntityId,
    pub share_class_id: ShareClassId,
    pub grant_type: Option<GrantType>,
    pub shares: i64,
    pub recipient_name: String,
    pub recipient_type: RecipientType,
    pub status: GrantStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SafeNoteResponse {
    pub safe_note_id: SafeNoteId,
    pub entity_id: EntityId,
    pub investor_name: String,
    pub principal_amount_cents: i64,
    pub safe_type: SafeType,
    pub valuation_cap_cents: Option<i64>,
    pub discount_rate: Option<f64>,
    pub status: SafeStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ValuationResponse {
    pub valuation_id: ValuationId,
    pub entity_id: EntityId,
    pub valuation_type: ValuationType,
    pub methodology: ValuationMethodology,
    pub fmv_per_share_cents: Option<i64>,
    pub enterprise_value_cents: Option<i64>,
    pub effective_date: NaiveDate,
    pub expiration_date: Option<NaiveDate>,
    pub status: ValuationStatus,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct TransferResponse {
    pub transfer_id: TransferId,
    pub entity_id: EntityId,
    pub share_class_id: ShareClassId,
    pub from_contact_id: ContactId,
    pub to_contact_id: ContactId,
    pub transfer_type: TransferType,
    pub shares: i64,
    pub price_per_share_cents: Option<i64>,
    pub status: TransferStatus,
    pub rofr_waived: bool,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct FundingRoundResponse {
    pub funding_round_id: FundingRoundId,
    pub entity_id: EntityId,
    pub round_name: String,
    pub pre_money_valuation_cents: Option<i64>,
    pub price_per_share_cents: Option<i64>,
    pub shares_issued: Option<i64>,
    pub status: FundingRoundStatus,
    pub lead_investor_id: Option<ContactId>,
    pub created_at: String,
}

// ── Conversion helpers ───────────────────────────────────────────────

fn grant_to_response(g: &EquityGrant) -> GrantResponse {
    GrantResponse {
        grant_id: g.grant_id(),
        entity_id: g.entity_id(),
        share_class_id: g.share_class_id(),
        grant_type: g.grant_type(),
        shares: g.share_count().raw(),
        recipient_name: g.recipient_name().to_owned(),
        recipient_type: g.recipient_type(),
        status: g.status(),
        created_at: g.created_at().to_rfc3339(),
    }
}

fn safe_to_response(s: &SafeNote) -> SafeNoteResponse {
    SafeNoteResponse {
        safe_note_id: s.safe_note_id(),
        entity_id: s.entity_id(),
        investor_name: s.investor_name().to_owned(),
        principal_amount_cents: s.principal_amount_cents().raw(),
        safe_type: s.safe_type(),
        valuation_cap_cents: s.valuation_cap_cents().map(|c| c.raw()),
        discount_rate: s.discount_rate(),
        status: s.status(),
        created_at: s.created_at().to_rfc3339(),
    }
}

fn valuation_to_response(v: &Valuation) -> ValuationResponse {
    ValuationResponse {
        valuation_id: v.valuation_id(),
        entity_id: v.entity_id(),
        valuation_type: v.valuation_type(),
        methodology: v.methodology(),
        fmv_per_share_cents: v.fmv_per_share_cents().map(|c| c.raw()),
        enterprise_value_cents: v.enterprise_value_cents().map(|c| c.raw()),
        effective_date: v.effective_date(),
        expiration_date: v.expiration_date(),
        status: v.status(),
        created_at: v.created_at().to_rfc3339(),
    }
}

fn transfer_to_response(t: &ShareTransfer) -> TransferResponse {
    TransferResponse {
        transfer_id: t.transfer_id(),
        entity_id: t.entity_id(),
        share_class_id: t.share_class_id(),
        from_contact_id: t.sender_contact_id(),
        to_contact_id: t.to_contact_id(),
        transfer_type: t.transfer_type(),
        shares: t.share_count().raw(),
        price_per_share_cents: t.price_per_share_cents().map(|c| c.raw()),
        status: t.status(),
        rofr_waived: t.rofr_waived(),
        created_at: t.created_at().to_rfc3339(),
    }
}

fn funding_round_to_response(r: &FundingRound) -> FundingRoundResponse {
    FundingRoundResponse {
        funding_round_id: r.funding_round_id(),
        entity_id: r.entity_id(),
        round_name: r.round_name().to_owned(),
        pre_money_valuation_cents: r.pre_money_valuation_cents().map(|c| c.raw()),
        price_per_share_cents: r.price_per_share_cents().map(|c| c.raw()),
        shares_issued: r.shares_issued().map(|s| s.raw()),
        status: r.status(),
        lead_investor_id: r.lead_investor_id(),
        created_at: r.created_at().to_rfc3339(),
    }
}

// ── Helper to open a store ───────────────────────────────────────────

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

// ── Handlers: Cap table ──────────────────────────────────────────────

async fn get_cap_table(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<CapTableResponse>, AppError> {
    let workspace_id = query.workspace_id;

    let resp = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Read share classes
            let class_ids = store.list_share_class_ids("main").map_err(|e| {
                AppError::Internal(format!("list share classes: {e}"))
            })?;

            let mut classes = Vec::new();
            for id in &class_ids {
                let sc = store.read_share_class("main", *id).map_err(|e| {
                    AppError::Internal(format!("read share class {id}: {e}"))
                })?;
                classes.push(sc);
            }

            // Read grants
            let grant_ids = store.list_grant_ids("main").unwrap_or_default();
            let mut grants = Vec::new();
            for id in &grant_ids {
                if let Ok(g) = store.read_grant("main", *id) {
                    grants.push(g);
                }
            }

            // Compute outstanding per class
            let mut class_summaries = Vec::new();
            let mut total_authorized: i64 = 0;
            let mut total_outstanding: i64 = 0;

            for sc in &classes {
                let outstanding: i64 = grants
                    .iter()
                    .filter(|g| {
                        g.share_class_id() == sc.share_class_id()
                            && g.status() == GrantStatus::Issued
                    })
                    .map(|g| g.share_count().raw())
                    .try_fold(0i64, |acc, v| acc.checked_add(v))
                    .ok_or_else(|| AppError::Internal("integer overflow computing outstanding shares".to_owned()))?;

                total_authorized = total_authorized
                    .checked_add(sc.authorized_shares().raw())
                    .ok_or_else(|| AppError::Internal("integer overflow computing total authorized shares".to_owned()))?;
                total_outstanding = total_outstanding
                    .checked_add(outstanding)
                    .ok_or_else(|| AppError::Internal("integer overflow computing total outstanding shares".to_owned()))?;

                class_summaries.push(ShareClassSummary {
                    share_class_id: sc.share_class_id(),
                    class_code: sc.class_code().to_owned(),
                    stock_type: sc.stock_type(),
                    authorized: sc.authorized_shares().raw(),
                    outstanding,
                });
            }

            // Build holder summaries
            let holders: Vec<HolderSummary> = grants
                .iter()
                .filter(|g| g.status() == GrantStatus::Issued)
                .map(|g| HolderSummary {
                    name: g.recipient_name().to_owned(),
                    recipient_type: g.recipient_type(),
                    shares: g.share_count().raw(),
                    share_class_id: g.share_class_id(),
                })
                .collect();

            // Count outstanding SAFEs
            let safe_ids = store.list_safe_note_ids("main").unwrap_or_default();
            let mut outstanding_safes = 0usize;
            for id in &safe_ids {
                if let Ok(s) = store.read_safe_note("main", *id) {
                    if s.status() == SafeStatus::Issued {
                        outstanding_safes += 1;
                    }
                }
            }

            Ok::<_, AppError>(CapTableResponse {
                entity_id,
                share_classes: class_summaries,
                total_authorized,
                total_outstanding,
                holders,
                outstanding_safes,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(resp))
}

// ── Handlers: Equity grants ──────────────────────────────────────────

async fn create_grant(
    State(state): State<AppState>,
    Json(req): Json<CreateGrantRequest>,
) -> Result<Json<GrantResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let grant = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            // Determine share class (use provided or find the first one)
            let share_class_id = match req.share_class_id {
                Some(id) => id,
                None => {
                    let ids = store.list_share_class_ids("main").map_err(|e| {
                        AppError::Internal(format!("list share classes: {e}"))
                    })?;
                    *ids.first().ok_or_else(|| {
                        AppError::BadRequest(
                            "no share classes exist; provide share_class_id".into(),
                        )
                    })?
                }
            };

            if req.shares <= 0 {
                return Err(AppError::BadRequest("shares must be positive".to_owned()));
            }

            let grant_id = EquityGrantId::new();
            let issuance_id = format!("grant-{}", grant_id);
            let grant = EquityGrant::new(
                grant_id,
                entity_id,
                share_class_id,
                issuance_id,
                req.recipient_name,
                req.recipient_type.unwrap_or(RecipientType::NaturalPerson),
                req.grant_type,
                ShareCount::new(req.shares),
                None,
                None,
                None,
                None,
                Some(true),
            )?;

            let path = format!("cap-table/grants/{}.json", grant_id);
            store
                .write_json(
                    "main",
                    &path,
                    &grant,
                    &format!("Issue equity grant {grant_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(grant)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(grant_to_response(&grant)))
}

// ── Handlers: SAFE notes ─────────────────────────────────────────────

async fn create_safe_note(
    State(state): State<AppState>,
    Json(req): Json<CreateSafeNoteRequest>,
) -> Result<Json<SafeNoteResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let safe = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let safe_id = SafeNoteId::new();
            let valuation_cap = req.valuation_cap_cents.map(Cents::new);

            let safe = SafeNote::new(
                safe_id,
                entity_id,
                req.investor_name,
                None, // investor_id
                Cents::new(req.principal_amount_cents),
                valuation_cap,
                req.discount_rate,
                req.safe_type,
                req.pro_rata_rights.unwrap_or(false),
                None, // document_id
                "shares".to_string(),
            )?;

            let path = format!("safe-notes/{}.json", safe_id);
            store
                .write_json(
                    "main",
                    &path,
                    &safe,
                    &format!("Issue SAFE note {safe_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(safe)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(safe_to_response(&safe)))
}

async fn list_safe_notes(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<SafeNoteResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let safes = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_safe_note_ids("main").map_err(|e| {
                AppError::Internal(format!("list safe notes: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let s = store.read_safe_note("main", id).map_err(|e| {
                    AppError::Internal(format!("read safe note {id}: {e}"))
                })?;
                results.push(safe_to_response(&s));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(safes))
}

async fn get_safe_note(
    State(state): State<AppState>,
    Path(safe_note_id): Path<SafeNoteId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<SafeNoteResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let safe = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            store.read_safe_note("main", safe_note_id).map_err(|_| {
                AppError::NotFound(format!("safe note {} not found", safe_note_id))
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(safe_to_response(&safe)))
}

// ── Handlers: Valuations ─────────────────────────────────────────────

async fn create_valuation(
    State(state): State<AppState>,
    Json(req): Json<CreateValuationRequest>,
) -> Result<Json<ValuationResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let val = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let val_id = ValuationId::new();
            let val = Valuation::new(
                val_id,
                entity_id,
                workspace_id,
                req.valuation_type,
                req.effective_date,
                req.fmv_per_share_cents.map(Cents::new),
                req.enterprise_value_cents.map(Cents::new),
                None, // hurdle_amount_cents
                req.methodology,
                None, // provider_contact_id
                None, // report_document_id
            );

            let path = format!("valuations/{}.json", val_id);
            store
                .write_json(
                    "main",
                    &path,
                    &val,
                    &format!("Create valuation {val_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(val)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(valuation_to_response(&val)))
}

async fn list_valuations(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<ValuationResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let vals = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_valuation_ids("main").map_err(|e| {
                AppError::Internal(format!("list valuations: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let v = store.read_valuation("main", id).map_err(|e| {
                    AppError::Internal(format!("read valuation {id}: {e}"))
                })?;
                results.push(valuation_to_response(&v));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(vals))
}

async fn get_current_409a(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Option<ValuationResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let result = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_valuation_ids("main").map_err(|e| {
                AppError::Internal(format!("list valuations: {e}"))
            })?;

            let mut current: Option<Valuation> = None;

            for id in ids {
                if let Ok(v) = store.read_valuation("main", id) {
                    if v.is_current_409a() {
                        match &current {
                            Some(existing) if v.effective_date() > existing.effective_date() => {
                                current = Some(v);
                            }
                            None => {
                                current = Some(v);
                            }
                            _ => {}
                        }
                    }
                }
            }

            Ok::<_, AppError>(current.map(|v| valuation_to_response(&v)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(result))
}

async fn get_valuation(
    State(state): State<AppState>,
    Path(valuation_id): Path<ValuationId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<ValuationResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let val = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            store.read_valuation("main", valuation_id).map_err(|_| {
                AppError::NotFound(format!("valuation {} not found", valuation_id))
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(valuation_to_response(&val)))
}

async fn approve_valuation(
    State(state): State<AppState>,
    Path(valuation_id): Path<ValuationId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<ValuationResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let val = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut val = store.read_valuation("main", valuation_id).map_err(|_| {
                AppError::NotFound(format!("valuation {} not found", valuation_id))
            })?;

            val.approve(None)?;

            let path = format!("valuations/{}.json", valuation_id);
            store
                .write_json("main", &path, &val, &format!("Approve valuation {valuation_id}"))
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(val)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(valuation_to_response(&val)))
}

async fn expire_valuation(
    State(state): State<AppState>,
    Path(valuation_id): Path<ValuationId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<ValuationResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let val = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut val = store.read_valuation("main", valuation_id).map_err(|_| {
                AppError::NotFound(format!("valuation {} not found", valuation_id))
            })?;

            val.expire()?;

            let path = format!("valuations/{}.json", valuation_id);
            store
                .write_json("main", &path, &val, &format!("Expire valuation {valuation_id}"))
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(val)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(valuation_to_response(&val)))
}

#[derive(Serialize)]
pub struct ExercisePriceCheckResponse {
    pub entity_id: EntityId,
    pub exercise_price_cents: i64,
    pub current_fmv_cents: Option<i64>,
    pub is_valid: bool,
    pub message: String,
}

#[derive(Deserialize)]
pub struct CheckExercisePriceRequest {
    pub exercise_price_cents: i64,
    #[serde(default)]
    pub workspace_id: Option<WorkspaceId>,
}

async fn check_exercise_price(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(req): Json<CheckExercisePriceRequest>,
) -> Result<Json<ExercisePriceCheckResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;

    let response = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let exercise_cents = req.exercise_price_cents;
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_valuation_ids("main").map_err(|e| {
                AppError::Internal(format!("list valuations: {e}"))
            })?;

            // Find current 409A
            let mut current_fmv: Option<i64> = None;
            for id in ids {
                if let Ok(v) = store.read_valuation("main", id) {
                    if v.is_current_409a() {
                        if let Some(fmv) = v.fmv_per_share_cents() {
                            current_fmv = Some(fmv.raw());
                        }
                    }
                }
            }

            let (is_valid, message) = match current_fmv {
                Some(fmv) if exercise_cents >= fmv => (
                    true,
                    format!("Exercise price {} >= FMV {}: valid", exercise_cents, fmv),
                ),
                Some(fmv) => (
                    false,
                    format!(
                        "Exercise price {} < FMV {}: must be at or above 409A FMV",
                        exercise_cents, fmv
                    ),
                ),
                None => (
                    false,
                    "No approved 409A valuation found — cannot validate exercise price".to_owned(),
                ),
            };

            Ok::<_, AppError>(ExercisePriceCheckResponse {
                entity_id,
                exercise_price_cents: exercise_cents,
                current_fmv_cents: current_fmv,
                is_valid,
                message,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(response))
}

// ── Handlers: Share transfers ────────────────────────────────────────

async fn create_transfer(
    State(state): State<AppState>,
    Json(req): Json<CreateTransferRequest>,
) -> Result<Json<TransferResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let transfer = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let transfer_id = TransferId::new();
            let price = req.price_per_share_cents.map(Cents::new);
            let transfer = ShareTransfer::new(
                transfer_id,
                entity_id,
                workspace_id,
                req.share_class_id,
                req.from_contact_id,
                req.to_contact_id,
                req.transfer_type,
                ShareCount::new(req.shares),
                price,
                None, // relationship_to_holder
                req.governing_doc_type
                    .unwrap_or(GoverningDocType::Bylaws),
                req.transferee_rights
                    .unwrap_or(TransfereeRights::FullMember),
            )?;

            let path = format!("cap-table/transfers/{}.json", transfer_id);
            store
                .write_json(
                    "main",
                    &path,
                    &transfer,
                    &format!("Create transfer {transfer_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(transfer)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(transfer_to_response(&transfer)))
}

async fn get_transfer(
    State(state): State<AppState>,
    Path(transfer_id): Path<TransferId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<TransferResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let transfer = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let t = store.read_transfer("main", transfer_id).map_err(|e| {
                AppError::NotFound(format!("transfer {transfer_id} not found: {e}"))
            })?;
            Ok::<_, AppError>(transfer_to_response(&t))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(transfer))
}

async fn list_transfers(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<TransferResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let transfers = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_transfer_ids("main").map_err(|e| {
                AppError::Internal(format!("list transfers: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let t = store.read_transfer("main", id).map_err(|e| {
                    AppError::Internal(format!("read transfer {id}: {e}"))
                })?;
                results.push(transfer_to_response(&t));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(transfers))
}

async fn submit_transfer_review(
    State(state): State<AppState>,
    Path(transfer_id): Path<TransferId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<TransferResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let transfer = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut t = store.read_transfer("main", transfer_id).map_err(|e| {
                AppError::NotFound(format!("transfer {transfer_id} not found: {e}"))
            })?;
            t.submit_for_review()?;
            let path = format!("cap-table/transfers/{}.json", transfer_id);
            store
                .write_json(
                    "main",
                    &path,
                    &t,
                    &format!("Submit transfer {transfer_id} for review"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(transfer_to_response(&t))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(transfer))
}

async fn record_bylaws_review(
    State(state): State<AppState>,
    Path(transfer_id): Path<TransferId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
    Json(req): Json<BylawsReviewRequest>,
) -> Result<Json<TransferResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let transfer = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut t = store.read_transfer("main", transfer_id).map_err(|e| {
                AppError::NotFound(format!("transfer {transfer_id} not found: {e}"))
            })?;
            t.record_bylaws_review(
                req.approved,
                req.notes.unwrap_or_default(),
                req.reviewer.unwrap_or_else(|| "system".to_string()),
            )?;
            let path = format!("cap-table/transfers/{}.json", transfer_id);
            store
                .write_json(
                    "main",
                    &path,
                    &t,
                    &format!("Bylaws review for transfer {transfer_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(transfer_to_response(&t))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(transfer))
}

async fn record_rofr_decision(
    State(state): State<AppState>,
    Path(transfer_id): Path<TransferId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
    Json(req): Json<RofrDecisionRequest>,
) -> Result<Json<TransferResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let transfer = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut t = store.read_transfer("main", transfer_id).map_err(|e| {
                AppError::NotFound(format!("transfer {transfer_id} not found: {e}"))
            })?;
            t.record_rofr_decision(req.offered, req.waived)?;
            let path = format!("cap-table/transfers/{}.json", transfer_id);
            store
                .write_json(
                    "main",
                    &path,
                    &t,
                    &format!("ROFR decision for transfer {transfer_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(transfer_to_response(&t))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(transfer))
}

async fn approve_transfer(
    State(state): State<AppState>,
    Path(transfer_id): Path<TransferId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<TransferResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let transfer = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut t = store.read_transfer("main", transfer_id).map_err(|e| {
                AppError::NotFound(format!("transfer {transfer_id} not found: {e}"))
            })?;
            t.approve(None)?;
            let path = format!("cap-table/transfers/{}.json", transfer_id);
            store
                .write_json(
                    "main",
                    &path,
                    &t,
                    &format!("Approve transfer {transfer_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(transfer_to_response(&t))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(transfer))
}

async fn execute_transfer(
    State(state): State<AppState>,
    Path(transfer_id): Path<TransferId>,
    Query(query): Query<super::WorkspaceEntityQuery>,
) -> Result<Json<TransferResponse>, AppError> {
    let workspace_id = query.workspace_id;
    let entity_id = query.entity_id;

    let transfer = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let mut t = store.read_transfer("main", transfer_id).map_err(|e| {
                AppError::NotFound(format!("transfer {transfer_id} not found: {e}"))
            })?;
            t.execute()?;
            let path = format!("cap-table/transfers/{}.json", transfer_id);
            store
                .write_json(
                    "main",
                    &path,
                    &t,
                    &format!("Execute transfer {transfer_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
            Ok::<_, AppError>(transfer_to_response(&t))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(transfer))
}

// ── Handlers: Funding rounds ─────────────────────────────────────────

async fn create_funding_round(
    State(state): State<AppState>,
    Json(req): Json<CreateFundingRoundRequest>,
) -> Result<Json<FundingRoundResponse>, AppError> {
    let workspace_id = req.workspace_id.ok_or_else(|| AppError::BadRequest("workspace_id is required".to_owned()))?;
    let entity_id = req.entity_id;

    let round = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;

            let round_id = FundingRoundId::new();
            let round = FundingRound::new(
                round_id,
                entity_id,
                req.round_name,
                req.pre_money_valuation_cents.map(Cents::new),
                req.lead_investor_id,
            );

            let path = format!("funding-rounds/{}.json", round_id);
            store
                .write_json(
                    "main",
                    &path,
                    &round,
                    &format!("Create funding round {round_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(round)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(funding_round_to_response(&round)))
}

async fn list_funding_rounds(
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<EntityQuery>,
) -> Result<Json<Vec<FundingRoundResponse>>, AppError> {
    let workspace_id = query.workspace_id;

    let rounds = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        move || {
            let store = open_store(&layout, workspace_id, entity_id)?;
            let ids = store.list_funding_round_ids("main").map_err(|e| {
                AppError::Internal(format!("list funding rounds: {e}"))
            })?;

            let mut results = Vec::new();
            for id in ids {
                let r = store.read_funding_round("main", id).map_err(|e| {
                    AppError::Internal(format!("read funding round {id}: {e}"))
                })?;
                results.push(funding_round_to_response(&r));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))?
    ?;

    Ok(Json(rounds))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn equity_routes() -> Router<AppState> {
    Router::new()
        // Cap table
        .route("/v1/entities/{entity_id}/cap-table", get(get_cap_table))
        // Equity grants
        .route("/v1/equity/grants", post(create_grant))
        // SAFE notes
        .route("/v1/safe-notes", post(create_safe_note))
        .route("/v1/safe-notes/{safe_note_id}", get(get_safe_note))
        .route("/v1/entities/{entity_id}/safe-notes", get(list_safe_notes))
        // Valuations
        .route("/v1/valuations", post(create_valuation))
        .route("/v1/valuations/{valuation_id}", get(get_valuation))
        .route("/v1/entities/{entity_id}/valuations", get(list_valuations))
        .route(
            "/v1/entities/{entity_id}/current-409a",
            get(get_current_409a),
        )
        .route(
            "/v1/valuations/{valuation_id}/approve",
            post(approve_valuation),
        )
        .route(
            "/v1/valuations/{valuation_id}/expire",
            post(expire_valuation),
        )
        .route(
            "/v1/entities/{entity_id}/check-exercise-price",
            post(check_exercise_price),
        )
        // Share transfers
        .route("/v1/share-transfers", post(create_transfer))
        .route("/v1/share-transfers/{transfer_id}", get(get_transfer))
        .route(
            "/v1/entities/{entity_id}/share-transfers",
            get(list_transfers),
        )
        .route(
            "/v1/share-transfers/{transfer_id}/submit-review",
            post(submit_transfer_review),
        )
        .route(
            "/v1/share-transfers/{transfer_id}/bylaws-review",
            post(record_bylaws_review),
        )
        .route(
            "/v1/share-transfers/{transfer_id}/rofr-decision",
            post(record_rofr_decision),
        )
        .route(
            "/v1/share-transfers/{transfer_id}/approve",
            post(approve_transfer),
        )
        .route(
            "/v1/share-transfers/{transfer_id}/execute",
            post(execute_transfer),
        )
        // Funding rounds
        .route("/v1/funding-rounds", post(create_funding_round))
        .route(
            "/v1/entities/{entity_id}/funding-rounds",
            get(list_funding_rounds),
        )
}
