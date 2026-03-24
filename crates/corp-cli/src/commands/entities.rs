//! `corp entities` — manage legal entities.

use serde_json::json;

use super::Context;
use crate::output;

#[derive(clap::Subcommand)]
#[command(
    long_about = "Manage corporate entities. Use `corp use <entity>` to set an active entity."
)]
pub enum EntitiesCommand {
    /// List all entities in the workspace
    List,
    /// Show entity details including formation status and key dates
    Show {
        /// Entity ID or name prefix
        entity_ref: String,
    },
    /// Create a new corporate entity and begin formation
    #[command(
        after_help = "Examples:\n  corp entities create --name \"Acme Corp\" --entity-type c_corp --jurisdiction DE\n  corp entities create --name \"My LLC\" --entity-type llc --jurisdiction WY"
    )]
    Create {
        /// Legal name of the entity (e.g. Anthropic PBC)
        #[arg(long)]
        name: String,
        /// Entity type: c_corp or llc
        #[arg(long, default_value = "c_corp", value_parser = ["c_corp", "llc"])]
        entity_type: String,
        /// US state code (e.g. DE, CA, WY, NV)
        #[arg(long, default_value = "DE")]
        jurisdiction: String,
    },
    /// Dissolve an entity (irreversible)
    #[command(
        after_help = "Examples:\n  corp entities dissolve <entity_id>\n  corp entities dissolve <entity_id> --reason \"Business wound down\""
    )]
    Dissolve {
        /// Entity ID or name prefix
        entity_ref: String,
        /// Reason for dissolution (recorded in governance records)
        #[arg(long)]
        reason: Option<String>,
    },
}

pub async fn run(cmd: EntitiesCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    match cmd {
        EntitiesCommand::List => {
            let value = ctx.client.get("/v1/entities").await?;
            output::print_value(&value, mode);
        }
        EntitiesCommand::Show { entity_ref } => {
            let value = ctx
                .client
                .get(&format!("/v1/entities/{entity_ref}"))
                .await?;
            output::print_value(&value, mode);
        }
        EntitiesCommand::Create {
            name,
            entity_type,
            jurisdiction,
        } => {
            let body = json!({ "legal_name": name, "entity_type": entity_type, "jurisdiction": jurisdiction });
            let value = ctx.client.post("/v1/entities", &body).await?;
            output::print_value(&value, mode);
            output::print_success("Entity created.", mode);
        }
        EntitiesCommand::Dissolve { entity_ref, reason } => {
            let body = json!({ "reason": reason });
            let value = ctx
                .client
                .post(&format!("/v1/entities/{entity_ref}/dissolve"), &body)
                .await?;
            output::print_value(&value, mode);
            output::print_success("Entity dissolved.", mode);
        }
    }
    Ok(())
}
