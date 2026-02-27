//! Stakeholder projection engine.
//!
//! Given an entity repo's access manifest (`.corp/access-manifest.json`),
//! produce a filtered view of the repo for each stakeholder. The projection
//! selects which files (and optionally which JSON fields) a stakeholder can see.
//!
//! Projected repos are read-only derived repos at:
//!   `{data_dir}/{workspace_id}/{entity_id}/{contact_id}.git`

use std::collections::HashMap;

use glob_match::glob_match;
use serde::{Deserialize, Serialize};

use crate::domain::ids::ContactId;

// ── Access Manifest ──────────────────────────────────────────────────

/// The top-level access manifest stored at `.corp/access-manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessManifest {
    stakeholders: HashMap<ContactId, StakeholderAccess>,
}

impl AccessManifest {
    /// Create an empty manifest.
    pub fn new() -> Self {
        Self {
            stakeholders: HashMap::new(),
        }
    }

    /// Add or replace a stakeholder's access rules.
    pub fn set_stakeholder(&mut self, contact_id: ContactId, access: StakeholderAccess) {
        self.stakeholders.insert(contact_id, access);
    }

    /// Remove a stakeholder's access.
    pub fn remove_stakeholder(&mut self, contact_id: &ContactId) -> bool {
        self.stakeholders.remove(contact_id).is_some()
    }

    /// Get a stakeholder's access rules.
    pub fn get_stakeholder(&self, contact_id: &ContactId) -> Option<&StakeholderAccess> {
        self.stakeholders.get(contact_id)
    }

    /// Iterate over all stakeholders.
    pub fn stakeholders(&self) -> impl Iterator<Item = (&ContactId, &StakeholderAccess)> {
        self.stakeholders.iter()
    }

    /// Number of stakeholders with access rules.
    pub fn stakeholder_count(&self) -> usize {
        self.stakeholders.len()
    }
}

impl Default for AccessManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Access configuration for a single stakeholder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeholderAccess {
    role: StakeholderRole,
    rules: Vec<AccessRule>,
}

impl StakeholderAccess {
    pub fn new(role: StakeholderRole, rules: Vec<AccessRule>) -> Self {
        Self { role, rules }
    }

    pub fn role(&self) -> StakeholderRole {
        self.role
    }

    pub fn rules(&self) -> &[AccessRule] {
        &self.rules
    }

    /// Check if a file path is visible to this stakeholder.
    pub fn can_see_path(&self, path: &str) -> bool {
        self.rules.iter().any(|rule| rule.matches_path(path))
    }

    /// Get the access level for a specific path.
    ///
    /// Returns the first matching rule's access level, or `None` if no rule matches.
    pub fn access_for_path(&self, path: &str) -> Option<&AccessLevel> {
        self.rules
            .iter()
            .find(|rule| rule.matches_path(path))
            .map(|rule| &rule.access)
    }
}

/// Role of the stakeholder in the company.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StakeholderRole {
    Founder,
    Investor,
    BoardMember,
    Officer,
    Employee,
    Advisor,
    Auditor,
    LegalCounsel,
}

/// A single access rule: a path pattern and what level of access is granted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRule {
    path: String,
    access: AccessLevel,
}

impl AccessRule {
    pub fn full(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            access: AccessLevel::Full,
        }
    }

    pub fn fields(path: impl Into<String>, fields: Vec<String>) -> Self {
        Self {
            path: path.into(),
            access: AccessLevel::Fields(fields),
        }
    }

    pub fn path_pattern(&self) -> &str {
        &self.path
    }

    pub fn access(&self) -> &AccessLevel {
        &self.access
    }

    /// Check if the given file path matches this rule's glob pattern.
    pub fn matches_path(&self, path: &str) -> bool {
        glob_match(&self.path, path)
    }
}

/// Level of access granted by a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    /// Full access to the file contents.
    Full,
    /// Access only to specific named fields in the JSON document.
    Fields(Vec<String>),
}

// ── Projection Logic ─────────────────────────────────────────────────

/// Determines which files from a set of paths are visible to a stakeholder,
/// and what access level they have.
pub fn compute_visible_files<'a>(
    access: &'a StakeholderAccess,
    all_paths: &'a [String],
) -> Vec<(&'a str, &'a AccessLevel)> {
    all_paths
        .iter()
        .filter_map(|path| {
            access
                .access_for_path(path)
                .map(|level| (path.as_str(), level))
        })
        .collect()
}

/// Apply field-level redaction to a JSON value.
///
/// If `allowed_fields` is provided, only those top-level keys are retained.
/// Nested objects are not recursively filtered — only top-level keys.
pub fn redact_json(
    value: &serde_json::Value,
    access: &AccessLevel,
) -> serde_json::Value {
    match access {
        AccessLevel::Full => value.clone(),
        AccessLevel::Fields(allowed) => {
            match value {
                serde_json::Value::Object(map) => {
                    let filtered: serde_json::Map<String, serde_json::Value> = map
                        .iter()
                        .filter(|(key, _)| allowed.iter().any(|a| a == key.as_str()))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    serde_json::Value::Object(filtered)
                }
                // Non-objects cannot be field-filtered; return null to avoid leaking.
                _ => serde_json::Value::Null,
            }
        }
    }
}

// ── Path Collection ──────────────────────────────────────────────────

use super::error::GitStorageError;
use super::repo::CorpRepo;

/// Recursively collect all file paths in a repo tree at the given ref.
pub fn collect_all_paths(
    repo: &CorpRepo,
    refname: &str,
) -> Result<Vec<String>, GitStorageError> {
    let mut paths = Vec::new();
    collect_paths_recursive(repo, refname, "", &mut paths)?;
    Ok(paths)
}

fn collect_paths_recursive(
    repo: &CorpRepo,
    refname: &str,
    prefix: &str,
    paths: &mut Vec<String>,
) -> Result<(), GitStorageError> {
    let entries = match repo.list_dir(refname, prefix) {
        Ok(entries) => entries,
        Err(GitStorageError::NotFound(_)) => return Ok(()),
        Err(e) => return Err(e),
    };

    for (name, is_dir) in entries {
        let full_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };

        if is_dir {
            collect_paths_recursive(repo, refname, &full_path, paths)?;
        } else {
            paths.push(full_path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_manifest_roundtrip() {
        let mut manifest = AccessManifest::new();
        let contact = ContactId::new();
        let access = StakeholderAccess::new(
            StakeholderRole::Investor,
            vec![
                AccessRule::full("cap-table/cap-table.json"),
                AccessRule::fields(
                    "corp.json",
                    vec![
                        "entity_id".to_owned(),
                        "legal_name".to_owned(),
                        "entity_type".to_owned(),
                    ],
                ),
                AccessRule::full("cap-table/grants/*.json"),
            ],
        );
        manifest.set_stakeholder(contact, access);

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: AccessManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.stakeholder_count(), 1);
        assert!(parsed.get_stakeholder(&contact).is_some());
    }

    #[test]
    fn glob_matching_exact_path() {
        let rule = AccessRule::full("cap-table/cap-table.json");
        assert!(rule.matches_path("cap-table/cap-table.json"));
        assert!(!rule.matches_path("cap-table/grants/abc.json"));
    }

    #[test]
    fn glob_matching_wildcard() {
        let rule = AccessRule::full("cap-table/grants/*.json");
        assert!(rule.matches_path("cap-table/grants/abc.json"));
        assert!(rule.matches_path("cap-table/grants/def.json"));
        assert!(!rule.matches_path("cap-table/cap-table.json"));
    }

    #[test]
    fn glob_matching_double_star() {
        let rule = AccessRule::full("governance/meetings/**/resolutions/*.json");
        assert!(rule.matches_path("governance/meetings/abc/resolutions/r1.json"));
        assert!(!rule.matches_path("governance/meetings/abc/votes/v1.json"));
    }

    #[test]
    fn stakeholder_can_see_path() {
        let access = StakeholderAccess::new(
            StakeholderRole::Investor,
            vec![
                AccessRule::full("cap-table/cap-table.json"),
                AccessRule::full("cap-table/grants/*.json"),
            ],
        );
        assert!(access.can_see_path("cap-table/cap-table.json"));
        assert!(access.can_see_path("cap-table/grants/g1.json"));
        assert!(!access.can_see_path("treasury/accounts/a1.json"));
    }

    #[test]
    fn compute_visible_files_filters_correctly() {
        let access = StakeholderAccess::new(
            StakeholderRole::Investor,
            vec![
                AccessRule::full("cap-table/cap-table.json"),
                AccessRule::fields(
                    "corp.json",
                    vec!["entity_id".to_owned(), "legal_name".to_owned()],
                ),
            ],
        );

        let all_paths = vec![
            "corp.json".to_owned(),
            "cap-table/cap-table.json".to_owned(),
            "treasury/chart-of-accounts.json".to_owned(),
            "governance/bodies/b1.json".to_owned(),
        ];

        let visible = compute_visible_files(&access, &all_paths);
        assert_eq!(visible.len(), 2);

        // corp.json should be field-filtered
        let (path, level) = &visible[0];
        assert_eq!(*path, "corp.json");
        assert!(matches!(level, AccessLevel::Fields(_)));

        // cap-table should be full access
        let (path, level) = &visible[1];
        assert_eq!(*path, "cap-table/cap-table.json");
        assert!(matches!(level, AccessLevel::Full));
    }

    #[test]
    fn redact_json_full_access() {
        let value = serde_json::json!({
            "entity_id": "abc",
            "legal_name": "Acme Corp",
            "ein": "12-3456789",
            "secret_field": "hidden"
        });

        let result = redact_json(&value, &AccessLevel::Full);
        assert_eq!(result, value);
    }

    #[test]
    fn redact_json_field_filtered() {
        let value = serde_json::json!({
            "entity_id": "abc",
            "legal_name": "Acme Corp",
            "ein": "12-3456789",
            "secret_field": "hidden"
        });

        let result = redact_json(
            &value,
            &AccessLevel::Fields(vec![
                "entity_id".to_owned(),
                "legal_name".to_owned(),
            ]),
        );

        let obj = result.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj["entity_id"], "abc");
        assert_eq!(obj["legal_name"], "Acme Corp");
        assert!(obj.get("ein").is_none());
        assert!(obj.get("secret_field").is_none());
    }

    #[test]
    fn redact_json_non_object_returns_null() {
        let value = serde_json::json!("just a string");
        let result = redact_json(
            &value,
            &AccessLevel::Fields(vec!["field".to_owned()]),
        );
        assert!(result.is_null());
    }

    #[test]
    fn stakeholder_roles_serde() {
        let roles = vec![
            StakeholderRole::Founder,
            StakeholderRole::Investor,
            StakeholderRole::BoardMember,
            StakeholderRole::Officer,
            StakeholderRole::Employee,
            StakeholderRole::Advisor,
            StakeholderRole::Auditor,
            StakeholderRole::LegalCounsel,
        ];
        for role in roles {
            let json = serde_json::to_string(&role).unwrap();
            let parsed: StakeholderRole = serde_json::from_str(&json).unwrap();
            assert_eq!(role, parsed);
        }
    }

    #[test]
    fn access_manifest_add_remove() {
        let mut manifest = AccessManifest::new();
        let c1 = ContactId::new();
        let c2 = ContactId::new();

        manifest.set_stakeholder(
            c1,
            StakeholderAccess::new(StakeholderRole::Investor, vec![]),
        );
        manifest.set_stakeholder(
            c2,
            StakeholderAccess::new(StakeholderRole::Employee, vec![]),
        );
        assert_eq!(manifest.stakeholder_count(), 2);

        assert!(manifest.remove_stakeholder(&c1));
        assert_eq!(manifest.stakeholder_count(), 1);
        assert!(manifest.get_stakeholder(&c1).is_none());
        assert!(manifest.get_stakeholder(&c2).is_some());

        // Remove again returns false
        assert!(!manifest.remove_stakeholder(&c1));
    }

    #[test]
    fn access_rule_accessors() {
        let rule = AccessRule::full("cap-table/*.json");
        assert_eq!(rule.path_pattern(), "cap-table/*.json");
        assert!(matches!(rule.access(), AccessLevel::Full));

        let rule2 = AccessRule::fields("corp.json", vec!["name".to_owned()]);
        assert_eq!(rule2.path_pattern(), "corp.json");
        assert!(matches!(rule2.access(), AccessLevel::Fields(_)));
    }
}
