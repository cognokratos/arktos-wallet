# API Contracts: Arktos Wallet

This document describes the network contracts for the Arktos Wallet Model Context Protocol (MCP) server. The server exposes a minimal HTTP interface and provides core functionalities through a set of MCP tools.

## HTTP Endpoints

The server exposes the following HTTP endpoints.

### 1. Health Check

A simple health check endpoint to verify server liveness.

*   **URL:** `/healthz`
*   **Method:** `GET`
*   **Description:** Returns a plain text "OK" if the server is running.
*   **Responses:**
    *   **`200 OK`**
        ```text
        OK
        ```

### 2. MCP Entrypoint

The single endpoint for all Model Context Protocol (MCP) communication.

*   **URL:** `/mcp`
*   **Method:** `POST` (`GET`/`DELETE` return `405`: there are no sessions or standalone streams)
*   **Protocol:** [MCP `2026-07-28`](https://modelcontextprotocol.io/specification/2026-07-28), stateless Streamable HTTP transport, implemented with the official `rmcp` 3.x SDK.
*   **Authentication:** `X-API-KEY: <client API key>` (issued via `/admin/api-keys`). Wallets are scoped to this key.
*   **Description:** Every request is self-contained. Clients call `server/discover` instead of `initialize`, and send `MCP-Protocol-Version: 2026-07-28`, the SEP-2243 `Mcp-Method` (and `Mcp-Name` for `tools/call`) headers and per-request `_meta` on each request. No `Mcp-Session-Id` is issued or required. Request and response formats are defined by the MCP specification; use an MCP client rather than hand-written requests.
*   **Responses:**
    *   **`200 OK`**: JSON-RPC response as `application/json` (or `text/event-stream` if the server streams intermediate messages). Protocol and tool errors are JSON-RPC errors; an unsupported protocol version yields error `-32022`.
    *   **`400 Bad Request`**: Malformed MCP request or inconsistent MCP headers.
    *   **`401 Unauthorized`**: Missing, invalid or revoked API key.
    *   **`403 Forbidden`**: `Host` header not in `MCP_ALLOWED_HOSTS` (DNS-rebinding protection).

Example discovery request (headers: `Mcp-Method: server/discover`, `MCP-Protocol-Version: 2026-07-28`, `X-API-KEY`):

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "server/discover",
  "params": {
    "_meta": {
      "io.modelcontextprotocol/protocolVersion": "2026-07-28",
      "io.modelcontextprotocol/clientInfo": { "name": "example-client", "version": "1.0.0" },
      "io.modelcontextprotocol/clientCapabilities": {}
    }
  }
}
```

Response:

```json
{
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
        "version": "0.1.0",
        "websiteUrl": "https://github.com/cognokratos/arktos-wallet"
      }
    }
  }
}
```

### 3. Admin API

REST endpoints for API-key administration (not MCP), authenticated with the admin key in `X-API-KEY`: `GET`/`POST /admin/api-keys`, `POST /admin/api-keys/{id}/revoke`, `POST /admin/api-keys/{id}/rotate`. See `/swagger-ui` for schemas.

## MCP Tools

The following tools are exposed through the MCP `/mcp` endpoint and can be called by an MCP agent.

### 1. Create Wallet

Creates a new wallet with a randomly generated recovery passphrase.

*   **Tool Name:** `create_wallet`
*   **Description:** Initiates the creation of a new wallet. The server generates a random recovery passphrase. The wallet details are then encrypted and stored in the database.
*   **Arguments:** Name (string): A name for the wallet.
*   **Returns:**
    *   A string confirming the creation of the wallet along with its ID, name, and creation timestamp.
        ```text
        Wallet Created: ID=1, Name=Wallet 1, CreatedAt=2026-01-03T17:07:02.377736+00:00
        ```

### 2. Get Bitcoin Public Address

Retrieves the Bitcoin Bech32 public address for an existing wallet.

*   **Tool Name:** `get_bitcoin_address`
*   **Description:** Derives and retrieves a Bitcoin Bech32 public address (Taproot) for the specified wallet using BIP39/BIP32 hierarchical deterministic wallet derivation.
*   **Arguments:**
    *   `wallet_name` (string, required): The name of the wallet to derive the Bitcoin address for.
    *   `account_index` (integer, optional, default=0): The account index for derivation (follows BIP44 standard).
*   **Returns:**
    *   A string containing the Bitcoin Bech32 (Taproot) address.
        ```text 
        BitcoinAddress: Wallet="MyWallet", Index=0, Address="bc1q...", CreatedAt="2026-01-04T19:22:56.934Z"
        ```

### 3. Get Ethereum Public Address

Retrieves the Ethereum public address for an existing wallet.

*   **Tool Name:** `get_ethereum_address`
*   **Description:** Derives and retrieves an Ethereum public address for the specified wallet using BIP39/BIP32 hierarchical deterministic wallet derivation (BIP44 path: m/44'/60'/0'/0/{account_index}).
*   **Arguments:**
    *   `wallet_name` (string, required): The name of the wallet to derive the Ethereum address for.
    *   `account_index` (integer, optional, default=0): The account index for derivation (follows BIP44 standard).
*   **Returns:**
    *   A string containing the Ethereum address (checksummed, 0x-prefixed, 42 characters total).
        ```text
        EthereumAddress: Wallet="MyWallet", Index=0, Address="0x...", CreatedAt="2026-01-04T19:22:56.934Z"
        ```

### 4. Ping

A simple tool to check the liveness and responsiveness of the MCP tool router.

*   **Tool Name:** `ping`
*   **Description:** Returns a simple "pong" response.
*   **Arguments:** (empty)
*   **Returns:**
    *   A string containing "pong".
        ```text
        pong
        ```
        