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

REST endpoints for API-key administration (not MCP), authenticated with the admin key in `X-API-KEY` (401 otherwise):

| Endpoint | Request | Success | Errors |
|----------|---------|---------|--------|
| `POST /admin/api-keys` | `{"name": "…"}` (1–255 chars, same rules as wallet names) | `{"api_key": "…"}` — shown only once | 400, 503 |
| `GET /admin/api-keys` | — | `{"api_keys": [{"id", "name", "is_revoked"}]}` | 503 |
| `POST /admin/api-keys/{id}/rotate` | — | `{"api_key": "…"}`; the old key stops working | 404, 503 |
| `POST /admin/api-keys/{id}/revoke` | — | `API key revoked` (text) | 404, 503 |

Errors use the same body as MCP tool errors: `{"error": {"code": "invalid_argument" | "not_found" | "unavailable" | "internal", "message": "…"}}`. Schemas are in `/openapi.json` (`/swagger-ui`).

### 4. Health

* `GET /healthz` — liveness, `OK`.
* `GET /readyz` — readiness (database accessible), `READY` or `503 NOT READY`.

## MCP Tools

Tools are discovered with `tools/list`, which publishes each tool's **input and
output JSON Schema** (generated from the Rust request/response types — the
authoritative contract). Wallet tools return the result as `structuredContent`
(with the same JSON mirrored in a text block for clients without structured
output support). Results contain public data only.

> **v0.2 change:** tool results used to be prose strings such as
> `BitcoinAddress: Wallet="…", Address="…"`. They are now structured JSON; tool
> names and parameters are unchanged.

### Conventions

* **Timestamps** — RFC 3339 UTC with milliseconds, e.g. `2026-10-03T22:30:10.189Z`.
* **`account_index`** — the non-hardened BIP32 *address index* (last path
  component), `0 … 2147483647`, default `0`. The name is kept for compatibility.
* **Public keys** — `public_key_hex`: compressed SEC1 secp256k1 public key, `0x`-hex
  (33 bytes). For Bitcoin it is the BIP86 internal key, before the Taproot tweak.
* **Wallet names** — 1–255 characters, any script; no leading/trailing
  whitespace, control characters, or bidi/zero-width characters. Compared exactly.

### Errors

Domain errors are **tool execution errors**: the result has `isError: true`
and a text block containing JSON, so the calling model can read and correct it:

```json
{"error": {"code": "not_found", "message": "wallet 'nope' not found"}}
```

| `code` | Meaning |
|--------|---------|
| `invalid_argument` | A field failed validation (message names the field) |
| `not_found` | The caller has no wallet with this name |
| `already_exists` | The caller already has a wallet with this name |

Server faults (storage, cryptography, derivation) are JSON-RPC errors
`-32603 "internal error"` without details; details are only logged.
Authentication failures never reach a tool (HTTP 401).

### `create_wallet`

Creates a wallet with a new 12-word BIP39 recovery phrase, encrypted before it
is stored. **Creates state.** The recovery phrase is never returned.

Arguments: `wallet_name` (string, required).

```json
{
  "wallet_id": 1,
  "wallet_name": "main",
  "created_at": "2026-10-03T22:30:10.189Z"
}
```

### `get_bitcoin_address`

Returns the Taproot (P2TR, BIP86) address at `account_index` on the server's
configured Bitcoin network (`BITCOIN_NETWORK`). Deterministic; the account is
recorded on first use.

Arguments: `wallet_name` (string, required), `account_index` (integer, optional, default `0`).

Derivation: BIP39 seed → BIP32 → BIP86 path `m/86'/0'/0'/0/{index}` on mainnet,
`m/86'/1'/0'/0/{index}` on testnet, signet and regtest. Example (testnet):

```json
{
  "wallet_name": "main",
  "account_index": 0,
  "chain": "bitcoin",
  "network": "testnet",
  "address_type": "p2tr",
  "derivation_path": "m/86'/1'/0'/0/0",
  "address": "tb1pamghs8l9g0rtktm9ppvh0ygddamekkucfnrd5h5n4fkpszfqk94qdjw5ml",
  "public_key_hex": "0x03c7b876ff9bbd2a9577f74fe73e8d982699e609a617d75dcd42782fa04891a5b9",
  "created_at": "2026-10-03T22:30:10.207Z"
}
```

Address prefixes: `bc1p…` (mainnet), `tb1p…` (testnet, signet), `bcrt1p…` (regtest).
Accounts are stored per network, so changing `BITCOIN_NETWORK` never returns an
address encoded for another network.

### `get_ethereum_address`

Returns the EIP-55 checksummed Ethereum address at `account_index`, with the
configured chain ID (`ETHEREUM_CHAIN_ID`). Deterministic; the account is
recorded on first use. The address is the same on every EVM chain; the chain ID
documents which chain the deployment targets.

Arguments: `wallet_name` (string, required), `account_index` (integer, optional, default `0`).

Derivation: BIP39 seed → BIP32 → BIP44 path `m/44'/60'/0'/0/{index}`; address =
last 20 bytes of Keccak-256 of the uncompressed public key, formatted with the
EIP-55 checksum.

```json
{
  "wallet_name": "main",
  "account_index": 0,
  "chain": "ethereum",
  "chain_id": 11155111,
  "derivation_path": "m/44'/60'/0'/0/0",
  "address": "0x154904dE0D299f37B7bEe0403546849892f3F25C",
  "public_key_hex": "0x03b60b0d680f709ae423e1d3dba4e43929bf2891290d3cc53e8f6d17931fc637d5",
  "created_at": "2026-10-03T22:30:10.222Z"
}
```

### `ping`

Returns the text `pong`. No arguments, no state.
