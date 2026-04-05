//! `corp setup` — interactive first-run wizard.
//! `corp config` — get/set/list persisted configuration.

use std::io::{self, Write};

use super::Context;
use crate::output;

// ── ConfigCommand ─────────────────────────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum ConfigCommand {
    /// Set a configuration value
    Set {
        /// Configuration key (api_url, api_key, workspace_id, active_entity_id)
        key: String,
        /// Value to set
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// List all configuration values
    List,
}

// ── run_setup ─────────────────────────────────────────────────────────────────

/// Interactive first-run setup wizard.
pub async fn run_setup(ctx: &Context) -> anyhow::Result<()> {
    println!("Welcome to the Corp CLI setup wizard.");
    println!("Press Enter to keep existing values.\n");

    let mut cfg = ctx.config.clone();

    let current_url = cfg
        .api_url
        .as_deref()
        .unwrap_or("https://api.thecorporation.ai");
    let api_url = prompt(&format!("API URL [{}]: ", current_url))?;
    if !api_url.is_empty() {
        cfg.api_url = Some(api_url);
    } else if cfg.api_url.is_none() {
        cfg.api_url = Some("https://api.thecorporation.ai".to_owned());
    }

    let api_key = prompt_secret("API key: ")?;
    if !api_key.is_empty() {
        cfg.api_key = Some(api_key);
    }

    let current_ws = cfg.workspace_id.as_deref().unwrap_or("");
    let workspace_id = prompt(&format!(
        "Workspace ID [{}]: ",
        if current_ws.is_empty() {
            "<unset>"
        } else {
            current_ws
        }
    ))?;
    if !workspace_id.is_empty() {
        cfg.workspace_id = Some(workspace_id);
    }

    cfg.save()?;
    output::print_success("Configuration saved.", ctx.mode());

    // Verify connectivity.
    println!("\nTesting connection…");
    match ctx.client.get("/v1/health").await {
        Ok(_) => output::print_success("Connected to API successfully.", ctx.mode()),
        Err(e) => output::print_warn(&format!("Could not connect: {e}")),
    }

    Ok(())
}

// ── run_config ────────────────────────────────────────────────────────────────

pub async fn run_config(cmd: ConfigCommand, ctx: &Context) -> anyhow::Result<()> {
    let mut cfg = ctx.config.clone();
    let mode = ctx.mode();

    match cmd {
        ConfigCommand::Set { key, value } => {
            cfg.set(&key, &value)?;
            cfg.save()?;
            output::print_success(&format!("{key} = {value}"), mode);
        }

        ConfigCommand::Get { key } => match cfg.get(&key) {
            Some(val) => {
                if ctx.json {
                    println!("{}", serde_json::json!({ &key: val }));
                } else {
                    output::kv(&key, &val, mode);
                }
            }
            None => {
                if ctx.json {
                    println!("null");
                } else {
                    output::kv(&key, "<unset>", mode);
                }
            }
        },

        ConfigCommand::List => {
            let fields = cfg.display_fields();
            if ctx.json {
                let obj: serde_json::Map<String, serde_json::Value> = fields
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), serde_json::Value::String(v)))
                    .collect();
                println!("{}", serde_json::to_string_pretty(&obj)?);
            } else {
                for (k, v) in fields {
                    output::kv(k, &v, mode);
                }
            }
        }
    }

    Ok(())
}

// ── Prompt helpers ────────────────────────────────────────────────────────────

fn prompt(label: &str) -> anyhow::Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_owned())
}

fn prompt_secret(label: &str) -> anyhow::Result<String> {
    // rpassword is not a dep; fall back to plain readline.
    // In a production build you'd swap in rpassword::read_password().
    prompt(label)
}
