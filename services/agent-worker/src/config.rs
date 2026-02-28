use serde::Deserialize;
use std::collections::HashMap;

/// Worker configuration loaded from environment variables.
#[derive(Debug, Deserialize)]
pub struct WorkerConfig {
    /// Redis connection URL.
    pub redis_url: String,

    /// Base URL of the api-rs service (e.g. "http://localhost:8000").
    pub api_base_url: String,

    /// Bearer token used for authenticated worker -> api-rs calls.
    pub api_bearer_token: String,

    /// Docker socket path (e.g. "unix:///var/run/docker.sock").
    #[serde(default = "default_docker_host")]
    pub docker_host: String,

    /// Root path for agent workspace volumes on the host.
    #[serde(default = "default_workspace_root")]
    pub workspace_root: String,

    /// Docker image to use for the Pi runtime container.
    #[serde(default = "default_runtime_image")]
    pub runtime_image: String,

    /// Default per-container memory limit in MB.
    #[serde(default = "default_runtime_memory_mb")]
    pub runtime_memory_mb: u64,

    /// Default per-container CPU limit (fractional CPUs).
    #[serde(default = "default_runtime_cpu_limit")]
    pub runtime_cpu_limit: f64,

    /// Default execution timeout in seconds.
    #[serde(default = "default_runtime_timeout_seconds")]
    pub runtime_timeout_seconds: u64,

    /// Maximum concurrent container executions (0 = auto-detect).
    #[serde(default)]
    pub max_concurrency: usize,

    /// Maximum number of jobs allowed in the queue (0 = unlimited).
    #[serde(default = "default_max_queue_depth")]
    pub max_queue_depth: u64,

    /// Maximum log entries kept in Redis per execution (older entries trimmed).
    #[serde(default = "default_max_log_entries_redis")]
    pub max_log_entries_redis: i64,

    /// Queue consumer poll interval in seconds.
    #[serde(default = "default_poll_seconds")]
    pub poll_seconds: f64,

    /// LLM proxy URL exposed to containers via host.docker.internal.
    #[serde(default = "default_llm_proxy_url")]
    pub llm_proxy_url: String,

    /// Model pricing: model name -> (input_cents_per_million, output_cents_per_million).
    #[serde(default = "default_model_pricing")]
    pub model_pricing: HashMap<String, ModelPricing>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelPricing {
    pub input: u64,
    pub output: u64,
}

fn default_docker_host() -> String {
    if let Ok(uid) = std::env::var("UID") {
        return format!("unix:///run/user/{uid}/docker.sock");
    }
    "unix:///var/run/docker.sock".to_owned()
}

fn default_workspace_root() -> String {
    "/var/lib/agents/workspaces".to_owned()
}

fn default_runtime_image() -> String {
    "agents-runtime:pi".to_owned()
}

fn default_runtime_memory_mb() -> u64 { 512 }
fn default_runtime_cpu_limit() -> f64 { 0.5 }
fn default_runtime_timeout_seconds() -> u64 { 300 }
fn default_max_queue_depth() -> u64 { 1000 }
fn default_max_log_entries_redis() -> i64 { 1000 }
fn default_poll_seconds() -> f64 { 2.0 }

fn default_llm_proxy_url() -> String {
    "http://host.docker.internal:8000/v1/llm/proxy".to_owned()
}

fn default_model_pricing() -> HashMap<String, ModelPricing> {
    let mut m = HashMap::new();
    m.insert("anthropic/claude-sonnet-4-6".to_owned(), ModelPricing { input: 300, output: 1500 });
    m.insert("anthropic/claude-haiku-4-5".to_owned(), ModelPricing { input: 80, output: 400 });
    m.insert("openai/gpt-4o".to_owned(), ModelPricing { input: 250, output: 1000 });
    m.insert("openai/gpt-4o-mini".to_owned(), ModelPricing { input: 15, output: 60 });
    m
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        let cfg: Self = envy::from_env()?;
        if cfg.api_bearer_token.trim().is_empty() {
            return Err(envy::Error::Custom("API_BEARER_TOKEN must not be empty".to_owned()));
        }
        Ok(cfg)
    }

    pub fn api_auth_header_value(&self) -> String {
        format!("Bearer {}", self.api_bearer_token)
    }

    /// Secrets proxy URL that containers will call back to.
    pub fn secrets_proxy_url(&self) -> String {
        format!("{}/v1/secrets", self.api_base_url)
            .replace("localhost", "host.docker.internal")
            .replace("127.0.0.1", "host.docker.internal")
    }

    /// Calculate cost in dollars for the given token usage.
    ///
    /// Returns `None` if the model is not in the pricing table.
    /// Pricing values are cents per million tokens.
    pub fn cost_for_model(&self, model: &str, input_tokens: u64, output_tokens: u64) -> Option<f64> {
        let p = self.model_pricing.get(model)?;
        // pricing is cents per million tokens → multiply then convert cents→dollars (/100) and per-million (/1_000_000)
        Some((input_tokens as f64 * p.input as f64 + output_tokens as f64 * p.output as f64) / 100_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_for_model_known() {
        let config = WorkerConfig {
            redis_url: String::new(),
            api_base_url: String::new(),
            api_bearer_token: String::new(),
            docker_host: String::new(),
            workspace_root: String::new(),
            runtime_image: String::new(),
            runtime_memory_mb: 512,
            runtime_cpu_limit: 0.5,
            runtime_timeout_seconds: 300,
            max_concurrency: 0,
            max_queue_depth: 1000,
            max_log_entries_redis: 1000,
            poll_seconds: 2.0,
            llm_proxy_url: String::new(),
            model_pricing: default_model_pricing(),
        };

        // anthropic/claude-sonnet-4-6: input=300, output=1500 (cents per million)
        // 1_000_000 input tokens → 300 cents = $3.00
        // 500_000 output tokens → 750 cents = $7.50
        // total = $10.50
        let cost = config.cost_for_model("anthropic/claude-sonnet-4-6", 1_000_000, 500_000).unwrap();
        assert!((cost - 10.50).abs() < 1e-9, "expected 10.50, got {cost}");
    }

    #[test]
    fn cost_for_model_unknown() {
        let config = WorkerConfig {
            redis_url: String::new(),
            api_base_url: String::new(),
            api_bearer_token: String::new(),
            docker_host: String::new(),
            workspace_root: String::new(),
            runtime_image: String::new(),
            runtime_memory_mb: 512,
            runtime_cpu_limit: 0.5,
            runtime_timeout_seconds: 300,
            max_concurrency: 0,
            max_queue_depth: 1000,
            max_log_entries_redis: 1000,
            poll_seconds: 2.0,
            llm_proxy_url: String::new(),
            model_pricing: HashMap::new(),
        };

        assert!(config.cost_for_model("unknown/model", 1000, 1000).is_none());
    }
}
