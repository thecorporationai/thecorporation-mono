//! JSON-LD context for the git log vocabulary.
//!
//! Defines the semantic mapping from short field names to IRIs.
//! The context is written as the first line of every log file so
//! each file is a self-describing NDJSON-LD stream.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const GIT_NS: &str = "https://thecorporation.com/ns/git#";
pub const PROV_NS: &str = "http://www.w3.org/ns/prov#";
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// The JSON-LD context header written to line 0 of every log file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdContext {
    #[serde(rename = "@context")]
    pub context: HashMap<String, serde_json::Value>,
}

impl Default for LdContext {
    fn default() -> Self {
        let mut ctx = HashMap::new();

        // Namespace prefixes
        ctx.insert("git".into(), json_str(GIT_NS));
        ctx.insert("prov".into(), json_str(PROV_NS));
        ctx.insert("xsd".into(), json_str(XSD_NS));

        // Term mappings
        ctx.insert("Commit".into(), json_str(&format!("{GIT_NS}Commit")));
        ctx.insert("sha".into(), json_str(&format!("{GIT_NS}sha")));
        ctx.insert(
            "parents".into(),
            json_typed_container(&format!("{GIT_NS}parent"), "@list"),
        );
        ctx.insert("author".into(), json_str(&format!("{GIT_NS}author")));
        ctx.insert("committer".into(), json_str(&format!("{GIT_NS}committer")));
        ctx.insert("message".into(), json_str(&format!("{GIT_NS}message")));
        ctx.insert(
            "timestamp".into(),
            json_typed(&format!("{PROV_NS}atTime"), &format!("{XSD_NS}dateTime")),
        );
        ctx.insert("sequence".into(), json_str(&format!("{GIT_NS}sequence")));

        // Actor trailer terms
        ctx.insert(
            "workspaceId".into(),
            json_str(&format!("{GIT_NS}workspaceId")),
        );
        ctx.insert("entityId".into(), json_str(&format!("{GIT_NS}entityId")));
        ctx.insert(
            "scopes".into(),
            json_typed_container(&format!("{GIT_NS}scope"), "@set"),
        );
        ctx.insert("signedBy".into(), json_str(&format!("{GIT_NS}signedBy")));

        // File change terms
        ctx.insert(
            "changes".into(),
            json_typed_container(&format!("{GIT_NS}change"), "@list"),
        );
        ctx.insert("path".into(), json_str(&format!("{GIT_NS}path")));
        ctx.insert("action".into(), json_str(&format!("{GIT_NS}action")));
        ctx.insert("blobSha".into(), json_str(&format!("{GIT_NS}blobSha")));
        ctx.insert("oldPath".into(), json_str(&format!("{GIT_NS}oldPath")));

        // Checkpoint terms
        ctx.insert("Checkpoint".into(), json_str(&format!("{GIT_NS}Checkpoint")));
        ctx.insert(
            "tree".into(),
            json_typed_container(&format!("{GIT_NS}treeEntry"), "@list"),
        );

        Self { context: ctx }
    }
}

fn json_str(s: &str) -> serde_json::Value {
    serde_json::Value::String(s.to_owned())
}

fn json_typed(id: &str, ty: &str) -> serde_json::Value {
    serde_json::json!({ "@id": id, "@type": ty })
}

fn json_typed_container(id: &str, container: &str) -> serde_json::Value {
    serde_json::json!({ "@id": id, "@container": container })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_round_trips() {
        let ctx = LdContext::default();
        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: LdContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.context.len(), ctx.context.len());
    }

    #[test]
    fn context_has_required_terms() {
        let ctx = LdContext::default();
        for term in ["Commit", "sha", "parents", "timestamp", "changes", "path", "action"] {
            assert!(ctx.context.contains_key(term), "missing term: {term}");
        }
    }
}
