//! HTTP application wiring shared by the binary and integration tests.
//!
//! ```text
//! /healthz   → health check (no auth)
//! /admin/*   → REST admin API (admin API key)
//! /mcp       → stateless MCP 2026-07-28 endpoint (client API key)
//! ```

use crate::api_info::{api_doc, health};
use crate::auth::{
    AppState, admin_auth, api_key_auth, create_api_key, list_api_keys, revoke_api_key,
    rotate_api_key,
};
use crate::mcp::mcp_service;
use crate::wallet_services::WalletServices;
use axum::Router;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Build the full Arktos HTTP router.
///
/// `mcp_allowed_hosts` restricts the `Host` header accepted on `/mcp`, and
/// cancelling `shutdown` ends in-flight MCP streams during graceful shutdown.
pub fn router(
    app_state: AppState,
    wallet_services: Arc<WalletServices>,
    mcp_allowed_hosts: Vec<String>,
    shutdown: CancellationToken,
) -> Router {
    let admin_routes = Router::new()
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/{id}/revoke", post(revoke_api_key))
        .route("/api-keys/{id}/rotate", post(rotate_api_key))
        .layer(from_fn_with_state(app_state.clone(), admin_auth));

    let mcp_routes = Router::new()
        .nest_service(
            "/mcp",
            mcp_service(wallet_services, mcp_allowed_hosts, shutdown),
        )
        .layer(from_fn_with_state(app_state.clone(), api_key_auth));

    Router::new()
        .route("/healthz", get(health))
        .nest("/admin", admin_routes)
        .merge(mcp_routes)
        .with_state(app_state)
        .merge(api_doc())
}
