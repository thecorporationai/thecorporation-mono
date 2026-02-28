//! Opaque token management for secrets.
//!
//! Containers never see real secret values. Instead, the worker creates
//! opaque tokens stored in Redis, and the container resolves them via
//! the secrets proxy at runtime.

use std::collections::HashMap;

use agent_types::ExecutionId;
use deadpool_redis::Pool;
use deadpool_redis::redis::AsyncCommands;

use crate::error::WorkerError;
use crate::redis::keys;

/// Create opaque tokens for an execution's secrets, stored in Redis.
///
/// Returns a map of `secret_name -> opaque_token`.
pub async fn create_token_map(
    pool: &Pool,
    execution_id: ExecutionId,
    secrets: &HashMap<String, String>,
) -> Result<HashMap<String, String>, WorkerError> {
    if secrets.is_empty() {
        return Ok(HashMap::new());
    }

    let mut conn = pool.get().await?;
    let tokens_key = keys::tokens(execution_id);
    let mut name_to_token = HashMap::new();

    for (name, value) in secrets {
        let token = generate_opaque_token();
        conn.hset::<_, _, _, ()>(&tokens_key, &token, value).await?;
        conn.set_ex::<_, _, ()>(&keys::token_reverse(&token), execution_id.to_string(), 3600).await?;
        name_to_token.insert(name.clone(), token);
    }

    Ok(name_to_token)
}

/// Resolve an opaque token to its real value.
pub async fn resolve_token(
    pool: &Pool,
    execution_id: ExecutionId,
    token: &str,
) -> Result<Option<String>, WorkerError> {
    let mut conn = pool.get().await?;
    let tokens_key = keys::tokens(execution_id);
    let value: Option<String> = conn.hget(&tokens_key, token).await?;
    Ok(value)
}

/// Revoke all tokens for an execution atomically using a Redis pipeline.
pub async fn revoke_tokens(pool: &Pool, execution_id: ExecutionId) -> Result<(), WorkerError> {
    let mut conn = pool.get().await?;
    let tokens_key = keys::tokens(execution_id);

    // Get all tokens to clean up reverse lookups
    let tokens: HashMap<String, String> = conn.hgetall(&tokens_key).await?;
    if tokens.is_empty() {
        return Ok(());
    }

    // Delete all reverse-lookup keys and the token hash in a single pipeline
    let mut pipe = deadpool_redis::redis::pipe();
    for token in tokens.keys() {
        pipe.del(keys::token_reverse(token)).ignore();
    }
    pipe.del(&tokens_key).ignore();
    pipe.query_async::<()>(&mut *conn).await?;

    Ok(())
}

/// Replace `{secret_name}` placeholders in a JSON string with opaque tokens.
pub fn rewrite_secret_refs(config_json: &str, name_to_token: &HashMap<String, String>) -> String {
    let mut result = config_json.to_owned();
    for (name, token) in name_to_token {
        result = result.replace(&format!("{{{name}}}"), token);
    }
    result
}

/// Generate a random opaque token.
fn generate_opaque_token() -> String {
    format!("tok_{}", hex::encode(rand::random::<[u8; 16]>()))
}
