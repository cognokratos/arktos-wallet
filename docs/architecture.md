# Arktos Wallet - Architecture

This document outlines the architecture of the Arktos Wallet application, positioning it as an open-source blueprint for agent-accessible, self-hosted non-custodial wallet services. Agents can create wallets and obtain public addresses; Arktos does not sign or broadcast transactions.

## 1. Executive Summary

**Arktos Wallet** is a backend HTTP server built with Rust, designed as a **customizable blueprint** for secure wallet management. It is implemented as a single, cohesive monolithic service acting as an "HTTP MCP server" with API-centric design.

### Blueprint Positioning

Arktos serves as an **open-source educational reference implementation** demonstrating:

- **Secure wallet creation** with BIP39/BIP32 cryptographic standards
- **Multi-account management** for blockchain address derivation
- **Non-custodial architecture** where system owners control all encryption keys
- **API key authentication** for secure Model Context Protocol (MCP) integration
- **Two encryption layers**: SQLCipher for the database file, plus AES-256-GCM field encryption of wallet secrets under purpose-specific keys
- **Production-ready patterns** for deployment, scaling, and compliance
- **Extensible design** for customization and regional compliance adaptation

This architecture is intentionally **simple and modular**, making it suitable for:
- Learning and educational purposes
- Customization for specific organizational needs
- Adaptation for regional compliance requirements (GDPR, HIPAA, etc.)
- Extension with new blockchains or integrations

## 2. Technology Stack

| Category          | Technology            | Version      | Justification                                |
|-------------------|-----------------------|--------------|----------------------------------------------|
| Language          | Rust                  | 2024 edition | Type-safe, memory-safe backend with zero-cost abstractions |
| Framework         | Axum                  | 0.8+         | Modular, composable web framework for building APIs |
| Async Runtime     | Tokio                 | 1.x          | Production-grade async runtime for high-performance I/O |
| MCP SDK           | rmcp (official MCP Rust SDK) | 3.x   | MCP `2026-07-28` server, stateless Streamable HTTP transport |
| Database          | SQLite + SQLCipher    | 3.x          | Lightweight, encrypted local persistence |
| Serialization     | Serde, Serde JSON     | 1.x          | Efficient, zero-copy serialization |
| Schema Generation | Schemars              | 1.x          | JSON schema generation for API documentation |
| Logging           | Tracing               | 0.1+         | Structured, composable logging framework |
| Cryptography      | secp256k1, bip39, bip32, tiny-keccak | Latest | Industry-standard blockchain key derivation |

## 3. Architecture Pattern

Arktos follows a **layered API-centric architecture** optimized for:
- **Stateless MCP protocol layer** (no MCP sessions; any request can be served independently)
- **Security-first design** (encryption at rest/in transit)
- **MCP-native integration** (AI agent workflows)
- **Extensibility** (modular design supports customization)

### Architectural Layers

```
┌─────────────────────────────────────────────┐
│ HTTP Layer (Axum Router)                    │
│ - Route handling                            │
│ - Request/response serialization            │
│ - Authentication middleware                 │
└────────────────────┬────────────────────────┘
                     │
┌────────────────────▼────────────────────────┐
│ API Handler Layer                           │
│ - MCP tool execution                        │
│ - Request validation & processing           │
│ - Response formatting                       │
└────────────────────┬────────────────────────┘
                     │
┌────────────────────▼────────────────────────┐
│ Business Logic Layer                        │
│ - Wallet creation & management              │
│ - Key derivation (Bitcoin, Ethereum, etc.)  │
│ - Account management                        │
│ - Audit logging                             │
└────────────────────┬────────────────────────┘
                     │
┌────────────────────▼────────────────────────┐
│ Data Access Layer                           │
│ - Database queries                          │
│ - Encryption/decryption                     │
│ - Transaction management                    │
└────────────────────┬────────────────────────┘
                     │
┌────────────────────▼────────────────────────┐
│ Persistence Layer (SQLCipher)               │
│ - Encrypted SQLite database                 │
│ - Wallet data & accounts                    │
│ - API-key hashes                            │
└─────────────────────────────────────────────┘
```

### Key Architectural Principles

1. **Stateless Protocol Layer**: No MCP session or transport state is kept between requests; persistent application state (wallets, accounts, API keys) lives in the database
2. **Security by Default**: All sensitive data encrypted at rest; TLS required in transit
3. **Non-Custodial Model**: System owner maintains complete control of encryption keys
4. **Single Responsibility**: Each module handles one concern (wallet management, auth, data access)
5. **API-First Design**: Core functionality exposed via standardized MCP tools
6. **Auditability**: All critical operations logged for compliance and debugging

## 4. Data Architecture

Arktos persists to a single local **SQLite database encrypted with SQLCipher**
(`rusqlite` with bundled SQLCipher). This is an intentional choice for a
self-hosted wallet service, not a placeholder — see
[Why SQLite + SQLCipher?](#why-sqlite--sqlcipher).

### Startup Sequence

```
open DATABASE_PATH (create parent dir 0700, file 0600)
  → PRAGMA key (SQLCipher)          first statement on the connection
  → verify PRAGMA cipher_version     refuse to run on plain SQLite
  → read sqlite_schema               fails fast on a wrong DATABASE_KEY
  → foreign_keys=ON, journal_mode=WAL, synchronous=FULL, busy_timeout=5s (verified)
  → run pending migrations           migrations/V*.sql, tracked in PRAGMA user_version
  → serve requests                   /readyz reports database accessibility
```

### Schema

```
api_keys (id, key_hash UNIQUE, key_name, is_revoked, created_at)
   │ 1
   │ *  FK ON DELETE RESTRICT
wallets  (id, key_id → api_keys, name, encrypted_passphrase, created_at)
         UNIQUE (key_id, name)                  one wallet name per owner
   │ 1
   │ *  FK ON DELETE RESTRICT
accounts (id, wallet_id → wallets, chain_type, network, account_index,
          derivation_path, public_key, address, created_at)
         UNIQUE (wallet_id, chain_type, network, account_index)
         CHECK  derivation_path = canonical path for chain + network + index
         CHECK  Ethereum addresses stored in lowercase
```

All tables are `STRICT`. The only secret column is `wallets.encrypted_passphrase`;
**account private keys are not stored** — they are re-derived from the recovery
phrase when needed. Timestamps are generated by SQLite (UTC,
`YYYY-MM-DDTHH:MM:SS.sssZ`) and read back with `RETURNING`, so responses always
match the stored value. See [Data Models](./data-models.md) for column details.

### Migrations

Schema changes are plain SQL files in `migrations/` (`V1__initial_schema.sql`,
…), embedded in the binary and applied in order by
[`rusqlite_migration`](https://crates.io/crates/rusqlite_migration) inside a
transaction. The applied version is stored in `PRAGMA user_version`, so each
migration runs exactly once; startup is idempotent. Arktos refuses to start on a
database whose version is newer than the binary, and on a database that has
tables but no migration version (created before migrations existed — start from
a new file). `arktos-wallet migrate` (`make migrate`) applies migrations without
starting the server.

### Connection Model and Blocking Boundary

```
Tokio worker threads (async handlers, MCP tools, services)
        │  Database::read / Database::write  (async)
        ▼
tokio::task::spawn_blocking  ──►  Mutex<rusqlite::Connection>  (one SQLCipher connection)
```

- **One connection** owned privately by `Database`. `KeyStore` (API keys) and
  `WalletStore` (wallets/accounts) hold SQL; services never see a connection.
- **No blocking on async threads**: every query runs on Tokio's blocking pool.
  The mutex is only held inside those closures, never across an `.await`.
- **Transactions**: `Database::write` runs its closure in `BEGIN IMMEDIATE` and
  commits only on success, so multi-statement writes are atomic.
- **Race safety**: uniqueness is enforced by the database. Concurrent
  `create_wallet` calls for the same name yield one wallet and
  `WalletAlreadyExists` for the rest; concurrent first derivations of an account
  use `INSERT … ON CONFLICT DO NOTHING` and all return the same row.
- **No panics on database content**: a poisoned mutex is recovered (the
  interrupted transaction was rolled back), and unexpected row values become
  `CorruptData` errors.

A single serialized connection is deliberately simple and predictable; wallet
operations are small and infrequent, so throughput is not the constraint.

### SQLite Configuration

| PRAGMA | Value | Why |
|--------|-------|-----|
| `key` | `DATABASE_KEY` | SQLCipher 4 defaults: AES-256 page encryption, HMAC-SHA512 page authentication, PBKDF2-HMAC-SHA512 (256k iterations) key derivation. Passed via `pragma_update` (quoted literal), never formatted into SQL. |
| `foreign_keys` | `ON` | SQLite ignores declared foreign keys unless enabled per connection; verified at startup. |
| `journal_mode` | `WAL` | Commits append to the `-wal` file (encrypted by SQLCipher like the main file); an operator shell or backup can read without blocking the server's writes. |
| `synchronous` | `FULL` | Every commit is fsynced. An address handed to a client must survive power loss, so durability is preferred over the small write cost of `NORMAL`. |
| `busy_timeout` | 5 s | Only other connections (e.g. `make sql`, a backup) can hold locks; writers wait instead of failing immediately. |

### Persistence Errors

Store methods return `StoreError` (`AlreadyExists`, `ForeignKeyViolation`,
`ConstraintViolation`, `Unavailable`, `WrongKeyOrNotADatabase`, `NotSqlCipher`,
`UnmanagedSchema`, `Migration`, `CorruptData`, `Internal`). Services map expected
cases to domain errors (e.g. duplicate name → `WalletAlreadyExists`, unknown
admin key id → HTTP 404); unexpected failures are logged with details and
reported to clients only as a generic "database error/unavailable".

### Files, Permissions and Backups

- The database directory is created owner-only (`0700`) and the database file is
  set to `0600`; SQLite creates `-wal`/`-shm` with the same mode.
- In WAL mode the database consists of `arktos.db`, `arktos.db-wal` and
  `arktos.db-shm`. Copying only `arktos.db` while the server runs can miss
  committed transactions. Consistent backups (verified with SQLCipher 4.10):
  - **Online**: from a keyed `sqlcipher` shell (`make sql`) run
    `VACUUM INTO '/backups/arktos-YYYYMMDD.db';` — a single, self-contained file
    encrypted with the same `DATABASE_KEY`, including the schema version.
  - **Offline**: stop Arktos, then copy `arktos.db` together with any
    `-wal`/`-shm` files.
  - Not suitable: the shell's `.backup` (unsupported for encrypted databases)
    and `sqlcipher_export()` (does not copy the schema version, so Arktos would
    reject the copy).
- Backups are still encrypted with `DATABASE_KEY`, and wallet recovery phrases
  inside them additionally require `MASTER_KEY`. Back up **both keys
  separately** from the database: losing either makes wallets unrecoverable.

### Encryption Strategy

- **At Rest**: two independent layers — see [Key Hierarchy & Secret Storage](#key-hierarchy--secret-storage)
- **In Transit**: Arktos serves plain HTTP; terminate TLS (1.2+) at a reverse proxy or load balancer
- **Key Management**: keys are supplied by the system owner through environment variables and are never written to the database

See [Data Models](./data-models.md) for detailed schema and [Regional Compliance](./regional-compliance.md) for encryption key management patterns.

## 5. API Design

The server exposes a minimal, intentionally-constrained HTTP interface to maximize security:

### HTTP Endpoints

| Endpoint | Method | Purpose | Authentication |
|----------|--------|---------|-----------------|
| `/healthz` | GET | Liveness probe (process is up) | None |
| `/readyz` | GET | Readiness probe (database accessible; migrations ran at startup) | None |
| `/mcp` | POST | Model Context Protocol endpoint (MCP `2026-07-28`) | API Key (`X-API-KEY`) |
| `/admin/api-keys`, `/admin/api-keys/{id}/revoke`, `/admin/api-keys/{id}/rotate` | GET/POST | API key administration (REST, not MCP) | Admin API Key |
| `/swagger-ui`, `/openapi.json` | GET | API documentation | None |

### MCP Transport

| Aspect | Value |
|--------|-------|
| MCP specification | [`2026-07-28`](https://modelcontextprotocol.io/specification/2026-07-28) (the only supported version) |
| SDK | [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk) 3.x (official Rust SDK) |
| Transport | Streamable HTTP, stateless (`NeverSessionManager`, `legacy_session_mode = false`) |
| Discovery | `server/discover` (no `initialize` handshake, no `Mcp-Session-Id`) |
| Responses | `application/json`; `text/event-stream` only when a handler streams intermediate messages |

Each `POST /mcp` carries everything needed to serve it: the `MCP-Protocol-Version`
and SEP-2243 `Mcp-Method`/`Mcp-Name` headers, per-request `_meta` (protocol
version, client info, client capabilities) and the `X-API-KEY` header. Arktos
builds a fresh `McpServer` for every request, so any instance can serve any
request and no sticky sessions are needed.

Requests negotiating an older protocol revision (including legacy `initialize`)
are rejected with JSON-RPC error `-32022` (*Unsupported protocol version*).
Arktos deliberately does not keep a legacy session layer for old clients.

Tool definitions are generated at compile time from `#[tool]` attributes, so
`tools/list` is identical across requests; the SDK attaches the MCP cache hints
(`ttlMs`, `cacheScope`).

### MCP Tools (Core API)

| Tool | Creates state | Result (structured) |
|------|---------------|---------------------|
| `ping` | no | text `pong` |
| `create_wallet` | yes (wallet) | `wallet_id`, `wallet_name`, `created_at` |
| `get_bitcoin_address` | on first use (account) | `network`, `address_type` (`p2tr`), BIP86 `derivation_path`, `address`, `public_key_hex`, … |
| `get_ethereum_address` | on first use (account) | `chain_id`, BIP44 `derivation_path`, EIP-55 `address`, `public_key_hex`, … |

Input and output JSON Schemas are generated from the Rust request/response
types in `wallet_services.rs` and published by `tools/list`; there is no
hand-written copy. See [API Contracts](./api-contracts.md#mcp-tools) for examples.

### Domain Model

| Concept | Type | Values / representation |
|---------|------|-------------------------|
| Chain | `Chain` | `bitcoin`, `ethereum` |
| Bitcoin network | `BitcoinNetwork` (`BITCOIN_NETWORK`) | `mainnet` (default), `testnet`, `signet`, `regtest`; invalid values fail at startup |
| Ethereum chain ID | `EthereumChainId` (`ETHEREUM_CHAIN_ID`) | positive EIP-155 ID, default `1`; reported, not used for derivation |
| Address space | `Network` | `Bitcoin(BitcoinNetwork)` or `Ethereum`; part of an account's stored identity |
| Index | `DerivationIndex` | non-hardened BIP32 address index `0 … 2^31-1` (`account_index` in the API) |
| Names | `WalletName`, `ApiKeyName` | validated once at the boundary |

Derivation (one function, `Network::derivation_path`):

| Network | Path | Standards |
|---------|------|-----------|
| Bitcoin mainnet | `m/86'/0'/0'/0/{i}` | BIP39 → BIP32 → BIP86 Taproot (P2TR) |
| Bitcoin testnet/signet/regtest | `m/86'/1'/0'/0/{i}` | BIP86, coin type 1' |
| Ethereum | `m/44'/60'/0'/0/{i}` | BIP39 → BIP32 → BIP44 path; address formatted with EIP-55 |

Ethereum addresses are stored in canonical lowercase and checksummed (EIP-55)
when returned. Ethereum accounts use the single network value `evm`, because
the same address is valid on every EVM chain.

### Error Model

`AppError` distinguishes client errors from server faults:

| Error | MCP result |
|-------|------------|
| `InvalidArgument`, `WalletNotFound`, `WalletAlreadyExists` | tool execution error: `isError: true`, text `{"error":{"code":"invalid_argument"/"not_found"/"already_exists","message":…}}` |
| `Storage`, `Crypto`, `DerivationFailed`, `Internal` | JSON-RPC `-32603 "internal error"`, details only in logs |

Authentication is enforced before MCP (HTTP 401/503). The admin API uses the
same `{"error":{"code","message"}}` body with HTTP status codes.

### Design Rationale

- **MCP-Centric**: All wallet operations via MCP ensures structured, typed interactions
- **API Key Auth**: Simple, stateless authentication suitable for service-to-service communication
- **No Sensitive Data in Responses**: Public addresses only; private keys never transmitted
- **Minimal HTTP Surface**: One MCP endpoint plus health, admin and documentation routes

For complete API specification, see [API Contracts](./api-contracts.md).

## 6. Security Architecture

### Authentication & Authorization

```
MCP Request Flow (every request is independent):
┌──────────────────┐
│ AI Agent/Client  │
└────────┬─────────┘
         │ POST /mcp  (X-API-KEY, MCP-Protocol-Version: 2026-07-28)
         ▼
┌────────────────────────────────────────┐
│ API Key Middleware (Axum)              │
│ - Verify key exists and is not revoked │
│ - Attach ApiKey to request extensions  │
│ - 401 otherwise                        │
└────────┬───────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ rmcp Streamable HTTP (stateless)       │
│ - Host allowlist (DNS rebinding) → 403 │
│ - Protocol version / header / _meta    │
│   validation → JSON-RPC errors         │
│ - Forwards HTTP Parts (incl. ApiKey)   │
└────────┬───────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ McpServer tool router                  │
│ - Reads ApiKey from request Parts      │
│ - Logs tool name + API key id          │
└────────┬───────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ WalletServices                         │
│ - Wallets scoped to the API key        │
│ - Return only public data              │
└────────────────────────────────────────┘
```

The API key never appears in tool parameters and is not visible to the model.

### Data Protection

1. **Encryption at Rest**: SQLCipher database encryption plus AES-256-GCM field encryption (below)
2. **Encryption in Transit**: TLS must be terminated in front of Arktos (it serves plain HTTP)
3. **Key Management**: Keys supplied by the system owner via environment; no HSM/KMS integration yet
4. **Access Control**: API key-based authentication, ownership-based authorization
5. **Audit Logging**: Operations logged with wallet names, API-key IDs and public addresses — never secrets

### Key Hierarchy & Secret Storage

Two independently generated secrets protect different things:

```
DATABASE_KEY  (independent random secret)
    └── SQLCipher: encrypts the whole SQLite database file

MASTER_KEY  (32 random bytes, base64)
    └── HKDF-SHA256 (RFC 5869, no salt, purpose label as "info")
          ├── "arktos/api-key-hmac/v1"            → API-key HMAC-SHA256 key
          └── "arktos/wallet-seed-encryption/v1"  → AES-256-GCM key for recovery phrases

```

- **Key separation**: the database key is never derived from the master key, so
  compromising one layer does not reveal the other. Generate each with
  `make secret` (32 bytes from the OS RNG, base64).
- **Domain separation**: each purpose has its own HKDF label and its own Rust
  type (`ApiKeyHmacKey`, `WalletSeedKey`), so one purpose's key
  cannot be used for another. The purpose is also bound into the AEAD
  associated data.
- **Startup validation**: `MASTER_KEY` must decode to exactly 32 bytes;
  `DATABASE_KEY` must differ from it. Errors name the variable, never the value.

**What each layer protects.** SQLCipher protects the database file at rest
(e.g. a copied disk or backup). Field encryption protects the recovery phrase
*inside* an opened database: anyone who can query the
database (with `DATABASE_KEY`, through a SQL console or a database dump) still
sees only ciphertext without `MASTER_KEY`. The layers protect against
different exposures; they do not "double" the strength of AES-256.

**Encrypted-secret envelope (v1).** New ciphertexts are stored as compact JSON:

```json
{"v":1,"alg":"A256GCM","nonce":"<base64url, 12 bytes>","ct":"<base64url ciphertext‖16-byte tag>"}
```

Every encryption uses a fresh random nonce from the OS RNG. The associated
data is `arktos:v1:A256GCM:<purpose>`. Unknown versions or algorithms, wrong
nonce lengths and malformed values fail with explicit errors; authentication
failures (wrong key, tampered data) are reported identically as "failed to
decrypt secret". Anything that is not a v1 envelope is rejected.

**Secret lifecycle.**

| Secret | Plaintext exists | Destroyed |
|--------|------------------|-----------|
| Recovery phrase | From generation (128-bit OS entropy) until encrypted in `create_wallet`; from decryption until account derivation | Zeroized on drop at the end of that block |
| BIP39 seed / extended private keys | During one account derivation | Seed zeroized immediately after `XPrv` creation; extended keys zeroized on drop once the public key is computed |
| Account private key | Never extracted | Only the public key and address leave the derivation function |
| Derived subkeys / master key | Process lifetime | Zeroized when dropped at shutdown |

Secret-bearing types redact themselves in `Debug` output, and secrets are
never logged, returned by MCP tools or admin endpoints, or included in error
messages. Zeroization reduces how long secrets stay in memory; it cannot
remove copies made inside third-party libraries (e.g. `bip39::Mnemonic`, the
BIP32 chain code), by the allocator, by swap/core dumps, or from the process
environment, where the configured keys remain readable.

**No private-key persistence.** The encrypted recovery phrase is the only
wallet secret stored. Accounts persist only public data (index, chain, public
key, address); private keys can always be re-derived from the phrase when a
future feature (e.g. signing) needs them.

### Defense in Depth

- Input validation on all API parameters
- Secure error handling (no sensitive data in error messages)
- SQL injection prevention via parameterized queries
- DNS-rebinding protection: `/mcp` only accepts allowlisted `Host` headers (`MCP_ALLOWED_HOSTS`, loopback by default)
- Vulnerability scanning via `cargo audit`
- Code quality via `cargo clippy` and formatting via `cargo fmt`

## 7. Performance & Scalability

### Performance Targets (NFR-compliant)

- **Wallet Creation**: < 500ms (p95) - cryptographic key generation
- **Address Retrieval**: < 100ms (p95) - deterministic derivation
- **Concurrency**: 100+ req/s (target)
- **Database Capacity**: 10,000 wallets, 50,000+ accounts per instance
- **Uptime Target**: 99.9% (production deployment with monitoring)

### Scalability Pattern

Two layers must be distinguished:

| Layer | State | Scaling |
|-------|-------|---------|
| MCP protocol layer | Stateless (MCP `2026-07-28`, no sessions) | Any request can be routed to any instance; no sticky sessions |
| Persistence | Local SQLCipher (SQLite) file | **Single instance** |

```
MCP clients ──► Arktos (single instance) ──► SQLCipher file (local volume)
                 stateless MCP requests       persistent wallets / API keys
```

Supported deployment: **one Arktos instance per SQLCipher database**, with the
database on a local (or block-storage) volume. Multiple Arktos instances sharing
one SQLite file — e.g. over NFS or a shared volume — is not supported: SQLite
locking is not designed for it, and the single-connection model assumes one
process owns the file. Running several instances would require a different
persistence and signing architecture; the stateless MCP transport only removes
the protocol-level obstacle.

## 8. Customization & Extension Points

Arktos is designed for customization. Key extension areas:

### Blockchain Support
Add new blockchains by implementing key derivation modules (see [Customization Guide](./customization-guide.md#1-adding-blockchain-support)).

### Authentication
Replace API key auth with OAuth2, JWT, or mTLS (see [Customization Guide](./customization-guide.md#2-custom-authentication)).

### Storage Backend
Persistence is isolated in `Database`, `KeyStore` and `WalletStore`; another backend would replace those while keeping the service layer (see [Customization Guide](./customization-guide.md#3-storage-backend-customization)).

### Observability
Integrate with external logging, metrics, and monitoring systems (see [Customization Guide](./customization-guide.md#5-observability--monitoring)).

See [Customization Guide](./customization-guide.md) for detailed patterns and implementation examples.

## 9. Compliance & Regional Adaptation

Arktos architecture supports compliance requirements for various regulations:

### Supported Compliance Frameworks

- **GDPR** (Europe): Data minimization, deletion rights, portability, audit logging
- **HIPAA** (Healthcare): Encryption, access controls, audit trails, backup/recovery
- **PCI DSS** (Payment): Network security, encryption, monitoring (if integrated with payment)
- **SOC 2** (Service Organization): Security controls, availability, integrity, confidentiality
- **Regional**: PDPA (Singapore), LGPD (Brazil), Privacy Act (Canada/Australia), etc.

### Key Compliance Features

- ✅ Encryption at rest (SQLCipher + AES-256-GCM field encryption)
- ✅ Encryption in transit (TLS 1.2+ via reverse proxy)
- ✅ Audit logging of all operations
- ✅ Access control (API key + ownership)
- ✅ Data minimization (only essential data)
- ✅ Non-custodial model (system owner controls keys)
- ✅ Stateless MCP protocol layer (no session state; secrets live in memory only as described in [Key Hierarchy & Secret Storage](#key-hierarchy--secret-storage))

See [Regional Compliance](./regional-compliance.md) for detailed patterns and implementation guidance.

## 10. Development & Deployment

- **Development**: Instructions for setting up the local environment, building, and running the application can be found in the [Development Guide](./development-guide.md).
- **Deployment**: Complete containerization guide, environment configuration, and production deployment patterns are documented in the [Deployment Guide](./deployment-guide.md).

## 11. Design Decisions & Rationale

### Why Monolithic Architecture?

- **Simplicity**: Easier to understand and deploy for educational purposes
- **Security**: Single deployment unit, no inter-service communication vulnerabilities
- **Performance**: No network latency between logical layers
- **Operations**: Single database, simpler monitoring and debugging

Monolithic architecture can evolve to microservices if needed for specific requirements.

### Why SQLite + SQLCipher?

- **Small attack surface**: an embedded library — no database server, network port or database credentials to manage
- **Encryption**: SQLCipher encrypts and authenticates every page (complemented by field encryption of secrets)
- **Transactional**: full ACID transactions and database-enforced constraints
- **Simple self-hosting**: one process, one volume; nothing else to deploy
- **Sufficient scale**: wallet operations are small and infrequent (targets: 10k wallets, 50k+ accounts)

Trade-offs: a single instance per database (no shared-database clustering), and
backups must account for WAL files. A distributed backend would be a separate
deployment profile with its own design.

### Why MCP as Primary API?

- **Standardization**: MCP is an emerging standard for AI agent integration
- **Structure**: Typed tools with clear input/output contracts
- **Security**: Request-response paradigm, no persistent connections
- **Simplicity**: Minimal HTTP surface, easy to secure

### Why API Keys (Not OAuth2 by Default)?

- **Simplicity**: Easier to implement and manage for backend-to-backend communication
- **Stateless**: No token refresh flows, simpler infrastructure
- **Suitable**: Designed for service-to-service MCP communication
- **Customizable**: Easy to replace with OAuth2/JWT if needed (see [Customization Guide](./customization-guide.md#2-custom-authentication))

## Conclusion

Arktos Wallet is a **blueprint architecture** designed for:
- ✅ Understanding secure wallet management patterns
- ✅ Learning Rust and async architecture best practices
- ✅ Extending with custom blockchain support
- ✅ Adapting for regional compliance requirements
- ✅ Deploying as a production service

The architecture emphasizes **clarity and customizability** over sophisticated patterns, making it ideal for learning and building upon.

For more details on customization and compliance, see:
- [Customization Guide](./customization-guide.md)
- [Regional Compliance](./regional-compliance.md)
- [Development Guide](./development-guide.md)
- [Deployment Guide](./deployment-guide.md)
