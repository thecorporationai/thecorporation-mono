//! `corp admin` — administrative operations (workspace management, API keys, health).

use serde_json::json;

use super::Context;
use crate::output;

// ── AdminCommand ──────────────────────────────────────────────────────────────

/// Administrative operations: API key management and workspace listing.
#[derive(clap::Subcommand)]
pub enum AdminCommand {
    /// Check API health
    Health,

    /// List all workspaces (super-admin only)
    Workspaces,

    /// List entities in a workspace (super-admin only)
    WorkspaceEntities {
        /// Workspace ID (from `corp admin workspaces`)
        workspace_id: String,
    },

    /// List API keys for the current workspace (keys use the corp_ prefix)
    ApiKeys,

    /// Create a new API key (keys use the corp_ prefix)
    CreateApiKey {
        /// Key display name (for identification in the dashboard)
        #[arg(long)]
        name: String,

        /// Comma-separated list of scopes. Valid values:
        /// formation-create, formation-read, equity-read, equity-write,
        /// governance-read, governance-write, treasury-read, treasury-write,
        /// contacts-read, contacts-write, admin, all
        #[arg(long)]
        scopes: Option<String>,

        /// Restrict key to a specific entity ID (from `corp entities list`, optional)
        #[arg(long)]
        entity_id: Option<String>,
    },

    /// Revoke an API key (by key ID from `corp admin api-keys`)
    RevokeApiKey {
        /// API key ID to revoke (from `corp admin api-keys`; keys use the corp_ prefix)
        key_id: String,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: AdminCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();

    match cmd {
        AdminCommand::Health => {
            // Health endpoint is at /health (no /v1 prefix), unauthenticated.
            let value = ctx.client.get("/health").await?;
            output::print_value(&value, mode);
        }

        AdminCommand::Workspaces => {
            let value = ctx.client.get("/v1/workspaces").await?;
            output::print_value(&value, mode);
        }

        AdminCommand::WorkspaceEntities { workspace_id } => {
            let path = format!("/v1/workspaces/{workspace_id}/entities");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        AdminCommand::ApiKeys => {
            let value = ctx.client.get("/v1/api-keys").await?;
            output::print_value(&value, mode);
        }

        AdminCommand::CreateApiKey {
            name,
            scopes,
            entity_id,
        } => {
            let scopes_list: Vec<&str> = scopes
                .as_deref()
                .map(|s| s.split(',').collect())
                .unwrap_or_default();
            let body = json!({
                "name": name,
                "scopes": scopes_list,
                "entity_id": entity_id,
            });
            let value = ctx.client.post("/v1/api-keys", &body).await?;

            if ctx.json {
                output::print_value(&value, mode);
            } else {
                // Surface the key plaintext prominently — it won't be shown again.
                let key_text = value
                    .get("raw_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<see JSON output>");
                output::print_success("API key created.", mode);
                output::print_warn("Store this key securely — it will not be shown again.");
                println!("Key: {key_text}");
            }
        }

        AdminCommand::RevokeApiKey { key_id } => {
            // Server uses POST /api-keys/{key_id}/revoke (not DELETE).
            let path = format!("/v1/api-keys/{key_id}/revoke");
            let value = ctx.post(&path, &json!({})).await?;
            output::print_value(&value, mode);
            output::print_success("API key revoked.", mode);
        }
    }

    Ok(())
}
