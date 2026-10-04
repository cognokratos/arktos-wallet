use arktos_wallet::app::router;
use arktos_wallet::auth::AppState;
use arktos_wallet::config::{Config, DatabaseConfig};
use arktos_wallet::key_services::KeyServices;
use arktos_wallet::keys::Keyring;
use arktos_wallet::{database::Database, wallet_services::WalletServices};
use clap::{Parser, Subcommand};
use secrecy::{ExposeSecret, SecretString};
use std::{net::SocketAddr, sync::Arc};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "arktos-wallet", version, about = "Arktos Wallet MCP server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP/MCP server (default)
    Serve,
    /// Open the database, apply pending migrations and exit
    Migrate,
    /// Print database diagnostics (no secrets) and exit
    DbInfo,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    match Cli::parse().command.unwrap_or(Command::Serve) {
        Command::Serve => serve().await,
        Command::Migrate => {
            let config = DatabaseConfig::from_env()?;
            let db = open_database(config.db_path.clone(), config.db_key).await?;
            let info = db.info().await?;
            println!(
                "{}: schema version {} (latest {})",
                config.db_path, info.schema_version, info.latest_schema_version
            );
            Ok(())
        }
        Command::DbInfo => {
            let config = DatabaseConfig::from_env()?;
            let info = open_database(config.db_path.clone(), config.db_key)
                .await?
                .info()
                .await?;
            println!("path:            {}", config.db_path);
            println!("sqlite version:  {}", info.sqlite_version);
            println!("sqlcipher:       {}", info.cipher_version);
            println!(
                "schema version:  {} (latest {})",
                info.schema_version, info.latest_schema_version
            );
            println!("journal mode:    {}", info.journal_mode);
            println!("synchronous:     {}", info.synchronous);
            println!("foreign keys:    {}", info.foreign_keys);
            println!("busy timeout:    {} ms", info.busy_timeout_ms);
            Ok(())
        }
    }
}

/// Open, configure and migrate the database off the async worker threads.
async fn open_database(path: String, key: SecretString) -> anyhow::Result<Arc<Database>> {
    let db =
        tokio::task::spawn_blocking(move || Database::new(&path, key.expose_secret())).await??;
    Ok(Arc::new(db))
}

async fn serve() -> anyhow::Result<()> {
    let config = Config::from_env()?;

    // SQLCipher uses DATABASE_KEY; field encryption and API-key hashing use
    // subkeys derived from MASTER_KEY. Migrations run before serving.
    let db = open_database(config.db_path.clone(), config.db_key).await?;
    let info = db.info().await?;
    tracing::info!(
        path = %config.db_path,
        schema_version = info.schema_version,
        sqlcipher = %info.cipher_version,
        journal_mode = %info.journal_mode,
        "Database ready"
    );
    let keyring = Keyring::new(&config.master_key);
    tracing::info!(
        bitcoin_network = %config.chains.bitcoin_network,
        ethereum_chain_id = config.chains.ethereum_chain_id.get(),
        "Chain configuration"
    );
    let wallet_services = Arc::new(WalletServices::new(
        db.clone(),
        keyring.wallet,
        config.chains,
    ));
    let key_services = Arc::new(KeyServices::new(db.clone(), keyring.api_keys));
    let app_state = AppState {
        key_services,
        admin_api_key: Arc::new(config.admin_key),
        database: db,
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
