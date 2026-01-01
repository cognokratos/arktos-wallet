use arktos_wallet::{api_handlers::ApiHandlers, db::Database};
use axum::{Router, routing::get};
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::StreamableHttpServerConfig,
    transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    },
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct App {
    tool_router: ToolRouter<App>,
    _handlers: Arc<ApiHandlers>,
}

#[tool_router]
impl App {
    pub fn new(handlers: Arc<ApiHandlers>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            _handlers: handlers,
        }
    }

    #[tool(name = "ping", description = "Return a simple liveness response.")]
    async fn ping(&self) -> Result<String, rmcp::ErrorData> {
        Ok("pong".to_string())
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
    let cipher_key =
        std::env::var("DATABASE_CIPHER_KEY").unwrap_or_else(|_| "default_cipher_key".to_string());
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "arktos.db".to_string());

    let db = Arc::new(Database::new(&db_path, &cipher_key)?);
    let handlers = Arc::new(ApiHandlers::new(db, cipher_key));

    // Cancellation token shared with MCP transport for graceful shutdown.
    let ct = CancellationToken::new();

    let mcp_service = StreamableHttpService::new(
        {
            let handlers = handlers.clone();
            move || Ok(App::new(handlers.clone()))
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
