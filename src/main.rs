use arktos_wallet::api_info::{api_doc, health};
use arktos_wallet::auth::{
    AppState, admin_auth, api_key_auth, create_api_key, list_api_keys, revoke_api_key,
    rotate_api_key,
};
use arktos_wallet::config::Config;
use arktos_wallet::key_services::KeyServices;
use arktos_wallet::mcp::McpServer;
use arktos_wallet::{database::Database, wallet_services::WalletServices};
use axum::middleware::from_fn_with_state;
use axum::routing::post;
use axum::{Router, routing::get};
use rmcp::{
    transport::StreamableHttpServerConfig,
    transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    },
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Initialize database with SQLCipher encryption
    let config = Config::from_env();
    let secret_key = config.secret_key;

    let db = Arc::new(Database::new(&config.db_path, &config.db_key)?);
    let wallet_services = Arc::new(WalletServices::new(db.clone(), secret_key.clone()));
    let key_services = Arc::new(KeyServices::new(db.clone(), secret_key));
    let app_state = AppState {
        key_services,
        admin_api_key: config.admin_key,
    };

    // Cancellation token shared with MCP transport for graceful shutdown.
    let ct = CancellationToken::new();

    let mcp_service = StreamableHttpService::new(
        {
            let services = wallet_services.clone();
            move || Ok(McpServer::new(services.clone()))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig {
            cancellation_token: ct.child_token(),
            sse_keep_alive: Some(Duration::from_secs(15)),
            stateful_mode: true,
        },
    );

    let admin_routes = Router::new()
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/{id}/revoke", post(revoke_api_key))
        .route("/api-keys/{id}/rotate", post(rotate_api_key))
        .layer(from_fn_with_state(app_state.clone(), admin_auth));

    let mcp_routes = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(from_fn_with_state(app_state.clone(), api_key_auth));

    let app = Router::new()
        .route("/healthz", get(health))
        .nest("/admin", admin_routes)
        .merge(mcp_routes)
        .with_state(app_state)
        .merge(api_doc());

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    tracing::info!("Listening on http://{addr}");
    tracing::info!("MCP endpoint: http://{addr}/mcp");
    tracing::info!("Health check: http://{addr}/healthz");
    tracing::info!("Swagger UI: http://{addr}/swagger-ui");
    tracing::info!("OpenAPI Spec: http://{addr}/openapi.json");

    // Axum server with graceful shutdown on Ctrl+C
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(ct.clone()))
        .await?;

    Ok(())
}

async fn shutdown_signal(ct: CancellationToken) {
    let _ = tokio::signal::ctrl_c().await;
    ct.cancel();
}
