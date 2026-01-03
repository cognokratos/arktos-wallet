use axum::http::HeaderMap;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::Serialize;
use sha2::Sha256;

#[derive(Clone, Debug, Serialize)]
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

    /// Generate a cryptographically secure random secret
    pub fn generate() -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Hash the API key using SHA-256
    pub fn hash(api_key: &str, secret: &str) -> String {
        let mut mac: Hmac<Sha256> =
            Hmac::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(api_key.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Extract credentials from request headers
    pub fn extract(headers: &HeaderMap) -> Option<String> {
        headers
            .get("x-api-key")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }
}
