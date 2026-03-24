//! `corp execution` — execution intents, obligations, and receipts.

use serde_json::json;

use crate::output;
use super::Context;

// ── ExecutionCommand ──────────────────────────────────────────────────────────

/// Execution intents, obligations, and receipts
///
/// Intent lifecycle: pending → evaluated → authorized → executed (or failed/cancelled)
/// Obligation lifecycle: required → in_progress → fulfilled (or waived/expired)
///
/// All side effects follow the intent→obligation→receipt pattern:
///   corp execution create-intent --intent-type "hire_employee" --description "..."
///   corp execution evaluate-intent <intent_id>
///   corp execution authorize-intent <intent_id>
///   corp execution execute-intent <intent_id>
#[derive(clap::Subcommand)]
#[command(long_about = "Track governance execution: intents declare what should happen, obligations track what must be done, receipts record completions.\n\nIntent lifecycle: pending → evaluated → authorized → executed (or cancelled)\nObligation lifecycle: required → in_progress → fulfilled (or waived)")]
pub enum ExecutionCommand {
    // ── Intents ───────────────────────────────────────────────────────────────

    /// List intents for the active entity
    Intents,

    /// Show a single intent
    ShowIntent {
        /// Intent ID (from `corp execution intents`)
        intent_id: String,
    },

    /// Create an execution intent
    CreateIntent {
        /// Dotted type (e.g. equity.grant.issue, governance.resolution.approve)
        #[arg(long)]
        intent_type: String,

        /// What this intent authorizes
        #[arg(long)]
        description: String,

        /// Required authorization: tier1, tier2, or tier3
        #[arg(long, default_value = "tier1")]
        authority_tier: String,
    },

    /// Evaluate an intent (pending → evaluated)
    EvaluateIntent {
        /// Intent ID (from `corp execution intents`)
        intent_id: String,
    },

    /// Authorize an intent (evaluated → authorized)
    AuthorizeIntent {
        /// Intent ID (from `corp execution intents`)
        intent_id: String,
    },

    /// Mark intent as executed (authorized → executed)
    ExecuteIntent {
        /// Intent ID (from `corp execution intents`)
        intent_id: String,
    },

    /// Cancel an intent (any non-executed state → cancelled)
    CancelIntent {
        /// Intent ID (from `corp execution intents`)
        intent_id: String,
    },

    /// Update an intent's metadata or description
    UpdateIntent {
        /// Intent ID (from `corp execution intents`)
        intent_id: String,

        /// New description
        #[arg(long)]
        description: Option<String>,
    },

    // ── Obligations ───────────────────────────────────────────────────────────

    /// List obligations for the active entity
    Obligations,

    /// Show a single obligation
    ShowObligation {
        /// Obligation ID (from `corp execution obligations`)
        obligation_id: String,
    },

    /// Create an obligation
    CreateObligation {
        /// Type (e.g. file_annual_report, transfer_shares)
        #[arg(long)]
        obligation_type: String,

        /// Who is responsible: internal, third_party, or human
        #[arg(long, default_value = "internal")]
        assignee_type: String,

        /// Assignee ID (contact ID, depending on --assignee-type)
        #[arg(long)]
        assignee_id: Option<String>,

        /// What must be done to fulfill this
        #[arg(long)]
        description: String,

        /// Deadline (YYYY-MM-DD)
        #[arg(long)]
        due_date: Option<String>,

        /// Link to authorizing intent (from `corp execution intents`)
        #[arg(long)]
        intent_id: Option<String>,
    },

    /// Begin work (required → in_progress)
    StartObligation {
        /// Obligation ID (from `corp execution obligations`)
        obligation_id: String,
    },

    /// Mark as fulfilled (in_progress → fulfilled)
    FulfillObligation {
        /// Obligation ID (from `corp execution obligations`)
        obligation_id: String,
    },

    /// Waive — no longer required
    WaiveObligation {
        /// Obligation ID (from `corp execution obligations`)
        obligation_id: String,
    },

    /// Update an obligation
    UpdateObligation {
        /// Obligation ID (from `corp execution obligations`)
        obligation_id: String,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// Assign to a contact (contact ID from `corp contacts list`)
        #[arg(long)]
        assignee_id: Option<String>,
    },

    // ── Receipts ──────────────────────────────────────────────────────────────

    /// List execution receipts
    Receipts,

    /// Show a single receipt
    ShowReceipt {
        /// Receipt ID (from `corp execution receipts`)
        receipt_id: String,
    },

    /// Create an execution receipt (records an idempotent execution result)
    CreateReceipt {
        /// Intent ID this receipt is for (from `corp execution intents`)
        #[arg(long)]
        intent_id: String,

        /// Idempotency key (unique string to prevent duplicate execution)
        #[arg(long)]
        idempotency_key: String,

        /// SHA-256 hash of the request payload
        #[arg(long)]
        request_hash: String,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: ExecutionCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    let entity_id = ctx.require_entity()?;

    match cmd {
        ExecutionCommand::Intents => {
            let path = format!("/v1/entities/{entity_id}/intents");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ExecutionCommand::ShowIntent { intent_id } => {
            let path = format!("/v1/entities/{entity_id}/intents/{intent_id}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ExecutionCommand::CreateIntent { intent_type, description, authority_tier } => {
            let path = format!("/v1/entities/{entity_id}/intents");
            let body = json!({
                "intent_type": intent_type,
                "description": description,
                "authority_tier": authority_tier,
                "metadata": {},
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Intent created.", mode);
        }

        ExecutionCommand::EvaluateIntent { intent_id } => {
            let path = format!("/v1/entities/{entity_id}/intents/{intent_id}/evaluate");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Intent evaluated.", mode);
        }

        ExecutionCommand::AuthorizeIntent { intent_id } => {
            let path = format!("/v1/entities/{entity_id}/intents/{intent_id}/authorize");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Intent authorized.", mode);
        }

        ExecutionCommand::ExecuteIntent { intent_id } => {
            let path = format!("/v1/entities/{entity_id}/intents/{intent_id}/execute");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Intent executed.", mode);
        }

        ExecutionCommand::CancelIntent { intent_id } => {
            let path = format!("/v1/entities/{entity_id}/intents/{intent_id}/cancel");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Intent cancelled.", mode);
        }

        ExecutionCommand::UpdateIntent { intent_id, description } => {
            let path = format!("/v1/entities/{entity_id}/intents/{intent_id}");
            let mut patch = serde_json::Map::new();
            if let Some(d) = description { patch.insert("description".into(), json!(d)); }
            let value = ctx.client.patch(&path, &serde_json::Value::Object(patch)).await?;
            output::print_value(&value, mode);
            output::print_success("Intent updated.", mode);
        }

        ExecutionCommand::Obligations => {
            let path = format!("/v1/entities/{entity_id}/obligations");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ExecutionCommand::ShowObligation { obligation_id } => {
            let path = format!("/v1/entities/{entity_id}/obligations/{obligation_id}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ExecutionCommand::CreateObligation {
            obligation_type,
            assignee_type,
            assignee_id,
            description,
            due_date,
            intent_id,
        } => {
            let path = format!("/v1/entities/{entity_id}/obligations");
            let body = json!({
                "obligation_type": obligation_type,
                "assignee_type": assignee_type,
                "assignee_id": assignee_id,
                "description": description,
                "due_date": due_date,
                "intent_id": intent_id,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Obligation created.", mode);
        }

        ExecutionCommand::StartObligation { obligation_id } => {
            let path = format!("/v1/entities/{entity_id}/obligations/{obligation_id}/start");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Obligation started.", mode);
        }

        ExecutionCommand::FulfillObligation { obligation_id } => {
            let path = format!("/v1/entities/{entity_id}/obligations/{obligation_id}/fulfill");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Obligation fulfilled.", mode);
        }

        ExecutionCommand::WaiveObligation { obligation_id } => {
            let path = format!("/v1/entities/{entity_id}/obligations/{obligation_id}/waive");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Obligation waived.", mode);
        }

        ExecutionCommand::UpdateObligation { obligation_id, description, assignee_id } => {
            let path = format!("/v1/entities/{entity_id}/obligations/{obligation_id}");
            let mut patch = serde_json::Map::new();
            if let Some(d) = description { patch.insert("description".into(), json!(d)); }
            if let Some(a) = assignee_id { patch.insert("assignee_id".into(), json!(a)); }
            let value = ctx.client.patch(&path, &serde_json::Value::Object(patch)).await?;
            output::print_value(&value, mode);
            output::print_success("Obligation updated.", mode);
        }

        ExecutionCommand::Receipts => {
            let path = format!("/v1/entities/{entity_id}/receipts");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ExecutionCommand::ShowReceipt { receipt_id } => {
            let path = format!("/v1/entities/{entity_id}/receipts/{receipt_id}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ExecutionCommand::CreateReceipt { intent_id, idempotency_key, request_hash } => {
            let path = format!("/v1/entities/{entity_id}/receipts");
            let body = json!({
                "intent_id": intent_id,
                "idempotency_key": idempotency_key,
                "request_hash": request_hash,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Receipt created.", mode);
        }
    }

    Ok(())
}
