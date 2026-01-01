use arktos_wallet::config::Config;
use arktos_wallet::services::CreateWalletRequest;
use arktos_wallet::{db::Database, services::Services};
use axum::{Router, routing::get};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::StreamableHttpServerConfig,
    transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    },
};
use schemars::_private::NoSerialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct App {
    tool_router: ToolRouter<App>,
    services: Arc<Services>,
}

#[tool_router]
impl App {
    pub fn new(services: Arc<Services>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            services,
        }
    }

    #[tool(name = "ping", description = "Return a simple liveness response.")]
    async fn ping(&self) -> Result<String, ErrorData> {
        Ok("pong".to_string())
    }

    #[tool(
        name = "create_wallet",
        description = "Create a new wallet with encrypted recovery passphrase."
    )]
    async fn create_wallet(
        &self,
        Parameters(req): Parameters<CreateWalletRequest>,
    ) -> Result<String, ErrorData> {
        self.services
            .create_wallet(req)
            .await
            .map(|r| r.to_string())
            .map_err(|e| {
                ErrorData::internal_error(
                    format!("Failed to create wallet: {}", e),
                    e.maybe_to_value(),
                )
            })
    }
}

#[tool_handler]
impl ServerHandler for App {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Arktos MCP server over Streamable HTTP".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Initialize database with SQLCipher encryption
    let config = Config::from_env();
    let cipher_key = config.cipher_key;
    let db_path = config.db_path;

    let db = Arc::new(Database::new(&db_path, &cipher_key)?);
    let services = Arc::new(Services::new(db, cipher_key));

    // Cancellation token shared with MCP transport for graceful shutdown.
    let ct = CancellationToken::new();

    let mcp_service = StreamableHttpService::new(
        {
            let services = services.clone();
            move || Ok(App::new(services.clone()))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig {
            cancellation_token: ct.child_token(),
            sse_keep_alive: Some(Duration::from_secs(15)),
            stateful_mode: true,
        },
    );

    let app = Router::new()
        .route("/healthz", get(|| async { "OK" }))
        .nest_service("/mcp", mcp_service);

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    tracing::info!("Listening on http://{addr}");
    tracing::info!("MCP endpoint: http://{addr}/mcp");

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
