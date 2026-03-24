//! Top-level command dispatch for the `corp` CLI.

use crate::client::CorpClient;
use crate::config::Config;
use crate::output::{self, OutputMode};
use crate::refs::{self, RefKind, RefStore};

pub mod admin;
pub mod agents;
pub mod cap_table;
pub mod contacts;
pub mod entities;
pub mod execution;
pub mod formation;
pub mod governance;
pub mod services;
pub mod setup;
pub mod treasury;
pub mod work_items;

// ── Context ──────────────────────────────────────────────────────────────────

/// Runtime context threaded through every command handler.
///
/// `client` is a trait object — either [`HttpClient`] (remote server) or
/// [`LocalClient`] (oneshot via `corp-server call`).  Command handlers don't
/// know or care which one they're talking to.
pub struct Context {
    pub client: Box<dyn CorpClient>,
    pub config: Config,
    pub refs: std::cell::RefCell<RefStore>,
    pub json: bool,
    pub quiet: bool,
}

impl Context {
    /// Resolved output mode from CLI flags.
    pub fn mode(&self) -> OutputMode {
        OutputMode::from_flags(self.json, self.quiet)
    }

    /// Return the active entity ID from config, or an error if none is set.
    pub fn require_entity(&self) -> anyhow::Result<String> {
        self.config.active_entity_id.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "no active entity — run `corp use <entity>` or `corp entities list` first"
            )
        })
    }

    /// Resolve a reference (UUID, @last, short ID, name) to a canonical UUID.
    ///
    /// If `candidates` is None and the ref isn't a UUID or @last, it's passed
    /// through as-is (the server will reject if invalid).
    pub fn resolve_ref(
        &self,
        input: &str,
        kind: RefKind,
        candidates: Option<&[serde_json::Value]>,
    ) -> anyhow::Result<String> {
        let entity_id = self.config.active_entity_id.as_deref();
        refs::resolve(
            input,
            kind,
            candidates,
            entity_id,
            &mut self.refs.borrow_mut(),
        )
    }

    /// Remember an ID from a command response (updates @last).
    pub fn remember(&self, kind: RefKind, response: &serde_json::Value) {
        let entity_id = self.config.active_entity_id.as_deref();
        refs::remember_from_response(kind, response, entity_id, &mut self.refs.borrow_mut());
    }

    /// Delegate `GET` to the underlying client.
    pub async fn get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        self.client.get(path).await
    }

    /// Delegate `POST` to the underlying client.
    pub async fn post(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.client.post(path, body).await
    }

    /// Delegate `PUT` to the underlying client.
    pub async fn put(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.client.put(path, body).await
    }

    /// Delegate `PATCH` to the underlying client.
    pub async fn patch(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.client.patch(path, body).await
    }

    /// Delegate `DELETE` to the underlying client.
    pub async fn delete(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        self.client.delete(path).await
    }
}

// ── Command enum ─────────────────────────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum Command {
    /// Interactive setup wizard — configures API connection and selects default workspace
    Setup,
    /// Configuration management
    Config {
        #[command(subcommand)]
        cmd: setup::ConfigCommand,
    },
    /// Entity management
    Entities {
        #[command(subcommand)]
        cmd: entities::EntitiesCommand,
    },
    /// Entity formation workflow
    Form {
        #[command(subcommand)]
        cmd: formation::FormCommand,
    },
    /// Cap table management
    CapTable {
        #[command(subcommand)]
        cmd: cap_table::CapTableCommand,
    },
    /// Governance bodies, meetings and votes
    Governance {
        #[command(subcommand)]
        cmd: governance::GovernanceCommand,
    },
    /// Treasury and finance
    Finance {
        #[command(subcommand)]
        cmd: treasury::FinanceCommand,
    },
    /// Execution intents and obligations
    Execution {
        #[command(subcommand)]
        cmd: execution::ExecutionCommand,
    },
    /// Contact management
    Contacts {
        #[command(subcommand)]
        cmd: contacts::ContactsCommand,
    },
    /// AI agent management
    Agents {
        #[command(subcommand)]
        cmd: agents::AgentsCommand,
    },
    /// Track tasks and work items for the active entity
    WorkItems {
        #[command(subcommand)]
        cmd: work_items::WorkItemsCommand,
    },
    /// Request professional services: state filings, registered agent, EIN, annual reports
    Services {
        #[command(subcommand)]
        cmd: services::ServicesCommand,
    },
    /// Administrative operations: API key management and workspace listing
    Admin {
        #[command(subcommand)]
        cmd: admin::AdminCommand,
    },
    /// Show workspace status summary (active entity, pending actions)
    Status,
    /// Show current CLI context (API URL, workspace, active entity)
    Context,
    /// Set the active entity for subsequent commands
    Use {
        /// Entity ID or name prefix
        entity_ref: String,
    },
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

pub async fn run(command: Command, ctx: Context) -> anyhow::Result<()> {
    match command {
        Command::Setup => setup::run_setup(&ctx).await,
        Command::Config { cmd } => setup::run_config(cmd, &ctx).await,
        Command::Entities { cmd } => entities::run(cmd, &ctx).await,
        Command::Form { cmd } => formation::run(cmd, &ctx).await,
        Command::CapTable { cmd } => cap_table::run(cmd, &ctx).await,
        Command::Governance { cmd } => governance::run(cmd, &ctx).await,
        Command::Finance { cmd } => treasury::run(cmd, &ctx).await,
        Command::Execution { cmd } => execution::run(cmd, &ctx).await,
        Command::Contacts { cmd } => contacts::run(cmd, &ctx).await,
        Command::Agents { cmd } => agents::run(cmd, &ctx).await,
        Command::WorkItems { cmd } => work_items::run(cmd, &ctx).await,
        Command::Services { cmd } => services::run(cmd, &ctx).await,
        Command::Admin { cmd } => admin::run(cmd, &ctx).await,
        Command::Status => run_status(&ctx).await,
        Command::Context => run_context(&ctx),
        Command::Use { entity_ref } => run_use(entity_ref, &ctx).await,
    }
}

// ── Built-in leaf handlers ───────────────────────────────────────────────────

async fn run_status(ctx: &Context) -> anyhow::Result<()> {
    let value = ctx.client.get("/v1/status").await?;
    output::print_value(&value, ctx.mode());
    Ok(())
}

fn run_context(ctx: &Context) -> anyhow::Result<()> {
    let mode = ctx.mode();
    if ctx.json {
        let obj = serde_json::json!({
            "api_url": ctx.config.api_url,
            "workspace_id": ctx.config.workspace_id,
            "active_entity_id": ctx.config.active_entity_id,
        });
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        output::kv(
            "API URL",
            ctx.config.api_url.as_deref().unwrap_or("<default>"),
            mode,
        );
        output::kv(
            "Workspace ID",
            ctx.config.workspace_id.as_deref().unwrap_or("<unset>"),
            mode,
        );
        output::kv(
            "Active entity",
            ctx.config.active_entity_id.as_deref().unwrap_or("<none>"),
            mode,
        );
    }
    Ok(())
}

async fn run_use(entity_ref: String, ctx: &Context) -> anyhow::Result<()> {
    let path = format!("/v1/entities/{entity_ref}");
    let value = ctx.client.get(&path).await?;
    let id = value
        .get("entity_id")
        .or_else(|| value.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or(&entity_ref)
        .to_owned();
    let mut cfg = ctx.config.clone();
    cfg.active_entity_id = Some(id.clone());
    cfg.save()?;
    output::print_success(&format!("Active entity set to {id}"), ctx.mode());
    Ok(())
}
