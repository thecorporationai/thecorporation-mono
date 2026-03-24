//! `corp work-items` — work item tracking.

use serde_json::json;

use super::Context;
use crate::output;

// ── WorkItemsCommand ──────────────────────────────────────────────────────────

/// Track tasks and work items for the active entity.
///
/// Work item lifecycle: open → claimed → completed (or cancelled)
///
/// Example:
///   corp work-items create --title "File Q1 taxes" --category tax --deadline 2026-04-15
///   corp work-items claim <item_id> --claimed-by agent:tax-agent
///   corp work-items complete <item_id> --completed-by agent:tax-agent --result "Filed via TurboTax"
#[derive(clap::Subcommand)]
pub enum WorkItemsCommand {
    /// List work items for the active entity
    List,

    /// Show a work item
    Show {
        /// Work item ID (from `corp work-items list`)
        item_ref: String,
    },

    /// Create a work item (open)
    Create {
        /// Short title describing the work to be done
        #[arg(long)]
        title: String,

        /// Category for routing and filtering (e.g. tax, legal, compliance, finance)
        #[arg(long)]
        category: String,

        /// Optional detailed description of the work item
        #[arg(long)]
        description: Option<String>,

        /// Deadline date (YYYY-MM-DD)
        #[arg(long)]
        deadline: Option<String>,

        /// Mark this item as ASAP priority
        #[arg(long)]
        asap: bool,
    },

    /// Claim a work item (open → claimed)
    Claim {
        /// Work item ID (from `corp work-items list`)
        item_ref: String,

        /// Claimant identifier (agent ID from `corp agents list`, user ID, or display name)
        #[arg(long)]
        claimed_by: String,

        /// Optional claim TTL in seconds (claim expires after this duration)
        #[arg(long)]
        claim_ttl_seconds: Option<u64>,
    },

    /// Release a claimed work item back to open
    Release {
        /// Work item ID (from `corp work-items list`)
        item_ref: String,
    },

    /// Complete a work item (claimed → completed)
    Complete {
        /// Work item ID (from `corp work-items list`)
        item_ref: String,

        /// Identifier of who completed the work (agent ID or user ID)
        #[arg(long)]
        completed_by: String,

        /// Result or completion note describing what was done
        #[arg(long)]
        result: Option<String>,
    },

    /// Cancel a work item
    Cancel {
        /// Work item ID (from `corp work-items list`)
        item_ref: String,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: WorkItemsCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    let entity_id = ctx.require_entity()?;

    match cmd {
        WorkItemsCommand::List => {
            let path = format!("/v1/entities/{entity_id}/work-items");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        WorkItemsCommand::Show { item_ref } => {
            let path = format!("/v1/entities/{entity_id}/work-items/{item_ref}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        WorkItemsCommand::Create {
            title,
            category,
            description,
            deadline,
            asap,
        } => {
            let path = format!("/v1/entities/{entity_id}/work-items");
            let body = json!({
                "title": title,
                "category": category,
                "description": description.unwrap_or_default(),
                "deadline": deadline,
                "asap": asap,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Work item created.", mode);
        }

        WorkItemsCommand::Claim {
            item_ref,
            claimed_by,
            claim_ttl_seconds,
        } => {
            let path = format!("/v1/entities/{entity_id}/work-items/{item_ref}/claim");
            let body = json!({
                "claimed_by": claimed_by,
                "claim_ttl_seconds": claim_ttl_seconds,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Work item claimed.", mode);
        }

        WorkItemsCommand::Release { item_ref } => {
            let path = format!("/v1/entities/{entity_id}/work-items/{item_ref}/release");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Work item released.", mode);
        }

        WorkItemsCommand::Complete {
            item_ref,
            completed_by,
            result,
        } => {
            let path = format!("/v1/entities/{entity_id}/work-items/{item_ref}/complete");
            let body = json!({
                "completed_by": completed_by,
                "result": result,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Work item completed.", mode);
        }

        WorkItemsCommand::Cancel { item_ref } => {
            let path = format!("/v1/entities/{entity_id}/work-items/{item_ref}/cancel");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Work item cancelled.", mode);
        }
    }

    Ok(())
}
