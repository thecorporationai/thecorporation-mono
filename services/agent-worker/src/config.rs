use serde::Deserialize;
use std::collections::HashMap;

/// Worker configuration loaded from environment variables.
#[derive(Debug, Deserialize)]
pub struct WorkerConfig {
    /// Redis connection URL.
    pub redis_url: String,

    /// Base URL of the api-rs service (e.g. "http://localhost:8000").
    pub api_base_url: String,

    /// Docker socket path (e.g. "unix:///var/run/docker.sock").
    #[serde(default = "default_docker_host")]
    pub docker_host: String,

    /// Fernet key for secret encryption (base64-encoded).
    #[serde(default)]
    pub fernet_key: Option<String>,

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
    // Prefer rootless Docker if available
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

fn default_runtime_memory_mb() -> u64 {
    512
}

fn default_runtime_cpu_limit() -> f64 {
    0.5
}

fn default_runtime_timeout_seconds() -> u64 {
    300
}

fn default_poll_seconds() -> f64 {
    2.0
}

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
        envy::from_env()
    }

    /// Unique identifier for this worker instance.
    pub fn worker_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Secrets proxy URL that containers will call back to.
    pub fn secrets_proxy_url(&self) -> String {
        format!("{}/v1/secrets", self.api_base_url)
            .replace("localhost", "host.docker.internal")
            .replace("127.0.0.1", "host.docker.internal")
    }
}
