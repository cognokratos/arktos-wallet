use axum::http::HeaderMap;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a cryptographically secure random API key
pub fn generate_api_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             abcdefghijklmnopqrstuvwxyz\
                             0123456789\
                             -_";
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Hash an API key using SHA256
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Extract API key from request headers
pub fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key_length() {
        let key = generate_api_key();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_generate_api_key_uniqueness() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_hash_api_key() {
        let key = "test_key_12345";
        let hash = hash_api_key(key);
        assert!(!hash.is_empty());
        assert_ne!(hash, key);
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
    }

    #[test]
    fn test_hash_api_key_deterministic() {
        let key = "test_key_12345";
        let hash1 = hash_api_key(key);
        let hash2 = hash_api_key(key);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_extract_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "test_key_value".parse().unwrap());

        let key = extract_api_key(&headers);
        assert_eq!(key, Some("test_key_value".to_string()));
    }

    #[test]
    fn test_extract_api_key_missing() {
        let headers = HeaderMap::new();
        let key = extract_api_key(&headers);
        assert_eq!(key, None);
    }
}
