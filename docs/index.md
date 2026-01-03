# Project Documentation Index: Arktos Wallet

This document is the master index for all documentation related to the Arktos Wallet project. It is intended to be the primary entry point for developers and AI assistants.

## 📖 Documentation Sections

### 🚀 Project Overview & Quick Start

- **[Project Overview](./project-overview.md)** - Executive summary, technology stack, and repository structure
- **[README.md](../README.md)** - Main project entry point with quick start guide and core features
- **Type:** Monolithic service
- **Primary Language:** Rust (2024 edition)
- **Architecture:** API-centric with MCP (Model Context Protocol) interface
- **Tech Stack:** Rust, Axum, Tokio, SQLCipher, RMCP SDK

### 🏗️ Architecture & Design

- **[Architecture](./architecture.md)** - Complete system design, patterns, technology stack decisions, and API design
- **[Source Tree Analysis](./source-tree-analysis.md)** - Project structure, module organization, and file layout
- **[Data Models](./data-models.md)** - Database schema, wallet structure, account management, and data relationships

### 🔌 API & Integration

- **[API Contracts](./api-contracts.md)** - HTTP endpoints, MCP tools specification, request/response formats, and error codes
- **[MCP Tools]** - `create_wallet`, `get_bitcoin_address`, `get_ethereum_address` exposed via `/mcp` endpoint
- **[Health Check]** - `/healthz` endpoint for system monitoring

### 📚 Development & Deployment

- **[Development Guide](./development-guide.md)** - Local setup, building, testing, and debugging instructions
- **[Deployment Guide](./deployment-guide.md)** - Docker containerization, environment configuration, and production deployment

### 🎯 Customization & Extensibility

- **[Customization Guide](./customization-guide.md)** - Patterns for adding blockchains, custom authentication, storage backends, and extending API
  - Adding blockchain support (e.g., Solana, Polkadot)
  - Custom authentication (OAuth2, JWT, mTLS)
  - Storage backend customization (PostgreSQL, MongoDB)
  - API extension patterns
  - Testing customizations
  - Deployment considerations

### 🔐 Regional Compliance & Security

- **[Regional Compliance](./regional-compliance.md)** - Compliance frameworks and implementation patterns
  - GDPR (Europe) - Data minimization, right to deletion, data portability
  - HIPAA (Healthcare - USA) - Encryption standards, audit logging, access control
  - PCI DSS (Payment Card Industry) - Security guidelines
  - SOC 2 (Service Organization Control) - Security, availability, integrity
  - Custom regional compliance (PDPA, LGPD, etc.)
  - Encryption key management
  - Compliance validation and testing

---

## 🧭 Getting Started Paths

### I want to understand Arktos at a high level
1. Start with [README.md](../README.md)
2. Read [Project Overview](./project-overview.md)
3. Explore [Architecture](./architecture.md)

### I want to set up development environment
1. Read [Development Guide](./development-guide.md)
2. Check [Source Tree Analysis](./source-tree-analysis.md) for project structure
3. Review [Data Models](./data-models.md) for database understanding

### I want to integrate with Arktos API
1. Start with [API Contracts](./api-contracts.md)
2. Review [Architecture](./architecture.md#section-5-api-design) for design patterns
3. Check examples in integration tests

### I want to customize Arktos for my needs
1. Read [Customization Guide](./customization-guide.md)
2. Review relevant customization section:
   - Adding blockchain support
   - Custom authentication
   - Storage backend adaptation
3. See pattern examples in guide
4. Consult [Architecture](./architecture.md) for design principles

### I need to ensure regulatory compliance
1. Read [Regional Compliance](./regional-compliance.md)
2. Identify applicable regulations (GDPR, HIPAA, etc.)
3. Review implementation patterns for your region
4. Use compliance validation checklist provided

### I want to deploy to production
1. Read [Deployment Guide](./deployment-guide.md)
2. Review [Architecture](./architecture.md#section-7-development--deployment) for deployment patterns
3. Check [Regional Compliance](./regional-compliance.md#8-compliance-deployment-checklist)
4. Ensure all compliance requirements are met

---

## 📋 Quick Reference

### Key Files & Directories

| Path | Purpose |
|------|---------|
| `README.md` | Project entry point |
| `src/main.rs` | Application entry point |
| `src/db.rs` | Database module |
| `Cargo.toml` | Project dependencies |
| `Dockerfile` | Docker build configuration |
| `docs/` | Complete documentation |
| `db/migrations/` | Database migration scripts |

### Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Language | Rust 2024 edition | Type-safe backend |
| Framework | Axum 0.8+ | HTTP server |
| Runtime | Tokio 1.x | Async I/O |
| Database | SQLite + SQLCipher | Encrypted persistence |
| Protocol | MCP (rmcp 0.12+) | AI agent integration |
| Crypto | secp256k1, bip39, bip32 | Blockchain standards |
| Serialization | Serde 1.x | JSON/binary encoding |

### Core MCP Tools

- `create_wallet` - Create new non-custodial wallet with recovery passphrase
- `get_bitcoin_address` - Derive and retrieve Bitcoin address for wallet
- `get_ethereum_address` - Derive and retrieve Ethereum address for wallet

### Endpoints

- `GET /healthz` - Health check
- `POST /mcp` - Model Context Protocol endpoint (authentication required)

---

## 🔗 Document Cross-References

### Architecture-Related Questions
→ See [Architecture](./architecture.md)

### Implementation Details
→ See [Development Guide](./development-guide.md)

### How to Deploy
→ See [Deployment Guide](./deployment-guide.md)

### API Integration
→ See [API Contracts](./api-contracts.md)

### Database Schema
→ See [Data Models](./data-models.md)

### Adding Features
→ See [Customization Guide](./customization-guide.md)

### Compliance & Security
→ See [Regional Compliance](./regional-compliance.md)

---

## 📞 Support Resources

For questions or issues:
1. Check relevant documentation section above
2. Review [Customization Guide](./customization-guide.md) for implementation patterns
3. Refer to [Regional Compliance](./regional-compliance.md) for compliance-specific guidance
4. Check project README for community resources

---

**Last Updated:** January 2026
**Status:** Complete - Ready for Production
**Arktos Wallet:** Open-source blueprint for AI-controlled non-custodial wallets
