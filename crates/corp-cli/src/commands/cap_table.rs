//! `corp cap-table` — cap table management.
//!
//! API paths all live under `/v1/entities/{entity_id}/` using the flat
//! resource names that the server exposes:
//!   cap-table, instruments, grants, safes, valuations, transfers, holders, rounds

use serde_json::json;

use super::Context;
use crate::output;

// ── CapTableCommand ───────────────────────────────────────────────────────────

/// Manage equity: instruments, grants, SAFEs, valuations, holders, transfers, and funding rounds.
///
/// Quick start:
///   corp cap-table init
///   corp cap-table create-instrument ...
///   corp cap-table create-holder ...
///   corp cap-table issue ...
#[derive(clap::Subcommand)]
pub enum CapTableCommand {
    /// Show the cap table aggregate for the active (or specified) entity
    Show {
        /// Entity ID (defaults to active entity)
        entity_ref: Option<String>,
    },

    /// Initialize a new cap table for the entity
    Init,

    /// List equity grants
    Grants,

    /// Show a single equity grant
    ShowGrant {
        /// Grant ID (from `corp cap-table grants`)
        grant_id: String,
    },

    /// Issue shares (create an equity grant)
    ///
    /// Examples:
    ///   corp cap-table issue --cap-table-id <ID> --instrument-id <ID> \
    ///     --recipient-contact-id <ID> --recipient-name "Jane Doe" \
    ///     --grant-type common_stock --shares 1000000
    ///   corp cap-table issue --cap-table-id <ID> --instrument-id <ID> \
    ///     --recipient-contact-id <ID> --recipient-name "Jane Doe" \
    ///     --grant-type iso --shares 500000 --price-per-share-cents 1
    Issue {
        /// Cap table ID (from `corp cap-table show`)
        #[arg(long)]
        cap_table_id: String,

        /// Instrument ID (from `corp cap-table instruments`)
        #[arg(long)]
        instrument_id: String,

        /// Recipient contact ID (from `corp contacts list`)
        #[arg(long)]
        recipient_contact_id: String,

        /// Recipient display name
        #[arg(long)]
        recipient_name: String,

        /// Grant type [possible values: common_stock, preferred_stock, membership_unit, stock_option, iso, nso, rsa]
        #[arg(long, default_value = "common_stock")]
        grant_type: String,

        /// Number of shares to issue
        #[arg(long)]
        shares: i64,

        /// Price per share in cents (e.g. $0.001 = 0; optional, omit for grants at no cost)
        #[arg(long)]
        price_per_share_cents: Option<i64>,
    },

    /// List SAFE notes
    Safes,

    /// Issue a SAFE note
    ///
    /// Examples:
    ///   corp cap-table issue-safe --cap-table-id <ID> \
    ///     --investor-contact-id <ID> --investor-name "Acme Ventures" \
    ///     --safe-type post_money --investment-amount-cents 50000000 \
    ///     --valuation-cap-cents 1000000000
    IssueSafe {
        /// Cap table ID (from `corp cap-table show`)
        #[arg(long)]
        cap_table_id: String,

        /// Investor contact ID (from `corp contacts list`)
        #[arg(long)]
        investor_contact_id: String,

        /// Investor display name
        #[arg(long)]
        investor_name: String,

        /// SAFE type [possible values: post_money, pre_money, mfn]
        #[arg(long, default_value = "post_money")]
        safe_type: String,

        /// Investment amount in cents
        #[arg(long)]
        investment_amount_cents: i64,

        /// Valuation cap in cents (optional)
        #[arg(long)]
        valuation_cap_cents: Option<i64>,

        /// Discount percentage, e.g. 20 for 20% (optional)
        #[arg(long)]
        discount_percent: Option<u32>,
    },

    /// Convert a SAFE note to equity
    ConvertSafe {
        /// SAFE note ID (from `corp cap-table safes`)
        safe_id: String,
    },

    /// List valuations
    Valuations,

    /// Create a valuation
    ///
    /// Examples:
    ///   corp cap-table create-valuation --cap-table-id <ID> \
    ///     --valuation-type four_oh_nine_a --methodology market \
    ///     --valuation-amount-cents 200000000 --effective-date 2026-01-01 \
    ///     --prepared-by "Valuation Firm LLC"
    CreateValuation {
        /// Cap table ID (from `corp cap-table show`)
        #[arg(long)]
        cap_table_id: String,

        /// Valuation type [possible values: four_oh_nine_a, fair_market_value, other]
        #[arg(long, default_value = "four_oh_nine_a")]
        valuation_type: String,

        /// Valuation methodology [possible values: income, market, asset, backsolve, hybrid, other]
        #[arg(long, default_value = "market")]
        methodology: String,

        /// Valuation amount in cents
        #[arg(long)]
        valuation_amount_cents: i64,

        /// Effective date (YYYY-MM-DD)
        #[arg(long)]
        effective_date: String,

        /// Name or firm that prepared the valuation (optional)
        #[arg(long)]
        prepared_by: Option<String>,
    },

    /// Submit a valuation for review (draft → submitted)
    SubmitValuation {
        /// Valuation ID (from `corp cap-table valuations`)
        valuation_id: String,
    },

    /// Approve a submitted valuation (submitted → approved)
    ApproveValuation {
        /// Valuation ID (from `corp cap-table valuations`)
        valuation_id: String,

        /// Name of the approver
        #[arg(long)]
        approved_by: String,
    },

    /// List share transfers
    Transfers,

    /// Create a share transfer
    CreateTransfer {
        /// Cap table ID (from `corp cap-table show`)
        #[arg(long)]
        cap_table_id: String,

        /// From holder ID (from `corp cap-table holders`)
        #[arg(long)]
        from_holder_id: String,

        /// To holder ID (from `corp cap-table holders`)
        #[arg(long)]
        to_holder_id: String,

        /// Instrument ID (from `corp cap-table instruments`)
        #[arg(long)]
        instrument_id: String,

        /// Number of shares to transfer
        #[arg(long)]
        shares: i64,

        /// Transfer type [possible values: gift, trust_transfer, secondary_sale, estate, other]
        #[arg(long, default_value = "secondary_sale")]
        transfer_type: String,

        /// Price per share in cents (e.g. $0.001 = 0; optional)
        #[arg(long)]
        price_per_share_cents: Option<i64>,
    },

    /// Approve a share transfer (pending → approved)
    ApproveTransfer {
        /// Transfer ID (from `corp cap-table transfers`)
        transfer_id: String,
    },

    /// Execute an approved share transfer (approved → executed)
    ExecuteTransfer {
        /// Transfer ID (from `corp cap-table transfers`)
        transfer_id: String,
    },

    /// List funding rounds
    Rounds,

    /// Create a funding round
    CreateRound {
        /// Cap table ID (from `corp cap-table show`)
        #[arg(long)]
        cap_table_id: String,

        /// Round name (e.g. Seed, Series A)
        #[arg(long)]
        name: String,

        /// Target raise amount in cents
        #[arg(long)]
        target_amount_cents: i64,

        /// Price per share in cents (e.g. $0.001 = 0; optional)
        #[arg(long)]
        price_per_share_cents: Option<i64>,
    },

    /// Advance a funding round to the next stage
    AdvanceRound {
        /// Round ID (from `corp cap-table rounds`)
        round_id: String,
    },

    /// Close a funding round
    CloseRound {
        /// Round ID (from `corp cap-table rounds`)
        round_id: String,
    },

    /// List all holders
    Holders,

    /// Create a holder
    CreateHolder {
        /// Holder display name
        #[arg(long)]
        name: String,

        /// Holder type [possible values: individual, entity, trust]
        #[arg(long, default_value = "individual")]
        holder_type: String,

        /// Contact ID to link (from `corp contacts list`; optional)
        #[arg(long)]
        contact_id: Option<String>,
    },

    // ── Vesting ───────────────────────────────────────────────────────────────
    /// List vesting schedules for an entity
    Vesting { entity_id: String },

    /// Create a vesting schedule for a grant
    CreateVesting {
        #[arg(long, help = "Grant ID (from `corp cap-table grants`)")]
        grant_id: String,

        #[arg(long, help = "Total shares to vest")]
        total_shares: i64,

        #[arg(long, help = "Vesting start date (YYYY-MM-DD)")]
        start_date: String,

        #[arg(long, default_value = "4yr/1yr cliff", help = "Schedule template name")]
        template: String,

        #[arg(long, default_value = "12", help = "Cliff period in months")]
        cliff_months: u32,

        #[arg(long, default_value = "48", help = "Total vesting period in months")]
        total_months: u32,
    },

    /// Materialize vesting events for a schedule
    MaterializeVesting {
        #[arg(long, help = "Vesting schedule ID")]
        schedule_id: String,
    },

    /// Vest a scheduled vesting event
    VestEvent {
        /// Vesting event ID
        event_id: String,
    },

    // ── Instruments ───────────────────────────────────────────────────────────
    /// List instruments
    Instruments,

    /// Create an instrument
    CreateInstrument {
        /// Cap table ID (from `corp cap-table show`)
        #[arg(long)]
        cap_table_id: String,

        /// Short symbol, e.g. CS-A or PREF-SEED
        #[arg(long)]
        symbol: String,

        /// Instrument kind [possible values: common_equity, preferred_equity, membership_unit, option_grant, safe, convertible_note, warrant]
        #[arg(long, value_parser = ["common_equity", "preferred_equity", "membership_unit", "option_grant", "safe", "convertible_note", "warrant"])]
        kind: String,

        /// Authorized unit count (optional)
        #[arg(long)]
        authorized_units: Option<i64>,

        /// Par value per unit (decimal string, e.g. "0.00001"; optional)
        #[arg(long)]
        par_value: Option<String>,

        /// Issue price in cents (optional)
        #[arg(long)]
        issue_price_cents: Option<i64>,

        /// Liquidation preference description (preferred only; optional)
        #[arg(long)]
        liquidation_preference: Option<String>,
    },

    // ── Positions ─────────────────────────────────────────────────────────────
    /// List positions (holdings)
    Positions { entity_id: String },

    /// Create a position (holding)
    CreatePosition {
        /// Holder ID (from `corp cap-table holders`)
        #[arg(long)]
        holder_id: String,

        /// Instrument ID (from `corp cap-table instruments`)
        #[arg(long)]
        instrument_id: String,

        /// Quantity of units held
        #[arg(long)]
        quantity_units: i64,

        /// Principal amount in cents (optional)
        #[arg(long)]
        principal_cents: Option<i64>,

        /// Source reference (e.g. grant ID, safe ID)
        #[arg(long)]
        source_reference: Option<String>,
    },

    /// Apply a delta to an existing position
    ApplyDelta {
        /// Position ID (from `corp cap-table positions`)
        #[arg(long)]
        position_id: String,

        /// Quantity change (positive to increase, negative to decrease)
        #[arg(long)]
        quantity_delta: i64,

        /// Principal change in cents (optional)
        #[arg(long)]
        principal_delta: Option<i64>,

        /// Source reference for this adjustment
        #[arg(long)]
        source_reference: Option<String>,
    },

    // ── Investor Ledger ───────────────────────────────────────────────────────
    /// List investor ledger entries
    InvestorLedger,

    /// Create an investor ledger entry
    CreateLedgerEntry {
        /// Investor holder ID (from `corp cap-table holders`)
        #[arg(long)]
        investor_id: String,

        /// Investor display name
        #[arg(long)]
        investor_name: String,

        /// Entry type [possible values: investment, conversion, distribution, repurchase, other]
        #[arg(long)]
        entry_type: String,

        /// Amount in cents
        #[arg(long)]
        amount_cents: i64,

        /// Number of shares received (optional)
        #[arg(long)]
        shares_received: Option<i64>,

        /// Whether investor is pro-rata eligible
        #[arg(long, default_value = "false")]
        pro_rata_eligible: bool,

        /// Memo note (optional)
        #[arg(long)]
        memo: Option<String>,

        /// Effective date (YYYY-MM-DD)
        #[arg(long)]
        effective_date: String,

        /// SAFE note ID to link (optional)
        #[arg(long)]
        safe_note_id: Option<String>,

        /// Funding round ID to link (optional)
        #[arg(long)]
        funding_round_id: Option<String>,
    },

    // ── Legal Entities ────────────────────────────────────────────────────────
    /// List legal entities (corporate structure)
    LegalEntities,

    /// Create a legal entity node
    CreateLegalEntity {
        /// Legal entity name
        #[arg(long)]
        name: String,

        /// Role [possible values: parent, subsidiary, affiliate, branch]
        #[arg(long)]
        role: String,

        /// Entity ID to link (optional)
        #[arg(long)]
        linked_entity_id: Option<String>,
    },

    // ── Control Links ─────────────────────────────────────────────────────────
    /// List control links between legal entities
    ControlLinks,

    /// Create a control link between two legal entities
    CreateControlLink {
        /// Parent legal entity ID (from `corp cap-table legal-entities`)
        #[arg(long)]
        parent_legal_entity_id: String,

        /// Child legal entity ID (from `corp cap-table legal-entities`)
        #[arg(long)]
        child_legal_entity_id: String,

        /// Control type [possible values: majority_ownership, minority_ownership, management_control, contractual]
        #[arg(long)]
        control_type: String,

        /// Voting power in basis points (e.g. 5100 = 51%; optional)
        #[arg(long)]
        voting_power_bps: Option<i32>,

        /// Descriptive notes (optional)
        #[arg(long)]
        notes: Option<String>,
    },

    // ── Repurchase Rights ─────────────────────────────────────────────────────
    /// List repurchase rights
    RepurchaseRights,

    /// Create a repurchase right on a grant
    CreateRepurchaseRight {
        /// Grant ID (from `corp cap-table grants`)
        #[arg(long)]
        grant_id: String,

        /// Number of shares subject to repurchase
        #[arg(long)]
        share_count: i64,

        /// Repurchase price per share in cents
        #[arg(long)]
        price_per_share_cents: i64,

        /// Expiration date (YYYY-MM-DD; optional)
        #[arg(long)]
        expiration_date: Option<String>,
    },

    /// Activate a repurchase right (draft → active)
    ActivateRepurchase {
        /// Repurchase right ID (from `corp cap-table repurchase-rights`)
        rr_id: String,
    },

    /// Close a repurchase right (mark as exercised)
    CloseRepurchase {
        /// Repurchase right ID (from `corp cap-table repurchase-rights`)
        rr_id: String,
    },

    /// Waive a repurchase right
    WaiveRepurchase {
        /// Repurchase right ID (from `corp cap-table repurchase-rights`)
        rr_id: String,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: CapTableCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    let entity_id = ctx.require_entity()?;

    match cmd {
        CapTableCommand::Show { entity_ref } => {
            let eid = entity_ref.unwrap_or(entity_id);
            let path = format!("/v1/entities/{eid}/cap-table");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::Init => {
            let path = format!("/v1/entities/{entity_id}/cap-table");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Cap table initialized.", mode);
        }

        CapTableCommand::Grants => {
            let path = format!("/v1/entities/{entity_id}/grants");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::ShowGrant { grant_id } => {
            let path = format!("/v1/entities/{entity_id}/grants/{grant_id}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::Issue {
            cap_table_id,
            instrument_id,
            recipient_contact_id,
            recipient_name,
            grant_type,
            shares,
            price_per_share_cents,
        } => {
            let path = format!("/v1/entities/{entity_id}/grants");
            let body = json!({
                "cap_table_id": cap_table_id,
                "instrument_id": instrument_id,
                "recipient_contact_id": recipient_contact_id,
                "recipient_name": recipient_name,
                "grant_type": grant_type,
                "shares": shares,
                "price_per_share": price_per_share_cents,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Equity grant created.", mode);
        }

        CapTableCommand::Safes => {
            let path = format!("/v1/entities/{entity_id}/safes");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::IssueSafe {
            cap_table_id,
            investor_contact_id,
            investor_name,
            safe_type,
            investment_amount_cents,
            valuation_cap_cents,
            discount_percent,
        } => {
            let path = format!("/v1/entities/{entity_id}/safes");
            let body = json!({
                "cap_table_id": cap_table_id,
                "investor_contact_id": investor_contact_id,
                "investor_name": investor_name,
                "safe_type": safe_type,
                "investment_amount_cents": investment_amount_cents,
                "valuation_cap_cents": valuation_cap_cents,
                "discount_percent": discount_percent,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("SAFE note issued.", mode);
        }

        CapTableCommand::ConvertSafe { safe_id } => {
            let path = format!("/v1/entities/{entity_id}/safes/{safe_id}/convert");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("SAFE note converted.", mode);
        }

        CapTableCommand::Valuations => {
            let path = format!("/v1/entities/{entity_id}/valuations");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateValuation {
            cap_table_id,
            valuation_type,
            methodology,
            valuation_amount_cents,
            effective_date,
            prepared_by,
        } => {
            let path = format!("/v1/entities/{entity_id}/valuations");
            let body = json!({
                "cap_table_id": cap_table_id,
                "valuation_type": valuation_type,
                "methodology": methodology,
                "valuation_amount_cents": valuation_amount_cents,
                "effective_date": effective_date,
                "prepared_by": prepared_by,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Valuation created.", mode);
        }

        CapTableCommand::SubmitValuation { valuation_id } => {
            let path = format!("/v1/entities/{entity_id}/valuations/{valuation_id}/submit");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Valuation submitted for approval.", mode);
        }

        CapTableCommand::ApproveValuation {
            valuation_id,
            approved_by,
        } => {
            let path = format!("/v1/entities/{entity_id}/valuations/{valuation_id}/approve");
            let body = json!({ "approved_by": approved_by });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Valuation approved.", mode);
        }

        CapTableCommand::Transfers => {
            let path = format!("/v1/entities/{entity_id}/transfers");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateTransfer {
            cap_table_id,
            from_holder_id,
            to_holder_id,
            instrument_id,
            shares,
            transfer_type,
            price_per_share_cents,
        } => {
            let path = format!("/v1/entities/{entity_id}/transfers");
            let body = json!({
                "cap_table_id": cap_table_id,
                "from_holder_id": from_holder_id,
                "to_holder_id": to_holder_id,
                "instrument_id": instrument_id,
                "shares": shares,
                "transfer_type": transfer_type,
                "price_per_share_cents": price_per_share_cents,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Transfer created.", mode);
        }

        CapTableCommand::ApproveTransfer { transfer_id } => {
            let path = format!("/v1/entities/{entity_id}/transfers/{transfer_id}/approve");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Transfer approved.", mode);
        }

        CapTableCommand::ExecuteTransfer { transfer_id } => {
            let path = format!("/v1/entities/{entity_id}/transfers/{transfer_id}/execute");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Transfer executed.", mode);
        }

        CapTableCommand::Rounds => {
            let path = format!("/v1/entities/{entity_id}/rounds");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateRound {
            cap_table_id,
            name,
            target_amount_cents,
            price_per_share_cents,
        } => {
            let path = format!("/v1/entities/{entity_id}/rounds");
            let body = json!({
                "cap_table_id": cap_table_id,
                "name": name,
                "target_amount_cents": target_amount_cents,
                "price_per_share_cents": price_per_share_cents,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Funding round created.", mode);
        }

        CapTableCommand::AdvanceRound { round_id } => {
            let path = format!("/v1/entities/{entity_id}/rounds/{round_id}/advance");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Round advanced.", mode);
        }

        CapTableCommand::CloseRound { round_id } => {
            let path = format!("/v1/entities/{entity_id}/rounds/{round_id}/close");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Round closed.", mode);
        }

        CapTableCommand::Holders => {
            let path = format!("/v1/entities/{entity_id}/holders");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateHolder {
            name,
            holder_type,
            contact_id,
        } => {
            let path = format!("/v1/entities/{entity_id}/holders");
            let body = json!({
                "name": name,
                "holder_type": holder_type,
                "contact_id": contact_id,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Holder created.", mode);
        }

        // ── Vesting ───────────────────────────────────────────────────────────
        CapTableCommand::Vesting { entity_id: eid } => {
            let path = format!("/v1/entities/{eid}/vesting-schedules");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateVesting {
            grant_id,
            total_shares,
            start_date,
            template,
            cliff_months,
            total_months,
        } => {
            let path = format!("/v1/entities/{entity_id}/vesting-schedules");
            let body = json!({
                "grant_id": grant_id,
                "total_shares": total_shares,
                "vesting_start_date": start_date,
                "template": template,
                "cliff_months": cliff_months,
                "total_months": total_months,
                "acceleration_single_trigger": false,
                "acceleration_double_trigger": false,
                "early_exercise_allowed": false,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Vesting schedule created.", mode);
        }

        CapTableCommand::MaterializeVesting { schedule_id } => {
            let path =
                format!("/v1/entities/{entity_id}/vesting-schedules/{schedule_id}/materialize");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Vesting events materialized.", mode);
        }

        CapTableCommand::VestEvent { event_id } => {
            let path = format!("/v1/entities/{entity_id}/vesting-events/{event_id}/vest");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Vesting event vested.", mode);
        }

        // ── Instruments ───────────────────────────────────────────────────────
        CapTableCommand::Instruments => {
            let path = format!("/v1/entities/{entity_id}/instruments");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateInstrument {
            cap_table_id,
            symbol,
            kind,
            authorized_units,
            par_value,
            issue_price_cents,
            liquidation_preference,
        } => {
            let path = format!("/v1/entities/{entity_id}/instruments");
            let body = json!({
                "cap_table_id": cap_table_id,
                "symbol": symbol,
                "kind": kind,
                "authorized_units": authorized_units,
                "par_value": par_value,
                "issue_price_cents": issue_price_cents,
                "liquidation_preference": liquidation_preference,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Instrument created.", mode);
        }

        // ── Positions ─────────────────────────────────────────────────────────
        CapTableCommand::Positions { entity_id: eid } => {
            let path = format!("/v1/entities/{eid}/positions");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreatePosition {
            holder_id,
            instrument_id,
            quantity_units,
            principal_cents,
            source_reference,
        } => {
            let path = format!("/v1/entities/{entity_id}/positions");
            let body = json!({
                "holder_id": holder_id,
                "instrument_id": instrument_id,
                "quantity_units": quantity_units,
                "principal_cents": principal_cents,
                "source_reference": source_reference,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Position created.", mode);
        }

        CapTableCommand::ApplyDelta {
            position_id,
            quantity_delta,
            principal_delta,
            source_reference,
        } => {
            let path = format!("/v1/entities/{entity_id}/positions/{position_id}/delta");
            let body = json!({
                "quantity_delta": quantity_delta,
                "principal_delta": principal_delta,
                "source_reference": source_reference,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Position delta applied.", mode);
        }

        // ── Investor Ledger ───────────────────────────────────────────────────
        CapTableCommand::InvestorLedger => {
            let path = format!("/v1/entities/{entity_id}/investor-ledger");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateLedgerEntry {
            investor_id,
            investor_name,
            entry_type,
            amount_cents,
            shares_received,
            pro_rata_eligible,
            memo,
            effective_date,
            safe_note_id,
            funding_round_id,
        } => {
            let path = format!("/v1/entities/{entity_id}/investor-ledger");
            let body = json!({
                "investor_id": investor_id,
                "investor_name": investor_name,
                "entry_type": entry_type,
                "amount_cents": amount_cents,
                "shares_received": shares_received,
                "pro_rata_eligible": pro_rata_eligible,
                "memo": memo,
                "effective_date": effective_date,
                "safe_note_id": safe_note_id,
                "funding_round_id": funding_round_id,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Ledger entry created.", mode);
        }

        // ── Legal Entities ────────────────────────────────────────────────────
        CapTableCommand::LegalEntities => {
            let path = format!("/v1/entities/{entity_id}/legal-entities");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateLegalEntity {
            name,
            role,
            linked_entity_id,
        } => {
            let path = format!("/v1/entities/{entity_id}/legal-entities");
            let body = json!({
                "name": name,
                "role": role,
                "linked_entity_id": linked_entity_id,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Legal entity created.", mode);
        }

        // ── Control Links ─────────────────────────────────────────────────────
        CapTableCommand::ControlLinks => {
            let path = format!("/v1/entities/{entity_id}/control-links");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateControlLink {
            parent_legal_entity_id,
            child_legal_entity_id,
            control_type,
            voting_power_bps,
            notes,
        } => {
            let path = format!("/v1/entities/{entity_id}/control-links");
            let body = json!({
                "parent_legal_entity_id": parent_legal_entity_id,
                "child_legal_entity_id": child_legal_entity_id,
                "control_type": control_type,
                "voting_power_bps": voting_power_bps,
                "notes": notes,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Control link created.", mode);
        }

        // ── Repurchase Rights ─────────────────────────────────────────────────
        CapTableCommand::RepurchaseRights => {
            let path = format!("/v1/entities/{entity_id}/repurchase-rights");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        CapTableCommand::CreateRepurchaseRight {
            grant_id,
            share_count,
            price_per_share_cents,
            expiration_date,
        } => {
            let path = format!("/v1/entities/{entity_id}/repurchase-rights");
            let body = json!({
                "grant_id": grant_id,
                "share_count": share_count,
                "price_per_share_cents": price_per_share_cents,
                "expiration_date": expiration_date,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Repurchase right created.", mode);
        }

        CapTableCommand::ActivateRepurchase { rr_id } => {
            let path = format!("/v1/entities/{entity_id}/repurchase-rights/{rr_id}/activate");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Repurchase right activated.", mode);
        }

        CapTableCommand::CloseRepurchase { rr_id } => {
            let path = format!("/v1/entities/{entity_id}/repurchase-rights/{rr_id}/close");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Repurchase right closed.", mode);
        }

        CapTableCommand::WaiveRepurchase { rr_id } => {
            let path = format!("/v1/entities/{entity_id}/repurchase-rights/{rr_id}/waive");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Repurchase right waived.", mode);
        }
    }

    Ok(())
}
