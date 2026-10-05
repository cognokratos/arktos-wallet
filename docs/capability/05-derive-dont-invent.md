# C5 — Derive, don't invent

> **Cryptographic identity should come from deterministic standards, never from model reasoning.**

**Question:** an address, a derivation path or a checksum each has exactly one correct value for a given wallet and index. Which component should produce that value, and what makes the result trustworthy?

This lesson is not a blockchain primer. It treats BIP39, BIP32, BIP44, BIP86 and EIP-55 purely as **software contracts**: what they make reproducible, what they make canonical, and what breaks when you deviate from them.

## Mental model

A model asked for "the Ethereum address of wallet `main`, index 0" can produce a string that *looks* right: `0x`, 40 hex characters, plausible mixed case. A look-alike that is wrong is worse than an error, because it is accepted, and funds sent to it are gone.

The standards turn that question into a pure function:

```text
f(recovery phrase, chain, network, index) → (path, public key, address)
```

Same inputs, same outputs, on any compliant implementation, forever. So the right division of labor is:

| Decision | Who makes it |
|---|---|
| *Which* wallet, *which* index | The caller (model), constrained by schema and ownership |
| *Which* network | Server configuration (`BITCOIN_NETWORK`). Never the model |
| The path, the key, the address, the checksum | The standards, executed by Arktos |

## The two derivations

**Bitcoin** (configured network):

```text
BIP39 phrase → seed
  ↓ BIP32
m/86'/coin'/0'/0/index        coin = 0' on mainnet, 1' on testnet/signet/regtest
  ↓ BIP86 (key-path-only Taproot, BIP341 tweak with no script tree)
P2TR address, bech32m          bc1p… | tb1p… | bcrt1p…
```

**Ethereum** (any EVM chain):

```text
BIP39 phrase → seed
  ↓ BIP32
m/44'/60'/0'/0/index
  ↓ secp256k1 public key, uncompressed, without the 0x04 prefix
  ↓ Keccak-256, last 20 bytes
canonical lowercase hex        stored in accounts.address
  ↓ EIP-55 mixed-case checksum
returned address
```

## In the code

| Property | Where it is enforced |
|---|---|
| **One source of truth for paths** | `Network::derivation_path` in [`src/domain.rs`](../../src/domain.rs) |
| **The database agrees** | A `CHECK` constraint in [`migrations/V2__account_network.sql`](../../migrations/V2__account_network.sql) rejects any row whose `derivation_path` is not the canonical path for its chain, network and index, and any Ethereum address that is not lowercase |
| **Stored equals derived** | `WalletStore::insert_account` in [`src/wallet_store.rs`](../../src/wallet_store.rs): if a concurrent request already stored the row, its address and public key must equal the new derivation, or the call fails with `CorruptData` |
| **Typed inputs** | `DerivationIndex` (non-hardened, `< 2^31`), `WalletName`, `BitcoinNetwork` (exactly `mainnet`/`testnet`/`signet`/`regtest`; `Mainnet` is rejected), `EthereumChainId` (positive) in [`src/domain.rs`](../../src/domain.rs) and [`src/config.rs`](../../src/config.rs). Invalid configuration fails at startup and is never defaulted |
| **Pinned vectors** | [`src/wallet_manager.rs`](../../src/wallet_manager.rs) tests: BIP86 reference vectors, independently generated test-network vectors, BIP44 Ethereum vectors, and the EIP-55 specification vectors, all for the public `abandon … about` test phrase |
| **Network is identity** | `accounts` is unique on `(wallet_id, chain_type, network, account_index)`. V2 added `network` because `BITCOIN_NETWORK` can change between restarts |

One naming wrinkle: the API calls the last path component `account_index`, but in BIP44 terms it is the *address index* (the `account'` level is fixed at `0'`). [`src/domain.rs`](../../src/domain.rs) records the name as kept "for compatibility". It is a small example of a public name that is now part of the contract.

## Lab

### Observe: published vectors

```bash
cargo test --lib wallet_manager::tests
cargo test --test secret_storage_tests known_mnemonic_derives_published_addresses_end_to_end
```

The second test seals the public test phrase exactly as `create_wallet` would, then goes through the full service path: decrypt, derive, store, return. It checks that the result is the BIP86 and BIP44 reference addresses. Any other standards-compliant wallet restoring that phrase arrives at the same addresses. **Interoperability is a recovery property.**

### Predict, then observe: stability

With the [lab server](../CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md#lab-setup):

```bash
mcp "$A" create_wallet '{"wallet_name":"det"}'
mcp "$A" get_bitcoin_address '{"wallet_name":"det","account_index":0}'
mcp "$A" get_bitcoin_address '{"wallet_name":"det","account_index":0}'
mcp "$A" get_bitcoin_address '{"wallet_name":"det","account_index":1}'
```

The two index-0 results are identical, including `created_at`, because the second call reads the stored row. Index 1 has a different path, key and address.

### Break: change the network

Stop the server and restart it with a different Bitcoin network, keeping the same database and keys. **Before each call, predict the path prefix, the address prefix, and whether the public key changes.**

```bash
BITCOIN_NETWORK=testnet cargo run --quiet --bin arktos-wallet   # then:
mcp "$A" get_bitcoin_address '{"wallet_name":"det","account_index":0}'

BITCOIN_NETWORK=signet cargo run --quiet --bin arktos-wallet    # then the same call
BITCOIN_NETWORK=regtest cargo run --quiet --bin arktos-wallet   # then the same call
```

### Inspect

| Network | Path | Public key vs mainnet | Address prefix |
|---|---|---|---|
| mainnet | `m/86'/0'/0'/0/0` | n/a | `bc1p` |
| testnet | `m/86'/1'/0'/0/0` | **different**: coin type `1'` is a different branch of the tree | `tb1p` |
| signet | `m/86'/1'/0'/0/0` | same key as testnet | `tb1p`, the **same address as testnet** |
| regtest | `m/86'/1'/0'/0/0` | same key as testnet | `bcrt1p`, the same key with a different encoding |

So "network" means two separate things: the **key domain** (coin type, which selects which key) and the **encoding domain** (the bech32 human-readable part, which selects how the key is written). `cargo test --lib bitcoin_addresses_parse_for_their_network_only` shows the encoding domain doing its job, because a test-network address does not parse as valid for mainnet. List the stored rows to see that each network got its own `accounts` row:

```bash
labsql "SELECT chain_type, network, account_index, derivation_path, address FROM accounts ORDER BY id;"
```

### Break: change the EVM chain

```bash
ETHEREUM_CHAIN_ID=11155111 cargo run --quiet --bin arktos-wallet   # then:
mcp "$A" get_ethereum_address '{"wallet_name":"det","account_index":0}'
```

The address is unchanged and only `chain_id` differs. Ethereum addresses do not depend on the chain, so Arktos stores Ethereum accounts under the single network value `evm` and does not store the chain ID at all. Why report it then? The chain ID becomes essential the moment anything is **signed**: EIP-155 puts it into the transaction signature so that a transaction signed for one chain cannot be replayed on another. The address identifies an *account*, and the chain ID identifies a *transaction domain*. See the [case study](CASE-STUDIES.md#why-ethereum-chain-id-is-reported-even-though-address-derivation-is-unchanged).

### Explain

An agent reports "your address is `0x9858Ef…`" from a conversation an hour ago. Should a downstream system trust that string, or call `get_ethereum_address` again? Consider cost, staleness, and the fact that a model's memory of an address is *text it produced*, while the tool result is *the output of a function*.

## Failure mode

- Asking the model to compute, reformat or recall an address, path or checksum.
- Letting the model choose the network ("use testnet for this one"), which turns configuration into a prompt-injectable parameter.
- A second, slightly different path formatter somewhere else in the code. The single `derivation_path` function removes the reason to write one, and the database `CHECK` rejects any non-canonical path that reaches storage.
- Changing a derivation detail without pinned vectors to catch it. Every address already handed out would silently stop being reproducible.

## Takeaway

> If a standard defines the answer deterministically, don't delegate it to a probabilistic system.
>
> Deterministic cryptography should remain deterministic.

## Related reference

- [API Contracts — `get_bitcoin_address`](../api-contracts.md#get_bitcoin_address), [`get_ethereum_address`](../api-contracts.md#get_ethereum_address)
- [Data Models — `accounts`](../data-models.md#accounts)
- [Customization Guide — Adding Blockchain Support](../customization-guide.md#1-adding-blockchain-support)

Previous: [C4](04-minimize-secret-lifetimes.md) · Next: [C6 — Bind identity to capability](06-bind-identity-to-capability.md)
