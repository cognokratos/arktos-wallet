# Arktos Wallet - Architecture

This document outlines the architecture of the Arktos Wallet application, positioning it as an open-source blueprint for AI-controlled non-custodial wallets.

## 1. Executive Summary

**Arktos Wallet** is a backend HTTP server built with Rust, designed as a **customizable blueprint** for secure wallet management. It is implemented as a single, cohesive monolithic service acting as an "HTTP MCP server" with API-centric design.

### Blueprint Positioning

Arktos serves as an **open-source educational reference implementation** demonstrating:

- **Secure wallet creation** with BIP39/BIP32 cryptographic standards
- **Multi-account management** for blockchain address derivation
- **Non-custodial architecture** where system owners control all encryption keys
- **API key authentication** for secure Model Context Protocol (MCP) integration
- **Encrypted data persistence** using AES-256 via SQLCipher
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
| MCP SDK           | rmcp (MCP Rust SDK)   | 0.12+        | Protocol implementation for AI agent integration |
| Database          | SQLite + SQLCipher    | 3.x          | Lightweight, encrypted local persistence |
| Serialization     | Serde, Serde JSON     | 1.x          | Efficient, zero-copy serialization |
| Schema Generation | Schemars              | 1.x          | JSON schema generation for API documentation |
| Logging           | Tracing               | 0.1+         | Structured, composable logging framework |
| Cryptography      | secp256k1, bip39, bip32, tiny-keccak | Latest | Industry-standard blockchain key derivation |

## 3. Architecture Pattern

Arktos follows a **layered API-centric architecture** optimized for:
- **Stateless operation** (enables horizontal scaling)
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
│ - Audit logs                                │
└─────────────────────────────────────────────┘
```

### Key Architectural Principles

1. **Stateless Design**: No session state kept between requests, enabling horizontal scaling
2. **Security by Default**: All sensitive data encrypted at rest; TLS required in transit
3. **Non-Custodial Model**: System owner maintains complete control of encryption keys
4. **Single Responsibility**: Each module handles one concern (wallet management, auth, data access)
5. **API-First Design**: Core functionality exposed via standardized MCP tools
6. **Auditability**: All critical operations logged for compliance and debugging

## 4. Data Architecture

### Data Model

The application uses **encrypted SQLite** for persistence, managed via SQLCipher:

```
Wallet Table:
├── id (UUID)
├── owner_id (reference to system owner)
├── encrypted_mnemonic (AES-256)
├── created_at
├── updated_at
└── metadata

Account Table:
├── id (UUID)
├── wallet_id (FK)
├── blockchain (bitcoin, ethereum, solana, etc.)
├── account_index (for HD wallet derivation)
├── public_address (derived, non-sensitive)
└── created_at
```

### Encryption Strategy

- **At Rest**: All wallet data encrypted with AES-256 via SQLCipher
- **In Transit**: TLS 1.2+ required for all HTTP communication
- **Key Management**: System owner manages master encryption key (not stored in Arktos)

See [Data Models](./data-models.md) for detailed schema and [Regional Compliance](./regional-compliance.md) for encryption key management patterns.

## 5. API Design

The server exposes a minimal, intentionally-constrained HTTP interface to maximize security:

### HTTP Endpoints

| Endpoint | Method | Purpose | Authentication |
|----------|--------|---------|-----------------|
| `/healthz` | GET | Health check / liveness probe | None |
| `/mcp` | POST | Model Context Protocol endpoint | API Key |

### MCP Tools (Core API)

Wallet functionality is exposed exclusively via MCP tools:

1. **`create_wallet`**
   - Creates new non-custodial wallet
   - Generates BIP39 recovery passphrase
   - Returns wallet ID for future operations

2. **`get_bitcoin_address`**
   - Derives Bitcoin address for wallet
   - Uses BIP32 HD wallet standard
   - Path: `m/44'/0'/0'/0/{account_index}`

3. **`get_ethereum_address`**
   - Derives Ethereum address for wallet
   - Uses Ethereum HD wallet derivation
   - Path: `m/44'/60'/0'/0/{account_index}`

### Design Rationale

- **MCP-Centric**: All wallet operations via MCP ensures structured, typed interactions
- **API Key Auth**: Simple, stateless authentication suitable for service-to-service communication
- **No Sensitive Data in Responses**: Public addresses only; private keys never transmitted
- **Minimal HTTP Surface**: Only two HTTP endpoints reduces attack surface

For complete API specification, see [API Contracts](./api-contracts.md).

## 6. Security Architecture

### Authentication & Authorization

```
MCP Request Flow:
┌──────────────────┐
│ AI Agent/Client  │
└────────┬─────────┘
         │
         ▼ (API Key in Authorization header)
┌────────────────────────────────────────┐
│ API Key Validation Middleware          │
│ - Verify key exists in database        │
│ - Verify key not revoked               │
└────────┬───────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ Authorization Check                    │
│ - Verify ownership of requested wallet │
│ - Enforce principle of least privilege │
└────────┬───────────────────────────────┘
         │
         ▼
┌────────────────────────────────────────┐
│ Execute MCP Tool                       │
│ - Log operation for audit trail        │
│ - Perform business logic               │
│ - Return only public data              │
└────────────────────────────────────────┘
```

### Data Protection

1. **Encryption at Rest**: AES-256 via SQLCipher for all sensitive wallet data
2. **Encryption in Transit**: TLS 1.2+ required for all HTTP communication
3. **Key Management**: Master key controlled by system owner, not stored in Arktos
4. **Access Control**: API key-based authentication, ownership-based authorization
5. **Audit Logging**: All critical operations logged with timestamp, actor, action

### Defense in Depth

- Input validation on all API parameters
- Secure error handling (no sensitive data in error messages)
- SQL injection prevention via parameterized queries
- CSRF protection via MCP token validation
- Vulnerability scanning via `cargo audit`
- Code quality via `cargo clippy` and formatting via `cargo fmt`

## 7. Performance & Scalability

### Performance Targets (NFR-compliant)

- **Wallet Creation**: < 500ms (p95) - cryptographic key generation
- **Address Retrieval**: < 100ms (p95) - deterministic derivation
- **Concurrency**: 100+ req/s with horizontal scaling
- **Database Capacity**: 10,000 wallets, 50,000+ accounts per instance
- **Uptime Target**: 99.9% (production deployment with monitoring)

### Scalability Pattern

Arktos is designed as a **stateless microservice**:

```
                    ┌──────────────────┐
                    │  Load Balancer   │
                    └────────┬─────────┘
                             │
        ┌────────────────────┼────────────────────┐
        ▼                    ▼                    ▼
    ┌────────┐          ┌────────┐          ┌────────┐
    │Arktos 1│          │Arktos 2│  ...     │Arktos N│
    └────┬───┘          └────┬───┘          └────┬───┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                       ┌─────▼──────┐
                       │   SQLite   │
                       │ (Encrypted)│
                       └────────────┘
```

Each instance:
- Has no local state
- Uses shared encrypted database
- Can be scaled horizontally
- Can be replaced without data loss

## 8. Customization & Extension Points

Arktos is designed for customization. Key extension areas:

### Blockchain Support
Add new blockchains by implementing key derivation modules (see [Customization Guide](./customization-guide.md#1-adding-blockchain-support)).

### Authentication
Replace API key auth with OAuth2, JWT, or mTLS (see [Customization Guide](./customization-guide.md#2-custom-authentication)).

### Storage Backend
Migrate to PostgreSQL, MongoDB, or other backends while maintaining API contract (see [Customization Guide](./customization-guide.md#3-storage-backend-customization)).

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

- ✅ Encryption at rest (AES-256)
- ✅ Encryption in transit (TLS 1.2+)
- ✅ Audit logging of all operations
- ✅ Access control (API key + ownership)
- ✅ Data minimization (only essential data)
- ✅ Non-custodial model (system owner controls keys)
- ✅ Stateless design (no in-memory secrets)

See [Regional Compliance](./regional-compliance.md) for detailed patterns and implementation guidance.

## 10. Source Tree

For detailed analysis of the project's file and directory structure, refer to the [Source Tree Analysis](./source-tree-analysis.md).

## 11. Development & Deployment

- **Development**: Instructions for setting up the local environment, building, and running the application can be found in the [Development Guide](./development-guide.md).
- **Deployment**: Complete containerization guide, environment configuration, and production deployment patterns are documented in the [Deployment Guide](./deployment-guide.md).

## 12. Design Decisions & Rationale

### Why Monolithic Architecture?

- **Simplicity**: Easier to understand and deploy for educational purposes
- **Security**: Single deployment unit, no inter-service communication vulnerabilities
- **Performance**: No network latency between logical layers
- **Operations**: Single database, simpler monitoring and debugging

Monolithic architecture can evolve to microservices if needed for specific requirements.

### Why SQLite + SQLCipher?

- **Encryption**: Built-in AES-256 encryption at rest
- **Simplicity**: No external database service required
- **Portability**: Single file database, easy to backup and restore
- **Sufficient Scale**: Supports requirements (10k wallets, 50k+ accounts)
- **Cost**: No database hosting fees
- **Familiar**: SQLite is widely used, well-documented

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
