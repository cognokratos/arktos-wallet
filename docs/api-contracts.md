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

The primary endpoint for all Model Context Protocol (MCP) communication.

*   **URL:** `/mcp`
*   **Method:** (Handled by MCP transport)
*   **Description:** This is the entrypoint for the Streamable HTTP transport of the MCP server. All MCP tool calls and server interactions are handled through this service.

## MCP Tools

The following tools are exposed through the MCP `/mcp` endpoint and can be called by an MCP agent.

### 1. Create Wallet

Creates a new wallet with a randomly generated recovery passphrase.

*   **Tool Name:** `create_wallet`
*   **Description:** Initiates the creation of a new wallet. The server generates a random recovery passphrase and a corresponding private key. The wallet details are then encrypted and stored in the database.
*   **Arguments:** (empty)
*   **Returns:**
    *   A JSON object containing the new wallet's ID and the recovery passphrase.
        ```json
        {
          "wallet_id": "uuid_of_new_wallet",
          "recovery_passphrase": "your generated recovery passphrase"
        }
        ```

### 2. Get Bitcoin Public Address

Retrieves the Bitcoin Bech32 public address for an existing wallet.

*   **Tool Name:** `get_bitcoin_address`
*   **Description:** Converts the private key associated with the specified wallet ID into a Bitcoin Bech32 public address.
*   **Arguments:**
    *   `wallet_id` (string, required): The unique identifier of the wallet.
*   **Returns:**
    *   A JSON object containing the Bitcoin address.
        ```json
        {
          "bitcoin_address": "bc1q..."
        }
        ```

### 3. Get Ethereum Public Address

Retrieves the Ethereum public address for an existing wallet.

*   **Tool Name:** `get_ethereum_address`
*   **Description:** Converts the private key associated with the specified wallet ID into an Ethereum public address.
*   **Arguments:**
    *   `wallet_id` (string, required): The unique identifier of the wallet.
*   **Returns:**
    *   A JSON object containing the Ethereum address.
        ```json
        {
          "ethereum_address": "0x..."
        }
        ```

### 4. Ping

A simple tool to check the liveness and responsiveness of the MCP tool router.

*   **Tool Name:** `ping`
*   **Description:** Returns a simple "pong" response.
*   **Arguments:** (empty)
*   **Returns:**
    *   A string containing "pong".

