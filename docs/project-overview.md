# Project Overview: Arktos Wallet

## Executive Summary

**Arktos Wallet** is an **open-source, educational blueprint** for self-hosted wallet capability services for AI agents: agents can create wallets and obtain public addresses, and the MCP surface returns no recovery phrases, seeds or private keys. Whoever operates an instance holds its keys and has effective custody of the stored wallet secrets. Arktos does not sign or broadcast transactions. It demonstrates secure wallet management, multi-account support, and blockchain integration patterns—designed for customization and regional compliance adaptation.

Arktos is a backend HTTP server developed in Rust. It functions as a monolithic service designed to act as an "HTTP MCP server" implementing [MCP `2026-07-28`](https://modelcontextprotocol.io/specification/2026-07-28) over the stateless Streamable HTTP transport, using the official `rmcp` 3.x SDK. The technology stack is modern and asynchronous, built on the Tokio runtime and the Axum web framework.

## Key Features

### 🎯 Core Capabilities

- **Secure Wallet Creation**: Self-hosted wallets with BIP39/BIP32 cryptographic standards
- **Multi-Account Support**: Manage multiple blockchain accounts under a single wallet
- **API Key Authentication**: Secure MCP server with API key-based authentication
- **Data Encryption**: SQLCipher database encryption plus AES-256-GCM field encryption of wallet secrets (HKDF-derived, purpose-separated keys)
- **Docker Deployment**: Multi-stage Docker builds for lean, production-ready containerization
- **Operation Logging**: Tool calls and wallet operations logged with API-key ids and public data only

### 🔧 Design Philosophy

Arktos is intentionally designed as a **customizable blueprint** for system owners:

- **Modular Architecture**: Clean separation of concerns enables easy extension
- **Self-Hosted Custody**: Whoever operates the instance controls the encryption keys; no third-party custodian when the wallet owner operates it
- **Stateless MCP Protocol**: No MCP sessions; every request is independent (wallet data itself is persistent)
- **Security-First Design**: Encryption, authentication, and authorization by default
- **Compliance-Ready**: Built-in patterns for GDPR, HIPAA, and other regulations

## Technology Stack Summary

| Category          | Technology            | Purpose                           |
|-------------------|-----------------------|-----------------------------------|
| **Language**      | Rust (2024 Edition)   | Type-safe, memory-safe backend    |
| **Web Framework** | Axum                  | Async HTTP server                 |
| **Async Runtime** | Tokio                 | Non-blocking I/O                  |
| **Database**      | SQLite + SQLCipher    | Encrypted local persistence       |
| **Protocol**      | MCP `2026-07-28` via `rmcp` 3.x | AI agent integration (stateless HTTP) |
| **Serialization** | Serde + JSON          | Data encoding                     |
| **Cryptography**  | secp256k1, bip39, bip32 | Blockchain standards            |

## Repository Structure

The project is a **monolith**, with a single, cohesive codebase designed to be simple enough for understanding yet production-ready.

```
arktos-wallet/
├── src/
│   ├── main.rs          # Application entry point, configuration, graceful shutdown
│   ├── app.rs           # HTTP router: /healthz, /readyz, /admin/*, /mcp
│   ├── mcp.rs           # MCP tools and stateless MCP transport
│   ├── database.rs      # SQLCipher connection, configuration, migrations, blocking boundary
│   ├── key_store.rs     # API-key persistence (SQL)
│   ├── wallet_store.rs  # Wallet/account persistence (SQL)
│   ├── crypto.rs, keys.rs  # Field encryption and key hierarchy
│   └── ...              # Services, MCP, auth
├── migrations/          # Versioned SQL migrations (V1__initial_schema.sql, …)
├── docs/                # Complete documentation
│   ├── index.md         # Documentation index: Learn / Reference / Operate
│   ├── CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md  # Learning path (front door)
│   ├── capability/      # Educational layer
│   │   ├── README.md
│   │   ├── 01-model-cryptographic-authority.md … 08-recovery-is-part-of-security.md
│   │   ├── SECRET-LIFECYCLE-WALKTHROUGH.md
│   │   ├── CASE-STUDIES.md
│   │   └── CHALLENGES.md
│   ├── architecture.md  # System design and patterns
│   ├── api-contracts.md # HTTP endpoints and MCP tools
│   ├── data-models.md   # Database schema
│   ├── development-guide.md  # Local development setup
│   ├── deployment-guide.md   # Production deployment
│   ├── customization-guide.md # How to extend Arktos
│   └── regional-compliance.md # Compliance patterns
├── tests/               # Integration tests
├── scripts/             # Documentation checker (make docs-check)
├── Cargo.toml          # Rust dependencies and metadata
├── Dockerfile          # Multi-stage Docker build
└── README.md           # Quick start guide
```

## Architecture Pattern

The application follows an **API-centric architecture** with these key characteristics:

### Layered Design

1. **HTTP Layer** (Axum Router)
   - Route handling
   - Request/response serialization
   - Authentication middleware

2. **API Handler Layer**
   - MCP tool execution
   - Request validation
   - Response formatting

3. **Business Logic Layer**
   - Wallet creation & management
   - Key derivation (Bitcoin, Ethereum, etc.)
   - Account management

4. **Data Access Layer**
   - Database queries
   - Encryption/decryption
   - Transaction management

5. **Persistence Layer** (SQLCipher)
   - Encrypted SQLite database
   - Wallet data & accounts
   - API-key hashes

### Stateless MCP, Persistent Data

The MCP protocol layer keeps no session or transport state between requests:
- ✅ No `initialize` handshake or `Mcp-Session-Id`; clients use `server/discover`
- ✅ No sticky sessions: any request can be served on its own
- ✅ Simple restarts: no MCP session state to lose

Wallets, accounts and API keys are persistent and stored in a local SQLCipher
database, so a deployment currently runs as a **single instance** (see
[Performance & Scalability](#performance--scalability)).

## Core Functionality

### MCP Tools (Primary API)

All wallet functionality is exposed via Model Context Protocol (MCP) tools:

0. **`ping`** - Liveness check of the MCP tool router
1. **`create_wallet`** - Create a wallet (encrypted BIP39 recovery phrase); returns `wallet_id`, `wallet_name`, `created_at`
2. **`get_bitcoin_address`** - BIP86 Taproot address on the configured Bitcoin network, with derivation path and public key
3. **`get_ethereum_address`** - EIP-55 Ethereum address (BIP44 path) with the configured chain ID

All wallet tools return structured JSON described by output schemas in `tools/list`; see [API Contracts](./api-contracts.md#mcp-tools).

### HTTP Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/healthz` | GET | Liveness probe |
| `/readyz` | GET | Readiness probe (database accessible) |
| `/mcp` | POST | MCP `2026-07-28` endpoint (client API key required) |
| `/admin/api-keys` (+ `/{id}/revoke`, `/{id}/rotate`) | GET/POST | API key administration (admin key required) |

## Security by Default

- ✅ **Encryption at Rest**: SQLCipher (`DATABASE_KEY`) plus AES-256-GCM field encryption under keys derived from `MASTER_KEY` ([details](./architecture.md#key-hierarchy--secret-storage))
- ✅ **Encryption in Transit**: TLS 1.2+ terminated in front of Arktos
- ✅ **API Key Authentication**: Secure MCP endpoint access
- ✅ **Audit Logging**: All operations logged with timestamp, actor, action
- ✅ **Access Control**: Ownership-based authorization
- ✅ **Self-Hosted Custody**: The operator controls all encryption keys (see [README — Custody](../README.md))

See [Architecture Document](./architecture.md#6-security-architecture) for detailed security patterns.

## Getting Started

### For Understanding the Project

1. Read this [Project Overview](./project-overview.md) (you're reading it!)
2. Review [Architecture](./architecture.md) for system design
3. Explore code in `src/` directory

### For Local Development

1. Follow [Development Guide](./development-guide.md)
2. Set up Rust development environment
3. Build and test locally: `cargo build && cargo test`
4. Run server: `cargo run`

### For Deployment

1. Read [Deployment Guide](./deployment-guide.md)
2. Build Docker image: `docker build -t arktos-wallet .`
3. Configure environment variables
4. Deploy to your infrastructure

### For Integration

1. Review [API Contracts](./api-contracts.md)
2. Understand MCP tool interface
3. Integrate AI agent to call Arktos MCP tools

## Customization & Extension

Arktos is designed for customization. Common extensions:

- **Add Blockchain Support**: Solana, Polkadot, or custom chains
  → See [Customization Guide](./customization-guide.md#1-adding-blockchain-support)

- **Custom Authentication**: OAuth2, JWT, mTLS
  → See [Customization Guide](./customization-guide.md#2-custom-authentication)

- **Alternative Database**: PostgreSQL, MongoDB
  → See [Customization Guide](./customization-guide.md#3-storage-backend-customization)

- **Extend API**: Add custom endpoints or MCP tools
  → See [Customization Guide](./customization-guide.md#4-api-customization)

## Compliance & Regional Adaptation

Arktos architecture supports adaptation for various compliance frameworks:

### Supported Regulations

- 🇪🇺 **GDPR** (Europe) - Data minimization, deletion rights, portability
- 🇺🇸 **HIPAA** (Healthcare) - Encryption, audit logs, access control
- 🇺🇸 **PCI DSS** (Payment) - Network security, encryption (if integrated)
- 🏢 **SOC 2** (Service Organization) - Security, availability, integrity
- 🌍 **Regional** - PDPA (Singapore), LGPD (Brazil), Privacy Act (Canada/Australia)

### Compliance Features

- ✅ Data encryption at rest and in transit
- ✅ Audit logging of all operations
- ✅ Access control and authentication
- ✅ Data minimization and retention policies
- ✅ Right to deletion (GDPR) and data export
- ✅ Encryption key management patterns

See [Regional Compliance Guide](./regional-compliance.md) for detailed implementation patterns.

## Performance & Scalability

### Performance Targets

- **Wallet Creation**: < 500ms (p95)
- **Address Retrieval**: < 100ms (p95)
- **Concurrency**: 100+ req/s (target)
- **Database Capacity**: 10,000 wallets, 50,000+ accounts per instance
- **Uptime Target**: 99.9% (production deployment)

### Scalability Pattern

```
MCP clients ──► Arktos (single instance) ──► SQLCipher file (local volume)
```

- **MCP protocol layer**: stateless and horizontally routable — requests carry
  everything needed to serve them.
- **Application deployment**: a single Arktos instance per database.
- **Persistence**: a local SQLCipher (SQLite) database with versioned migrations,
  enforced foreign keys and WAL. SQLite is an intentional choice for a
  self-hosted wallet (no database server or credentials, transactional,
  encrypted). Sharing one database file between instances is not supported.

## Key Documentation

For detailed information on specific topics:

| Topic | Document |
|-------|----------|
| **Learning path** | [Cryptographic Capability Learning Path](./CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md) |
| **Architecture & Design** | [Architecture](./architecture.md) |
| **API Specification** | [API Contracts](./api-contracts.md) |
| **Database Schema** | [Data Models](./data-models.md) |
| **Local Development** | [Development Guide](./development-guide.md) |
| **Production Deployment** | [Deployment Guide](./deployment-guide.md) |
| **Customization Patterns** | [Customization Guide](./customization-guide.md) |
| **Compliance & Security** | [Regional Compliance](./regional-compliance.md) |

## About Arktos as a Blueprint

Arktos is intentionally **not** a complete, off-the-shelf solution. Instead, it serves as:

- 📚 **Educational Reference**: Learn secure wallet patterns in Rust
- 🏗️ **Customizable Foundation**: Build your specific wallet service
- 📋 **Compliance Framework**: Adapt for your regional requirements
- 🔌 **Integration Gateway**: Connect AI agents to blockchain

Think of Arktos as a **well-documented starter project** rather than a black-box service. You're expected to:
- Understand the architecture and design
- Customize it for your specific needs
- Adapt it for your compliance requirements
- Test it thoroughly in your environment

This approach provides maximum control and flexibility while demonstrating best practices.

## Next Steps

1. **Explore the Architecture**: Read [Architecture Document](./architecture.md)
2. **Review the Code**: Check the `src/` directory
3. **Set Up Development**: Follow [Development Guide](./development-guide.md)
4. **Plan Customizations**: Review [Customization Guide](./customization-guide.md) for extension patterns
5. **Check Compliance**: Read [Regional Compliance](./regional-compliance.md) for your requirements

---

**Arktos Wallet: A blueprint for secure, customizable, AI-powered wallet management.**

## Key Documentation

*   [Architecture](./architecture.md)
*   [Development Guide](./development-guide.md)
