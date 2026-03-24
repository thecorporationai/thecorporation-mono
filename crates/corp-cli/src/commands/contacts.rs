//! `corp contacts` — contact management.

use serde_json::json;

use crate::output;
use super::Context;

// ── ContactsCommand ───────────────────────────────────────────────────────────

#[derive(clap::Subcommand)]
#[command(long_about = "Manage contacts: officers, board members, investors, employees, and service providers.")]
pub enum ContactsCommand {
    /// List contacts for the active entity
    List,

    /// Show contact details
    Show {
        /// Contact ID (from `corp contacts list`)
        contact_ref: String,
    },

    /// Add a new contact
    Add {
        #[arg(long, help = "Contact's full name")]
        name: String,

        #[arg(long, help = "Email address")]
        email: Option<String>,

        #[arg(long, help = "Phone number")]
        phone: Option<String>,

        #[arg(long, help = "individual or organization", default_value = "individual")]
        contact_type: Option<String>,

        #[arg(long, help = "employee, contractor, board_member, law_firm, valuation_firm, accounting_firm, investor, officer, founder, member, other")]
        category: Option<String>,
    },

    /// Update a contact
    Edit {
        /// Contact ID (from `corp contacts list`)
        contact_ref: String,

        /// New display name
        #[arg(long, help = "Contact's full name")]
        name: Option<String>,

        /// New email address
        #[arg(long, help = "Email address")]
        email: Option<String>,

        /// New category
        #[arg(long, help = "employee, contractor, board_member, law_firm, valuation_firm, accounting_firm, investor, officer, founder, member, other")]
        category: Option<String>,

        /// Phone number
        #[arg(long, help = "Phone number")]
        phone: Option<String>,
    },
}

// ── run ───────────────────────────────────────────────────────────────────────

pub async fn run(cmd: ContactsCommand, ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    let entity_id = ctx.require_entity()?;

    match cmd {
        ContactsCommand::List => {
            let path = format!("/v1/entities/{entity_id}/contacts");
            let value = ctx.client.get(&path).await?;
            output::print_value(&value, mode);
        }

        ContactsCommand::Show { contact_ref } => {
            let path = format!("/v1/entities/{entity_id}/contacts/{contact_ref}");
            let value = ctx.get(&path).await?;
            output::print_value(&value, mode);
        }

        ContactsCommand::Add { name, email, phone, contact_type, category } => {
            let path = format!("/v1/entities/{entity_id}/contacts");
            let body = json!({
                "name": name,
                "email": email,
                "phone": phone,
                "category": category.unwrap_or_else(|| "other".into()),
                "contact_type": contact_type.unwrap_or_else(|| "individual".into()),
            });
            let value = ctx.client.post(&path, &body).await?;
            output::print_value(&value, mode);
            output::print_success("Contact added.", mode);
        }

        ContactsCommand::Edit { contact_ref, name, email, category, phone } => {
            let path = format!("/v1/entities/{entity_id}/contacts/{contact_ref}");
            let mut patch = serde_json::Map::new();
            if let Some(n) = name { patch.insert("name".into(), json!(n)); }
            if let Some(e) = email { patch.insert("email".into(), json!(e)); }
            if let Some(c) = category { patch.insert("category".into(), json!(c)); }
            if let Some(p) = phone { patch.insert("phone".into(), json!(p)); }
            let value = ctx.client.patch(&path, &serde_json::Value::Object(patch)).await?;
            output::print_value(&value, mode);
            output::print_success("Contact updated.", mode);
        }
    }

    Ok(())
}
