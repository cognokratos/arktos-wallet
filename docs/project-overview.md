# Project Overview: Arktos Wallet

## Executive Summary

**Arktos Wallet** is an **open-source, educational blueprint** for building AI-controlled non-custodial wallets. It demonstrates secure wallet management, multi-account support, and blockchain integration patterns—designed for customization and regional compliance adaptation.

Arktos is a backend HTTP server developed in Rust. It functions as a monolithic service designed to act as an "HTTP MCP server" (Model Context Protocol). The technology stack is modern and asynchronous, built on the Tokio runtime and the Axum web framework.

## Key Features

### 🎯 Core Capabilities

- **Secure Wallet Creation**: Non-custodial wallets with BIP39/BIP32 cryptographic standards
- **Multi-Account Support**: Manage multiple blockchain accounts under a single wallet
- **API Key Authentication**: Secure MCP server with API key-based authentication
- **Data Encryption**: AES-256 encryption at rest using SQLCipher for sensitive data
- **Docker Deployment**: Multi-stage Docker builds for lean, production-ready containerization
- **Audit Logging**: Comprehensive logging of all critical wallet operations

### 🔧 Design Philosophy

Arktos is intentionally designed as a **customizable blueprint** for system owners:

- **Modular Architecture**: Clean separation of concerns enables easy extension
- **Non-Custodial Model**: System owners maintain complete control of encryption keys
- **Stateless Service**: Enables horizontal scaling and resilience
- **Security-First Design**: Encryption, authentication, and authorization by default
- **Compliance-Ready**: Built-in patterns for GDPR, HIPAA, and other regulations

## Technology Stack Summary

| Category          | Technology            | Purpose                           |
|-------------------|-----------------------|-----------------------------------|
| **Language**      | Rust (2024 Edition)   | Type-safe, memory-safe backend    |
| **Web Framework** | Axum                  | Async HTTP server                 |
| **Async Runtime** | Tokio                 | Non-blocking I/O                  |
| **Database**      | SQLite + SQLCipher    | Encrypted local persistence       |
| **Protocol**      | RMCP (MCP SDK)        | AI agent integration              |
| **Serialization** | Serde + JSON          | Data encoding                     |
| **Cryptography**  | secp256k1, bip39, bip32 | Blockchain standards            |

## Repository Structure

The project is a **monolith**, with a single, cohesive codebase designed to be simple enough for understanding yet production-ready.

```
arktos-wallet/
├── src/
│   ├── main.rs          # Application entry point, HTTP server, MCP handling
│   ├── database.rs      # Database operations, encryption/decryption
│   ├── wallet.rs        # Wallet management logic
│   └── ...              # Additional modules as needed
├── db/
│   └── migrations/       # Database migration scripts (refinery)
├── docs/                # Complete documentation
│   ├── index.md         # Documentation index (you are here!)
│   ├── architecture.md  # System design and patterns
│   ├── api-contracts.md # HTTP endpoints and MCP tools
│   ├── data-models.md   # Database schema
│   ├── development-guide.md  # Local development setup
│   ├── deployment-guide.md   # Production deployment
│   ├── customization-guide.md # How to extend Arktos
│   └── regional-compliance.md # Compliance patterns
├── tests/               # Integration tests
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
   - Audit logs

### Stateless Design

No in-memory state between requests enables:
- ✅ Horizontal scaling (multiple instances)
- ✅ Load balancing
- ✅ Fault tolerance (instance replacement without data loss)
- ✅ Simplified operations and monitoring

## Core Functionality

### MCP Tools (Primary API)

All wallet functionality is exposed via Model Context Protocol (MCP) tools:

1. **`create_wallet`** - Create new wallet with recovery passphrase
2. **`get_bitcoin_address`** - Derive Bitcoin address for wallet
3. **`get_ethereum_address`** - Derive Ethereum address for wallet

### HTTP Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/healthz` | GET | Health check / liveness probe |
| `/mcp` | POST | Model Context Protocol endpoint (auth required) |

## Security by Default

- ✅ **Encryption at Rest**: AES-256 via SQLCipher
- ✅ **Encryption in Transit**: TLS 1.2+ required
- ✅ **API Key Authentication**: Secure MCP endpoint access
- ✅ **Audit Logging**: All operations logged with timestamp, actor, action
- ✅ **Access Control**: Ownership-based authorization
- ✅ **Non-Custodial**: System owner controls all encryption keys

See [Architecture Document](./architecture.md#6-security-architecture) for detailed security patterns.

## Getting Started

### For Understanding the Project

1. Read this [Project Overview](./project-overview.md) (you're reading it!)
2. Review [Architecture](./architecture.md) for system design
3. Check [Source Tree Analysis](./source-tree-analysis.md) for project structure
4. Explore code in `src/` directory

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
- **Concurrency**: 100+ req/s with horizontal scaling
- **Database Capacity**: 10,000 wallets, 50,000+ accounts per instance
- **Uptime Target**: 99.9% (production deployment)

### Scalability Pattern

```
    ┌──────────────────┐
    │  Load Balancer   │
    └────────┬─────────┘
             │
    ┌────────┼────────┐
    ▼        ▼        ▼
┌────────┬────────┬────────┐
│Arktos 1│Arktos 2│Arktos N│
└────────┴───┬────┴────────┘
              │
         ┌────▼──────┐
         │ SQLCipher │
         │ Database  │
         └───────────┘
```

Each instance is stateless and can be scaled horizontally with load balancing and shared encrypted database.

## Key Documentation

For detailed information on specific topics:

| Topic | Document |
|-------|----------|
| **Architecture & Design** | [Architecture](./architecture.md) |
| **Project Structure** | [Source Tree Analysis](./source-tree-analysis.md) |
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
2. **Review the Code**: Check `src/` directory and [Source Tree](./source-tree-analysis.md)
3. **Set Up Development**: Follow [Development Guide](./development-guide.md)
4. **Plan Customizations**: Review [Customization Guide](./customization-guide.md) for extension patterns
5. **Check Compliance**: Read [Regional Compliance](./regional-compliance.md) for your requirements

---

**Arktos Wallet: A blueprint for secure, customizable, AI-powered wallet management.**

## Key Documentation

*   [Architecture](./architecture.md)
*   [Source Tree Analysis](./source-tree-analysis.md)
*   [Development Guide](./development-guide.md)
