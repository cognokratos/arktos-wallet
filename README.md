# Άρκτος Wallet

An **open-source, educational blueprint** for building AI-controlled non-custodial wallets. Arktos demonstrates secure wallet management, multi-account support, and blockchain integration patterns—designed for customization and regional compliance adaptation.

![](docs/bg.png)

## 🎯 What is Arktos Wallet?

Arktos Wallet is a production-ready reference implementation that showcases best practices for:

- **Secure Wallet Management**: Non-custodial wallet creation with BIP39/BIP32 cryptographic standards
- **Multi-Account Support**: Manage multiple blockchain accounts under a single system owner
- **API Key Authentication**: Secure MCP (Model Context Protocol) server with API key-based authentication
- **Data Encryption**: AES-256 encryption at rest using SQLCipher for sensitive wallet data
- **Docker Deployment**: Multi-stage Docker builds for lean, production-ready containerization
- **Extensibility**: Designed as a customizable foundation for builders and system owners

## 🚀 Quick Start

### Prerequisites
- Rust 2024 edition
- Docker (for containerized deployment)
- SQLite 3.x

### Development Setup
```bash
git clone <repository-url>
cd arktos-wallet
cargo build
cargo test
cargo run
```

For detailed development instructions, see [Development Guide](./docs/development-guide.md).

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
- ✅ HTTP MCP endpoint with API key authentication
- ✅ Health check endpoint (`/healthz`)
- ✅ Comprehensive audit logging
- ✅ Stateless microservice architecture for horizontal scaling

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
- **Concurrency**: 100+ req/s with horizontal scaling
- **Database Capacity**: 10,000 wallets, 50,000+ accounts
- **Uptime Target**: 99.9% (production deployment)

## 🛠️ Technology Stack

| Component | Technology | Version | Purpose |
|-----------|-----------|---------|---------|
| Language | Rust | 2024 edition | Type-safe, high-performance backend |
| Web Framework | Axum | 0.8+ | Async HTTP server |
| Async Runtime | Tokio | 1.x | Non-blocking I/O |
| Database | SQLite + SQLCipher | 3.x | Encrypted local persistence |
| Protocol | MCP (Model Context Protocol) | rmcp 0.12+ | AI agent integration |
| Cryptography | secp256k1, bip39, bip32 | Latest | Blockchain standards |
| Serialization | Serde | 1.x | Data encoding (JSON, binary) |

## 🤝 Contributing

Arktos Wallet is an open-source educational project. Contributions, forks, and adaptations are encouraged!

- **For enhancements**: Open issues and pull requests
- **For custom implementations**: This repository serves as a reference—fork and adapt it to your specific needs
- **For compliance work**: See [Regional Compliance Guide](./docs/regional-compliance.md) for patterns and considerations

## 📝 License

[Specify your license here]

## 🆘 Support & Community

- **Documentation**: See [docs/](./docs/) for comprehensive guides
- **Issues**: Report bugs or request features via GitHub Issues
- **Discussions**: Join discussions for implementation patterns and architectural questions

---

**Arktos Wallet is a blueprint—not a one-size-fits-all solution. Customize, extend, and adapt it for your regional and organizational needs.**
