//! `corp services` — service request management.

use serde_json::json;

use super::Context;
use crate::output;

// ── ServicesCommand ───────────────────────────────────────────────────────────

/// Request professional services: state filings, registered agent, EIN, annual reports.
///
/// Service request lifecycle: pending → checkout → paid → fulfilling → fulfilled
#[derive(clap::Subcommand)]
pub enum ServicesCommand {
    /// List service requests for the active entity
    List,

    /// Show a service request
    Show {
        /// Service request ID (from `corp services list`)
        request_ref: String,
    },

    /// Purchase a service by slug (creates a pending service request)
    Buy {
        /// Service product slug (e.g. registered-agent-de, annual-report-de)
        slug: String,

        /// Price in cents (e.g. 49900 = $499.00)
        #[arg(long)]
        amount_cents: i64,
    },

    /// Begin checkout for a service request (pending → checkout)
    Checkout {
        /// Service request ID (from `corp services list`)
        request_ref: String,
    },

    /// Mark a service request as paid (checkout → paid)
    Pay {
        /// Service request ID (from `corp services list`)
        request_ref: String,
    },

    /// Mark a service request as fulfilled — admin only (paid → fulfilled)
    Fulfill {
        /// Service request ID (from `corp services list`)
        request_ref: String,

        /// Fulfillment note describing what was delivered
        #[arg(long)]
        note: Option<String>,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: ServicesCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    let entity_id = ctx.require_entity()?;

    match cmd {
        ServicesCommand::List => {
            let path = format!("/v1/entities/{entity_id}/service-requests");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ServicesCommand::Show { request_ref } => {
            let path = format!("/v1/entities/{entity_id}/service-requests/{request_ref}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ServicesCommand::Buy { slug, amount_cents } => {
            let path = format!("/v1/entities/{entity_id}/service-requests");
            let body = json!({
                "service_slug": slug,
                "amount_cents": amount_cents,
            });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Service request created.", mode);
        }

        ServicesCommand::Checkout { request_ref } => {
            let path = format!("/v1/entities/{entity_id}/service-requests/{request_ref}/checkout");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Checkout started.", mode);
        }

        ServicesCommand::Pay { request_ref } => {
            let path = format!("/v1/entities/{entity_id}/service-requests/{request_ref}/pay");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("Service request marked paid.", mode);
        }

        ServicesCommand::Fulfill { request_ref, note } => {
            let path = format!("/v1/entities/{entity_id}/service-requests/{request_ref}/fulfill");
            let body = json!({ "fulfillment_note": note });
            let value = ctx.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Service request fulfilled.", mode);
        }
    }

    Ok(())
}
