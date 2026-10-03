//! Protocol-level tests for the `/mcp` endpoint.
//!
//! Each test starts the real Arktos router (API-key middleware + stateless MCP
//! transport) on an ephemeral port and talks to it with the official `rmcp`
//! client or, where the HTTP exchange itself is under test, with raw requests.

use arktos_wallet::app::router;
use arktos_wallet::auth::AppState;
use arktos_wallet::config::DEFAULT_MCP_ALLOWED_HOSTS;
use arktos_wallet::database::Database;
use arktos_wallet::key_services::KeyServices;
use arktos_wallet::wallet_services::WalletServices;
use http::{HeaderName, HeaderValue};
use rmcp::model::{CallToolRequestParams, CallToolResult, JsonObject, ProtocolVersion};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{ClientLifecycleMode, ClientServiceExt, RoleClient};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

/// Deterministic key hierarchy for tests (fixed, non-secret master key).
fn test_keyring() -> arktos_wallet::keys::Keyring {
    arktos_wallet::keys::Keyring::new(&arktos_wallet::keys::MasterKey::from_bytes([0x42; 32]))
}

const EXPECTED_TOOLS: [&str; 4] = [
    "create_wallet",
    "get_bitcoin_address",
    "get_ethereum_address",
    "ping",
];

struct TestServer {
    url: String,
    key_services: Arc<KeyServices>,
    shutdown: CancellationToken,
    _db_dir: TempDir,
}

impl TestServer {
    async fn start() -> Self {
        let db_dir = TempDir::new().expect("temp dir");
        let db_path = db_dir.path().join("mcp.db");
        let db = Arc::new(
            Database::new(db_path.to_str().expect("utf-8 path"), "test_cipher_key")
                .expect("database"),
        );
        let wallet_services = Arc::new(WalletServices::new(db.clone(), test_keyring().wallet));
        let key_services = Arc::new(KeyServices::new(db, test_keyring().api_keys));
        let app_state = AppState {
            key_services: key_services.clone(),
            admin_api_key: Arc::new(secrecy::SecretString::from("admin-key")),
        };
        let shutdown = CancellationToken::new();
        let app = router(
            app_state,
            wallet_services,
            DEFAULT_MCP_ALLOWED_HOSTS
                .iter()
                .map(|h| h.to_string())
                .collect(),
            shutdown.clone(),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server_shutdown = shutdown.clone();
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(server_shutdown.cancelled_owned())
                .await
                .expect("server");
        });

        Self {
            url: format!("http://{addr}/mcp"),
            key_services,
            shutdown,
            _db_dir: db_dir,
        }
    }

    async fn api_key(&self, name: &str) -> String {
        self.key_services.create(name).await.expect("api key")
    }

    /// Connect with the official rmcp client using the 2026-07-28
    /// `server/discover` lifecycle (no `initialize`, no session).
    async fn connect(
        &self,
        api_key: &str,
    ) -> Result<RunningService<RoleClient, ()>, rmcp::service::ClientInitializeError> {
        let headers = HashMap::from([(
            HeaderName::from_static("x-api-key"),
            HeaderValue::from_str(api_key).expect("header value"),
        )]);
        let transport = StreamableHttpClientTransport::with_client(
            reqwest::Client::new(),
            StreamableHttpClientTransportConfig::with_uri(self.url.as_str())
                .custom_headers(headers),
        );
        ().serve_with_lifecycle(
            transport,
            ClientLifecycleMode::Discover {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            },
        )
        .await
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

fn args(value: Value) -> JsonObject {
    value.as_object().expect("object").clone()
}

fn text(result: &CallToolResult) -> &str {
    result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.as_str())
        .expect("text content")
}

/// Extract the `Address="..."` field from a tool's text output.
fn address(output: &str) -> &str {
    output
        .split("Address=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("address field")
}

async fn call(
    client: &RunningService<RoleClient, ()>,
    tool: &'static str,
    arguments: Value,
) -> Result<CallToolResult, rmcp::ServiceError> {
    client
        .call_tool(CallToolRequestParams::new(tool).with_arguments(args(arguments)))
        .await
}

/// Self-contained 2026-07-28 JSON-RPC request with the required per-request
/// `_meta` and SEP-2243 headers, sent without any session header.
async fn raw_request(
    url: &str,
    api_key: Option<&str>,
    method: &str,
    name: Option<&str>,
    mut params: Value,
) -> reqwest::Response {
    params["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": { "name": "raw-http-test", "version": "1.0.0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    let mut request = reqwest::Client::new()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", method)
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }));
    if let Some(name) = name {
        request = request.header("Mcp-Name", name);
    }
    if let Some(api_key) = api_key {
        request = request.header("X-API-KEY", api_key);
    }
    request.send().await.expect("request")
}

// ---------------------------------------------------------------------------
// Discovery / protocol version
// ---------------------------------------------------------------------------

#[tokio::test]
async fn discover_negotiates_mcp_2026_07_28_and_reports_server_metadata() {
    let server = TestServer::start().await;
    let client = server
        .connect(&server.api_key("discover").await)
        .await
        .expect("connect");

    let info = client.peer_info().expect("peer info");
    assert_eq!(info.protocol_version, ProtocolVersion::V_2026_07_28);
    assert!(info.capabilities.tools.is_some(), "tools capability");
    let server_info = info.server_info.as_ref().expect("server info");
    assert_eq!(server_info.title.as_deref(), Some("Arktos Wallet"));
    assert_eq!(server_info.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(
        server_info.website_url.as_deref(),
        Some("https://github.com/cognokratos/arktos-wallet")
    );
}

#[tokio::test]
async fn discover_advertises_only_mcp_2026_07_28() {
    let server = TestServer::start().await;
    let api_key = server.api_key("discover-raw").await;

    let response = raw_request(
        &server.url,
        Some(&api_key),
        "server/discover",
        None,
        json!({}),
    )
    .await;

    assert_eq!(response.status(), 200);
    assert!(response.headers().get("mcp-session-id").is_none());
    let body: Value = response.json().await.expect("json");
    assert_eq!(body["result"]["supportedVersions"], json!(["2026-07-28"]));
}

#[tokio::test]
async fn legacy_initialize_does_not_create_a_session() {
    let server = TestServer::start().await;
    let api_key = server.api_key("legacy").await;

    let response = reqwest::Client::new()
        .post(&server.url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("X-API-KEY", &api_key)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "legacy-client", "version": "1.0.0" }
            }
        }))
        .send()
        .await
        .expect("request");

    assert!(response.headers().get("mcp-session-id").is_none());
    let body: Value = response.json().await.expect("json");
    assert_eq!(body["error"]["code"], -32022, "{body}");
    assert_eq!(body["error"]["data"]["supported"], json!(["2026-07-28"]));
}

#[tokio::test]
async fn foreign_host_header_is_rejected() {
    let server = TestServer::start().await;
    let api_key = server.api_key("rebinding").await;

    // DNS-rebinding protection: only allowlisted Host values reach the server.
    let response = reqwest::Client::new()
        .post(&server.url)
        .header("Host", "attacker.example")
        .header("X-API-KEY", &api_key)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", "2026-07-28")
        .header("Mcp-Method", "server/discover")
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "server/discover", "params": {} }))
        .send()
        .await
        .expect("request");

    assert_eq!(response.status(), 403);
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tools_list_exposes_wallet_tools_deterministically() {
    let server = TestServer::start().await;
    let api_key = server.api_key("tools").await;

    let first = server.connect(&api_key).await.expect("connect");
    let second = server.connect(&api_key).await.expect("connect");
    let first_tools = first.list_all_tools().await.expect("tools/list");
    let second_tools = second.list_all_tools().await.expect("tools/list");

    let mut names: Vec<&str> = first_tools.iter().map(|t| t.name.as_ref()).collect();
    names.sort_unstable();
    assert_eq!(names, EXPECTED_TOOLS);
    assert_eq!(
        serde_json::to_value(&first_tools).expect("serialize"),
        serde_json::to_value(&second_tools).expect("serialize"),
        "tool definitions must be identical across requests"
    );
}

#[tokio::test]
async fn ping_tool_returns_pong() {
    let server = TestServer::start().await;
    let client = server
        .connect(&server.api_key("ping").await)
        .await
        .expect("connect");

    let result = call(&client, "ping", json!({})).await.expect("ping");

    assert_ne!(result.is_error, Some(true));
    assert_eq!(text(&result), "pong");
}

#[tokio::test]
async fn wallet_tools_work_end_to_end() {
    let server = TestServer::start().await;
    let client = server
        .connect(&server.api_key("wallets").await)
        .await
        .expect("connect");

    let created = call(&client, "create_wallet", json!({ "wallet_name": "main" }))
        .await
        .expect("create_wallet");
    assert!(
        text(&created).contains("Name=\"main\""),
        "{}",
        text(&created)
    );

    let btc = call(
        &client,
        "get_bitcoin_address",
        json!({ "wallet_name": "main" }),
    )
    .await
    .expect("get_bitcoin_address");
    assert!(text(&btc).contains("Address=\"bc1"), "{}", text(&btc));

    let eth = call(
        &client,
        "get_ethereum_address",
        json!({ "wallet_name": "main", "account_index": 0 }),
    )
    .await
    .expect("get_ethereum_address");
    assert!(text(&eth).contains("0x"), "{}", text(&eth));

    // Derivation is deterministic: asking again returns the same address.
    let btc_again = call(
        &client,
        "get_bitcoin_address",
        json!({ "wallet_name": "main" }),
    )
    .await
    .expect("get_bitcoin_address");
    assert_eq!(address(text(&btc)), address(text(&btc_again)));
}

#[tokio::test]
async fn service_errors_become_mcp_errors() {
    let server = TestServer::start().await;
    let client = server
        .connect(&server.api_key("errors").await)
        .await
        .expect("connect");

    let error = call(
        &client,
        "get_bitcoin_address",
        json!({ "wallet_name": "does-not-exist" }),
    )
    .await
    .expect_err("unknown wallet must fail");

    let message = error.to_string();
    assert!(
        message.contains("Failed to get Bitcoin address"),
        "{message}"
    );
}

// ---------------------------------------------------------------------------
// Authentication
// ---------------------------------------------------------------------------

#[tokio::test]
async fn missing_api_key_is_rejected() {
    let server = TestServer::start().await;

    let response = raw_request(&server.url, None, "server/discover", None, json!({})).await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn invalid_api_key_is_rejected() {
    let server = TestServer::start().await;

    let response = raw_request(
        &server.url,
        Some("not-a-real-key"),
        "tools/call",
        Some("ping"),
        json!({ "name": "ping", "arguments": {} }),
    )
    .await;
    assert_eq!(response.status(), 401);

    assert!(
        server.connect("not-a-real-key").await.is_err(),
        "rmcp client must not connect with an invalid key"
    );
}

#[tokio::test]
async fn revoked_api_key_is_rejected() {
    let server = TestServer::start().await;
    let api_key = server.api_key("revoked").await;
    let id = server
        .key_services
        .validate(&api_key)
        .await
        .expect("valid key")
        .id;
    server.key_services.revoke(id).await.expect("revoke");

    let response = raw_request(
        &server.url,
        Some(&api_key),
        "server/discover",
        None,
        json!({}),
    )
    .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn valid_api_key_is_accepted() {
    let server = TestServer::start().await;
    let api_key = server.api_key("valid").await;

    let response = raw_request(
        &server.url,
        Some(&api_key),
        "tools/call",
        Some("ping"),
        json!({ "name": "ping", "arguments": {} }),
    )
    .await;

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.expect("json");
    assert_eq!(body["result"]["content"][0]["text"], "pong");
}

// ---------------------------------------------------------------------------
// Statelessness
// ---------------------------------------------------------------------------

#[tokio::test]
async fn independent_requests_need_no_session() {
    let server = TestServer::start().await;
    let api_key = server.api_key("stateless").await;

    // Request A: discovery. No session is issued.
    let a = raw_request(
        &server.url,
        Some(&api_key),
        "server/discover",
        None,
        json!({}),
    )
    .await;
    assert_eq!(a.status(), 200);
    assert!(a.headers().get("mcp-session-id").is_none());

    // Request B: a state-changing tool call with no prior context at all.
    let b = raw_request(
        &server.url,
        Some(&api_key),
        "tools/call",
        Some("create_wallet"),
        json!({ "name": "create_wallet", "arguments": { "wallet_name": "stateless" } }),
    )
    .await;
    assert_eq!(b.status(), 200);
    assert!(b.headers().get("mcp-session-id").is_none());
    let body: Value = b.json().await.expect("json");
    assert!(
        body["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("Name=\"stateless\"")),
        "{body}"
    );

    // Request C: reads the wallet created by B on a brand-new connection; only
    // the persistent wallet store links them, not the MCP transport.
    let c = raw_request(
        &server.url,
        Some(&api_key),
        "tools/call",
        Some("get_ethereum_address"),
        json!({ "name": "get_ethereum_address", "arguments": { "wallet_name": "stateless" } }),
    )
    .await;
    assert_eq!(c.status(), 200);
    let body: Value = c.json().await.expect("json");
    assert!(
        body["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("Address=\"0x")),
        "{body}"
    );
}

// ---------------------------------------------------------------------------
// Wallet ownership
// ---------------------------------------------------------------------------

#[tokio::test]
async fn wallets_are_isolated_per_api_key() {
    let server = TestServer::start().await;
    let alice = server
        .connect(&server.api_key("alice").await)
        .await
        .expect("connect alice");
    let bob = server
        .connect(&server.api_key("bob").await)
        .await
        .expect("connect bob");

    call(&alice, "create_wallet", json!({ "wallet_name": "savings" }))
        .await
        .expect("alice creates wallet");

    // Bob cannot use Alice's wallet...
    call(
        &bob,
        "get_bitcoin_address",
        json!({ "wallet_name": "savings" }),
    )
    .await
    .expect_err("bob must not see alice's wallet");

    // ...and a wallet with the same name under Bob's key is a different wallet.
    call(&bob, "create_wallet", json!({ "wallet_name": "savings" }))
        .await
        .expect("bob creates his own wallet");
    let alice_addr = call(
        &alice,
        "get_bitcoin_address",
        json!({ "wallet_name": "savings" }),
    )
    .await
    .expect("alice address");
    let bob_addr = call(
        &bob,
        "get_bitcoin_address",
        json!({ "wallet_name": "savings" }),
    )
    .await
    .expect("bob address");
    assert_ne!(address(text(&alice_addr)), address(text(&bob_addr)));
}
