use axum::http::HeaderMap;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize)]
pub struct Client {
    pub id: i64,
    pub name: String,
    pub is_revoked: bool,
}

impl Client {
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
    pub fn generate_secret() -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Extract credentials from request headers
    pub fn extract_credentials(headers: &HeaderMap) -> Option<(String, String)> {
        let name = headers
            .get("x-client-name")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        let secret = headers
            .get("x-client-secret")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        match (name, secret) {
            (Some(name), Some(secret)) => Some((name, secret)),
            _ => None,
        }
    }
}
