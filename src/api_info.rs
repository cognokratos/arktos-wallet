use crate::auth::{CreateApiKeyRequest, ListApiKeysResponse, NewApiKeyResponse};
use crate::wallet_services::{
    BitcoinAddressResponse, CreateWalletRequest, CreateWalletResponse, EthereumAddressResponse,
    GetBitcoinAddressRequest, GetEthereumAddressRequest,
};
use axum::response::IntoResponse;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::openapi::path::{OperationBuilder, ParameterBuilder, ParameterIn};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::{
    Components, ContentBuilder, HttpMethod, ObjectBuilder, Ref, RefOr, Required, Response,
    ResponseBuilder, ResponsesBuilder, Schema,
};
/// OpenAPI/Swagger UI configuration for the Arktos Wallet API.
///
/// This module provides OpenAPI specification generation and Swagger UI serving
/// using the `utoipa` library. The generated API documentation automatically
/// reflects all API endpoints and their schemas.
use utoipa::{Modify, OpenApi, ToSchema, openapi};
use utoipa_swagger_ui::SwaggerUi;

// -------------------------
// Schemas for MCP endpoint
// -------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct McpRequest {
    /// JSON-RPC version (example: "2.0")
    pub jsonrpc: String,
    /// Method name (example: "tools/list")
    pub method: String,
    /// Free-form parameters
    pub params: serde_json::Value,
    /// Optional request id
    pub id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct McpResponse {
    /// JSON-RPC version (example: "2.0")
    pub jsonrpc: String,
    /// Result payload (if successful)
    pub result: Option<serde_json::Value>,
    /// Error payload (if failed)
    pub error: Option<serde_json::Value>,
    /// Mirrors the request id when present
    pub id: Option<serde_json::Value>,
}

// -------------------------
// Real axum health endpoint
// -------------------------

#[utoipa::path(
    get,
    path = "/healthz",
    responses(
        (status = 200, description = "OK", content_type = "text/plain")
    ),
    tag = "health"
)]
pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK").into_response()
}

// ---------------------------------------------
// Manual OpenAPI path injection for MCP endpoint
// ---------------------------------------------

struct McpPath;

impl Modify for McpPath {
    fn modify(&self, openapi: &mut openapi::OpenApi) {
        let mcp_request_example: Value = json!({
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {
                    "sampling": {},
                    "elicitation": {},
                    "roots": {
                        "listChanged": true
                    }
                },
                "clientInfo": {
                    "name": "inspector-client",
                    "version": "0.18.0"
                }
            },
            "jsonrpc": "2.0",
            "id": 0
        });

        let mcp_response_example: Value = json!({
            "data": {
                "jsonrpc": "2.0",
                "id": 0,
                "result": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
            "name": "arktos_wallet",
            "title": "Arktos Wallet MCP Server",
            "version": "0.1.0",
            "websiteUrl": "https://github.com/cognokratos/arktos-wallet"
        },
        "instructions": "This is the MCP server for Arktos Wallet. Use an MCP-compatible client to interact with it."
                }
            }
        });

        // Ensure the components section exists
        if openapi.components.is_none() {
            openapi.components = Some(Components::new());
        }

        // Add MCP schemas to components if not present already
        // (We also include them via #[openapi(components(...))], but this keeps it robust.)
        {
            let components = openapi.components.as_mut().unwrap();

            components
                .schemas
                .entry("McpRequest".to_string())
                .or_insert_with(|| {
                    RefOr::T(Schema::Object(
                        ObjectBuilder::new()
                            .description(Some("MCP request (JSON-RPC-like)"))
                            .build(),
                    ))
                });

            components
                .schemas
                .entry("McpResponse".to_string())
                .or_insert_with(|| {
                    RefOr::T(Schema::Object(
                        ObjectBuilder::new()
                            .description(Some("MCP response (JSON-RPC-like)"))
                            .build(),
                    ))
                });
        }

        let params = ParameterBuilder::new()
            .description(Some("Client API key"))
            .name("X-API-KEY")
            .required(Required::True)
            .parameter_in(ParameterIn::Header)
            .build();

        // Request body: application/json referencing McpRequest schema
        let request_body = RequestBodyBuilder::new()
            .description(Some("MCP request payload"))
            .required(Some(Required::True))
            .content(
                "application/json",
                ContentBuilder::new()
                    .schema(Some(Ref::from_schema_name("McpRequest")))
                    .example(Some(mcp_request_example))
                    .build(),
            )
            .build();

        // Responses: 200 application/json referencing McpResponse schema
        let responses = ResponsesBuilder::new()
            .response(
                "200",
                ResponseBuilder::new()
                    .description("MCP response (JSON or Streaming)")
                    .content(
                        "application/json, text/event-stream",
                        ContentBuilder::new()
                            .schema(Some(Ref::from_schema_name("McpResponse")))
                            .example(Some(mcp_response_example))
                            .build(),
                    )
                    .build(),
            )
            .response("400", Response::new("Bad request"))
            .response("500", Response::new("Server error"))
            .build();

        let op = OperationBuilder::new()
            .summary(Some("MCP endpoint"))
            .description(Some(
                "MCP endpoint served externally / not implemented as an Axum handler in this binary.",
            ))
            .parameter(params)
            .request_body(Some(request_body))
            .responses(responses)
            .tag("mcp")
            .build();

        // Add POST /mcp to paths
        openapi
            .paths
            .add_path_operation("/mcp", Vec::from([HttpMethod::Post]), op);
    }
}

/// OpenAPI documentation for Arktos Wallet API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Arktos Wallet API",
        description = "Non-custodial, AI agent-controlled cold wallet server for Bitcoin and Ethereum.",
        version = "0.1.0",
        contact(
            name = "Arktos Development",
            url = "https://github.com/CognoKratos/arktos-wallet"
        )
    ),
    modifiers(&McpPath),
    paths(
        health,
        crate::auth::create_api_key,
        crate::auth::list_api_keys,
        crate::auth::revoke_api_key,
        crate::auth::rotate_api_key,
    ),
    components(
        schemas(
            McpRequest,
            McpResponse,
            CreateWalletRequest,
            CreateWalletResponse,
            GetBitcoinAddressRequest,
            BitcoinAddressResponse,
            GetEthereumAddressRequest,
            EthereumAddressResponse,
            CreateApiKeyRequest,
            NewApiKeyResponse,
            ListApiKeysResponse,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "mcp", description = "Model Context Protocol tools"),
        (name = "admin", description = "Administrative API endpoints"),
    )
)]
pub struct ApiDoc;

/// Provides the Swagger UI instance for serving the OpenAPI specification.
///
/// Returns a Swagger UI instance that serves the OpenAPI specification at `/api-docs/openapi.json`.
/// This allows users to explore and interact with the API documentation through a web interface.
pub fn api_doc() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi())
}
