use arktos_wallet::app::router;
use arktos_wallet::auth::AppState;
use arktos_wallet::config::Config;
use arktos_wallet::key_services::KeyServices;
use arktos_wallet::keys::Keyring;
use arktos_wallet::{database::Database, wallet_services::WalletServices};
use secrecy::ExposeSecret;
use std::{net::SocketAddr, sync::Arc};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;

    // SQLCipher uses DATABASE_KEY; field encryption and API-key hashing use
    // subkeys derived from MASTER_KEY.
    let db = Arc::new(Database::new(
        &config.db_path,
        config.db_key.expose_secret(),
    )?);
    let keyring = Keyring::new(&config.master_key);
    let wallet_services = Arc::new(WalletServices::new(db.clone(), keyring.wallet));
    let key_services = Arc::new(KeyServices::new(db.clone(), keyring.api_keys));
    let app_state = AppState {
        key_services,
        admin_api_key: Arc::new(config.admin_key),
    };

    // Cancelled on shutdown so in-flight MCP streaming responses terminate.
    let shutdown = CancellationToken::new();
    tracing::info!(allowed_hosts = ?config.mcp_allowed_hosts, "MCP Host allowlist");
    let app = router(
        app_state,
        wallet_services,
        config.mcp_allowed_hosts,
        shutdown.clone(),
    );

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    tracing::info!("Listening on http://{addr}");
    tracing::info!("MCP endpoint (MCP 2026-07-28, stateless): http://{addr}/mcp");
    tracing::info!("Health check: http://{addr}/healthz");
    tracing::info!("Swagger UI: http://{addr}/swagger-ui");
    tracing::info!("OpenAPI Spec: http://{addr}/openapi.json");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown))
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}

/// Resolve on Ctrl+C (SIGINT) or SIGTERM, then cancel in-flight MCP streams.
async fn shutdown_signal(shutdown: CancellationToken) {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to listen for Ctrl+C");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("Shutdown signal received, draining connections");
    shutdown.cancel();
}
