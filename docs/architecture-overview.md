# Architecture overview

Arktos is a cryptographic capability service for AI agents. Its most important architectural boundary is not the HTTP endpoint or the database: it is the **authority boundary between what the agent may request and what cryptographic material the model may ever observe**.

![Arktos Wallet cryptographic capability architecture](./assets/arktos-wallet-architecture.svg)

## The design in one sentence

> Give agents capabilities, never secrets.

The model can select one of four typed MCP tools and provide validated arguments. Authentication happens outside the model-visible tool arguments. The trusted Arktos process resolves the authenticated caller, scopes wallet access to that caller, performs any required cryptographic operation, and returns public data only.

## Three security domains

### 1. Model-visible capability surface

The agent can request `ping`, `create_wallet`, `get_bitcoin_address`, and `get_ethereum_address`. Those tools may create durable state, but none grants signing, broadcasting, arbitrary message signing, private-key export, seed export, or recovery-phrase export.

The returned values are public or non-secret application data: wallet ids and names, timestamps, derivation paths, public keys, and addresses. The model therefore has useful wallet capabilities without receiving the material that would make it a custodian of cryptographic authority.

### 2. Trusted Arktos service

Each MCP request is independently authenticated with `X-API-KEY`. Arktos HMACs the credential with the purpose-specific API-key HMAC key and resolves the caller out of band; identity is not a tool argument chosen by the model.

`WalletServices` applies owner-scoped operations. When a new address must be derived, the service opens the encrypted recovery phrase inside a short-lived secret boundary, derives the standards-defined result using BIP39/BIP32 and BIP86 or BIP44, and returns only public material.

An already-derived account short-circuits this path: its public account row can be returned without decrypting the recovery phrase again.

### 3. Operator and secret authority

The operator supplies two independent root secrets:

- `MASTER_KEY`, from which HKDF derives separate `WalletSeedKey` and `ApiKeyHmacKey` purposes;
- `DATABASE_KEY`, which keys SQLCipher independently of the field-encryption hierarchy.

The running service therefore does have access to wallet secrets when needed. A plaintext recovery phrase, BIP39 seed, and extended private keys can exist transiently in process memory during creation or first-use derivation. They are never persisted as plaintext and never returned through MCP. Arktos-owned secret buffers are dropped and zeroized where implemented; library-internal copies remain subject to their libraries' memory behavior.

## Persistence model

The MCP protocol layer is stateless, but wallet state is persistent in a local, single-instance SQLCipher database:

| State | Protection | Model-visible? |
|---|---|---:|
| API-key credential | Stored only as an HMAC-SHA256 value | No |
| Recovery phrase | AES-256-GCM versioned envelope inside SQLCipher | No |
| Account private keys | Not stored; re-derived when required | No |
| Addresses, public keys, derivation paths | SQLCipher-protected database rows | Yes, through typed tool results |

This distinction matters: **stateless transport does not mean stateless application data**, and **self-hosted does not automatically mean non-custodial for every participant**. Whoever operates Arktos and controls its root keys has effective custody of the stored wallet secrets.

## Why the tool surface is the authority model

For an agent-facing system, adding an endpoint is not merely an API change. Adding a tool transfers a new capability to a probabilistic caller. A future signing tool would therefore change Arktos' authority model substantially: the current architecture can create wallets and derive public addresses, but no current MCP capability can move value.

For the precise implementation, key hierarchy, database behavior, MCP transport and security decisions, continue with [Architecture](./architecture.md). To follow one recovery phrase from entropy to a public address, read the [Secret Lifecycle Walkthrough](./capability/SECRET-LIFECYCLE-WALKTHROUGH.md).
