use crate::api_key::ApiKey;
use crate::key_services::KeyServices;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use subtle::ConstantTimeEq;
use utoipa::ToSchema;

#[derive(Clone)]
pub struct AppState {
    pub key_services: Arc<KeyServices>,
    pub admin_api_key: Arc<SecretString>,
}

/// Constant-time comparison of a presented admin key (length is not hidden).
fn is_admin_key(presented: &str, expected: &SecretString) -> bool {
    presented
        .as_bytes()
        .ct_eq(expected.expose_secret().as_bytes())
        .into()
}

pub async fn admin_auth(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    if let Some(api_key) = ApiKey::extract(req.headers()) {
        if is_admin_key(&api_key, &state.admin_api_key) {
            // Proceed to the next middleware/handler
            next.run(req).await
        } else {
            (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: Invalid admin API key",
            )
                .into_response()
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: Missing admin API key",
        )
            .into_response()
    }
}

pub async fn api_key_auth(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    if let Some(api_key) = ApiKey::extract(req.headers()) {
        if let Ok(api_key) = state.key_services.validate(&api_key).await {
            // Proceed to the next middleware/handler
            req.extensions_mut().insert(api_key);
            next.run(req).await
        } else {
            (StatusCode::UNAUTHORIZED, "Unauthorized: Invalid API key").into_response()
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: Invalid or missing API key",
        )
            .into_response()
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct NewApiKeyResponse {
    pub api_key: String,
}

#[utoipa::path(
    post,
    path = "/admin/api-keys",
    params(
        ("X-API-KEY" = String, Header, description = "Admin API key for authentication")
    ),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 200, description = "API key created", body = NewApiKeyResponse)
    ),
    tag = "admin"
)]
pub async fn create_api_key(
    State(state): State<AppState>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Json<NewApiKeyResponse> {
    let api_key = state.key_services.create(&payload.name).await.unwrap();
    Json(NewApiKeyResponse { api_key })
}

#[utoipa::path(
    post,
    path = "/admin/api-keys/{id}/rotate",
    params(
        ("X-API-KEY" = String, Header, description = "Admin API key for authentication")
    ),
    responses(
        (status = 200, description = "API key rotated", body = NewApiKeyResponse)
    ),
    tag = "admin"
)]
pub async fn rotate_api_key(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Json<NewApiKeyResponse> {
    let new_api_key = state.key_services.rotate(id).await.unwrap();
    Json(NewApiKeyResponse {
        api_key: new_api_key,
    })
}

#[derive(Serialize, ToSchema)]
pub struct ListApiKeysResponse {
    pub api_keys: Vec<ApiKey>,
}

#[utoipa::path(
    get,
    path = "/admin/api-keys",
    params(
        ("X-API-KEY" = String, Header, description = "Admin API key for authentication")
    ),
    responses(
        (status = 200, description = "List of API keys", body = ListApiKeysResponse)
    ),
    tag = "admin"
)]
pub async fn list_api_keys(State(state): State<AppState>) -> Json<ListApiKeysResponse> {
    let api_keys = state.key_services.list().await.unwrap(); // Replace with actual fetching logic
    Json(ListApiKeysResponse { api_keys })
}

#[derive(Deserialize, ToSchema)]
pub struct RevokeApiKeyRequest {
    pub api_key: String,
}

#[utoipa::path(
    post,
    path = "/admin/api-keys/{id}/revoke",
    params(
        ("X-API-KEY" = String, Header, description = "Admin API key for authentication")
    ),
    responses(
        (status = 200, description = "API key revoked")
    ),
    tag = "admin"
)]
pub async fn revoke_api_key(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    state.key_services.revoke(id).await.unwrap();
    (StatusCode::OK, "API key revoked").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    use crate::keys::{Keyring, MasterKey};

    fn hmac_keys() -> Keyring {
        Keyring::new(&MasterKey::from_bytes([3; 32]))
    }

    #[test]
    fn test_generate_api_key_length() {
        let key = ApiKey::generate().unwrap();
        assert_eq!(key.len(), 43);
    }

    #[test]
    fn test_generate_api_key_uniqueness() {
        let key1 = ApiKey::generate().unwrap();
        let key2 = ApiKey::generate().unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_hash_api_key() {
        let key = "test_key_12345";
        let hash = hmac_keys().api_keys.hmac.hash(key);
        assert_ne!(hash, key);
        assert_eq!(hash.len(), 64); // HMAC-SHA256 produces 64 hex characters
    }

    #[test]
    fn test_hash_api_key_deterministic() {
        let key = "test_key_12345";
        assert_eq!(
            hmac_keys().api_keys.hmac.hash(key),
            hmac_keys().api_keys.hmac.hash(key)
        );
    }

    #[test]
    fn test_admin_key_comparison() {
        let expected = SecretString::from("correct-admin-key");
        assert!(is_admin_key("correct-admin-key", &expected));
        assert!(!is_admin_key("correct-admin-kez", &expected));
        assert!(!is_admin_key("correct-admin-key-longer", &expected));
        assert!(!is_admin_key("", &expected));
    }

    #[test]
    fn test_extract_api_key_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "test_key_value".parse().unwrap());

        let key = ApiKey::extract(&headers);
        assert_eq!(key, Some("test_key_value".to_string()));
    }

    #[test]
    fn test_extract_api_key_missing() {
        let headers = HeaderMap::new();
        let key = ApiKey::extract(&headers);
        assert_eq!(key, None);
    }
}
