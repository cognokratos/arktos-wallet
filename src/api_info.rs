//! OpenAPI/Swagger UI configuration for the Arktos Wallet API.
//!
//! The REST endpoints (`/healthz`, `/admin/*`) are documented from their
//! handlers. `/mcp` is described only at the HTTP level: its payloads are
//! defined by the MCP specification, not mirrored here.

use crate::auth::AppState;
use crate::auth::{CreateApiKeyRequest, ListApiKeysResponse, NewApiKeyResponse};
use crate::wallet_services::{
    BitcoinAddressResponse, CreateWalletRequest, CreateWalletResponse, EthereumAddressResponse,
    GetBitcoinAddressRequest, GetEthereumAddressRequest,
};
use axum::extract::State;
use axum::response::IntoResponse;
use http::StatusCode;
use serde_json::{Value, json};
use utoipa::openapi::example::ExampleBuilder;
use utoipa::openapi::path::{OperationBuilder, ParameterBuilder, ParameterIn};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::{
    ContentBuilder, HttpMethod, ObjectBuilder, Required, Response, ResponseBuilder,
    ResponsesBuilder, Type,
};
use utoipa::{Modify, OpenApi, openapi};
use utoipa_swagger_ui::SwaggerUi;

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

/// Readiness: migrations ran at startup and the database is readable now.
/// Read-only and cheap (one schema query on the blocking pool).
#[utoipa::path(
    get,
    path = "/readyz",
    responses(
        (status = 200, description = "Ready to serve requests", content_type = "text/plain"),
        (status = 503, description = "Database not accessible", content_type = "text/plain")
    ),
    tag = "health"
)]
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    match state.database.ping().await {
        Ok(()) => (StatusCode::OK, "READY"),
        Err(error) => {
            tracing::warn!(%error, "readiness check failed");
            (StatusCode::SERVICE_UNAVAILABLE, "NOT READY")
        }
    }
}

// ---------------------------------------------
// Lightweight OpenAPI description of /mcp
// ---------------------------------------------

const MCP_DESCRIPTION: &str = "\
Model Context Protocol endpoint implementing \
[MCP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28) over the \
stateless Streamable HTTP transport. Use an MCP client rather than hand-written requests: \
message formats, required headers and per-request `_meta` are defined by the MCP \
specification, not by this document.

* Every request is self-contained: there is no `initialize` handshake and no \
`Mcp-Session-Id`. Clients discover the server with `server/discover`.
* Only protocol version `2026-07-28` is supported; other versions are rejected with \
JSON-RPC error `-32022`.
* Arktos requires a valid client API key in `X-API-KEY`; wallets are scoped to that key.
* Tools: `ping`, `create_wallet`, `get_bitcoin_address`, `get_ethereum_address`.";

const CLIENT_META: &str = r#"{
  "io.modelcontextprotocol/protocolVersion": "2026-07-28",
  "io.modelcontextprotocol/clientInfo": { "name": "example-client", "version": "1.0.0" },
  "io.modelcontextprotocol/clientCapabilities": {}
}"#;

fn header(name: &str, description: &str, example: &str) -> openapi::path::Parameter {
    ParameterBuilder::new()
        .name(name)
        .description(Some(description))
        .required(Required::True)
        .parameter_in(ParameterIn::Header)
        .example(Some(Value::from(example)))
        .build()
}

fn example(summary: &str, value: Value) -> openapi::example::Example {
    ExampleBuilder::new()
        .summary(summary)
        .value(Some(value))
        .build()
}

struct McpPath;

impl Modify for McpPath {
    fn modify(&self, openapi: &mut openapi::OpenApi) {
        let meta: Value = serde_json::from_str(CLIENT_META).expect("valid example metadata");

        let request_body = RequestBodyBuilder::new()
            .description(Some("A single MCP JSON-RPC message (see the MCP specification)."))
            .required(Some(Required::True))
            .content(
                "application/json",
                ContentBuilder::new()
                    .schema(Some(ObjectBuilder::new().schema_type(Type::Object)))
                    .examples_from_iter([
                        (
                            "server/discover",
                            example(
                                "Discover server capabilities (headers: Mcp-Method: server/discover)",
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": 1,
                                    "method": "server/discover",
                                    "params": { "_meta": meta }
                                }),
                            ),
                        ),
                        (
                            "tools/call ping",
                            example(
                                "Call the ping tool (headers: Mcp-Method: tools/call, Mcp-Name: ping)",
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": 2,
                                    "method": "tools/call",
                                    "params": { "name": "ping", "arguments": {}, "_meta": meta }
                                }),
                            ),
                        ),
                    ])
                    .build(),
            )
            .build();

        let responses = ResponsesBuilder::new()
            .response(
                "200",
                ResponseBuilder::new()
                    .description(
                        "MCP response: `application/json`, or `text/event-stream` when the \
                         server streams intermediate messages.",
                    )
                    .content(
                        "application/json",
                        ContentBuilder::new()
                            .schema(Some(ObjectBuilder::new().schema_type(Type::Object)))
                            .examples_from_iter([
                                (
                                    "server/discover",
                                    example(
                                        "Discovery result",
                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": 1,
                                            "result": {
                                                "resultType": "complete",
                                                "supportedVersions": ["2026-07-28"],
                                                "capabilities": { "tools": {} },
                                                "instructions": "This is the MCP server for Arktos Wallet. Use an MCP-compatible client to interact with it.",
                                                "ttlMs": 0,
                                                "cacheScope": "private",
                                                "_meta": {
                                                    "io.modelcontextprotocol/serverInfo": {
                                                        "name": "arktos_wallet",
                                                        "title": "Arktos Wallet",
                                                        "version": env!("CARGO_PKG_VERSION"),
                                                        "websiteUrl": "https://github.com/cognokratos/arktos-wallet"
                                                    }
                                                }
                                            }
                                        }),
                                    ),
                                ),
                                (
                                    "tools/call ping",
                                    example(
                                        "Tool result",
                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": 2,
                                            "result": {
                                                "resultType": "complete",
                                                "content": [{ "type": "text", "text": "pong" }],
                                                "isError": false
                                            }
                                        }),
                                    ),
                                ),
                            ])
                            .build(),
                    )
                    .build(),
            )
            .response(
                "400",
                Response::new("Invalid MCP request, headers or unsupported protocol version"),
            )
            .response("401", Response::new("Missing, invalid or revoked API key"))
            .response("403", Response::new("Host header not in the MCP allowlist"))
            .build();

        let op = OperationBuilder::new()
            .summary(Some("MCP endpoint (MCP 2026-07-28, stateless)"))
            .description(Some(MCP_DESCRIPTION))
            .parameter(header("X-API-KEY", "Client API key", "<api-key>"))
            .parameter(header(
                "MCP-Protocol-Version",
                "MCP protocol version",
                "2026-07-28",
            ))
            .parameter(header(
                "Mcp-Method",
                "JSON-RPC method of the body (SEP-2243)",
                "server/discover",
            ))
            .request_body(Some(request_body))
            .responses(responses)
            .tag("mcp")
            .build();

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
        ready,
        crate::auth::create_api_key,
        crate::auth::list_api_keys,
        crate::auth::revoke_api_key,
        crate::auth::rotate_api_key,
    ),
    components(
        schemas(
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
        (name = "mcp", description = "Model Context Protocol endpoint (MCP 2026-07-28)"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_current_mcp_protocol_only() {
        let spec = serde_json::to_value(ApiDoc::openapi()).expect("serializable spec");
        let mcp = &spec["paths"]["/mcp"]["post"];
        assert!(mcp.is_object(), "/mcp must be documented");

        let text = spec.to_string();
        assert!(text.contains("2026-07-28"));
        for obsolete in ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"] {
            assert!(
                !text.contains(obsolete),
                "obsolete protocol version {obsolete}"
            );
        }
        let examples = &mcp["requestBody"]["content"]["application/json"]["examples"];
        let methods: Vec<&str> = examples
            .as_object()
            .expect("examples")
            .values()
            .filter_map(|e| e["value"]["method"].as_str())
            .collect();
        assert!(!methods.is_empty());
        assert!(!methods.contains(&"initialize"), "no initialize examples");
    }
}
