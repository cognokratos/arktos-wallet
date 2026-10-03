//! MCP adapter: exposes [`WalletServices`] as MCP tools over the stateless
//! Streamable HTTP transport of MCP `2026-07-28`.
//!
//! Every HTTP request is served independently: there is no MCP session, no
//! `initialize` handshake and no session ID. The caller identity is the
//! [`ApiKey`] resolved by the HTTP authentication middleware, which `rmcp`
//! forwards to tools through the request's [`Parts`].

use crate::api_key::ApiKey;
use crate::error::AppError;
use crate::wallet_services::{
    CreateWalletRequest, GetBitcoinAddressRequest, GetEthereumAddressRequest, WalletServices,
};
use http::request::Parts;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerConfig};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use std::borrow::Cow;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// The only MCP protocol revision Arktos implements.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V_2026_07_28;

const SUPPORTED_PROTOCOL_VERSIONS: &[ProtocolVersion] = &[PROTOCOL_VERSION];

/// HTTP service for the `/mcp` endpoint.
pub type McpService = StreamableHttpService<McpServer, NeverSessionManager>;

/// Build the stateless MCP HTTP service.
///
/// * `allowed_hosts` — accepted `Host` header values (DNS-rebinding protection).
/// * `shutdown` — cancelled on server shutdown to end in-flight SSE responses.
pub fn mcp_service(
    services: Arc<WalletServices>,
    allowed_hosts: Vec<String>,
    shutdown: CancellationToken,
) -> McpService {
    let config = StreamableHttpServerConfig::default()
        // No sessions: requests negotiating a legacy (< 2026-07-28) revision
        // are not served through a session either.
        .with_legacy_session_mode(false)
        // Require the per-request protocol header and `_meta` that make each
        // 2026-07-28 request self-contained.
        .with_stateless_protocol_metadata_required(true)
        // Plain JSON for request/response tools; rmcp still falls back to SSE
        // when a handler streams intermediate messages.
        .with_json_response(true)
        .with_allowed_hosts(allowed_hosts)
        .with_cancellation_token(shutdown);

    StreamableHttpService::new(
        move || Ok(McpServer::new(services.clone())),
        Arc::new(NeverSessionManager::default()),
        config,
    )
}

/// Tool definitions come from the `#[tool]` attributes below, so `tools/list`
/// is identical for every request.
#[derive(Clone)]
pub struct McpServer {
    services: Arc<WalletServices>,
}

/// Resolve the authenticated caller attached by the API-key middleware.
fn caller(parts: &Parts) -> Result<&ApiKey, ErrorData> {
    parts
        .extensions
        .get::<ApiKey>()
        .ok_or_else(|| ErrorData::invalid_request("Missing API Key".to_string(), None))
}

/// Map a wallet-service error onto an MCP error, keeping the serialized
/// [`AppError`] as error data.
fn service_error(action: &str, error: AppError) -> ErrorData {
    tracing::warn!(%error, "{action} failed");
    ErrorData::internal_error(
        format!("Failed to {action}: {error}"),
        serde_json::to_value(&error).ok(),
    )
}

#[tool_router]
impl McpServer {
    pub fn new(services: Arc<WalletServices>) -> Self {
        Self { services }
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
        Extension(parts): Extension<Parts>,
        Parameters(req): Parameters<CreateWalletRequest>,
    ) -> Result<String, ErrorData> {
        let api_key = caller(&parts)?;
        tracing::info!(
            tool = "create_wallet",
            api_key_id = api_key.id,
            "MCP tool call"
        );
        self.services
            .create_wallet(api_key, req)
            .await
            .map(|r| r.to_string())
            .map_err(|e| service_error("create wallet", e))
    }

    #[tool(
        name = "get_bitcoin_address",
        description = "Derive and retrieve a Bitcoin public address for a specified wallet ID."
    )]
    async fn get_bitcoin_address(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(req): Parameters<GetBitcoinAddressRequest>,
    ) -> Result<String, ErrorData> {
        let api_key = caller(&parts)?;
        tracing::info!(
            tool = "get_bitcoin_address",
            api_key_id = api_key.id,
            "MCP tool call"
        );
        self.services
            .get_bitcoin_address(api_key, req)
            .await
            .map(|r| r.to_string())
            .map_err(|e| service_error("get Bitcoin address", e))
    }

    #[tool(
        name = "get_ethereum_address",
        description = "Derive and retrieve an Ethereum public address for a specified wallet ID."
    )]
    async fn get_ethereum_address(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(req): Parameters<GetEthereumAddressRequest>,
    ) -> Result<String, ErrorData> {
        let api_key = caller(&parts)?;
        tracing::info!(
            tool = "get_ethereum_address",
            api_key_id = api_key.id,
            "MCP tool call"
        );
        self.services
            .get_ethereum_address(api_key, req)
            .await
            .map(|r| r.to_string())
            .map_err(|e| service_error("get Ethereum address", e))
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    /// Server identity and capabilities, returned by `server/discover`.
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(PROTOCOL_VERSION)
            .with_server_info(
                Implementation::new(env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"))
                    .with_title("Arktos Wallet")
                    .with_website_url("https://github.com/cognokratos/arktos-wallet"),
            )
            .with_instructions(
                "This is the MCP server for Arktos Wallet. Use an MCP-compatible client to interact with it.",
            )
    }

    /// Advertise and accept only MCP `2026-07-28`; older revisions are rejected
    /// during negotiation instead of being served through a legacy lifecycle.
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(SUPPORTED_PROTOCOL_VERSIONS)
    }
}
