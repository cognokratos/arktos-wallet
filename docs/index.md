# Arktos Wallet documentation

Arktos is a self-hosted, agent-accessible, non-custodial wallet blueprint. It is a Rust MCP server that lets AI agents create HD wallets and obtain Bitcoin and Ethereum addresses, without ever giving them a recovery phrase, a seed or a private key. It does not sign or broadcast transactions.

The documentation has three parts:

| Part | Use it to | Status |
|---|---|---|
| [Learn](#learn) | Understand *why* the system is shaped this way: Cryptographic Capability Engineering | Educational layer |
| [Reference](#reference) | Look up *what* the system does: architecture, contracts, schema | **Canonical** |
| [Operate / Customize](#operate--customize) | Run, deploy, extend and adapt it | How-to guides |

If the Learn material and the Reference disagree, the Reference and the code win. Please report the discrepancy.

## Learn

The Arktos track of the CognoKratos curriculum is **Cryptographic Capability Engineering**: *give agents capabilities, never secrets.*

- **[Cryptographic Capability Learning Path](./CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md)** is the front door. It covers positioning, prerequisites, stages C1–C8 and the lab setup.
- [Lessons C1–C8](./capability/README.md): authority, key hierarchies, envelopes, secret lifetimes, deterministic derivation, identity, least-capability tools, recovery.
- **[Secret Lifecycle Walkthrough](./capability/SECRET-LIFECYCLE-WALKTHROUGH.md)** follows one wallet secret from OS entropy to a public address.
- [Case Studies](./capability/CASE-STUDIES.md) explain the real architectural decisions and their alternatives.
- [Challenges](./capability/CHALLENGES.md) are open design problems, including safe transaction signing (future design).

Related tracks: [simple-agent-template](https://github.com/cognokratos/simple-agent-template) (production agent engineering), [sophos-agent](https://github.com/cognokratos/sophos-agent) (durable agent runtime engineering) and [etf-research-agent](https://github.com/cognokratos/etf-research-agent) (governed decision engineering).

## Reference

- **[Architecture](./architecture.md)**: layers, data architecture, MCP transport, security architecture, the [key hierarchy and secret storage](./architecture.md#key-hierarchy--secret-storage), and scalability
- **[API Contracts](./api-contracts.md)**: HTTP endpoints, the MCP entrypoint, tool schemas, error codes
- **[Data Models](./data-models.md)**: the `api_keys`, `wallets` and `accounts` tables, relationships and encryption
- **[Project Overview](./project-overview.md)**: summary, technology stack and repository structure
- [SECURITY.md](../SECURITY.md): how to report vulnerabilities

## Operate / Customize

- **[Development Guide](./development-guide.md)**: setup, building, testing, database operations, troubleshooting
- **[Deployment Guide](./deployment-guide.md)**: Docker, configuration, secret generation, persistence and backups, scaling limits
- **[Customization Guide](./customization-guide.md)**: adding blockchains, custom authentication, storage backends, extending the API
- **[Regional Compliance](./regional-compliance.md)**: patterns for GDPR, HIPAA, PCI DSS, SOC 2 and regional frameworks
- [CONTRIBUTING.md](../CONTRIBUTING.md): the checks every change must pass (`make ci`)

## Getting started paths

| I want to… | Read |
|---|---|
| Understand what an agent can and cannot do through Arktos | [Learning path](./CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md), then [C1](./capability/01-model-cryptographic-authority.md) |
| Understand the system at a high level | [README](../README.md), [Project Overview](./project-overview.md), [Architecture](./architecture.md) |
| Set up a development environment | [Development Guide](./development-guide.md), [Data Models](./data-models.md) |
| Integrate an MCP client | [API Contracts](./api-contracts.md), [Architecture — API Design](./architecture.md#5-api-design), the protocol tests in `tests/mcp_protocol_tests.rs` |
| Add a tool or a chain | [C7 — Design least-capability tools](./capability/07-design-least-capability-tools.md) first, then the [Customization Guide](./customization-guide.md) |
| Deploy to production | [Deployment Guide](./deployment-guide.md), [C8 — Recovery is part of security](./capability/08-recovery-is-part-of-security.md), [Regional Compliance checklist](./regional-compliance.md#8-compliance-deployment-checklist) |

## Quick reference

### MCP tools (the complete agent surface)

| Tool | Effect |
|---|---|
| `ping` | Returns `"pong"` |
| `create_wallet` | Creates a wallet (12-word BIP39 phrase, stored encrypted) owned by the caller's API key. The phrase is never returned |
| `get_bitcoin_address` | BIP86 Taproot address on the configured Bitcoin network. Recorded on first use |
| `get_ethereum_address` | BIP44 Ethereum address, EIP-55 checksummed, with the configured chain ID. Recorded on first use |

### Endpoints

| Endpoint | Auth |
|---|---|
| `GET /healthz`, `GET /readyz` | none |
| `POST /mcp` (MCP `2026-07-28`, stateless) | client API key (`X-API-KEY`) |
| `/admin/api-keys` | admin API key (`X-API-KEY`) |
| `/swagger-ui`, `/openapi.json` | none |

### Key files

| Path | Purpose |
|---|---|
| `src/main.rs` | Entry point: `serve`, `migrate`, `db-info` |
| `src/app.rs` | HTTP router |
| `src/mcp.rs` | MCP tool surface |
| `src/wallet_services.rs` | Wallet use cases and public request/response types |
| `src/wallet_manager.rs` | BIP39 generation, BIP32 derivation, address encoding |
| `src/keys.rs`, `src/crypto.rs` | Key hierarchy, AES-256-GCM envelopes |
| `src/auth.rs`, `src/key_services.rs` | API-key authentication and administration |
| `src/database.rs`, `src/wallet_store.rs`, `src/key_store.rs` | SQLCipher connection, migrations and stores |
| `migrations/` | Versioned SQL migrations |
| `src/bin/secret.rs` | Operator key tool (`make secret`, `make encrypt`, `make decrypt`, `make hash`) |
| `scripts/verify_docs.py` | Documentation checker (`make docs-check`) |
