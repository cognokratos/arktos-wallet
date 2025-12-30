# Арктос Wallet
- **Tech Stack:**
  - **Rust**
    - Prefer Functional Programming
  - **SQLite**
    ```toml
    [dependencies]
    # Use rusqlite and bundle SQLCipher + a vendored OpenSSL
    rusqlite = { version = "0.38", features = ["bundled-sqlcipher-vendored-openssl"] }
    ```
  - **Blockchain**
    - _generate_private_key_ using `secp256k1` and a recovery passphrase to generate a private key via `BIP‑39` + `BIP‑32`.
    - _read_bitcoin_address_ converting this private key to a `Bitcoin Bech32 address` (using `rust-bitcoin`).
    - _read_ethereum_address_ converting the same private key to an `Ethereum address` (using `tiny_keccak`).
    ```toml
    [dependencies]
    bip39 = "2"
    bitcoin = { version = "0.32", features = ["std"] }
    tiny-keccak = "2"
    ```
  - _Model Context Protocol_ server with **RMCP**
    ```toml
    [dependencies]
    # Official MCP Rust SDK
    rmcp = { version = "0.12", features = [
        "server",
        "macros",
        "transport-streamable-http-server"
    ] }
    ```
  - **Docker**
    - use `lukemathwalker/cargo-chef` for deps and build
    - use `gcr.io/distroless/static-debian12:nonroot` for runtime
    - use multi-stage:
      1. Install dependencies
      2. Isolated build
      3. Lean runtime
- **Features:**
  - **Agent** creates new _Wallet_ with random recovery _Passphrase_
  - **Server** generates _Private Key_ with _Wallet Passphrase_
  - **Agent** reads Bitcoin _Public Address_ for existing _Wallet_ by ID
  - **Agent** reads Ethereum _Public Address_ for existing _Wallet_ by ID
  - **Server** encrypts and saves _Wallets_ and _Private Keys_ in SQLite
- **Data Schemas:**
  - _Wallet_:
    - _Name_: String
    - _Passphrase_: Text
    - _Accounts_: List of _Account_
  - _Account_:
    - _ID_: Incremented Integer
    - _Private Key_: Text
- **Supported Blockchains:**
  - Bitcoin
  - Ethereum
- **Supported Interfaces:**
  - HTTP MCP Server