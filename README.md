# Άρκτος Wallet

An **open-source, educational blueprint** for building AI-controlled non-custodial wallets. Arktos demonstrates secure wallet management, multi-account support, and blockchain integration patterns—designed for customization and regional compliance adaptation.

![](docs/bg.png)

## 🎯 What is Arktos Wallet?

Arktos Wallet is a production-ready reference implementation that showcases best practices for:

- **Secure Wallet Management**: Non-custodial wallet creation with BIP39/BIP32 cryptographic standards
- **Multi-Account Support**: Manage multiple blockchain accounts under a single system owner
- **Modern MCP**: Stateless [MCP `2026-07-28`](https://modelcontextprotocol.io/specification/2026-07-28) server built on the official `rmcp` 3.x SDK
- **API Key Authentication**: MCP access protected by per-client API keys; wallets are scoped to the key
- **Data Encryption**: AES-256 encryption at rest using SQLCipher for sensitive wallet data
- **Docker Deployment**: Multi-stage Docker builds for lean, production-ready containerization
- **Extensibility**: Designed as a customizable foundation for builders and system owners

## 🚀 Quick Start

### Prerequisites
- [rustup](https://rustup.rs/) — the Rust toolchain (currently 1.97.1, 2024 edition) is pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) and installed automatically
- A C toolchain, `make` and `perl` (SQLCipher and OpenSSL are built from source; no system SQLite needed)
- Docker (for containerized deployment)

### Development Setup
```bash
git clone <repository-url>
cd arktos-wallet
cargo build
cargo test
cargo run
```

Run `make ci` for the full set of local checks (formatting, Clippy, tests, `cargo audit`, `cargo deny`). See [CONTRIBUTING.md](./CONTRIBUTING.md) for the required tools.

For detailed development instructions, see [Development Guide](./docs/development-guide.md).

### Connecting an MCP client

Point a client that supports MCP `2026-07-28` at `http://localhost:8080/mcp` and send a client API key (created via `POST /admin/api-keys`) in the `X-API-KEY` header. Arktos only speaks `2026-07-28`: there is no `initialize` handshake or session, and clients discover the server with `server/discover`. See [API Contracts](./docs/api-contracts.md#2-mcp-entrypoint).

### Docker Deployment
```bash
docker build -t arktos-wallet:latest .
docker run -p 8080:8080 arktos-wallet:latest
```

For deployment details, see [Deployment Guide](./docs/deployment-guide.md).

## 📋 Core Features

- ✅ Wallet creation with secure recovery passphrases
- ✅ Bitcoin & Ethereum address derivation
- ✅ Multi-account management per wallet
- ✅ Encrypted SQLite database with SQLCipher
- ✅ Stateless MCP `2026-07-28` HTTP endpoint (`server/discover`, no sessions) with API key authentication
- ✅ Health check endpoint (`/healthz`)
- ✅ Comprehensive audit logging
- ✅ Stateless MCP protocol layer (persistent wallet data in a local, single-instance SQLCipher database)

## 📚 Documentation

### Project Overview
- **[Project Overview](./docs/project-overview.md)** - High-level introduction and technology stack
- **[Architecture](./docs/architecture.md)** - System design, patterns, and technical decisions
- **[Source Tree Analysis](./docs/source-tree-analysis.md)** - Project structure and module organization

### Development & Deployment
- **[Development Guide](./docs/development-guide.md)** - Setup, building, testing, and local development
- **[Deployment Guide](./docs/deployment-guide.md)** - Containerization, environment configuration, and production deployment

### API & Data
- **[API Contracts](./docs/api-contracts.md)** - HTTP endpoints and MCP tools specification
- **[Data Models](./docs/data-models.md)** - Database schema, wallet structure, and account management

### Extensibility
- **[Customization Guide](./docs/customization-guide.md)** - Adding new blockchain support, custom authentication, and feature extension patterns
- **[Regional Compliance](./docs/regional-compliance.md)** - Adapting Arktos for compliance requirements and privacy regulations

For a complete documentation index, see [Documentation Index](./docs/index.md).

## 🔧 Customization for Your Needs

Arktos is designed as a **customizable blueprint**. System owners can adapt it for specific requirements:

### Add New Blockchain Support
Extend the architecture to support additional blockchains (e.g., Solana, Polkadot) by implementing custom key derivation modules. See [Customization Guide](./docs/customization-guide.md#adding-blockchain-support) for patterns and examples.

### Integrate Custom Authentication
Replace API key authentication with your identity provider (e.g., OAuth2, JWT, mTLS). The modular security layer allows seamless substitution. See [Customization Guide](./docs/customization-guide.md#custom-authentication).

### Adapt for Regional Compliance
The architecture supports encryption and audit logging requirements for GDPR, HIPAA, and other regulations. See [Regional Compliance](./docs/regional-compliance.md) for guidance.

### Customize Database & Storage
Swap SQLite for PostgreSQL, MongoDB, or other storage backends while maintaining the same API contract.

## 🔐 Security & Privacy

- **Encryption at Rest**: AES-256 encryption via SQLCipher
- **Secure Communication**: TLS 1.2+ for all communication
- **API Key Management**: Secure API key storage and validation
- **Audit Logging**: Comprehensive logging of critical wallet operations
- **No Custodial Control**: System owners maintain full control of encryption keys

For security details, see [Architecture](./docs/architecture.md#security-architecture).

## 📊 Performance Characteristics

- **Wallet Creation**: < 500ms (p95)
- **Address Retrieval**: < 100ms (p95)
- **Concurrency**: 100+ req/s (target, single instance)
- **Database Capacity**: 10,000 wallets, 50,000+ accounts
- **Uptime Target**: 99.9% (production deployment)

## 🛠️ Technology Stack

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Rust | 1.97 (2024 edition) | Type-safe, high-performance backend |
| Web Framework | Axum | 0.8+ | Async HTTP server |
| Async Runtime | Tokio | 1.x | Non-blocking I/O |
| Database | SQLite + SQLCipher | 3.x | Encrypted local persistence |
| Protocol | MCP (Model Context Protocol) | spec `2026-07-28`, rmcp 3.x | AI agent integration (stateless HTTP) |
| Cryptography | secp256k1, bip39, bip32 | Latest | Blockchain standards |
| Serialization | Serde | 1.x | Data encoding (JSON, binary) |

## 🤝 Contributing

Arktos Wallet is an open-source educational project. Contributions, forks, and adaptations are encouraged!

- **For enhancements**: Open issues and pull requests — see [CONTRIBUTING.md](./CONTRIBUTING.md)
- **For security issues**: Report privately as described in [SECURITY.md](./SECURITY.md); do not open public issues
- **For custom implementations**: This repository serves as a reference—fork and adapt it to your specific needs
- **For compliance work**: See [Regional Compliance Guide](./docs/regional-compliance.md) for patterns and considerations

## 📝 License

> **TODO (repository owner):** No license has been chosen yet. Until a `LICENSE` file is added, no license is granted and default copyright applies. A license must be selected before any release. The crate is marked `publish = false` in the meantime.

## 🆘 Support & Community

- **Documentation**: See [docs/](./docs/) for comprehensive guides
- **Issues**: Report bugs or request features via GitHub Issues
- **Discussions**: Join discussions for implementation patterns and architectural questions

---

**Arktos Wallet is a blueprint—not a one-size-fits-all solution. Customize, extend, and adapt it for your regional and organizational needs.**
