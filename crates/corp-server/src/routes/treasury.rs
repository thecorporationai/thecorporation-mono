//! Treasury domain routes.
//!
//! Covers general-ledger accounts, journal entries, invoices, payments,
//! bank accounts, payroll runs, and reconciliations.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

use corp_auth::{RequireTreasuryRead, RequireTreasuryWrite};
use corp_core::ids::{
    AccountId, BankAccountId, EntityId, InvoiceId, JournalEntryId, PayrollRunId, ReconciliationId,
};
use corp_core::treasury::types::{BankAccountType, Currency, GlAccountCode, PaymentMethod};
use corp_core::treasury::{
    Account, BankAccount, Invoice, JournalEntry, JournalLine, Payment, PayrollRun, Reconciliation,
};

use crate::error::AppError;
use crate::state::AppState;

// ── Request body types ────────────────────────────────────────────────────────

/// Request body for `POST /entities/{entity_id}/accounts`.
#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub account_code: GlAccountCode,
    pub account_name: String,
    pub currency: Currency,
}

/// Request body for `POST /entities/{entity_id}/journal-entries`.
#[derive(Debug, Deserialize)]
pub struct CreateJournalEntryRequest {
    pub date: NaiveDate,
    pub description: String,
    /// All debit and credit lines; must balance (debits == credits) before posting.
    pub lines: Vec<JournalLine>,
}

/// Request body for `POST /entities/{entity_id}/invoices`.
#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub customer_name: String,
    pub customer_email: Option<String>,
    /// Invoice total in whole cents.
    pub amount_cents: i64,
    pub currency: Currency,
    pub description: String,
    pub due_date: NaiveDate,
}

/// Request body for `POST /entities/{entity_id}/payments`.
#[derive(Debug, Deserialize)]
pub struct CreatePaymentRequest {
    pub recipient_name: String,
    /// Payment amount in whole cents.
    pub amount_cents: i64,
    pub method: PaymentMethod,
    /// External reference, e.g. ACH trace ID or check number.
    pub reference: Option<String>,
    pub paid_at: DateTime<Utc>,
}

/// Request body for `POST /entities/{entity_id}/bank-accounts`.
#[derive(Debug, Deserialize)]
pub struct CreateBankAccountRequest {
    pub institution: String,
    pub account_type: BankAccountType,
    pub account_number_last4: Option<String>,
    pub routing_number_last4: Option<String>,
}

/// Request body for `POST /entities/{entity_id}/payroll-runs`.
#[derive(Debug, Deserialize)]
pub struct CreatePayrollRunRequest {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_gross_cents: i64,
    pub total_net_cents: i64,
    pub employee_count: u32,
}

/// Request body for `POST /entities/{entity_id}/reconciliations`.
#[derive(Debug, Deserialize)]
pub struct CreateReconciliationRequest {
    pub account_id: AccountId,
    pub period_end: NaiveDate,
    pub statement_balance_cents: i64,
    pub book_balance_cents: i64,
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        // Accounts
        .route(
            "/entities/{entity_id}/accounts",
            get(list_accounts).post(create_account),
        )
        .route(
            "/entities/{entity_id}/accounts/{account_id}/deactivate",
            post(deactivate_account),
        )
        // Journal entries
        .route(
            "/entities/{entity_id}/journal-entries",
            get(list_journal_entries).post(create_journal_entry),
        )
        .route(
            "/entities/{entity_id}/journal-entries/{entry_id}/post",
            post(post_journal_entry),
        )
        .route(
            "/entities/{entity_id}/journal-entries/{entry_id}/void",
            post(void_journal_entry),
        )
        // Invoices
        .route(
            "/entities/{entity_id}/invoices",
            get(list_invoices).post(create_invoice),
        )
        .route(
            "/entities/{entity_id}/invoices/{invoice_id}/send",
            post(send_invoice),
        )
        .route(
            "/entities/{entity_id}/invoices/{invoice_id}/pay",
            post(pay_invoice),
        )
        .route(
            "/entities/{entity_id}/invoices/{invoice_id}/void",
            post(void_invoice),
        )
        // Payments
        .route(
            "/entities/{entity_id}/payments",
            get(list_payments).post(create_payment),
        )
        // Bank accounts
        .route(
            "/entities/{entity_id}/bank-accounts",
            get(list_bank_accounts).post(create_bank_account),
        )
        .route(
            "/entities/{entity_id}/bank-accounts/{bank_id}/activate",
            post(activate_bank_account),
        )
        .route(
            "/entities/{entity_id}/bank-accounts/{bank_id}/close",
            post(close_bank_account),
        )
        // Payroll
        .route(
            "/entities/{entity_id}/payroll-runs",
            get(list_payroll_runs).post(create_payroll_run),
        )
        .route(
            "/entities/{entity_id}/payroll-runs/{run_id}/approve",
            post(approve_payroll_run),
        )
        .route(
            "/entities/{entity_id}/payroll-runs/{run_id}/process",
            post(process_payroll_run),
        )
        // Reconciliation
        .route(
            "/entities/{entity_id}/reconciliations",
            get(list_reconciliations).post(create_reconciliation),
        )
        .route(
            "/entities/{entity_id}/reconciliations/{reconciliation_id}/reconcile",
            post(reconcile),
        )
}

// ── Account handlers ──────────────────────────────────────────────────────────

async fn list_accounts(
    RequireTreasuryRead(principal): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Account>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let accounts = store.read_all::<Account>("main").await?;
    Ok(Json(accounts))
}

async fn create_account(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateAccountRequest>,
) -> Result<Json<Account>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let account = Account::new(
        entity_id,
        body.account_code,
        body.account_name,
        body.currency,
    );
    store
        .write::<Account>(&account, account.account_id, "main", "create account")
        .await?;
    Ok(Json(account))
}

/// `POST /entities/{entity_id}/accounts/{account_id}/deactivate`
///
/// Marks a GL account as inactive. Inactive accounts cannot receive new journal
/// lines. This operation is idempotent.
async fn deactivate_account(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, account_id)): Path<(EntityId, AccountId)>,
) -> Result<Json<Account>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut account = store.read::<Account>(account_id, "main").await?;
    account.deactivate();
    store
        .write::<Account>(&account, account_id, "main", "deactivate account")
        .await?;
    Ok(Json(account))
}

// ── Journal entry handlers ────────────────────────────────────────────────────

async fn list_journal_entries(
    RequireTreasuryRead(principal): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<JournalEntry>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let entries = store.read_all::<JournalEntry>("main").await?;
    Ok(Json(entries))
}

async fn create_journal_entry(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateJournalEntryRequest>,
) -> Result<Json<JournalEntry>, AppError> {
    for line in &body.lines {
        if line.amount_cents <= 0 {
            return Err(AppError::BadRequest(
                "journal entry line amount_cents must be greater than zero".into(),
            ));
        }
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let entry = JournalEntry::new(entity_id, body.date, body.description, body.lines);
    store
        .write::<JournalEntry>(&entry, entry.entry_id, "main", "create journal entry")
        .await?;
    Ok(Json(entry))
}

/// `POST /entities/{entity_id}/journal-entries/{entry_id}/post`
///
/// Validates that the entry balances (debits == credits) and marks it posted.
/// Returns `400 Bad Request` if the entry is already posted, voided, or unbalanced.
async fn post_journal_entry(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, entry_id)): Path<(EntityId, JournalEntryId)>,
) -> Result<Json<JournalEntry>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut entry = store.read::<JournalEntry>(entry_id, "main").await?;
    entry
        .post()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<JournalEntry>(&entry, entry_id, "main", "post journal entry")
        .await?;
    Ok(Json(entry))
}

/// `POST /entities/{entity_id}/journal-entries/{entry_id}/void`
///
/// Voids a posted or unposted entry. Returns `400 Bad Request` if already voided.
async fn void_journal_entry(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, entry_id)): Path<(EntityId, JournalEntryId)>,
) -> Result<Json<JournalEntry>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut entry = store.read::<JournalEntry>(entry_id, "main").await?;
    entry
        .void()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<JournalEntry>(&entry, entry_id, "main", "void journal entry")
        .await?;
    Ok(Json(entry))
}

// ── Invoice handlers ──────────────────────────────────────────────────────────

async fn list_invoices(
    RequireTreasuryRead(principal): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Invoice>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let invoices = store.read_all::<Invoice>("main").await?;
    Ok(Json(invoices))
}

async fn create_invoice(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateInvoiceRequest>,
) -> Result<Json<Invoice>, AppError> {
    if body.amount_cents <= 0 {
        return Err(AppError::BadRequest(
            "invoice amount_cents must be greater than zero".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let invoice = Invoice::new(
        entity_id,
        body.customer_name,
        body.customer_email,
        body.amount_cents,
        body.currency,
        body.description,
        body.due_date,
    );
    store
        .write::<Invoice>(&invoice, invoice.invoice_id, "main", "create invoice")
        .await?;
    Ok(Json(invoice))
}

/// `POST /entities/{entity_id}/invoices/{invoice_id}/send`
///
/// Transitions `Draft → Sent`. Returns `400` if the invoice is not in `Draft`.
async fn send_invoice(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, invoice_id)): Path<(EntityId, InvoiceId)>,
) -> Result<Json<Invoice>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut invoice = store.read::<Invoice>(invoice_id, "main").await?;
    invoice
        .send()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Invoice>(&invoice, invoice_id, "main", "send invoice")
        .await?;
    Ok(Json(invoice))
}

/// `POST /entities/{entity_id}/invoices/{invoice_id}/pay`
///
/// Transitions `Sent → Paid`. Returns `400` if the invoice is not in `Sent`.
async fn pay_invoice(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, invoice_id)): Path<(EntityId, InvoiceId)>,
) -> Result<Json<Invoice>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut invoice = store.read::<Invoice>(invoice_id, "main").await?;
    invoice
        .mark_paid()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Invoice>(&invoice, invoice_id, "main", "pay invoice")
        .await?;
    Ok(Json(invoice))
}

/// `POST /entities/{entity_id}/invoices/{invoice_id}/void`
///
/// Voids an invoice. Allowed from `Draft` or `Sent`. Returns `400` if the
/// invoice is in a state that cannot be voided (e.g. already `Paid` or `Voided`).
async fn void_invoice(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, invoice_id)): Path<(EntityId, InvoiceId)>,
) -> Result<Json<Invoice>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut invoice = store.read::<Invoice>(invoice_id, "main").await?;
    invoice
        .void()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Invoice>(&invoice, invoice_id, "main", "void invoice")
        .await?;
    Ok(Json(invoice))
}

// ── Payment handlers ──────────────────────────────────────────────────────────

async fn list_payments(
    RequireTreasuryRead(principal): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Payment>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let payments = store.read_all::<Payment>("main").await?;
    Ok(Json(payments))
}

async fn create_payment(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreatePaymentRequest>,
) -> Result<Json<Payment>, AppError> {
    if body.amount_cents <= 0 {
        return Err(AppError::BadRequest(
            "payment amount_cents must be greater than zero".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let payment = Payment::new(
        entity_id,
        body.recipient_name,
        body.amount_cents,
        body.method,
        body.reference,
        body.paid_at,
    );
    store
        .write::<Payment>(&payment, payment.payment_id, "main", "create payment")
        .await?;
    Ok(Json(payment))
}

// ── Bank account handlers ─────────────────────────────────────────────────────

async fn list_bank_accounts(
    RequireTreasuryRead(principal): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<BankAccount>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let accounts = store.read_all::<BankAccount>("main").await?;
    Ok(Json(accounts))
}

async fn create_bank_account(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateBankAccountRequest>,
) -> Result<Json<BankAccount>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let bank_account = BankAccount::new(
        entity_id,
        body.institution,
        body.account_type,
        body.account_number_last4,
        body.routing_number_last4,
    );
    store
        .write::<BankAccount>(
            &bank_account,
            bank_account.bank_account_id,
            "main",
            "create bank account",
        )
        .await?;
    Ok(Json(bank_account))
}

/// `POST /entities/{entity_id}/bank-accounts/{bank_id}/activate`
///
/// Transitions `PendingReview → Active`. Returns `400` if not in `PendingReview`.
async fn activate_bank_account(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, bank_id)): Path<(EntityId, BankAccountId)>,
) -> Result<Json<BankAccount>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut bank_account = store.read::<BankAccount>(bank_id, "main").await?;
    bank_account
        .activate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<BankAccount>(&bank_account, bank_id, "main", "activate bank account")
        .await?;
    Ok(Json(bank_account))
}

/// `POST /entities/{entity_id}/bank-accounts/{bank_id}/close`
///
/// Transitions `Active → Closed`. Returns `400` if the account is not `Active`.
async fn close_bank_account(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, bank_id)): Path<(EntityId, BankAccountId)>,
) -> Result<Json<BankAccount>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut bank_account = store.read::<BankAccount>(bank_id, "main").await?;
    bank_account
        .close()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<BankAccount>(&bank_account, bank_id, "main", "close bank account")
        .await?;
    Ok(Json(bank_account))
}

// ── Payroll run handlers ──────────────────────────────────────────────────────

async fn list_payroll_runs(
    RequireTreasuryRead(principal): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<PayrollRun>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let runs = store.read_all::<PayrollRun>("main").await?;
    Ok(Json(runs))
}

async fn create_payroll_run(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreatePayrollRunRequest>,
) -> Result<Json<PayrollRun>, AppError> {
    if body.total_gross_cents < body.total_net_cents {
        return Err(AppError::BadRequest(
            "total_gross_cents must be >= total_net_cents".into(),
        ));
    }
    if body.employee_count == 0 {
        return Err(AppError::BadRequest(
            "employee_count must be at least 1".into(),
        ));
    }
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let run = PayrollRun::new(
        entity_id,
        body.period_start,
        body.period_end,
        body.total_gross_cents,
        body.total_net_cents,
        body.employee_count,
    );
    store
        .write::<PayrollRun>(&run, run.payroll_run_id, "main", "create payroll run")
        .await?;
    Ok(Json(run))
}

/// `POST /entities/{entity_id}/payroll-runs/{run_id}/approve`
///
/// Transitions `Draft → Approved`. Returns `400` if not in `Draft`.
async fn approve_payroll_run(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, run_id)): Path<(EntityId, PayrollRunId)>,
) -> Result<Json<PayrollRun>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut run = store.read::<PayrollRun>(run_id, "main").await?;
    run.approve()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<PayrollRun>(&run, run_id, "main", "approve payroll run")
        .await?;
    Ok(Json(run))
}

/// `POST /entities/{entity_id}/payroll-runs/{run_id}/process`
///
/// Transitions `Approved → Processed`. Returns `400` if not in `Approved`.
async fn process_payroll_run(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, run_id)): Path<(EntityId, PayrollRunId)>,
) -> Result<Json<PayrollRun>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut run = store.read::<PayrollRun>(run_id, "main").await?;
    run.process()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<PayrollRun>(&run, run_id, "main", "process payroll run")
        .await?;
    Ok(Json(run))
}

// ── Reconciliation handlers ───────────────────────────────────────────────────

async fn list_reconciliations(
    RequireTreasuryRead(principal): RequireTreasuryRead,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
) -> Result<Json<Vec<Reconciliation>>, AppError> {
    let store = state
        .open_entity_store(principal.workspace_id, entity_id)
        .await?;
    let recs = store.read_all::<Reconciliation>("main").await?;
    Ok(Json(recs))
}

async fn create_reconciliation(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path(entity_id): Path<EntityId>,
    Json(body): Json<CreateReconciliationRequest>,
) -> Result<Json<Reconciliation>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let rec = Reconciliation::new(
        entity_id,
        body.account_id,
        body.period_end,
        body.statement_balance_cents,
        body.book_balance_cents,
    );
    store
        .write::<Reconciliation>(&rec, rec.reconciliation_id, "main", "create reconciliation")
        .await?;
    Ok(Json(rec))
}

/// `POST /entities/{entity_id}/reconciliations/{reconciliation_id}/reconcile`
///
/// Marks a reconciliation as complete. Returns `400` if it has already been
/// reconciled.
async fn reconcile(
    RequireTreasuryWrite(principal): RequireTreasuryWrite,
    State(state): State<AppState>,
    Path((entity_id, reconciliation_id)): Path<(EntityId, ReconciliationId)>,
) -> Result<Json<Reconciliation>, AppError> {
    let store = state
        .open_entity_store_for_write(principal.workspace_id, entity_id)
        .await?;
    let mut rec = store
        .read::<Reconciliation>(reconciliation_id, "main")
        .await?;
    rec.mark_reconciled()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    store
        .write::<Reconciliation>(&rec, reconciliation_id, "main", "mark reconciled")
        .await?;
    Ok(Json(rec))
}
