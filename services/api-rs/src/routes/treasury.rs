//! Treasury HTTP routes.
//!
//! Endpoints for accounts, journal entries, invoices, and bank accounts.

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::AppState;
use super::validation::{
    require_non_empty_trimmed, validate_date_order, validate_not_too_far_future,
    validate_not_too_far_past,
};
use crate::auth::{RequireTreasuryRead, RequireTreasuryWrite};
use crate::domain::formation::types::FormationStatus;
use crate::domain::ids::{
    AccountId, BankAccountId, DistributionId, EntityId, InvoiceId, JournalEntryId, LedgerLineId,
    PaymentId, PayrollRunId, ReconciliationId, SpendingLimitId,
};
use crate::domain::treasury::{
    account::Account,
    bank_account::BankAccount,
    distribution::{Distribution, DistributionStatus, DistributionType},
    invoice::Invoice,
    journal_entry::{JournalEntry, JournalEntryStatus, LedgerLine},
    payment::{Payment, PaymentStatus},
    payroll::{PayrollRun, PayrollStatus},
    reconciliation::{Reconciliation, ReconciliationStatus},
    types::*,
};
use crate::error::AppError;
use crate::store::entity_store::EntityStore;

// ── Request types ────────────────────────────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateAccountRequest {
    pub entity_id: EntityId,
    pub account_code: GlAccountCode,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LedgerLineRequest {
    pub account_id: AccountId,
    pub side: Side,
    pub amount_cents: i64,
    #[serde(default)]
    pub memo: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateJournalEntryRequest {
    pub entity_id: EntityId,
    pub description: String,
    pub effective_date: NaiveDate,
    pub lines: Vec<LedgerLineRequest>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateInvoiceRequest {
    pub entity_id: EntityId,
    pub customer_name: String,
    pub amount_cents: i64,
    pub description: String,
    pub due_date: NaiveDate,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateBankAccountRequest {
    pub entity_id: EntityId,
    pub bank_name: String,
    #[serde(default)]
    pub account_type: Option<BankAccountType>,
}

// ── Response types ───────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct AccountResponse {
    pub account_id: AccountId,
    pub entity_id: EntityId,
    pub account_code: GlAccountCode,
    pub account_name: String,
    pub account_type: AccountType,
    pub normal_balance: Side,
    pub currency: Currency,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct JournalEntryResponse {
    pub journal_entry_id: JournalEntryId,
    pub entity_id: EntityId,
    pub description: String,
    pub effective_date: NaiveDate,
    pub total_debits_cents: i64,
    pub total_credits_cents: i64,
    pub status: JournalEntryStatus,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct InvoiceResponse {
    pub invoice_id: InvoiceId,
    pub entity_id: EntityId,
    pub customer_name: String,
    pub amount_cents: i64,
    pub description: String,
    pub due_date: NaiveDate,
    pub status: InvoiceStatus,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BankAccountResponse {
    pub bank_account_id: BankAccountId,
    pub entity_id: EntityId,
    pub bank_name: String,
    pub account_type: BankAccountType,
    pub currency: Currency,
    pub status: BankAccountStatus,
    pub created_at: String,
}

// ── Conversion helpers ───────────────────────────────────────────────

fn account_to_response(a: &Account) -> AccountResponse {
    AccountResponse {
        account_id: a.account_id(),
        entity_id: a.entity_id(),
        account_code: a.account_code(),
        account_name: a.account_name().to_owned(),
        account_type: a.account_type(),
        normal_balance: a.normal_balance(),
        currency: a.currency(),
        is_active: a.is_active(),
        created_at: a.created_at().to_rfc3339(),
    }
}

fn journal_entry_to_response(je: &JournalEntry) -> JournalEntryResponse {
    JournalEntryResponse {
        journal_entry_id: je.journal_entry_id(),
        entity_id: je.entity_id(),
        description: je.description().to_owned(),
        effective_date: je.effective_date(),
        total_debits_cents: je.total_debits().raw(),
        total_credits_cents: je.total_credits().raw(),
        status: je.status(),
        created_at: je.created_at().to_rfc3339(),
    }
}

fn invoice_to_response(inv: &Invoice) -> InvoiceResponse {
    InvoiceResponse {
        invoice_id: inv.invoice_id(),
        entity_id: inv.entity_id(),
        customer_name: inv.customer_name().to_owned(),
        amount_cents: inv.amount_cents().raw(),
        description: inv.description().to_owned(),
        due_date: inv.due_date(),
        status: inv.status(),
        created_at: inv.created_at().to_rfc3339(),
    }
}

fn bank_account_to_response(ba: &BankAccount) -> BankAccountResponse {
    BankAccountResponse {
        bank_account_id: ba.bank_account_id(),
        entity_id: ba.entity_id(),
        bank_name: ba.bank_name().to_owned(),
        account_type: ba.account_type(),
        currency: ba.currency(),
        status: ba.status(),
        created_at: ba.created_at().to_rfc3339(),
    }
}

fn payment_to_response(payment: &Payment) -> PaymentResponse {
    PaymentResponse {
        payment_id: payment.payment_id(),
        entity_id: payment.entity_id(),
        amount_cents: payment.amount_cents().raw(),
        recipient: payment.recipient().to_owned(),
        payment_method: payment.payment_method(),
        description: payment.description().to_owned(),
        status: payment.status(),
        created_at: payment.created_at().to_rfc3339(),
    }
}

fn payroll_run_to_response(run: &PayrollRun) -> PayrollRunResponse {
    PayrollRunResponse {
        payroll_run_id: run.payroll_run_id(),
        entity_id: run.entity_id(),
        pay_period_start: run.pay_period_start(),
        pay_period_end: run.pay_period_end(),
        status: run.status(),
        created_at: run.created_at().to_rfc3339(),
    }
}

fn distribution_to_response(dist: &Distribution) -> DistributionResponse {
    DistributionResponse {
        distribution_id: dist.distribution_id(),
        entity_id: dist.entity_id(),
        distribution_type: dist.distribution_type(),
        total_amount_cents: dist.total_amount_cents().raw(),
        description: dist.description().to_owned(),
        status: dist.status(),
        created_at: dist.created_at().to_rfc3339(),
    }
}

fn reconciliation_to_response(recon: &Reconciliation) -> ReconciliationResponse {
    ReconciliationResponse {
        reconciliation_id: recon.reconciliation_id(),
        entity_id: recon.entity_id(),
        as_of_date: recon.as_of_date(),
        total_debits_cents: recon.total_debits_cents().raw(),
        total_credits_cents: recon.total_credits_cents().raw(),
        difference_cents: recon.difference_cents().raw(),
        status: recon.status(),
        created_at: recon.created_at().to_rfc3339(),
    }
}


const MAX_FINANCIAL_AMOUNT_CENTS: i64 = 1_000_000_000_000; // $10B

fn validate_reasonable_amount(amount_cents: i64, field_name: &str) -> Result<(), AppError> {
    if amount_cents > MAX_FINANCIAL_AMOUNT_CENTS {
        return Err(AppError::BadRequest(format!(
            "{field_name} exceeds the supported maximum of {MAX_FINANCIAL_AMOUNT_CENTS} cents"
        )));
    }
    Ok(())
}

fn payment_method_requires_active_bank_account(method: PaymentMethod) -> bool {
    !matches!(method, PaymentMethod::Card)
}

fn ensure_active_bank_account_available(store: &EntityStore<'_>) -> Result<(), AppError> {
    let account_ids = store
        .list_ids::<BankAccount>("main")
        .map_err(|e| AppError::Internal(format!("list bank accounts: {e}")))?;
    let mut has_pending_review = false;
    for account_id in account_ids {
        let account = store
            .read::<BankAccount>("main", account_id)
            .map_err(|e| AppError::Internal(format!("read bank account {account_id}: {e}")))?;
        match account.status() {
            BankAccountStatus::Active => return Ok(()),
            BankAccountStatus::PendingReview => has_pending_review = true,
            BankAccountStatus::Closed => {}
        }
    }
    if has_pending_review {
        return Err(AppError::BadRequest(
            "payments require an active bank account; pending_review accounts cannot be used"
                .to_owned(),
        ));
    }
    Err(AppError::BadRequest(
        "payments require an active bank account".to_owned(),
    ))
}

fn ensure_entity_ready_for_treasury(
    store: &EntityStore<'_>,
    operation: &str,
) -> Result<(), AppError> {
    let entity = store
        .read_entity("main")
        .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
    if matches!(
        entity.formation_status(),
        FormationStatus::Pending | FormationStatus::Rejected | FormationStatus::Dissolved
    ) {
        return Err(AppError::BadRequest(format!(
            "{operation} requires a formed entity, currently {}",
            entity.formation_status()
        )));
    }
    Ok(())
}

// ── Handlers: Accounts ───────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/treasury/accounts",
    tag = "treasury",
    request_body = CreateAccountRequest,
    responses(
        (status = 200, description = "Account created", body = AccountResponse),
    ),
)]
async fn create_account(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<Json<AccountResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.account.create", workspace_id, 120, 60)?;

    let account = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "account creation")?;

            let account_id = AccountId::new();
            let account = Account::new(account_id, entity_id, req.account_code);

            let path = format!("treasury/accounts/{}.json", account_id);
            store
                .write_json(
                    "main",
                    &path,
                    &account,
                    &format!("Create GL account {account_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(account)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(account_to_response(&account)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/accounts",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of accounts", body = Vec<AccountResponse>),
    ),
)]
async fn list_accounts(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<AccountResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let accounts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<Account>("main")
                .map_err(|e| AppError::Internal(format!("list accounts: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let a = store
                    .read::<Account>("main", id)
                    .map_err(|e| AppError::Internal(format!("read account {id}: {e}")))?;
                results.push(account_to_response(&a));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(accounts))
}

// ── Handlers: Journal Entries ────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/treasury/journal-entries",
    tag = "treasury",
    request_body = CreateJournalEntryRequest,
    responses(
        (status = 200, description = "Journal entry created", body = JournalEntryResponse),
    ),
)]
async fn create_journal_entry(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateJournalEntryRequest>,
) -> Result<Json<JournalEntryResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.journal_entry.create", workspace_id, 120, 60)?;

    for line in &req.lines {
        if line.amount_cents < 0 {
            return Err(AppError::BadRequest(
                "journal entry line amounts must be non-negative".to_owned(),
            ));
        }
        validate_reasonable_amount(line.amount_cents, "line.amount_cents")?;
    }

    let entry = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "journal entry creation")?;

            let entry_id = JournalEntryId::new();
            let lines: Vec<LedgerLine> = req
                .lines
                .into_iter()
                .map(|l| {
                    LedgerLine::new(
                        LedgerLineId::new(),
                        l.account_id,
                        l.side,
                        Cents::new(l.amount_cents),
                        l.memo,
                    )
                })
                .collect();

            let entry = JournalEntry::new(
                entry_id,
                entity_id,
                req.description,
                req.effective_date,
                lines,
            )?;

            let path = format!("treasury/journal-entries/{}.json", entry_id);
            store
                .write_json(
                    "main",
                    &path,
                    &entry,
                    &format!("Create journal entry {entry_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(entry)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(journal_entry_to_response(&entry)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/journal-entries",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of journal entries", body = Vec<JournalEntryResponse>),
    ),
)]
async fn list_journal_entries(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<JournalEntryResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let entries = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<JournalEntry>("main")
                .map_err(|e| AppError::Internal(format!("list journal entries: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let je = store
                    .read::<JournalEntry>("main", id)
                    .map_err(|e| AppError::Internal(format!("read journal entry {id}: {e}")))?;
                results.push(journal_entry_to_response(&je));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(entries))
}

#[utoipa::path(
    post,
    path = "/v1/journal-entries/{entry_id}/post",
    tag = "treasury",
    params(
        ("entry_id" = JournalEntryId, Path, description = "Journal entry ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Journal entry posted", body = JournalEntryResponse),
    ),
)]
async fn post_journal_entry(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entry_id): Path<JournalEntryId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<JournalEntryResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let entry = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let mut entry = store
                .read::<JournalEntry>("main", entry_id)
                .map_err(|_| AppError::NotFound(format!("journal entry {} not found", entry_id)))?;

            entry.post()?;

            let path = format!("treasury/journal-entries/{}.json", entry_id);
            store
                .write_json(
                    "main",
                    &path,
                    &entry,
                    &format!("Post journal entry {entry_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(entry)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(journal_entry_to_response(&entry)))
}

#[utoipa::path(
    post,
    path = "/v1/journal-entries/{entry_id}/void",
    tag = "treasury",
    params(
        ("entry_id" = JournalEntryId, Path, description = "Journal entry ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Journal entry voided", body = JournalEntryResponse),
    ),
)]
async fn void_journal_entry(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entry_id): Path<JournalEntryId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<JournalEntryResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let entry = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let mut entry = store
                .read::<JournalEntry>("main", entry_id)
                .map_err(|_| AppError::NotFound(format!("journal entry {} not found", entry_id)))?;

            entry.void()?;

            let path = format!("treasury/journal-entries/{}.json", entry_id);
            store
                .write_json(
                    "main",
                    &path,
                    &entry,
                    &format!("Void journal entry {entry_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(entry)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(journal_entry_to_response(&entry)))
}

// ── Handlers: Invoices ───────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/treasury/invoices",
    tag = "treasury",
    request_body = CreateInvoiceRequest,
    responses(
        (status = 200, description = "Invoice created", body = InvoiceResponse),
    ),
)]
async fn create_invoice(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.invoice.create", workspace_id, 120, 60)?;
    let customer_name = require_non_empty_trimmed(&req.customer_name, "customer_name")?;

    if req.amount_cents <= 0 {
        return Err(AppError::BadRequest(
            "amount_cents must be positive".to_owned(),
        ));
    }
    validate_not_too_far_past("due_date", req.due_date, 365)?;
    validate_reasonable_amount(req.amount_cents, "amount_cents")?;

    let invoice = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let customer_name = customer_name.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "invoice creation")?;

            let invoice_id = InvoiceId::new();
            let invoice = Invoice::new(
                invoice_id,
                entity_id,
                customer_name,
                Cents::new(req.amount_cents),
                req.description,
                req.due_date,
            );

            let path = format!("treasury/invoices/{}.json", invoice_id);
            store
                .write_json(
                    "main",
                    &path,
                    &invoice,
                    &format!("Create invoice {invoice_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(invoice)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(invoice_to_response(&invoice)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/invoices",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of invoices", body = Vec<InvoiceResponse>),
    ),
)]
async fn list_invoices(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<InvoiceResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let invoices = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<Invoice>("main")
                .map_err(|e| AppError::Internal(format!("list invoices: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let inv = store
                    .read::<Invoice>("main", id)
                    .map_err(|e| AppError::Internal(format!("read invoice {id}: {e}")))?;
                results.push(invoice_to_response(&inv));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(invoices))
}

#[utoipa::path(
    post,
    path = "/v1/invoices/{invoice_id}/send",
    tag = "treasury",
    params(
        ("invoice_id" = InvoiceId, Path, description = "Invoice ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Invoice sent", body = InvoiceResponse),
    ),
)]
async fn send_invoice(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(invoice_id): Path<InvoiceId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let invoice = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let mut invoice = store
                .read::<Invoice>("main", invoice_id)
                .map_err(|_| AppError::NotFound(format!("invoice {} not found", invoice_id)))?;

            invoice.send()?;

            let path = format!("treasury/invoices/{}.json", invoice_id);
            store
                .write_json(
                    "main",
                    &path,
                    &invoice,
                    &format!("Send invoice {invoice_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(invoice)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(invoice_to_response(&invoice)))
}

#[utoipa::path(
    post,
    path = "/v1/invoices/{invoice_id}/mark-paid",
    tag = "treasury",
    params(
        ("invoice_id" = InvoiceId, Path, description = "Invoice ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Invoice marked as paid", body = InvoiceResponse),
    ),
)]
async fn mark_invoice_paid(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(invoice_id): Path<InvoiceId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let invoice = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let mut invoice = store
                .read::<Invoice>("main", invoice_id)
                .map_err(|_| AppError::NotFound(format!("invoice {} not found", invoice_id)))?;

            invoice.mark_paid()?;

            let path = format!("treasury/invoices/{}.json", invoice_id);
            store
                .write_json(
                    "main",
                    &path,
                    &invoice,
                    &format!("Mark invoice {invoice_id} paid"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(invoice)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(invoice_to_response(&invoice)))
}

#[utoipa::path(
    get,
    path = "/v1/invoices/{invoice_id}/status",
    tag = "treasury",
    params(
        ("invoice_id" = InvoiceId, Path, description = "Invoice ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Invoice status", body = InvoiceResponse),
    ),
)]
async fn get_invoice_status(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(invoice_id): Path<InvoiceId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let invoice = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            store
                .read::<Invoice>("main", invoice_id)
                .map_err(|_| AppError::NotFound(format!("invoice {} not found", invoice_id)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(invoice_to_response(&invoice)))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PayInstructionsResponse {
    pub invoice_id: InvoiceId,
    pub amount_cents: i64,
    pub currency: String,
    pub payment_method: String,
    pub instructions: String,
}

#[utoipa::path(
    get,
    path = "/v1/invoices/{invoice_id}/pay-instructions",
    tag = "treasury",
    params(
        ("invoice_id" = InvoiceId, Path, description = "Invoice ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Payment instructions for invoice", body = PayInstructionsResponse),
    ),
)]
async fn get_pay_instructions(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(invoice_id): Path<InvoiceId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<PayInstructionsResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let invoice = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            store
                .read::<Invoice>("main", invoice_id)
                .map_err(|_| AppError::NotFound(format!("invoice {} not found", invoice_id)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(PayInstructionsResponse {
        invoice_id: invoice.invoice_id(),
        amount_cents: invoice.amount_cents().raw(),
        currency: format!("{:?}", invoice.currency()),
        payment_method: "bank_transfer".to_owned(),
        instructions: format!(
            "Pay {} cents to {} for: {}",
            invoice.amount_cents().raw(),
            invoice.customer_name(),
            invoice.description()
        ),
    }))
}

// ── Handlers: Bank Accounts ──────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/treasury/bank-accounts",
    tag = "treasury",
    request_body = CreateBankAccountRequest,
    responses(
        (status = 200, description = "Bank account created", body = BankAccountResponse),
    ),
)]
async fn create_bank_account(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateBankAccountRequest>,
) -> Result<Json<BankAccountResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.bank_account.create", workspace_id, 60, 60)?;
    let bank_name = require_non_empty_trimmed(&req.bank_name, "bank_name")?;

    let bank_account = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let bank_name = bank_name.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "bank account creation")?;
            let existing_ids = store
                .list_ids::<BankAccount>("main")
                .map_err(|e| AppError::Internal(format!("list bank accounts: {e}")))?;
            let normalized_bank = bank_name.to_ascii_lowercase();
            for existing_id in existing_ids {
                let existing = store
                    .read::<BankAccount>("main", existing_id)
                    .map_err(|e| {
                        AppError::Internal(format!("read bank account {existing_id}: {e}"))
                    })?;
                if existing.entity_id() == entity_id
                    && existing.status() != BankAccountStatus::Closed
                    && existing.bank_name().trim().to_ascii_lowercase() == normalized_bank
                {
                    return Err(AppError::Conflict(format!(
                        "open bank account already exists for {}",
                        bank_name
                    )));
                }
            }

            let bank_account_id = BankAccountId::new();
            let bank_account = BankAccount::new(
                bank_account_id,
                entity_id,
                bank_name,
                req.account_type.unwrap_or_default(),
            );

            let path = format!("treasury/bank-accounts/{}.json", bank_account_id);
            store
                .write_json(
                    "main",
                    &path,
                    &bank_account,
                    &format!("Create bank account {bank_account_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(bank_account)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(bank_account_to_response(&bank_account)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/bank-accounts",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of bank accounts", body = Vec<BankAccountResponse>),
    ),
)]
async fn list_bank_accounts(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<BankAccountResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let bank_accounts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<BankAccount>("main")
                .map_err(|e| AppError::Internal(format!("list bank accounts: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let ba = store
                    .read::<BankAccount>("main", id)
                    .map_err(|e| AppError::Internal(format!("read bank account {id}: {e}")))?;
                results.push(bank_account_to_response(&ba));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(bank_accounts))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/payments",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of payments", body = Vec<PaymentResponse>),
    ),
)]
async fn list_payments(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<PaymentResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let payments = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<Payment>("main")
                .map_err(|e| AppError::Internal(format!("list payments: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let payment = store
                    .read::<Payment>("main", id)
                    .map_err(|e| AppError::Internal(format!("read payment {id}: {e}")))?;
                results.push(payment_to_response(&payment));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(payments))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/payroll-runs",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of payroll runs", body = Vec<PayrollRunResponse>),
    ),
)]
async fn list_payroll_runs(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<PayrollRunResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let payroll_runs = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<PayrollRun>("main")
                .map_err(|e| AppError::Internal(format!("list payroll runs: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let run = store
                    .read::<PayrollRun>("main", id)
                    .map_err(|e| AppError::Internal(format!("read payroll run {id}: {e}")))?;
                results.push(payroll_run_to_response(&run));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(payroll_runs))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/distributions",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of distributions", body = Vec<DistributionResponse>),
    ),
)]
async fn list_distributions(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<DistributionResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let distributions = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<Distribution>("main")
                .map_err(|e| AppError::Internal(format!("list distributions: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let dist = store
                    .read::<Distribution>("main", id)
                    .map_err(|e| AppError::Internal(format!("read distribution {id}: {e}")))?;
                results.push(distribution_to_response(&dist));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(distributions))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/reconciliations",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of reconciliations", body = Vec<ReconciliationResponse>),
    ),
)]
async fn list_reconciliations(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<ReconciliationResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let reconciliations = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<Reconciliation>("main")
                .map_err(|e| AppError::Internal(format!("list reconciliations: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let recon = store
                    .read::<Reconciliation>("main", id)
                    .map_err(|e| AppError::Internal(format!("read reconciliation {id}: {e}")))?;
                results.push(reconciliation_to_response(&recon));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(reconciliations))
}

#[utoipa::path(
    post,
    path = "/v1/bank-accounts/{bank_account_id}/activate",
    tag = "treasury",
    params(
        ("bank_account_id" = BankAccountId, Path, description = "Bank account ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Bank account activated", body = BankAccountResponse),
    ),
)]
async fn activate_bank_account(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(bank_account_id): Path<BankAccountId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<BankAccountResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let bank_account = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let mut ba = store
                .read::<BankAccount>("main", bank_account_id)
                .map_err(|_| {
                    AppError::NotFound(format!("bank account {} not found", bank_account_id))
                })?;

            ba.activate()?;

            let path = format!("treasury/bank-accounts/{}.json", bank_account_id);
            store
                .write_json(
                    "main",
                    &path,
                    &ba,
                    &format!("Activate bank account {bank_account_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(ba)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(bank_account_to_response(&bank_account)))
}

#[utoipa::path(
    post,
    path = "/v1/bank-accounts/{bank_account_id}/close",
    tag = "treasury",
    params(
        ("bank_account_id" = BankAccountId, Path, description = "Bank account ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Bank account closed", body = BankAccountResponse),
    ),
)]
async fn close_bank_account(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(bank_account_id): Path<BankAccountId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<BankAccountResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let bank_account = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let mut ba = store
                .read::<BankAccount>("main", bank_account_id)
                .map_err(|_| {
                    AppError::NotFound(format!("bank account {} not found", bank_account_id))
                })?;

            ba.close()?;

            let path = format!("treasury/bank-accounts/{}.json", bank_account_id);
            store
                .write_json(
                    "main",
                    &path,
                    &ba,
                    &format!("Close bank account {bank_account_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(ba)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(bank_account_to_response(&bank_account)))
}

// ── Request types: Payments, Payroll, Distributions, Reconciliation ──

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SubmitPaymentRequest {
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub recipient: String,
    #[serde(default = "default_payment_method")]
    pub payment_method: PaymentMethod,
    pub description: String,
}

fn default_payment_method() -> PaymentMethod {
    PaymentMethod::Ach
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreatePayrollRunRequest {
    pub entity_id: EntityId,
    pub pay_period_start: chrono::NaiveDate,
    pub pay_period_end: chrono::NaiveDate,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateDistributionRequest {
    pub entity_id: EntityId,
    #[serde(default = "default_distribution_type")]
    pub distribution_type: DistributionType,
    pub total_amount_cents: i64,
    pub description: String,
}

fn default_distribution_type() -> DistributionType {
    DistributionType::Dividend
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ReconcileLedgerRequest {
    pub entity_id: EntityId,
    #[serde(default)]
    pub as_of_date: Option<chrono::NaiveDate>,
    #[serde(default)]
    pub start_date: Option<chrono::NaiveDate>,
    #[serde(default)]
    pub end_date: Option<chrono::NaiveDate>,
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct PaymentResponse {
    pub payment_id: PaymentId,
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub recipient: String,
    pub payment_method: PaymentMethod,
    pub description: String,
    pub status: PaymentStatus,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PayrollRunResponse {
    pub payroll_run_id: PayrollRunId,
    pub entity_id: EntityId,
    pub pay_period_start: chrono::NaiveDate,
    pub pay_period_end: chrono::NaiveDate,
    pub status: PayrollStatus,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DistributionResponse {
    pub distribution_id: DistributionId,
    pub entity_id: EntityId,
    pub distribution_type: DistributionType,
    pub total_amount_cents: i64,
    pub description: String,
    pub status: DistributionStatus,
    pub created_at: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ReconciliationResponse {
    pub reconciliation_id: ReconciliationId,
    pub entity_id: EntityId,
    pub as_of_date: chrono::NaiveDate,
    pub total_debits_cents: i64,
    pub total_credits_cents: i64,
    pub difference_cents: i64,
    pub status: ReconciliationStatus,
    pub created_at: String,
}

// ── Handlers: Payments ──────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/payments",
    tag = "treasury",
    request_body = SubmitPaymentRequest,
    responses(
        (status = 200, description = "Payment submitted", body = PaymentResponse),
    ),
)]
async fn submit_payment(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<SubmitPaymentRequest>,
) -> Result<Json<PaymentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.payment.create", workspace_id, 120, 60)?;
    Cents::new(req.amount_cents)
        .require_positive()
        .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    validate_reasonable_amount(req.amount_cents, "amount_cents")?;

    let payment = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "payment submission")?;
            if payment_method_requires_active_bank_account(req.payment_method) {
                ensure_active_bank_account_available(&store)?;
            }

            let payment_id = PaymentId::new();
            let payment = Payment::new(
                payment_id,
                entity_id,
                Cents::new(req.amount_cents),
                req.recipient,
                req.payment_method,
                req.description,
            );

            let path = format!("treasury/payments/{}.json", payment_id);
            store
                .write_json(
                    "main",
                    &path,
                    &payment,
                    &format!("Submit payment {payment_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(payment)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(PaymentResponse {
        payment_id: payment.payment_id(),
        entity_id: payment.entity_id(),
        amount_cents: payment.amount_cents().raw(),
        recipient: payment.recipient().to_owned(),
        payment_method: payment.payment_method(),
        description: payment.description().to_owned(),
        status: payment.status(),
        created_at: payment.created_at().to_rfc3339(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/payments/execute",
    tag = "treasury",
    request_body = SubmitPaymentRequest,
    responses(
        (status = 200, description = "Payment executed", body = PaymentResponse),
    ),
)]
async fn execute_payment(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<SubmitPaymentRequest>,
) -> Result<Json<PaymentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.payment.execute", workspace_id, 120, 60)?;
    Cents::new(req.amount_cents)
        .require_positive()
        .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    validate_reasonable_amount(req.amount_cents, "amount_cents")?;

    let payment = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "payment execution")?;
            if payment_method_requires_active_bank_account(req.payment_method) {
                ensure_active_bank_account_available(&store)?;
            }

            let payment_id = PaymentId::new();
            let mut payment = Payment::new(
                payment_id,
                entity_id,
                Cents::new(req.amount_cents),
                req.recipient,
                req.payment_method,
                req.description,
            );
            payment.mark_completed();

            let path = format!("treasury/payments/{}.json", payment_id);
            store
                .write_json(
                    "main",
                    &path,
                    &payment,
                    &format!("Execute payment {payment_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(payment)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(PaymentResponse {
        payment_id: payment.payment_id(),
        entity_id: payment.entity_id(),
        amount_cents: payment.amount_cents().raw(),
        recipient: payment.recipient().to_owned(),
        payment_method: payment.payment_method(),
        description: payment.description().to_owned(),
        status: payment.status(),
        created_at: payment.created_at().to_rfc3339(),
    }))
}

// ── Handlers: Payroll ───────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/payroll/runs",
    tag = "treasury",
    request_body = CreatePayrollRunRequest,
    responses(
        (status = 200, description = "Payroll run created", body = PayrollRunResponse),
    ),
)]
async fn create_payroll_run(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreatePayrollRunRequest>,
) -> Result<Json<PayrollRunResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.payroll.create", workspace_id, 60, 60)?;
    validate_date_order(
        "pay_period_start",
        req.pay_period_start,
        "pay_period_end",
        req.pay_period_end,
    )?;
    if req.pay_period_start == req.pay_period_end {
        return Err(AppError::BadRequest(
            "pay_period_end must be after pay_period_start".to_owned(),
        ));
    }
    validate_not_too_far_future("pay_period_start", req.pay_period_start, 366)?;
    validate_not_too_far_future("pay_period_end", req.pay_period_end, 366)?;

    let run = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "payroll creation")?;
            let existing_ids = store
                .list_ids::<PayrollRun>("main")
                .map_err(|e| AppError::Internal(format!("list payroll runs: {e}")))?;
            for existing_id in existing_ids {
                let existing = store.read::<PayrollRun>("main", existing_id).map_err(|e| {
                    AppError::Internal(format!("read payroll run {existing_id}: {e}"))
                })?;
                let overlaps = req.pay_period_start <= existing.pay_period_end()
                    && req.pay_period_end >= existing.pay_period_start();
                if overlaps {
                    return Err(AppError::Conflict(format!(
                        "payroll period overlaps existing run {} ({} to {})",
                        existing.payroll_run_id(),
                        existing.pay_period_start(),
                        existing.pay_period_end()
                    )));
                }
            }

            let run_id = PayrollRunId::new();
            let run = PayrollRun::new(run_id, entity_id, req.pay_period_start, req.pay_period_end);

            let path = format!("treasury/payroll/{}.json", run_id);
            store
                .write_json("main", &path, &run, &format!("Create payroll run {run_id}"))
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(run)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(PayrollRunResponse {
        payroll_run_id: run.payroll_run_id(),
        entity_id: run.entity_id(),
        pay_period_start: run.pay_period_start(),
        pay_period_end: run.pay_period_end(),
        status: run.status(),
        created_at: run.created_at().to_rfc3339(),
    }))
}

// ── Handlers: Distributions ─────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/distributions",
    tag = "treasury",
    request_body = CreateDistributionRequest,
    responses(
        (status = 200, description = "Distribution created", body = DistributionResponse),
    ),
)]
async fn create_distribution(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateDistributionRequest>,
) -> Result<Json<DistributionResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.distribution.create", workspace_id, 60, 60)?;
    Cents::new(req.total_amount_cents)
        .require_positive()
        .map_err(|e| AppError::BadRequest(e.to_owned()))?;
    validate_reasonable_amount(req.total_amount_cents, "total_amount_cents")?;

    let dist = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let entity = store
                .read_entity("main")
                .map_err(|e| AppError::Internal(format!("read entity: {e}")))?;
            if req.distribution_type == DistributionType::Dividend
                && entity.formation_status() != FormationStatus::Active
            {
                return Err(AppError::BadRequest(
                    "dividend distributions require an active entity".to_owned(),
                ));
            }
            if req.distribution_type == DistributionType::Liquidation
                && entity.formation_status() != FormationStatus::Dissolved
            {
                return Err(AppError::BadRequest(
                    "liquidation distributions require a dissolved entity".to_owned(),
                ));
            }
            if matches!(
                entity.formation_status(),
                FormationStatus::Pending
                    | FormationStatus::DocumentsGenerated
                    | FormationStatus::FilingSubmitted
                    | FormationStatus::Filed
            ) {
                return Err(AppError::BadRequest(
                    "distributions require an active or dissolved entity".to_owned(),
                ));
            }

            let dist_id = DistributionId::new();
            let dist = Distribution::new(
                dist_id,
                entity_id,
                req.distribution_type,
                Cents::new(req.total_amount_cents),
                req.description,
            );

            let path = format!("treasury/distributions/{}.json", dist_id);
            store
                .write_json(
                    "main",
                    &path,
                    &dist,
                    &format!("Create distribution {dist_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(dist)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(DistributionResponse {
        distribution_id: dist.distribution_id(),
        entity_id: dist.entity_id(),
        distribution_type: dist.distribution_type(),
        total_amount_cents: dist.total_amount_cents().raw(),
        description: dist.description().to_owned(),
        status: dist.status(),
        created_at: dist.created_at().to_rfc3339(),
    }))
}

// ── Handlers: Reconciliation ────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/v1/ledger/reconcile",
    tag = "treasury",
    request_body = ReconcileLedgerRequest,
    responses(
        (status = 200, description = "Ledger reconciled", body = ReconciliationResponse),
    ),
)]
async fn reconcile_ledger(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<ReconcileLedgerRequest>,
) -> Result<Json<ReconciliationResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    let today = chrono::Utc::now().date_naive();
    let as_of_date = req.end_date.or(req.as_of_date).unwrap_or(today);
    let start_date = req.start_date;
    if let Some(start_date) = start_date
        && as_of_date < start_date
    {
        return Err(AppError::BadRequest(
            "end_date must be on or after start_date".to_owned(),
        ));
    }

    let recon = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "ledger reconciliation")?;

            // Sum up all journal entries to compute totals
            let entry_ids = store
                .list_ids::<JournalEntry>("main")
                .map_err(|e| AppError::Internal(format!("list entries: {e}")))?;

            let mut total_debits = Cents::ZERO;
            let mut total_credits = Cents::ZERO;

            for id in entry_ids {
                if let Ok(entry) = store.read::<JournalEntry>("main", id) {
                    if entry.effective_date() > as_of_date {
                        continue;
                    }
                    if start_date.is_some_and(|start| entry.effective_date() < start) {
                        continue;
                    }
                    total_debits += entry.total_debits();
                    total_credits += entry.total_credits();
                }
            }

            for id in store.list_ids::<Invoice>("main").unwrap_or_default() {
                if let Ok(invoice) = store.read::<Invoice>("main", id) {
                    if invoice.status() != InvoiceStatus::Paid {
                        continue;
                    }
                    if invoice.due_date() > as_of_date {
                        continue;
                    }
                    if start_date.is_some_and(|start| invoice.due_date() < start) {
                        continue;
                    }
                    // Paid invoices represent incoming revenue → credit.
                    total_credits += invoice.amount_cents();
                }
            }
            for id in store.list_ids::<Payment>("main").unwrap_or_default() {
                if let Ok(payment) = store.read::<Payment>("main", id) {
                    if payment.status() != PaymentStatus::Completed {
                        continue;
                    }
                    let payment_date = payment.created_at().date_naive();
                    if payment_date > as_of_date {
                        continue;
                    }
                    if start_date.is_some_and(|start| payment_date < start) {
                        continue;
                    }
                    // Completed payments represent outgoing funds → debit.
                    total_debits += payment.amount_cents();
                }
            }
            for id in store.list_ids::<Distribution>("main").unwrap_or_default() {
                if let Ok(distribution) = store.read::<Distribution>("main", id) {
                    let distribution_date = distribution.created_at().date_naive();
                    if distribution_date > as_of_date {
                        continue;
                    }
                    if start_date.is_some_and(|start| distribution_date < start) {
                        continue;
                    }
                    total_debits += distribution.total_amount_cents();
                    total_credits += distribution.total_amount_cents();
                }
            }

            let recon_id = ReconciliationId::new();
            let recon =
                Reconciliation::new(recon_id, entity_id, as_of_date, total_debits, total_credits);

            let path = format!("treasury/reconciliations/{}.json", recon_id);
            store
                .write_json(
                    "main",
                    &path,
                    &recon,
                    &format!("Reconcile ledger {recon_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(recon)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(ReconciliationResponse {
        reconciliation_id: recon.reconciliation_id(),
        entity_id: recon.entity_id(),
        as_of_date: recon.as_of_date(),
        total_debits_cents: recon.total_debits_cents().raw(),
        total_credits_cents: recon.total_credits_cents().raw(),
        difference_cents: recon.difference_cents().raw(),
        status: recon.status(),
        created_at: recon.created_at().to_rfc3339(),
    }))
}

// ── Treasury Advanced ──────────────────────────────────────────────

use crate::domain::ids::StripeConnectionId;
use crate::domain::treasury::spending_limit::SpendingLimit;
use crate::domain::treasury::stripe_connection::StripeConnection;

#[derive(Serialize, utoipa::ToSchema)]
pub struct StripeAccountResponse {
    pub entity_id: EntityId,
    pub stripe_account_id: String,
    pub status: String,
    pub created_at: String,
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/stripe-account",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Stripe account details", body = StripeAccountResponse),
    ),
)]
async fn get_stripe_account(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<StripeAccountResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let conn = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;

            // Try to read existing connection
            match store.read_json::<StripeConnection>("main", "treasury/stripe-connection.json") {
                Ok(c) => Ok::<_, AppError>(c),
                Err(crate::git::error::GitStorageError::NotFound(_)) => {
                    // Create a new connection only on first access (not found)
                    let conn = StripeConnection::new(
                        StripeConnectionId::new(),
                        entity_id,
                        format!("acct_{}", uuid::Uuid::new_v4().simple()),
                    );
                    store
                        .write_json(
                            "main",
                            "treasury/stripe-connection.json",
                            &conn,
                            "Create Stripe connection",
                        )
                        .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
                    Ok(conn)
                }
                Err(e) => Err(AppError::Internal(format!("read stripe connection: {e}"))),
            }
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(StripeAccountResponse {
        entity_id: conn.entity_id(),
        stripe_account_id: conn.stripe_account_id().to_owned(),
        status: conn.status().to_owned(),
        created_at: conn.created_at().to_rfc3339(),
    }))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SpendingLimitResponse {
    pub spending_limit_id: SpendingLimitId,
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub period: String,
    pub category: String,
    pub created_at: String,
}

fn spending_limit_to_response(sl: &SpendingLimit) -> SpendingLimitResponse {
    SpendingLimitResponse {
        spending_limit_id: sl.spending_limit_id(),
        entity_id: sl.entity_id(),
        amount_cents: sl.amount_cents(),
        period: sl.period().to_owned(),
        category: sl.category().to_owned(),
        created_at: sl.created_at().to_rfc3339(),
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateSpendingLimitRequest {
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub period: String,
    pub category: String,
}

#[utoipa::path(
    post,
    path = "/v1/spending-limits",
    tag = "treasury",
    request_body = CreateSpendingLimitRequest,
    responses(
        (status = 200, description = "Spending limit created", body = SpendingLimitResponse),
    ),
)]
async fn create_spending_limit(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateSpendingLimitRequest>,
) -> Result<Json<SpendingLimitResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let sl = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let sl_id = SpendingLimitId::new();
            let sl =
                SpendingLimit::new(sl_id, entity_id, req.amount_cents, req.period, req.category);
            let path = format!("treasury/spending-limits/{}.json", sl_id);
            store
                .write_json(
                    "main",
                    &path,
                    &sl,
                    &format!("Create spending limit {sl_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(sl)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(spending_limit_to_response(&sl)))
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/spending-limits",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "List of spending limits", body = Vec<SpendingLimitResponse>),
    ),
)]
async fn list_spending_limits(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<SpendingLimitResponse>>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let limits = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids: Vec<SpendingLimitId> = store
                .list_ids_in_dir("main", "treasury/spending-limits")
                .unwrap_or_default();

            let mut results = Vec::new();
            for id in ids {
                let path = format!("treasury/spending-limits/{}.json", id);
                if let Ok(sl) = store.read_json::<SpendingLimit>("main", &path) {
                    results.push(spending_limit_to_response(&sl));
                }
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(limits))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct FinancialStatementResponse {
    pub entity_id: EntityId,
    pub statement_type: String,
    pub period_start: String,
    pub period_end: String,
    pub total_assets_cents: i64,
    pub total_liabilities_cents: i64,
    pub total_equity_cents: i64,
    pub net_income_cents: i64,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct FinancialStatementQuery {
    #[serde(default = "default_statement_type")]
    pub statement_type: String,
}

fn default_statement_type() -> String {
    "balance_sheet".to_owned()
}

#[utoipa::path(
    get,
    path = "/v1/entities/{entity_id}/financial-statements",
    tag = "treasury",
    params(
        ("entity_id" = EntityId, Path, description = "Entity ID"),
        FinancialStatementQuery,
    ),
    responses(
        (status = 200, description = "Financial statements", body = FinancialStatementResponse),
    ),
)]
async fn get_financial_statements(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Query(query): Query<FinancialStatementQuery>,
) -> Result<Json<FinancialStatementResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let statement = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;

            // Read all accounts and compute totals by type
            let account_ids = store.list_ids::<Account>("main").unwrap_or_default();
            let mut total_assets: i64 = 0;
            let mut total_liabilities: i64 = 0;
            let mut total_equity: i64 = 0;
            let mut total_revenue: i64 = 0;
            let mut total_expenses: i64 = 0;

            // Build a map of account_id -> account_type
            let mut acct_types = std::collections::HashMap::new();
            for &acct_id in &account_ids {
                if let Ok(acct) = store.read::<Account>("main", acct_id) {
                    acct_types.insert(acct.account_id(), acct.account_type());
                }
            }

            // Read journal entries to sum by account type
            let entry_ids = store.list_ids::<JournalEntry>("main").unwrap_or_default();
            for entry_id in entry_ids {
                if let Ok(entry) = store.read::<JournalEntry>("main", entry_id) {
                    for line in entry.lines() {
                        if let Some(&acct_type) = acct_types.get(&line.account_id()) {
                            let signed = match line.side() {
                                Side::Debit => line.amount().raw(),
                                Side::Credit => -line.amount().raw(),
                            };
                            match acct_type {
                                AccountType::Asset => total_assets += signed,
                                AccountType::Liability => total_liabilities -= signed,
                                AccountType::Equity => total_equity -= signed,
                                AccountType::Revenue => total_revenue -= signed,
                                AccountType::Expense => total_expenses += signed,
                            }
                        }
                    }
                }
            }

            Ok::<_, AppError>(FinancialStatementResponse {
                entity_id,
                statement_type: query.statement_type,
                period_start: "2026-01-01".to_owned(),
                period_end: "2026-12-31".to_owned(),
                total_assets_cents: total_assets,
                total_liabilities_cents: total_liabilities,
                total_equity_cents: total_equity,
                net_income_cents: total_revenue - total_expenses,
            })
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(statement))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SeedChartOfAccountsRequest {
    pub entity_id: EntityId,
    #[serde(default = "default_template")]
    pub template: String,
}

fn default_template() -> String {
    "standard".to_owned()
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SeedChartOfAccountsResponse {
    pub entity_id: EntityId,
    pub accounts_created: usize,
    pub template: String,
}

#[utoipa::path(
    post,
    path = "/v1/treasury/seed-chart-of-accounts",
    tag = "treasury",
    request_body = SeedChartOfAccountsRequest,
    responses(
        (status = 200, description = "Chart of accounts seeded", body = SeedChartOfAccountsResponse),
    ),
)]
async fn seed_chart_of_accounts(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<SeedChartOfAccountsRequest>,
) -> Result<Json<SeedChartOfAccountsResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let count = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;

            let codes = vec![
                GlAccountCode::Cash,
                GlAccountCode::AccountsReceivable,
                GlAccountCode::AccountsPayable,
                GlAccountCode::Revenue,
                GlAccountCode::OperatingExpenses,
                GlAccountCode::FounderCapital,
            ];

            let mut created = 0;
            for code in &codes {
                let acct_id = AccountId::new();
                let acct = Account::new(acct_id, entity_id, *code);
                let path = format!("treasury/accounts/{}.json", acct_id);
                store
                    .write_json("main", &path, &acct, &format!("Seed account {:?}", code))
                    .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;
                created += 1;
            }

            Ok::<_, AppError>(created)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(SeedChartOfAccountsResponse {
        entity_id,
        accounts_created: count,
        template: req.template,
    }))
}

// ── Handlers: Get Invoice ────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/v1/invoices/{invoice_id}",
    tag = "treasury",
    params(
        ("invoice_id" = InvoiceId, Path, description = "Invoice ID"),
        super::EntityIdQuery,
    ),
    responses(
        (status = 200, description = "Invoice details", body = InvoiceResponse),
    ),
)]
async fn get_invoice(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(invoice_id): Path<InvoiceId>,
    Query(query): Query<super::EntityIdQuery>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = query.entity_id;

    let invoice = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            store
                .read::<Invoice>("main", invoice_id)
                .map_err(|_| AppError::NotFound(format!("invoice {} not found", invoice_id)))
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(invoice_to_response(&invoice)))
}

// ── Handlers: Invoice from agent request ────────────────────────────

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AgentInvoiceRequest {
    pub entity_id: EntityId,
    pub customer_name: String,
    pub amount_cents: i64,
    pub description: String,
    pub due_date: NaiveDate,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PaymentOfferResponse {
    pub invoice_id: InvoiceId,
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub payment_url: String,
    pub status: InvoiceStatus,
}

#[utoipa::path(
    post,
    path = "/v1/invoices/from-agent-request",
    tag = "treasury",
    request_body = AgentInvoiceRequest,
    responses(
        (status = 200, description = "Invoice created from agent request", body = PaymentOfferResponse),
    ),
)]
async fn from_agent_request(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<AgentInvoiceRequest>,
) -> Result<Json<PaymentOfferResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    if req.amount_cents <= 0 {
        return Err(AppError::BadRequest(
            "amount_cents must be positive".to_owned(),
        ));
    }
    validate_reasonable_amount(req.amount_cents, "amount_cents")?;

    let invoice = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "invoice creation")?;

            let invoice_id = InvoiceId::new();
            let mut invoice = Invoice::new(
                invoice_id,
                entity_id,
                req.customer_name,
                Cents::new(req.amount_cents),
                req.description,
                req.due_date,
            );

            // Auto-send for agent-initiated invoices
            let _ = invoice.send();

            let path = format!("treasury/invoices/{}.json", invoice_id);
            store
                .write_json(
                    "main",
                    &path,
                    &invoice,
                    &format!("Agent-created invoice {invoice_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit error: {e}")))?;

            Ok::<_, AppError>(invoice)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(PaymentOfferResponse {
        invoice_id: invoice.invoice_id(),
        entity_id: invoice.entity_id(),
        amount_cents: invoice.amount_cents().raw(),
        payment_url: format!("/v1/invoices/{}/pay-instructions", invoice.invoice_id()),
        status: invoice.status(),
    }))
}

// ── Handlers: Treasury advanced (stripe-accounts, chart-of-accounts, payouts, payment-intents) ──

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateStripeAccountRequest {
    pub entity_id: EntityId,
}

#[utoipa::path(
    post,
    path = "/v1/treasury/stripe-accounts",
    tag = "treasury",
    request_body = CreateStripeAccountRequest,
    responses(
        (status = 200, description = "Stripe account created", body = StripeAccountResponse),
    ),
)]
async fn create_stripe_account(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreateStripeAccountRequest>,
) -> Result<Json<StripeAccountResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;

    let conn = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;

            // Check if a Stripe connection already exists — reject duplicates.
            match store.read_json::<StripeConnection>("main", "treasury/stripe-connection.json") {
                Ok(_existing) => {
                    return Err(AppError::Conflict(
                        "stripe connection already exists for this entity".to_owned(),
                    ));
                }
                Err(crate::git::error::GitStorageError::NotFound(_)) => { /* expected */ }
                Err(e) => {
                    return Err(AppError::Internal(format!("read stripe connection: {e}")));
                }
            }

            let conn = StripeConnection::new(
                StripeConnectionId::new(),
                entity_id,
                format!("acct_{}", uuid::Uuid::new_v4().simple()),
            );
            store
                .write_json(
                    "main",
                    "treasury/stripe-connection.json",
                    &conn,
                    "Create Stripe connection",
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(conn)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(StripeAccountResponse {
        entity_id: conn.entity_id(),
        stripe_account_id: conn.stripe_account_id().to_owned(),
        status: conn.status().to_owned(),
        created_at: conn.created_at().to_rfc3339(),
    }))
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ChartOfAccountsResponse {
    pub entity_id: EntityId,
    pub accounts: Vec<AccountResponse>,
}

#[utoipa::path(
    get,
    path = "/v1/treasury/chart-of-accounts/{entity_id}",
    tag = "treasury",
    params(("entity_id" = EntityId, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Chart of accounts", body = ChartOfAccountsResponse),
    ),
)]
async fn get_chart_of_accounts(
    RequireTreasuryRead(auth): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<ChartOfAccountsResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());

    let accounts = tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            let ids = store
                .list_ids::<Account>("main")
                .map_err(|e| AppError::Internal(format!("list accounts: {e}")))?;

            let mut results = Vec::new();
            for id in ids {
                let a = store
                    .read::<Account>("main", id)
                    .map_err(|e| AppError::Internal(format!("read account {id}: {e}")))?;
                results.push(account_to_response(&a));
            }
            Ok::<_, AppError>(results)
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(ChartOfAccountsResponse {
        entity_id,
        accounts,
    }))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreatePayoutRequest {
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub destination: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PayoutResponse {
    pub payout_id: String,
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub destination: String,
    pub status: String,
    pub created_at: String,
}

#[utoipa::path(
    post,
    path = "/v1/treasury/payouts",
    tag = "treasury",
    request_body = CreatePayoutRequest,
    responses(
        (status = 200, description = "Payout created", body = PayoutResponse),
    ),
)]
async fn create_payout(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreatePayoutRequest>,
) -> Result<Json<PayoutResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.payout.create", workspace_id, 60, 60)?;

    if req.amount_cents <= 0 {
        return Err(AppError::BadRequest(
            "amount_cents must be positive".to_owned(),
        ));
    }
    validate_reasonable_amount(req.amount_cents, "amount_cents")?;

    let payout_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let payout_id = payout_id.clone();
        let destination = req.destination.clone();
        let description = req.description.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            store
                .write_json(
                    "main",
                    &format!("treasury/payouts/{}.json", payout_id),
                    &serde_json::json!({
                        "payout_id": payout_id,
                        "entity_id": entity_id,
                        "amount_cents": req.amount_cents,
                        "destination": destination,
                        "description": description,
                        "status": "pending",
                        "created_at": now.to_rfc3339(),
                    }),
                    &format!("Create payout {payout_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(PayoutResponse {
        payout_id,
        entity_id,
        amount_cents: req.amount_cents,
        destination: req.destination,
        status: "pending".to_owned(),
        created_at: now.to_rfc3339(),
    }))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreatePaymentIntentRequest {
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub currency: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PaymentIntentResponse {
    pub payment_intent_id: String,
    pub entity_id: EntityId,
    pub amount_cents: i64,
    pub currency: String,
    pub status: String,
    pub client_secret: String,
    pub created_at: String,
}

#[utoipa::path(
    post,
    path = "/v1/treasury/payment-intents",
    tag = "treasury",
    request_body = CreatePaymentIntentRequest,
    responses(
        (status = 200, description = "Payment intent created", body = PaymentIntentResponse),
    ),
)]
async fn create_payment_intent(
    RequireTreasuryWrite(auth): RequireTreasuryWrite,
    State(state): State<AppState>,
    Json(req): Json<CreatePaymentIntentRequest>,
) -> Result<Json<PaymentIntentResponse>, AppError> {
    let workspace_id = auth.workspace_id();
    let entity_scope = auth.entity_ids().map(|ids| ids.to_vec());
    let entity_id = req.entity_id;
    state.enforce_creation_rate_limit("treasury.payment_intent.create", workspace_id, 120, 60)?;

    if req.amount_cents <= 0 {
        return Err(AppError::BadRequest(
            "amount_cents must be positive".to_owned(),
        ));
    }
    validate_reasonable_amount(req.amount_cents, "amount_cents")?;

    let pi_id = format!("pi_{}", uuid::Uuid::new_v4().simple());
    let client_secret = format!("{}_secret_{}", pi_id, uuid::Uuid::new_v4().simple());
    let currency = req.currency.unwrap_or_else(|| "usd".to_owned());
    let now = chrono::Utc::now();

    tokio::task::spawn_blocking({
        let layout = state.layout.clone();
        let valkey_client = state.valkey_client.clone();
        let pi_id = pi_id.clone();
        let currency = currency.clone();
        let client_secret = client_secret.clone();
        let description = req.description.clone();
        move || {
            let store = super::shared::open_entity_store(&layout, workspace_id, entity_id, entity_scope.as_deref(), valkey_client.as_ref())?;
            ensure_entity_ready_for_treasury(&store, "payment intent creation")?;
            store
                .write_json(
                    "main",
                    &format!("treasury/payment-intents/{}.json", pi_id),
                    &serde_json::json!({
                        "payment_intent_id": pi_id,
                        "entity_id": entity_id,
                        "amount_cents": req.amount_cents,
                        "currency": currency,
                        "description": description,
                        "status": "requires_confirmation",
                        "client_secret": client_secret,
                        "created_at": now.to_rfc3339(),
                    }),
                    &format!("Create payment intent {pi_id}"),
                )
                .map_err(|e| AppError::Internal(format!("commit: {e}")))?;
            Ok::<_, AppError>(())
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("task join error: {e}")))??;

    Ok(Json(PaymentIntentResponse {
        payment_intent_id: pi_id,
        entity_id,
        amount_cents: req.amount_cents,
        currency,
        status: "requires_confirmation".to_owned(),
        client_secret,
        created_at: now.to_rfc3339(),
    }))
}

#[utoipa::path(
    post,
    path = "/v1/treasury/webhooks/stripe",
    tag = "treasury",
    request_body(content = String, content_type = "application/json"),
    responses(
        (status = 200, description = "Webhook received", body = serde_json::Value),
    ),
)]
async fn treasury_stripe_webhook(
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    let expected = std::env::var("TREASURY_STRIPE_WEBHOOK_SECRET").map_err(|_| {
        AppError::Internal("TREASURY_STRIPE_WEBHOOK_SECRET is not configured".to_owned())
    })?;
    let provided = headers
        .get("x-webhook-secret")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing webhook secret".to_owned()))?;
    if provided != expected {
        return Err(AppError::Unauthorized("invalid webhook secret".to_owned()));
    }

    let payload: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| AppError::BadRequest(format!("invalid webhook JSON: {e}")))?;
    tracing::info!("Received treasury Stripe webhook");
    Ok(Json(serde_json::json!({
        "received": true,
        "event_id": payload.get("id").and_then(|v| v.as_str()),
    })))
}

// ── Router ───────────────────────────────────────────────────────────

pub fn treasury_routes() -> Router<AppState> {
    Router::new()
        // Accounts
        .route("/v1/treasury/accounts", post(create_account))
        .route("/v1/entities/{entity_id}/accounts", get(list_accounts))
        // Journal entries
        .route("/v1/treasury/journal-entries", post(create_journal_entry))
        .route(
            "/v1/entities/{entity_id}/journal-entries",
            get(list_journal_entries),
        )
        .route(
            "/v1/journal-entries/{entry_id}/post",
            post(post_journal_entry),
        )
        .route(
            "/v1/journal-entries/{entry_id}/void",
            post(void_journal_entry),
        )
        // Invoices
        .route("/v1/treasury/invoices", post(create_invoice))
        .route("/v1/entities/{entity_id}/invoices", get(list_invoices))
        .route("/v1/invoices/{invoice_id}/send", post(send_invoice))
        .route(
            "/v1/invoices/{invoice_id}/mark-paid",
            post(mark_invoice_paid),
        )
        .route("/v1/invoices/{invoice_id}/status", get(get_invoice_status))
        .route(
            "/v1/invoices/{invoice_id}/pay-instructions",
            get(get_pay_instructions),
        )
        // Bank accounts
        .route("/v1/treasury/bank-accounts", post(create_bank_account))
        .route(
            "/v1/entities/{entity_id}/bank-accounts",
            get(list_bank_accounts),
        )
        .route(
            "/v1/bank-accounts/{bank_account_id}/activate",
            post(activate_bank_account),
        )
        .route(
            "/v1/bank-accounts/{bank_account_id}/close",
            post(close_bank_account),
        )
        // Payments
        .route("/v1/payments", post(submit_payment))
        .route("/v1/entities/{entity_id}/payments", get(list_payments))
        .route("/v1/payments/execute", post(execute_payment))
        // Payroll
        .route("/v1/payroll/runs", post(create_payroll_run))
        .route(
            "/v1/entities/{entity_id}/payroll-runs",
            get(list_payroll_runs),
        )
        // Distributions
        .route("/v1/distributions", post(create_distribution))
        .route(
            "/v1/entities/{entity_id}/distributions",
            get(list_distributions),
        )
        // Reconciliation
        .route("/v1/ledger/reconcile", post(reconcile_ledger))
        .route(
            "/v1/entities/{entity_id}/reconciliations",
            get(list_reconciliations),
        )
        // Treasury advanced
        .route(
            "/v1/entities/{entity_id}/stripe-account",
            get(get_stripe_account),
        )
        .route("/v1/spending-limits", post(create_spending_limit))
        .route(
            "/v1/entities/{entity_id}/spending-limits",
            get(list_spending_limits),
        )
        .route(
            "/v1/entities/{entity_id}/financial-statements",
            get(get_financial_statements),
        )
        .route(
            "/v1/treasury/seed-chart-of-accounts",
            post(seed_chart_of_accounts),
        )
        // Get single invoice
        .route("/v1/invoices/{invoice_id}", get(get_invoice))
        // Agent invoice creation
        .route("/v1/invoices/from-agent-request", post(from_agent_request))
        // Treasury advanced
        .route("/v1/treasury/stripe-accounts", post(create_stripe_account))
        .route(
            "/v1/treasury/chart-of-accounts/{entity_id}",
            get(get_chart_of_accounts),
        )
        .route("/v1/treasury/payouts", post(create_payout))
        .route("/v1/treasury/payment-intents", post(create_payment_intent))
        .route(
            "/v1/treasury/webhooks/stripe",
            post(treasury_stripe_webhook),
        )
        // Alias: bank-accounts
        .route("/v1/bank-accounts", post(create_bank_account))
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create_account,
        list_accounts,
        create_journal_entry,
        list_journal_entries,
        post_journal_entry,
        void_journal_entry,
        create_invoice,
        list_invoices,
        send_invoice,
        mark_invoice_paid,
        get_invoice_status,
        get_pay_instructions,
        create_bank_account,
        list_bank_accounts,
        activate_bank_account,
        close_bank_account,
        submit_payment,
        list_payments,
        execute_payment,
        create_payroll_run,
        list_payroll_runs,
        create_distribution,
        list_distributions,
        reconcile_ledger,
        list_reconciliations,
        get_stripe_account,
        create_spending_limit,
        list_spending_limits,
        get_financial_statements,
        seed_chart_of_accounts,
        get_invoice,
        from_agent_request,
        create_stripe_account,
        get_chart_of_accounts,
        create_payout,
        create_payment_intent,
        treasury_stripe_webhook,
    ),
    components(schemas(
        CreateAccountRequest,
        AccountResponse,
        LedgerLineRequest,
        CreateJournalEntryRequest,
        JournalEntryResponse,
        CreateInvoiceRequest,
        InvoiceResponse,
        CreateBankAccountRequest,
        BankAccountResponse,
        PayInstructionsResponse,
        SubmitPaymentRequest,
        PaymentResponse,
        CreatePayrollRunRequest,
        PayrollRunResponse,
        CreateDistributionRequest,
        DistributionResponse,
        ReconcileLedgerRequest,
        ReconciliationResponse,
        StripeAccountResponse,
        SpendingLimitResponse,
        CreateSpendingLimitRequest,
        FinancialStatementResponse,
        SeedChartOfAccountsRequest,
        SeedChartOfAccountsResponse,
        AgentInvoiceRequest,
        PaymentOfferResponse,
        CreateStripeAccountRequest,
        ChartOfAccountsResponse,
        CreatePayoutRequest,
        PayoutResponse,
        CreatePaymentIntentRequest,
        PaymentIntentResponse,
    )),
    tags((name = "treasury", description = "Treasury, banking, and financial operations")),
)]
pub struct TreasuryApi;
