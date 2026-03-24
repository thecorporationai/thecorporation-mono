//! JWT token encoding and decoding.
//!
//! Uses HMAC-SHA256 (HS256) with configurable expiry.  All expiry validation
//! is handled here so callers never receive an un-checked `Claims` value.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};

use corp_core::auth::Claims;

use crate::error::AuthError;

const DEFAULT_EXPIRY_SECS: i64 = 86_400; // 24 h

/// Configuration for JWT encoding and decoding.
///
/// Construct with [`JwtConfig::new`], then call [`JwtConfig::encode`] /
/// [`JwtConfig::decode`] as needed.  The config is cheaply cloneable and is
/// designed to be held in your application state.
#[derive(Clone)]
pub struct JwtConfig {
    secret: Vec<u8>,
    default_expiry_secs: i64,
}

impl JwtConfig {
    /// Create a new `JwtConfig` with a 24-hour default token lifetime.
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
            default_expiry_secs: DEFAULT_EXPIRY_SECS,
        }
    }

    /// Override the default expiry duration (in seconds).
    pub fn with_default_expiry(mut self, secs: i64) -> Self {
        self.default_expiry_secs = secs;
        self
    }

    /// Encode `claims` using the default expiry.
    ///
    /// The `claims.exp` field is **replaced** with the computed deadline so
    /// callers do not need to set it manually; `claims.iat` is used as-is and
    /// should be set to the current Unix timestamp before calling this method.
    pub fn encode(&self, claims: &Claims) -> Result<String, AuthError> {
        self.encode_with_expiry(claims, self.default_expiry_secs)
    }

    /// Encode `claims` with an explicit expiry duration.
    ///
    /// `expiry_secs` is added to `claims.iat` to produce the `exp` field.
    pub fn encode_with_expiry(
        &self,
        claims: &Claims,
        expiry_secs: i64,
    ) -> Result<String, AuthError> {
        // Build a mutable copy so we can override exp.
        let mut payload = claims.clone();
        payload.exp = claims.iat + expiry_secs;

        jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &payload,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| AuthError::InternalError(format!("jwt encode: {e}")))
    }

    /// Decode and validate `token`.
    ///
    /// Returns an [`AuthError::ExpiredToken`] when the token has passed its
    /// `exp` claim, and [`AuthError::InvalidToken`] for any other failure
    /// (bad signature, malformed structure, etc.).
    pub fn decode(&self, token: &str) -> Result<Claims, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        // We rely on the library to check exp; ensure required_spec_claims has exp.
        validation.validate_exp = true;
        // No audience / issuer checks at this layer — callers can add them.
        validation.set_required_spec_claims(&["exp", "sub"]);

        jsonwebtoken::decode::<Claims>(token, &DecodingKey::from_secret(&self.secret), &validation)
            .map(|data| data.claims)
            .map_err(|e| {
                use jsonwebtoken::errors::ErrorKind;
                match e.kind() {
                    ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
                    _ => AuthError::InvalidToken,
                }
            })
    }
}

impl std::fmt::Debug for JwtConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtConfig")
            .field("default_expiry_secs", &self.default_expiry_secs)
            .field("secret", &"<redacted>")
            .finish()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use corp_core::auth::{PrincipalType, Scope};
    use corp_core::ids::WorkspaceId;

    /// Return the current Unix timestamp in seconds.
    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn make_claims(iat: i64, exp: i64) -> Claims {
        Claims {
            sub: "test-user".to_owned(),
            workspace_id: WorkspaceId::new(),
            entity_id: None,
            contact_id: None,
            entity_ids: None,
            principal_type: PrincipalType::User,
            scopes: vec![Scope::FormationRead],
            iat,
            exp,
        }
    }

    #[test]
    fn roundtrip_encode_decode() {
        let cfg = JwtConfig::new(b"super-secret-key-for-tests");
        let t = now();
        let claims = make_claims(t, t + 3600);

        let token = cfg.encode(&claims).unwrap();
        let decoded = cfg.decode(&token).unwrap();

        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.workspace_id, claims.workspace_id);
        assert_eq!(decoded.scopes, claims.scopes);
    }

    #[test]
    fn expired_token_returns_expired_error() {
        let cfg = JwtConfig::new(b"secret");
        // iat and exp both in the distant past.
        let claims = make_claims(1_000, 1_001);
        let token = cfg.encode_with_expiry(&claims, 1).unwrap();
        match cfg.decode(&token) {
            Err(AuthError::ExpiredToken) => {}
            other => panic!("expected ExpiredToken, got {:?}", other),
        }
    }

    #[test]
    fn invalid_signature_returns_invalid_token() {
        let cfg1 = JwtConfig::new(b"key-one");
        let cfg2 = JwtConfig::new(b"key-two");

        let t = now();
        let claims = make_claims(t, t + 3600);
        let token = cfg1.encode(&claims).unwrap();

        match cfg2.decode(&token) {
            Err(AuthError::InvalidToken) => {}
            other => panic!("expected InvalidToken, got {:?}", other),
        }
    }

    #[test]
    fn encode_with_expiry_overrides_exp() {
        let cfg = JwtConfig::new(b"secret");
        let t = now();
        let mut claims = make_claims(t, 0); // exp = 0 will be overridden
        claims.iat = t;
        let token = cfg.encode_with_expiry(&claims, 7200).unwrap();
        let decoded = cfg.decode(&token).unwrap();
        assert_eq!(decoded.exp, t + 7200);
    }
}
