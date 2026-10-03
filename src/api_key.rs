use axum::http::HeaderMap;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ApiKey {
    pub id: i64,
    pub name: String,
    pub is_revoked: bool,
}

impl ApiKey {
    /// Create a new ApiKey instance
    pub fn new(id: i64, name: String) -> Self {
        Self {
            id,
            name,
            is_revoked: false,
        }
    }

    pub fn read((id, name, is_revoked): (i64, String, bool)) -> Self {
        Self {
            id,
            name,
            is_revoked,
        }
    }

    /// Generate a new 256-bit API key from the OS random number generator.
    ///
    /// Only its HMAC (see [`ApiKeyHmacKey`](crate::keys::ApiKeyHmacKey)) is stored.
    pub fn generate() -> anyhow::Result<String> {
        let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
        getrandom::fill(bytes.as_mut_slice())
            .map_err(|_| anyhow::anyhow!("OS random number generator failed"))?;
        Ok(URL_SAFE_NO_PAD.encode(bytes.as_slice()))
    }

    /// Extract credentials from request headers
    pub fn extract(headers: &HeaderMap) -> Option<String> {
        headers
            .get("x-api-key")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }
}
