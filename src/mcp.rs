use crate::api_key::ApiKey;
use crate::wallet_services::{CreateWalletRequest, WalletServices};
use http::request::Parts;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use schemars::_private::NoSerialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct McpServer {
    tool_router: ToolRouter<McpServer>,
    services: Arc<WalletServices>,
}

#[tool_router]
impl McpServer {
    pub fn new(services: Arc<WalletServices>) -> Self {
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
        Extension(parts): Extension<Parts>,
        Parameters(req): Parameters<CreateWalletRequest>,
    ) -> Result<String, ErrorData> {
        let api_key = parts
            .extensions
            .get::<ApiKey>()
            .ok_or_else(|| ErrorData::invalid_request("Missing API Key".to_string(), None))?;
        self.services
            .create_wallet(api_key, req)
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
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Arktos MCP server over Streamable HTTP".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
