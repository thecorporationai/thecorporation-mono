mod config;
mod client;
mod output;
mod commands;
mod local;
pub mod refs;

use clap::Parser;
use client::{HttpClient, LocalClient};

#[derive(Parser)]
#[command(name = "corp", about = "Corporate governance at the speed of code — manage entities, equity, governance, treasury, and compliance", version,
    long_about = "Corporate governance at the speed of code — manage entities, equity, governance, treasury, and compliance\n\nQuick start (local):\n  corp --local form create --name \"Anthropic PBC\" --entity-type c_corp --jurisdiction DE\n  corp --local use <ENTITY_ID>\n  corp --local form advance <ENTITY_ID>\n\nQuick start (remote):\n  corp setup\n  corp form create --name \"Anthropic PBC\""
)]
struct Cli {
    #[command(subcommand)]
    command: commands::Command,

    /// API URL for remote mode (overrides config)
    #[arg(long, env = "CORP_API_URL")]
    api_url: Option<String>,

    /// API key (overrides config)
    #[arg(long, env = "CORP_API_KEY")]
    api_key: Option<String>,

    /// Run locally via corp-server oneshot — no running server needed
    #[arg(long)]
    local: bool,

    /// Data directory for local mode (implies --local). Entities stored as bare git repos.
    #[arg(long, env = "CORP_DATA_DIR")]
    data_dir: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Quiet mode (IDs only)
    #[arg(long, global = true)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cfg = config::Config::load()?;

    let is_local = cli.local || cli.data_dir.is_some();

    let client: Box<dyn client::CorpClient> = if is_local {
        let data_dir = cli.data_dir.unwrap_or_else(|| "./corp-data".into());
        // In local mode, auto() generates its own JWT — don't override with
        // an external API key from config.
        Box::new(LocalClient::auto(data_dir)?)
    } else {
        let url = cli.api_url
            .or(cfg.api_url.clone())
            .unwrap_or_else(|| "http://localhost:8000".into());
        let key = cli.api_key.or(cfg.api_key.clone());
        Box::new(HttpClient::new(url, key))
    };

    let ctx = commands::Context {
        client,
        config: cfg,
        refs: std::cell::RefCell::new(refs::RefStore::load()),
        json: cli.json,
        quiet: cli.quiet,
    };

    commands::run(cli.command, ctx).await
}
