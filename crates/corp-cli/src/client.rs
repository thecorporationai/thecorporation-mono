//! API client abstraction for the Corp CLI.
//!
//! [`CorpClient`] is a trait with two implementations:
//!
//! - [`HttpClient`] — talks to a running `corp-server` via HTTP (remote mode)
//! - [`LocalClient`] — shells out to `corp-server call` for in-process oneshot
//!   execution against a local git repo (no running server needed)

use anyhow::{Context, bail};
use async_trait::async_trait;
use reqwest::{
    StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde_json::Value;
use std::process::Command;

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Uniform interface for making API calls — either over HTTP or via a local
/// binary oneshot.  All command modules use `dyn CorpClient` so they don't
/// care which backend they're talking to.
#[async_trait]
pub trait CorpClient: Send + Sync {
    async fn get(&self, path: &str) -> anyhow::Result<Value>;
    async fn post(&self, path: &str, body: &Value) -> anyhow::Result<Value>;
    async fn put(&self, path: &str, body: &Value) -> anyhow::Result<Value>;
    async fn patch(&self, path: &str, body: &Value) -> anyhow::Result<Value>;
    async fn delete(&self, path: &str) -> anyhow::Result<Value>;
}

// ── HttpClient ───────────────────────────────────────────────────────────────

/// Remote-mode client: talks to a running corp-server via HTTP.
pub struct HttpClient {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl HttpClient {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("corp-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build reqwest client");
        let base_url = base_url.trim_end_matches('/').to_owned();
        Self {
            base_url,
            api_key,
            http,
        }
    }

    fn url(&self, path: &str) -> String {
        let path = if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        };
        format!("{}{}", self.base_url, path)
    }

    fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(key) = &self.api_key {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {key}")) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        headers
    }

    async fn handle_response(&self, resp: reqwest::Response) -> anyhow::Result<Value> {
        let status = resp.status();
        if status.is_success() {
            if status == StatusCode::NO_CONTENT {
                return Ok(Value::Object(Default::default()));
            }
            return resp.json().await.context("reading response body");
        }
        let body_text = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<Value>(&body_text)
            .ok()
            .and_then(|v| {
                v.get("error")
                    .or_else(|| v.get("message"))
                    .and_then(|m| m.as_str())
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| {
                if body_text.is_empty() {
                    status.to_string()
                } else {
                    body_text
                }
            });
        bail!("API error {status}: {message}")
    }
}

#[async_trait]
impl CorpClient for HttpClient {
    async fn get(&self, path: &str) -> anyhow::Result<Value> {
        let resp = self
            .http
            .get(self.url(path))
            .headers(self.auth_headers())
            .send()
            .await
            .with_context(|| format!("GET {path}"))?;
        self.handle_response(resp).await
    }

    async fn post(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        let resp = self
            .http
            .post(self.url(path))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await
            .with_context(|| format!("POST {path}"))?;
        self.handle_response(resp).await
    }

    async fn put(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        let resp = self
            .http
            .put(self.url(path))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await
            .with_context(|| format!("PUT {path}"))?;
        self.handle_response(resp).await
    }

    async fn patch(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        let resp = self
            .http
            .patch(self.url(path))
            .headers(self.auth_headers())
            .json(body)
            .send()
            .await
            .with_context(|| format!("PATCH {path}"))?;
        self.handle_response(resp).await
    }

    async fn delete(&self, path: &str) -> anyhow::Result<Value> {
        let resp = self
            .http
            .delete(self.url(path))
            .headers(self.auth_headers())
            .send()
            .await
            .with_context(|| format!("DELETE {path}"))?;
        self.handle_response(resp).await
    }
}

// ── LocalClient ──────────────────────────────────────────────────────────────

/// Local-mode client: shells out to `corp-server call <method> <path> [body]`.
///
/// No running server needed — each call spawns `corp-server` as a subprocess
/// that builds the Axum router in-process, dispatches the single request
/// through tower's oneshot, prints the JSON response, and exits.
pub struct LocalClient {
    /// Path to the `corp-server` binary.
    server_bin: String,
    /// Env vars to pass through (CORP_DATA_DIR, CORP_JWT_SECRET, etc).
    env: Vec<(String, String)>,
}

impl LocalClient {
    pub fn new(server_bin: String, data_dir: String, jwt_secret: String) -> Self {
        let env = vec![
            ("CORP_DATA_DIR".into(), data_dir),
            ("CORP_JWT_SECRET".into(), jwt_secret),
            ("CORP_STORAGE_BACKEND".into(), "git".into()),
            // Suppress tracing noise from the subprocess.
            ("RUST_LOG".into(), "error".into()),
        ];
        Self { server_bin, env }
    }

    /// Build a LocalClient that auto-discovers the server binary next to the
    /// current CLI binary.
    ///
    /// Generates a local JWT with `Scope::All` so the subprocess accepts every
    /// request.  The JWT secret is fixed and not exposed over any network.
    pub fn auto(data_dir: String) -> anyhow::Result<Self> {
        let self_exe = std::env::current_exe().context("current_exe")?;
        let dir = self_exe.parent().context("no parent dir")?;
        let server_bin = dir.join("corp-server");
        if !server_bin.exists() {
            bail!(
                "corp-server binary not found at {}.\n\
                 Build it with: cargo build -p corp-server\n\
                 Or use --api-url to connect to a remote server.",
                server_bin.display()
            );
        }

        let jwt_secret = "local-cli-mode-secret-not-for-production".to_owned();

        // Ensure data dir exists.
        std::fs::create_dir_all(&data_dir).ok();

        // Load or create a persistent workspace ID.
        // Stored in {data_dir}/.workspace-id so it survives across invocations
        // and is unique per workspace (not derived from the path).
        let ws_id_path = std::path::Path::new(&data_dir).join(".workspace-id");
        let ws_id = if ws_id_path.exists() {
            let raw = std::fs::read_to_string(&ws_id_path).context("reading .workspace-id")?;
            raw.trim()
                .parse::<corp_core::ids::WorkspaceId>()
                .map_err(|_| anyhow::anyhow!("corrupt .workspace-id: {}", raw.trim()))?
        } else {
            let id = corp_core::ids::WorkspaceId::new();
            std::fs::write(&ws_id_path, id.to_string()).context("writing .workspace-id")?;
            id
        };

        // Mint a JWT that the subprocess will accept.
        let jwt_cfg = corp_auth::JwtConfig::new(jwt_secret.as_bytes());
        let now = chrono::Utc::now().timestamp();
        let claims = corp_core::auth::Claims {
            sub: "local-cli".into(),
            workspace_id: ws_id,
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: corp_core::auth::PrincipalType::User,
            scopes: vec![corp_core::auth::Scope::All],
            iat: now,
            exp: now + 86400, // 24h
        };
        let token = jwt_cfg.encode(&claims).context("mint local JWT")?;

        let mut client = Self::new(
            server_bin.to_string_lossy().into_owned(),
            data_dir,
            jwt_secret,
        );
        client.env.push(("CORP_API_KEY".into(), token));
        Ok(client)
    }

    /// Set the auth token (JWT) that the subprocess will use.
    pub fn with_api_key(mut self, key: String) -> Self {
        self.env.push(("CORP_API_KEY".into(), key));
        self
    }

    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> anyhow::Result<Value> {
        let mut cmd = Command::new(&self.server_bin);
        cmd.arg("call").arg(method).arg(path);

        if let Some(b) = body {
            cmd.arg(serde_json::to_string(b)?);
        }

        // Set our env vars (these override any inherited parent vars).
        for (k, v) in &self.env {
            cmd.env(k, v);
        }

        let output = cmd
            .output()
            .with_context(|| format!("running corp-server call {method} {path}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Try to parse error from stdout (server prints JSON errors there).
            if let Ok(v) = serde_json::from_str::<Value>(stdout.trim()) {
                if let Some(msg) = v.get("error").and_then(|e| e.as_str()) {
                    bail!("{msg}");
                }
            }
            let msg = if !stderr.is_empty() {
                stderr.to_string()
            } else {
                stdout.to_string()
            };
            bail!("corp-server call failed: {}", msg.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Ok(Value::Object(Default::default()));
        }
        serde_json::from_str(trimmed)
            .with_context(|| format!("parsing response from corp-server call {method} {path}"))
    }
}

#[async_trait]
impl CorpClient for LocalClient {
    async fn get(&self, path: &str) -> anyhow::Result<Value> {
        self.call("GET", path, None)
    }

    async fn post(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        self.call("POST", path, Some(body))
    }

    async fn put(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        self.call("PUT", path, Some(body))
    }

    async fn patch(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        self.call("PATCH", path, Some(body))
    }

    async fn delete(&self, path: &str) -> anyhow::Result<Value> {
        self.call("DELETE", path, None)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_construction_no_double_slash() {
        let c = HttpClient::new("https://api.example.com".into(), None);
        assert_eq!(c.url("/v1/entities"), "https://api.example.com/v1/entities");
    }

    #[test]
    fn url_construction_trailing_slash_stripped() {
        let c = HttpClient::new("https://api.example.com/".into(), None);
        assert_eq!(c.url("/v1/entities"), "https://api.example.com/v1/entities");
    }

    #[test]
    fn url_construction_path_without_leading_slash() {
        let c = HttpClient::new("https://api.example.com".into(), None);
        assert_eq!(c.url("v1/entities"), "https://api.example.com/v1/entities");
    }

    #[test]
    fn auth_headers_with_key() {
        let c = HttpClient::new("https://api.example.com".into(), Some("mykey".into()));
        let h = c.auth_headers();
        let auth = h.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert_eq!(auth, "Bearer mykey");
    }

    #[test]
    fn auth_headers_without_key() {
        let c = HttpClient::new("https://api.example.com".into(), None);
        let h = c.auth_headers();
        assert!(h.get(AUTHORIZATION).is_none());
    }
}
