//! Typed enums that replace stringly-typed fields across the agent runtime.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── Error type for FromStr impls ─────────────────────────────────────

/// Error returned when parsing an enum variant from a string.
#[derive(Debug, Clone)]
pub struct ParseEnumError {
    pub type_name: &'static str,
    pub value: String,
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid {}: {:?}", self.type_name, self.value)
    }
}

impl std::error::Error for ParseEnumError {}

// ── AgentStatus ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Paused,
    Disabled,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Disabled => "disabled",
        }
    }
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AgentStatus {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "disabled" => Ok(Self::Disabled),
            _ => Err(ParseEnumError { type_name: "AgentStatus", value: s.to_owned() }),
        }
    }
}

// ── HttpMethod ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

impl Default for HttpMethod {
    fn default() -> Self {
        Self::Get
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            "PATCH" => Ok(Self::Patch),
            "DELETE" => Ok(Self::Delete),
            "HEAD" => Ok(Self::Head),
            "OPTIONS" => Ok(Self::Options),
            _ => Err(ParseEnumError { type_name: "HttpMethod", value: s.to_owned() }),
        }
    }
}

// ── ChannelType ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Email,
    Webhook,
    Cron,
    Manual,
}

impl ChannelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Webhook => "webhook",
            Self::Cron => "cron",
            Self::Manual => "manual",
        }
    }
}

impl fmt::Display for ChannelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ChannelType {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "email" => Ok(Self::Email),
            "webhook" => Ok(Self::Webhook),
            "cron" => Ok(Self::Cron),
            "manual" => Ok(Self::Manual),
            _ => Err(ParseEnumError { type_name: "ChannelType", value: s.to_owned() }),
        }
    }
}

// ── NetworkEgress ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkEgress {
    Restricted,
    Open,
}

impl NetworkEgress {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Restricted => "restricted",
            Self::Open => "open",
        }
    }
}

impl Default for NetworkEgress {
    fn default() -> Self {
        Self::Restricted
    }
}

impl fmt::Display for NetworkEgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for NetworkEgress {
    type Err = ParseEnumError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "restricted" => Ok(Self::Restricted),
            "open" => Ok(Self::Open),
            _ => Err(ParseEnumError { type_name: "NetworkEgress", value: s.to_owned() }),
        }
    }
}

// ── Transport ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Stdio,
    Http,
}

impl Transport {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Http => "http",
        }
    }
}

impl Default for Transport {
    fn default() -> Self {
        Self::Stdio
    }
}

impl fmt::Display for Transport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── LogLevel ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_status_serde() {
        let s = AgentStatus::Active;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"active\"");
        let parsed: AgentStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, s);
    }

    #[test]
    fn agent_status_fromstr() {
        assert_eq!("active".parse::<AgentStatus>().unwrap(), AgentStatus::Active);
        assert!("invalid".parse::<AgentStatus>().is_err());
    }

    #[test]
    fn http_method_serde() {
        let m = HttpMethod::Post;
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, "\"POST\"");
        let parsed: HttpMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, m);
    }

    #[test]
    fn http_method_fromstr_case_insensitive() {
        assert_eq!("get".parse::<HttpMethod>().unwrap(), HttpMethod::Get);
        assert_eq!("GET".parse::<HttpMethod>().unwrap(), HttpMethod::Get);
        assert_eq!("Post".parse::<HttpMethod>().unwrap(), HttpMethod::Post);
    }

    #[test]
    fn channel_type_serde() {
        let c = ChannelType::Cron;
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"cron\"");
    }

    #[test]
    fn network_egress_default() {
        assert_eq!(NetworkEgress::default(), NetworkEgress::Restricted);
    }

    #[test]
    fn log_level_serde() {
        let l = LogLevel::Error;
        let json = serde_json::to_string(&l).unwrap();
        assert_eq!(json, "\"error\"");
        let parsed: LogLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, l);
    }
}
