//! `corp finance` — treasury, accounts, journal entries, invoices, payments,
//! bank accounts, payroll runs, and reconciliation.

use serde_json::json;

use super::Context;
use crate::output;

// ── FinanceCommand ────────────────────────────────────────────────────────────

/// Treasury and finance — invoices, payments, bank accounts, payroll, GL
#[derive(clap::Subcommand)]
#[command(
    long_about = "Treasury and finance: GL accounts, journal entries, invoices, payments, bank accounts, payroll, and reconciliation.\n\nAccount codes: cash, accounts_receivable, accounts_payable, accrued_expenses, founder_capital, revenue, operating_expenses, cogs\n\nAll monetary amounts are in cents (e.g. $50,000.00 = 5000000).\n\nInvoice lifecycle: draft → sent → paid\nBank account lifecycle: pending_review → active → closed\nPayroll lifecycle: draft → approved → processed\nJournal entry lifecycle: draft → posted (or voided)"
)]
pub enum FinanceCommand {
    // ── Accounts ──────────────────────────────────────────────────────────────
    /// List GL accounts
    Accounts,

    /// Create a GL account
    CreateAccount {
        #[arg(
            long,
            help = "GL account code: cash, accounts_receivable, accounts_payable, accrued_expenses, founder_capital, revenue, operating_expenses, cogs"
        )]
        account_code: String,

        #[arg(long, help = "Human-readable account name")]
        account_name: String,

        #[arg(
            long,
            default_value = "usd",
            help = "ISO currency code (currently only: usd)"
        )]
        currency: String,
    },

    // ── Journal entries ───────────────────────────────────────────────────────
    /// List journal entries
    JournalEntries,

    /// Create a new double-entry journal entry
    ///
    /// Requires balanced debit/credit lines. Example JSON for --lines:
    ///   '[{"account_id":"...","amount_cents":10000,"side":"debit"},{"account_id":"...","amount_cents":10000,"side":"credit"}]'
    #[command(alias = "create-entry")]
    CreateJournalEntry {
        #[arg(long, help = "Entry date (YYYY-MM-DD)")]
        date: String,

        #[arg(long, help = "Description of the journal entry")]
        description: String,

        #[arg(
            long,
            help = "JSON array of lines, e.g. '[{\"account_id\":\"...\",\"amount_cents\":5000,\"side\":\"debit\"}]'"
        )]
        lines: String,
    },

    /// Post a journal entry (validates debits equal credits, draft → posted)
    PostEntry {
        #[arg(help = "Journal entry ID (from `corp finance journal-entries`)")]
        entry_id: String,
    },

    /// Void a journal entry (posted → voided)
    #[command(about = "Void a journal entry (posted → voided)")]
    VoidEntry {
        #[arg(help = "Journal entry ID (from `corp finance journal-entries`)")]
        entry_id: String,
    },

    // ── Invoices ──────────────────────────────────────────────────────────────
    /// List invoices
    Invoices,

    /// Create an invoice
    ///
    /// Example:
    ///   corp finance create-invoice --customer "Acme Corp" --email billing@acme.com \
    ///     --amount-cents 150000 --description "Q1 consulting" --due-date 2026-04-30
    CreateInvoice {
        #[arg(long, help = "Customer or counterparty name")]
        customer: String,

        #[arg(long, help = "Customer email (needed for send-invoice)")]
        email: Option<String>,

        #[arg(long, help = "Amount in cents (e.g. $500.00 = 50000)")]
        amount_cents: i64,

        #[arg(
            long,
            default_value = "usd",
            help = "ISO currency code (currently only: usd)"
        )]
        currency: String,

        /// Invoice description (line item summary)
        #[arg(long)]
        description: String,

        #[arg(long, help = "Due date (YYYY-MM-DD)")]
        due_date: String,
    },

    /// Send an invoice (draft → sent)
    SendInvoice {
        #[arg(help = "Invoice ID (from `corp finance invoices`)")]
        invoice_id: String,
    },

    /// Mark an invoice as paid (sent → paid)
    #[command(about = "Mark an invoice as paid (sent → paid)")]
    PayInvoice {
        #[arg(help = "Invoice ID (from `corp finance invoices`)")]
        invoice_id: String,
    },

    // ── Payments ──────────────────────────────────────────────────────────────
    /// List payments
    Payments,

    /// Record a payment
    CreatePayment {
        #[arg(long, help = "Payment recipient name")]
        recipient_name: String,

        #[arg(long, help = "Amount in cents (e.g. $500.00 = 50000)")]
        amount_cents: i64,

        #[arg(
            long,
            default_value = "ach",
            help = "Payment method: ach, wire, check, card, or bank_transfer"
        )]
        method: String,

        #[arg(long, help = "External reference (e.g. ACH trace ID, check number)")]
        reference: Option<String>,
    },

    // ── Bank accounts ─────────────────────────────────────────────────────────
    /// List bank accounts
    BankAccounts,

    /// Open a new bank account
    OpenAccount {
        #[arg(long, help = "Bank name (e.g. Silicon Valley Bank, Mercury)")]
        institution: String,

        #[arg(
            long,
            default_value = "checking",
            help = "Bank account type: checking or savings"
        )]
        account_type: String,

        /// Last 4 digits of account number
        #[arg(long)]
        account_number_last4: Option<String>,

        /// Last 4 digits of routing number
        #[arg(long)]
        routing_number_last4: Option<String>,
    },

    /// Activate a bank account after review (pending_review → active)
    ActivateAccount {
        #[arg(help = "Bank account ID (from `corp finance bank-accounts`)")]
        bank_id: String,
    },

    /// Close a bank account (active → closed)
    #[command(about = "Close a bank account (active → closed)")]
    CloseAccount {
        #[arg(help = "Bank account ID (from `corp finance bank-accounts`)")]
        bank_id: String,
    },

    // ── Payroll ───────────────────────────────────────────────────────────────
    /// List payroll runs
    Payroll,

    /// Create a payroll run (draft)
    CreatePayrollRun {
        #[arg(long, help = "Period date (YYYY-MM-DD)")]
        period_start: String,

        #[arg(long, help = "Period date (YYYY-MM-DD)")]
        period_end: String,

        #[arg(long, help = "Amount in cents")]
        total_gross_cents: i64,

        #[arg(long, help = "Amount in cents")]
        total_net_cents: i64,

        #[arg(long, help = "Number of employees in this payroll run")]
        employee_count: u32,
    },

    /// Approve a payroll run (draft → approved)
    ApprovePayroll {
        #[arg(help = "Payroll run ID (from `corp finance payroll`)")]
        run_id: String,
    },

    /// Process an approved payroll run (approved → processed)
    ProcessPayroll {
        #[arg(help = "Payroll run ID (from `corp finance payroll`)")]
        run_id: String,
    },

    // ── Reconciliation ────────────────────────────────────────────────────────
    /// List reconciliations
    Reconciliations,

    /// Create a reconciliation
    Reconcile {
        #[arg(long, help = "GL account ID (from `corp finance accounts`)")]
        account_id: String,

        #[arg(long, help = "Period date (YYYY-MM-DD)")]
        period_end: String,

        #[arg(long, help = "Balance in cents")]
        statement_balance_cents: i64,

        #[arg(long, help = "Balance in cents")]
        book_balance_cents: i64,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: FinanceCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    let entity_id = ctx.require_entity()?;

    match cmd {
        FinanceCommand::Accounts => {
            let path = format!("/v1/entities/{entity_id}/accounts");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        FinanceCommand::CreateAccount {
            account_code,
            account_name,
            currency,
        } => {
            let path = format!("/v1/entities/{entity_id}/accounts");
            let body = json!({
                "account_code": account_code,
                "account_name": account_name,
                "currency": currency,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Account created.", mode);
        }

        FinanceCommand::JournalEntries => {
            let path = format!("/v1/entities/{entity_id}/journal-entries");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        FinanceCommand::CreateJournalEntry {
            description,
            date,
            lines,
        } => {
            let path = format!("/v1/entities/{entity_id}/journal-entries");
            let lines_parsed: serde_json::Value = serde_json::from_str(&lines)
                .map_err(|e| anyhow::anyhow!("Invalid --lines JSON: {e}"))?;
            let body = json!({
                "description": description,
                "date": date,
                "lines": lines_parsed,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Journal entry created.", mode);
        }

        FinanceCommand::PostEntry { entry_id } => {
            let path = format!("/v1/entities/{entity_id}/journal-entries/{entry_id}/post");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Journal entry posted.", mode);
        }

        FinanceCommand::VoidEntry { entry_id } => {
            let path = format!("/v1/entities/{entity_id}/journal-entries/{entry_id}/void");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Journal entry voided.", mode);
        }

        FinanceCommand::Invoices => {
            let path = format!("/v1/entities/{entity_id}/invoices");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        FinanceCommand::CreateInvoice {
            customer,
            email,
            amount_cents,
            currency,
            description,
            due_date,
        } => {
            let path = format!("/v1/entities/{entity_id}/invoices");
            let body = json!({
                "customer_name": customer,
                "customer_email": email,
                "amount_cents": amount_cents,
                "currency": currency,
                "description": description,
                "due_date": due_date,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Invoice created.", mode);
        }

        FinanceCommand::SendInvoice { invoice_id } => {
            let path = format!("/v1/entities/{entity_id}/invoices/{invoice_id}/send");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Invoice sent.", mode);
        }

        FinanceCommand::PayInvoice { invoice_id } => {
            let path = format!("/v1/entities/{entity_id}/invoices/{invoice_id}/pay");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Invoice marked paid.", mode);
        }

        FinanceCommand::Payments => {
            let path = format!("/v1/entities/{entity_id}/payments");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        FinanceCommand::CreatePayment {
            recipient_name,
            amount_cents,
            method,
            reference,
        } => {
            let path = format!("/v1/entities/{entity_id}/payments");
            let body = json!({
                "recipient_name": recipient_name,
                "amount_cents": amount_cents,
                "method": method,
                "reference": reference,
                "paid_at": chrono::Utc::now().to_rfc3339(),
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Payment recorded.", mode);
        }

        FinanceCommand::BankAccounts => {
            let path = format!("/v1/entities/{entity_id}/bank-accounts");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        FinanceCommand::OpenAccount {
            institution,
            account_type,
            account_number_last4,
            routing_number_last4,
        } => {
            let path = format!("/v1/entities/{entity_id}/bank-accounts");
            let body = json!({
                "institution": institution,
                "account_type": account_type,
                "account_number_last4": account_number_last4,
                "routing_number_last4": routing_number_last4,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Bank account opened.", mode);
        }

        FinanceCommand::ActivateAccount { bank_id } => {
            let path = format!("/v1/entities/{entity_id}/bank-accounts/{bank_id}/activate");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Bank account activated.", mode);
        }

        FinanceCommand::CloseAccount { bank_id } => {
            let path = format!("/v1/entities/{entity_id}/bank-accounts/{bank_id}/close");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Bank account closed.", mode);
        }

        FinanceCommand::Payroll => {
            let path = format!("/v1/entities/{entity_id}/payroll-runs");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        FinanceCommand::CreatePayrollRun {
            period_start,
            period_end,
            total_gross_cents,
            total_net_cents,
            employee_count,
        } => {
            let path = format!("/v1/entities/{entity_id}/payroll-runs");
            let body = json!({
                "period_start": period_start,
                "period_end": period_end,
                "total_gross_cents": total_gross_cents,
                "total_net_cents": total_net_cents,
                "employee_count": employee_count,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Payroll run created.", mode);
        }

        FinanceCommand::ApprovePayroll { run_id } => {
            let path = format!("/v1/entities/{entity_id}/payroll-runs/{run_id}/approve");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Payroll run approved.", mode);
        }

        FinanceCommand::ProcessPayroll { run_id } => {
            let path = format!("/v1/entities/{entity_id}/payroll-runs/{run_id}/process");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Payroll run processed.", mode);
        }

        FinanceCommand::Reconciliations => {
            let path = format!("/v1/entities/{entity_id}/reconciliations");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        FinanceCommand::Reconcile {
            account_id,
            period_end,
            statement_balance_cents,
            book_balance_cents,
        } => {
            let path = format!("/v1/entities/{entity_id}/reconciliations");
            let body = json!({
                "account_id": account_id,
                "period_end": period_end,
                "statement_balance_cents": statement_balance_cents,
                "book_balance_cents": book_balance_cents,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Reconciliation recorded.", mode);
        }
    }

    Ok(())
}
