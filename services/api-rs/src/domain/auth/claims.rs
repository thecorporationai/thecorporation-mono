//! JWT claims model — encode and decode workspace-scoped tokens.

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use super::error::AuthError;
use super::scopes::Scope;
use crate::domain::ids::{ContactId, EntityId, WorkspaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalType {
    #[default]
    User,
    InternalWorker,
    Agent,
}

/// JWT claims payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the workspace ID as a string.
    sub: String,
    /// The workspace this token is scoped to.
    workspace_id: WorkspaceId,
    /// Optional entity ID (if the token represents a specific officer/member).
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_id: Option<EntityId>,
    /// Optional contact ID for contact-scoped keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    contact_id: Option<ContactId>,
    /// Optional entity scope for entity-scoped keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_ids: Option<Vec<EntityId>>,
    /// The type of principal represented by this token.
    #[serde(default)]
    principal_type: PrincipalType,
    /// Granted scopes.
    scopes: Vec<Scope>,
    /// Issued-at (Unix timestamp).
    iat: i64,
    /// Expiry (Unix timestamp).
    exp: i64,
}

impl Claims {
    /// Create a new claims payload.
    pub fn new(
        workspace_id: WorkspaceId,
        entity_id: Option<EntityId>,
        contact_id: Option<ContactId>,
        entity_ids: Option<Vec<EntityId>>,
        principal_type: PrincipalType,
        scopes: Vec<Scope>,
        iat: i64,
        exp: i64,
    ) -> Self {
        Self {
            sub: workspace_id.to_string(),
            workspace_id,
            entity_id,
            contact_id,
            entity_ids,
            principal_type,
            scopes,
            iat,
            exp,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn sub(&self) -> &str {
        &self.sub
    }

    pub fn workspace_id(&self) -> WorkspaceId {
        self.workspace_id
    }

    pub fn entity_id(&self) -> Option<EntityId> {
        self.entity_id
    }

    pub fn contact_id(&self) -> Option<ContactId> {
        self.contact_id
    }

    pub fn entity_ids(&self) -> Option<&[EntityId]> {
        self.entity_ids.as_deref()
    }

    pub fn principal_type(&self) -> PrincipalType {
        self.principal_type
    }

    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    pub fn iat(&self) -> i64 {
        self.iat
    }

    pub fn exp(&self) -> i64 {
        self.exp
    }
}

/// Encode claims into a signed JWT (HS256).
pub fn encode_token(claims: &Claims, secret: &[u8]) -> Result<String, AuthError> {
    let key = EncodingKey::from_secret(secret);
    encode(&Header::default(), claims, &key)
        .map_err(|e| AuthError::InvalidToken(format!("encoding failed: {e}")))
}

/// Decode and validate a JWT, returning the claims.
///
/// Rejects expired tokens automatically (the `exp` field is checked by `jsonwebtoken`).
pub fn decode_token(token: &str, secret: &[u8]) -> Result<Claims, AuthError> {
    let key = DecodingKey::from_secret(secret);
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["exp", "iat", "sub"]);

    let data = decode::<Claims>(token, &key, &validation).map_err(|e| {
        use jsonwebtoken::errors::ErrorKind;
        match e.kind() {
            ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken(e.to_string()),
        }
    })?;

    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_secret() -> Vec<u8> {
        b"test-secret-key-for-unit-tests-only".to_vec()
    }

    #[test]
    fn claims_serde_roundtrip() {
        let ws = WorkspaceId::new();
        let entity = EntityId::new();
        let claims = Claims::new(
            ws,
            Some(entity),
            None,
            None,
            PrincipalType::User,
            vec![Scope::FormationCreate, Scope::EquityRead],
            1000,
            2000,
        );
        let json = serde_json::to_string(&claims).expect("serialize Claims");
        let parsed: Claims = serde_json::from_str(&json).expect("deserialize Claims");
        assert_eq!(parsed.workspace_id(), ws);
        assert_eq!(parsed.entity_id(), Some(entity));
        assert_eq!(parsed.contact_id(), None);
        assert_eq!(parsed.entity_ids(), None);
        assert_eq!(parsed.principal_type(), PrincipalType::User);
        assert_eq!(parsed.scopes().len(), 2);
        assert_eq!(parsed.iat(), 1000);
        assert_eq!(parsed.exp(), 2000);
    }

    #[test]
    fn claims_serde_without_entity() {
        let ws = WorkspaceId::new();
        let claims = Claims::new(
            ws,
            None,
            None,
            None,
            PrincipalType::User,
            vec![Scope::All],
            1000,
            2000,
        );
        let json = serde_json::to_string(&claims).expect("serialize Claims");
        // entity_id should not appear in JSON
        assert!(!json.contains("entity_id"));
        let parsed: Claims = serde_json::from_str(&json).expect("deserialize Claims");
        assert_eq!(parsed.entity_id(), None);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let secret = test_secret();
        let ws = WorkspaceId::new();
        let now = Utc::now().timestamp();
        let claims = Claims::new(
            ws,
            None,
            None,
            None,
            PrincipalType::User,
            vec![Scope::Admin],
            now,
            now + 3600,
        );

        let token = encode_token(&claims, &secret).expect("encode token");
        assert!(!token.is_empty());

        let decoded = decode_token(&token, &secret).expect("decode token");
        assert_eq!(decoded.workspace_id(), ws);
        assert_eq!(decoded.scopes(), &[Scope::Admin]);
    }

    #[test]
    fn decode_with_wrong_secret_fails() {
        let secret = test_secret();
        let ws = WorkspaceId::new();
        let now = Utc::now().timestamp();
        let claims = Claims::new(
            ws,
            None,
            None,
            None,
            PrincipalType::User,
            vec![Scope::All],
            now,
            now + 3600,
        );

        let token = encode_token(&claims, &secret).expect("encode token");

        let wrong_secret = b"wrong-secret";
        let result = decode_token(&token, wrong_secret);
        assert!(result.is_err());
    }

    #[test]
    fn decode_expired_token_returns_token_expired() {
        let secret = test_secret();
        let ws = WorkspaceId::new();
        let past = Utc::now().timestamp() - 7200;
        let claims = Claims::new(
            ws,
            None,
            None,
            None,
            PrincipalType::User,
            vec![Scope::All],
            past - 3600,
            past,
        );

        let token = encode_token(&claims, &secret).expect("encode token");
        let result = decode_token(&token, &secret);
        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[test]
    fn sub_matches_workspace_id() {
        let ws = WorkspaceId::new();
        let claims = Claims::new(ws, None, None, None, PrincipalType::User, vec![], 100, 200);
        assert_eq!(claims.sub(), ws.to_string());
    }
}
