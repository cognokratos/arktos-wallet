//! Protocol-level tests for the `/mcp` endpoint.
//!
//! Each test starts the real Arktos router (API-key middleware + stateless MCP
//! transport) on an ephemeral port and talks to it with the official `rmcp`
//! client or, where the HTTP exchange itself is under test, with raw requests.

use arktos_wallet::app::router;
use arktos_wallet::auth::AppState;
use arktos_wallet::config::DEFAULT_MCP_ALLOWED_HOSTS;
use arktos_wallet::database::Database;
use arktos_wallet::domain::{BitcoinNetwork, Chain, EthereumChainId};
use arktos_wallet::key_services::KeyServices;
use arktos_wallet::wallet_manager::eip55_checksum;
use arktos_wallet::wallet_services::{
    BitcoinAddressResponse, BitcoinAddressType, ChainConfig, CreateWalletResponse,
    EthereumAddressResponse, WalletServices,
};
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
        Self::start_with(ChainConfig::default()).await
    }

    async fn start_with(chains: ChainConfig) -> Self {
        let db_dir = TempDir::new().expect("temp dir");
        let db_path = db_dir.path().join("mcp.db");
        let db = Arc::new(
            Database::new(db_path.to_str().expect("utf-8 path"), "test_cipher_key")
                .expect("database"),
        );
        let wallet_services = Arc::new(WalletServices::new(
            db.clone(),
            test_keyring().wallet,
            chains,
        ));
        let key_services = Arc::new(KeyServices::new(db.clone(), test_keyring().api_keys));
        let app_state = AppState {
            key_services: key_services.clone(),
            admin_api_key: Arc::new(secrecy::SecretString::from("admin-key")),
            database: db,
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

/// The structured payload of a successful tool result.
fn structured(result: &CallToolResult) -> &Value {
    assert_ne!(
        result.is_error,
        Some(true),
        "unexpected tool error: {result:?}"
    );
    result
        .structured_content
        .as_ref()
        .expect("structured content")
}

/// Typed view of a successful tool result.
fn typed<T: serde::de::DeserializeOwned>(result: &CallToolResult) -> T {
    serde_json::from_value(structured(result).clone()).expect("payload matches the typed contract")
}

/// The `{"code","message"}` object of a tool execution error.
fn tool_error(result: &CallToolResult) -> Value {
    assert_eq!(
        result.is_error,
        Some(true),
        "expected a tool error: {result:?}"
    );
    assert!(result.structured_content.is_none());
    let body: Value = serde_json::from_str(text(result)).expect("JSON error body");
    body["error"].clone()
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
async fn wallet_tools_return_structured_results() {
    let server = TestServer::start().await;
    let client = server
        .connect(&server.api_key("wallets").await)
        .await
        .expect("connect");

    let created = call(&client, "create_wallet", json!({ "wallet_name": "main" }))
        .await
        .expect("create_wallet");
    let created: CreateWalletResponse = typed(&created);
    assert_eq!(created.wallet_name, "main");
    assert!(created.wallet_id > 0);
    assert!(
        is_rfc3339_utc(&created.created_at),
        "{}",
        created.created_at
    );

    let btc_result = call(
        &client,
        "get_bitcoin_address",
        json!({ "wallet_name": "main" }),
    )
    .await
    .expect("get_bitcoin_address");
    let btc: BitcoinAddressResponse = typed(&btc_result);
    assert_eq!(btc.account_index, 0);
    assert_eq!(btc.chain, Chain::Bitcoin);
    assert_eq!(btc.network, BitcoinNetwork::Mainnet);
    assert_eq!(btc.address_type, BitcoinAddressType::P2tr);
    assert_eq!(btc.derivation_path, "m/86'/0'/0'/0/0");
    assert!(btc.address.starts_with("bc1p"), "{}", btc.address);
    assert!(btc.public_key_hex.starts_with("0x0") && btc.public_key_hex.len() == 68);
    // The text block mirrors the structured payload for clients without
    // structured-output support.
    let mirrored: Value = serde_json::from_str(text(&btc_result)).unwrap();
    assert_eq!(&mirrored, structured(&btc_result));

    let eth_result = call(
        &client,
        "get_ethereum_address",
        json!({ "wallet_name": "main", "account_index": 3 }),
    )
    .await
    .expect("get_ethereum_address");
    let eth: EthereumAddressResponse = typed(&eth_result);
    assert_eq!(eth.account_index, 3);
    assert_eq!(eth.chain, Chain::Ethereum);
    assert_eq!(eth.chain_id, 1);
    assert_eq!(eth.derivation_path, "m/44'/60'/0'/0/3");
    assert_eq!(eip55_checksum(&eth.address).unwrap(), eth.address, "EIP-55");

    // Deterministic: the same request returns the same payload.
    let again = call(
        &client,
        "get_bitcoin_address",
        json!({ "wallet_name": "main", "account_index": 0 }),
    )
    .await
    .unwrap();
    assert_eq!(typed::<BitcoinAddressResponse>(&again), btc);
}

#[tokio::test]
async fn responses_contain_no_secret_fields() {
    let server = TestServer::start().await;
    let client = server
        .connect(&server.api_key("secrets").await)
        .await
        .expect("connect");
    let payloads = [
        call(&client, "create_wallet", json!({ "wallet_name": "w" }))
            .await
            .unwrap(),
        call(
            &client,
            "get_bitcoin_address",
            json!({ "wallet_name": "w" }),
        )
        .await
        .unwrap(),
        call(
            &client,
            "get_ethereum_address",
            json!({ "wallet_name": "w" }),
        )
        .await
        .unwrap(),
    ];
    const FORBIDDEN: [&str; 8] = [
        "mnemonic",
        "passphrase",
        "seed",
        "private_key",
        "encrypted_passphrase",
        "encrypted_private_key",
        "secret_key",
        "database_key",
    ];
    for result in &payloads {
        let object = structured(result).as_object().expect("object payload");
        for key in object.keys() {
            assert!(
                !FORBIDDEN.iter().any(|f| key.contains(f)),
                "forbidden field {key} in {object:?}"
            );
        }
    }
    // Typed decoding with deny_unknown_fields: no field beyond the contract.
    let _: CreateWalletResponse = typed(&payloads[0]);
    let _: BitcoinAddressResponse = typed(&payloads[1]);
    let _: EthereumAddressResponse = typed(&payloads[2]);
}

#[tokio::test]
async fn tools_publish_input_and_output_schemas() {
    let server = TestServer::start().await;
    let client = server
        .connect(&server.api_key("schemas").await)
        .await
        .expect("connect");
    let tools = client.list_all_tools().await.expect("tools/list");
    let tool = |name: &str| tools.iter().find(|t| t.name == name).expect(name).clone();

    for (name, required) in [
        (
            "create_wallet",
            vec!["wallet_id", "wallet_name", "created_at"],
        ),
        (
            "get_bitcoin_address",
            vec![
                "wallet_name",
                "account_index",
                "chain",
                "network",
                "address_type",
                "derivation_path",
                "address",
                "public_key_hex",
                "created_at",
            ],
        ),
        (
            "get_ethereum_address",
            vec![
                "wallet_name",
                "account_index",
                "chain",
                "chain_id",
                "derivation_path",
                "address",
                "public_key_hex",
                "created_at",
            ],
        ),
    ] {
        let schema =
            serde_json::to_value(tool(name).output_schema.expect("output schema")).unwrap();
        let mut listed: Vec<&str> = schema["required"]
            .as_array()
            .expect("required list")
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        listed.sort_unstable();
        let mut expected = required.clone();
        expected.sort_unstable();
        assert_eq!(listed, expected, "{name}");
    }

    let input = serde_json::to_value(&tool("get_bitcoin_address").input_schema).unwrap();
    let index = &input["properties"]["account_index"];
    assert_eq!(index["maximum"], 2147483647, "{index}");
    assert!(
        index["description"]
            .as_str()
            .unwrap()
            .contains("Defaults to 0")
    );
    assert_eq!(input["required"], json!(["wallet_name"]));
}

#[tokio::test]
async fn domain_errors_are_tool_errors_with_codes() {
    let server = TestServer::start().await;
    let client = server
        .connect(&server.api_key("errors").await)
        .await
        .expect("connect");

    let missing = call(
        &client,
        "get_bitcoin_address",
        json!({ "wallet_name": "does-not-exist" }),
    )
    .await
    .expect("tool errors are results, not protocol errors");
    assert_eq!(tool_error(&missing)["code"], "not_found");

    let invalid = call(
        &client,
        "create_wallet",
        json!({ "wallet_name": " padded " }),
    )
    .await
    .unwrap();
    let error = tool_error(&invalid);
    assert_eq!(error["code"], "invalid_argument");
    assert_eq!(
        error["message"],
        "wallet_name must not start or end with whitespace"
    );

    call(&client, "create_wallet", json!({ "wallet_name": "dup" }))
        .await
        .unwrap();
    let dup = call(&client, "create_wallet", json!({ "wallet_name": "dup" }))
        .await
        .unwrap();
    assert_eq!(tool_error(&dup)["code"], "already_exists");

    let out_of_range = call(
        &client,
        "get_ethereum_address",
        json!({ "wallet_name": "dup", "account_index": 2147483648u64 }),
    )
    .await
    .unwrap();
    assert_eq!(tool_error(&out_of_range)["code"], "invalid_argument");
}

#[tokio::test]
async fn chain_configuration_is_reflected_in_results() {
    let server = TestServer::start_with(ChainConfig {
        bitcoin_network: BitcoinNetwork::Regtest,
        ethereum_chain_id: EthereumChainId::new(11155111).unwrap(),
    })
    .await;
    let client = server
        .connect(&server.api_key("regtest").await)
        .await
        .expect("connect");
    call(&client, "create_wallet", json!({ "wallet_name": "w" }))
        .await
        .unwrap();

    let btc: BitcoinAddressResponse = typed(
        &call(
            &client,
            "get_bitcoin_address",
            json!({ "wallet_name": "w" }),
        )
        .await
        .unwrap(),
    );
    assert_eq!(btc.network, BitcoinNetwork::Regtest);
    assert_eq!(btc.derivation_path, "m/86'/1'/0'/0/0");
    assert!(btc.address.starts_with("bcrt1p"), "{}", btc.address);

    let eth: EthereumAddressResponse = typed(
        &call(
            &client,
            "get_ethereum_address",
            json!({ "wallet_name": "w" }),
        )
        .await
        .unwrap(),
    );
    assert_eq!(eth.chain_id, 11155111);
}

fn is_rfc3339_utc(ts: &str) -> bool {
    let b = ts.as_bytes();
    b.len() == 24
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'.'
        && b[23] == b'Z'
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
    assert_eq!(
        body["result"]["structuredContent"]["wallet_name"], "stateless",
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
    let payload = &body["result"]["structuredContent"];
    assert!(
        payload["address"]
            .as_str()
            .is_some_and(|a| a.starts_with("0x")),
        "{body}"
    );
    assert_eq!(payload["chain_id"], 1);
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
    let denied = call(
        &bob,
        "get_bitcoin_address",
        json!({ "wallet_name": "savings" }),
    )
    .await
    .unwrap();
    assert_eq!(tool_error(&denied)["code"], "not_found");

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
    assert_ne!(
        structured(&alice_addr)["address"],
        structured(&bob_addr)["address"]
    );
}

// ---------------------------------------------------------------------------
// Health, readiness and admin error handling
// ---------------------------------------------------------------------------

fn base_url(server: &TestServer) -> String {
    server.url.trim_end_matches("/mcp").to_string()
}

#[tokio::test]
async fn readiness_reports_database_access() {
    let server = TestServer::start().await;
    let response = reqwest::get(format!("{}/readyz", base_url(&server)))
        .await
        .expect("request");
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "READY");
}

#[tokio::test]
async fn admin_endpoints_return_controlled_errors() {
    let server = TestServer::start().await;
    let client = reqwest::Client::new();
    let base = base_url(&server);

    for path in ["/admin/api-keys/9999/rotate", "/admin/api-keys/9999/revoke"] {
        let response = client
            .post(format!("{base}{path}"))
            .header("X-API-KEY", "admin-key")
            .send()
            .await
            .expect("request");
        assert_eq!(response.status(), 404, "{path}");
    }

    let response = client
        .post(format!("{base}/admin/api-keys"))
        .header("X-API-KEY", "admin-key")
        .json(&json!({ "name": "" }))
        .send()
        .await
        .expect("request");
    assert_eq!(response.status(), 400, "empty key name violates the schema");
    assert!(
        !response.text().await.unwrap().contains("CHECK"),
        "no raw SQL errors"
    );
}
