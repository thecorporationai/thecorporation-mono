//! Git pack protocol — parsing utilities for SSH and HTTP transports.
//!
//! Provides service type definitions, path parsing, and error types
//! shared by the native transport handlers.

use crate::domain::ids::{EntityId, WorkspaceId};

/// Which git service is being requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitService {
    UploadPack,
    ReceivePack,
}

impl GitService {
    /// The git binary subcommand name.
    pub fn command_name(self) -> &'static str {
        match self {
            Self::UploadPack => "upload-pack",
            Self::ReceivePack => "receive-pack",
        }
    }

    /// Content-Type for the HTTP advertisement response.
    pub fn advertisement_content_type(self) -> String {
        format!("application/x-git-{}-advertisement", self.command_name())
    }

    /// Content-Type for the HTTP result response.
    pub fn result_content_type(self) -> String {
        format!("application/x-git-{}-result", self.command_name())
    }
}

/// Error type for git protocol operations.
#[derive(Debug, thiserror::Error)]
pub enum GitProtocolError {
    #[error("repository not found")]
    RepoNotFound,
    #[error("invalid service: {0}")]
    InvalidService(String),
    #[error("invalid repository path: {0}")]
    InvalidPath(String),
    #[error("git subprocess failed: {0}")]
    SubprocessError(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Parse a git SSH exec command string.
///
/// Git clients send commands like:
/// - `git-upload-pack '/workspace_id/entity_id.git'`
/// - `git-receive-pack '/workspace_id/entity_id.git'`
pub fn parse_git_command(command: &str) -> Result<(GitService, &str), GitProtocolError> {
    let command = command.trim();

    let (service, rest) = if let Some(rest) = command.strip_prefix("git-upload-pack") {
        (GitService::UploadPack, rest)
    } else if let Some(rest) = command.strip_prefix("git-receive-pack") {
        (GitService::ReceivePack, rest)
    } else if let Some(rest) = command.strip_prefix("git upload-pack") {
        (GitService::UploadPack, rest)
    } else if let Some(rest) = command.strip_prefix("git receive-pack") {
        (GitService::ReceivePack, rest)
    } else {
        return Err(GitProtocolError::InvalidService(command.to_owned()));
    };

    // Strip leading whitespace and surrounding quotes from the path
    let path = rest.trim();
    let path = path
        .strip_prefix('\'')
        .and_then(|p| p.strip_suffix('\''))
        .or_else(|| path.strip_prefix('"').and_then(|p| p.strip_suffix('"')))
        .unwrap_or(path);
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        return Err(GitProtocolError::InvalidPath("empty path".to_owned()));
    }

    Ok((service, path))
}

/// Parse a repository path into workspace and entity IDs.
///
/// Expected formats:
/// - `workspace_id/entity_id.git`
/// - `workspace_id/entity_id`
pub fn parse_repo_path(path: &str) -> Result<(WorkspaceId, EntityId), GitProtocolError> {
    let path = path.trim_start_matches('/').trim_end_matches('/');

    let (ws_str, ent_str) = path
        .split_once('/')
        .ok_or_else(|| GitProtocolError::InvalidPath(format!("missing slash in: {path}")))?;

    let ent_str = ent_str.strip_suffix(".git").unwrap_or(ent_str);

    let workspace_id: WorkspaceId = ws_str
        .parse()
        .map_err(|_| GitProtocolError::InvalidPath(format!("invalid workspace ID: {ws_str}")))?;

    let entity_id: EntityId = ent_str
        .parse()
        .map_err(|_| GitProtocolError::InvalidPath(format!("invalid entity ID: {ent_str}")))?;

    Ok((workspace_id, entity_id))
}

/// Resolve workspace + entity IDs to a bare repo path on disk, verifying it exists.
/// Parse the `service` query parameter from an HTTP info/refs request.
pub fn parse_service_param(service: &str) -> Result<GitService, GitProtocolError> {
    match service {
        "git-upload-pack" => Ok(GitService::UploadPack),
        "git-receive-pack" => Ok(GitService::ReceivePack),
        other => Err(GitProtocolError::InvalidService(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_upload_pack_single_quotes() {
        let (service, path) = parse_git_command("git-upload-pack '/abc/def.git'").unwrap();
        assert_eq!(service, GitService::UploadPack);
        assert_eq!(path, "abc/def.git");
    }

    #[test]
    fn parse_receive_pack_double_quotes() {
        let (service, path) = parse_git_command("git-receive-pack \"/abc/def.git\"").unwrap();
        assert_eq!(service, GitService::ReceivePack);
        assert_eq!(path, "abc/def.git");
    }

    #[test]
    fn parse_git_space_form() {
        let (service, path) = parse_git_command("git upload-pack '/abc/def.git'").unwrap();
        assert_eq!(service, GitService::UploadPack);
        assert_eq!(path, "abc/def.git");
    }

    #[test]
    fn parse_no_quotes() {
        let (service, path) = parse_git_command("git-upload-pack /abc/def.git").unwrap();
        assert_eq!(service, GitService::UploadPack);
        assert_eq!(path, "abc/def.git");
    }

    #[test]
    fn parse_invalid_command() {
        assert!(parse_git_command("git-status").is_err());
    }

    #[test]
    fn parse_repo_path_with_git_suffix() {
        let ws = uuid::Uuid::new_v4();
        let ent = uuid::Uuid::new_v4();
        let path = format!("{ws}/{ent}.git");
        let (parsed_ws, parsed_ent) = parse_repo_path(&path).unwrap();
        assert_eq!(parsed_ws.to_string(), ws.to_string());
        assert_eq!(parsed_ent.to_string(), ent.to_string());
    }

    #[test]
    fn parse_repo_path_without_suffix() {
        let ws = uuid::Uuid::new_v4();
        let ent = uuid::Uuid::new_v4();
        let path = format!("{ws}/{ent}");
        let (parsed_ws, parsed_ent) = parse_repo_path(&path).unwrap();
        assert_eq!(parsed_ws.to_string(), ws.to_string());
        assert_eq!(parsed_ent.to_string(), ent.to_string());
    }

    #[test]
    fn parse_repo_path_no_slash() {
        assert!(parse_repo_path("just-one-segment").is_err());
    }

    #[test]
    fn parse_repo_path_invalid_uuid() {
        assert!(parse_repo_path("not-uuid/also-not-uuid.git").is_err());
    }

    #[test]
    fn service_content_types() {
        assert_eq!(
            GitService::UploadPack.advertisement_content_type(),
            "application/x-git-upload-pack-advertisement"
        );
        assert_eq!(
            GitService::ReceivePack.result_content_type(),
            "application/x-git-receive-pack-result"
        );
    }
}
