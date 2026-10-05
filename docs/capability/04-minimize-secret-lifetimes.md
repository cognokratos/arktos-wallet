# C4 — Minimize secret lifetimes

> **Secret use and secret disclosure are different operations.**

**Question:** to give an agent an address, Arktos has to decrypt a recovery phrase and walk a private key hierarchy. Where exactly does plaintext secret material exist while that happens, for how long, and what does "destroyed" really mean?

## Mental model

Sort every value in the system into one of three classes:

| Class | Arktos values | Rule |
|---|---|---|
| **Persistent secret** | The encrypted recovery phrase (`wallets.encrypted_passphrase`) | Stored only as ciphertext, inside an encrypted file |
| **Temporary secret** | Plaintext recovery phrase, `bip39::Mnemonic`, BIP39 seed, BIP32 extended private keys (including the account private key) | On the server request path, exists only inside one bounded block of code, then dropped. Arktos zeroizes the buffers it owns; library-internal copies follow the library's own lifecycle |
| **Public data** | Derivation path, account index, public key, address, network, wallet name and id | May be stored, logged and returned |
| **Not persisted** | Account private keys | Exist transiently during derivation; re-derived from the phrase when needed; not written to storage and not returned |

The design goal is not "never touch a secret". To produce an address you *must* use one. The goal is that **use happens inside the service, in the smallest scope possible, and the only thing that crosses back to the caller is public.**

```text
OS entropy (16 bytes)
   ↓ BIP39
plaintext recovery phrase ─────────────── temporary secret
   ↓ AES-256-GCM seal (WalletSeedKey)
encrypted envelope ────────────────────── persistent secret (ciphertext)
   ↓ INSERT
SQLCipher database file ───────────────── persistent, page-encrypted

later, on first use of (wallet, network, index):

encrypted envelope
   ↓ AES-256-GCM open
plaintext recovery phrase ─────────────── temporary secret
   ↓ BIP39 (empty passphrase)
64-byte seed ──────────────────────────── temporary secret (zeroized right after next step)
   ↓ BIP32
master extended private key
   ↓ BIP32 child derivation along the path
account extended private key ──────────── temporary secret (not extracted)
   ↓
compressed public key ─────────────────── public
   ↓ BIP86 tweak + bech32m  |  Keccak-256 + EIP-55
address ───────────────────────────────── public → stored, returned
```

## In the code

**In the Arktos server request path, plaintext recovery phrases exist only inside the two regions marked `SECRET-BOUNDARY`** in [`src/wallet_services.rs`](../../src/wallet_services.rs):

- **Creation** (`create_wallet`): `phrase` is created and sealed inside a `{ … }` block. The block evaluates to the envelope `String`. The phrase is dropped, and zeroized, at the closing brace, *before* the database write starts.
- **Derivation** (`account`): the decrypted phrase and all derivation happen inside one block, which evaluates to `AccountData`. That struct has only public fields (`derivation_path`, `public_key`, `address`).

That statement is scoped to the server on purpose. Arktos also ships operator tooling, [`src/bin/secret.rs`](../../src/bin/secret.rs), whose `decrypt` command (`make decrypt`) deliberately decrypts a stored envelope and prints the phrase:

```text
server capability surface   the agent cannot obtain a phrase; the server uses it internally
operator tooling            a trusted operator holding MASTER_KEY can decrypt a phrase on purpose
```

This reinforces the course principle rather than weakening it: capabilities are assigned by principal, and the operator has capabilities the agent does not ([lesson 07](07-design-least-capability-tools.md#the-operator-has-capabilities-the-agent-does-not)).

Inside [`src/wallet_manager.rs`](../../src/wallet_manager.rs):

| Value | Owner and container | Lifetime handling |
|---|---|---|
| 16 bytes of entropy | Arktos: `Zeroizing<[u8; 16]>` | Zeroized when `generate_recovery_passphrase` returns |
| `bip39::Mnemonic` (parsed words) | **Library-owned** | Dropped at the end of the scope that built it. Not zeroized by this build (the crate's zeroize feature is not enabled) |
| Phrase text | Arktos: `RecoveryPhrase(Zeroizing<String>)`, pre-sized to `MAX_PHRASE_LEN` so formatting does not reallocate and leave a stray copy of that buffer | Zeroized when the `SECRET-BOUNDARY` block ends |
| Decrypted bytes | Arktos: `crypto::open` returns `Zeroizing<Vec<u8>>`. `RecoveryPhrase::from_utf8` takes ownership **without copying** | Same |
| Seed | Arktos: `Zeroizing<[u8; 64]>` | Explicit `drop(seed)`, zeroizing it, immediately after `XPrv::new` |
| Extended private keys | **Library-owned**: `bip32::XPrv`, reassigned at each child step | Explicit `drop(xprv)` as soon as the public key bytes are taken. Whether and how the scalar and chain code are wiped is up to the `bip32`/`k256` crates |
| Account private key | Inside the final `XPrv` | Not extracted into an Arktos variable, not persisted, not returned |

The strongest honest summary: **Arktos intentionally shortens secret lifetimes. It zeroizes buffers it owns, and library-internal copies are outside its control.** Nothing here guarantees that a secret is erased from memory.

Secret-bearing types also **redact themselves**: `RecoveryPhrase`, `MasterKey`, `ApiKeyHmacKey`, `AeadKey`, `Config` and the `Wallet` record all print `[REDACTED]` in `Debug`. That matters because the most common way secrets leak is through a well-meant `tracing::debug!(?value)`.

## What zeroization does not do

Zeroization is hygiene, not a security boundary. It overwrites a buffer *that you own* when it is dropped. That shortens the window during which a memory disclosure (a heap-read bug, a crash dump, a debugger) can find the secret. It does **not** give memory secrecy, and the code comments say so. Specifically:

- **Library-internal copies.** `bip39::Mnemonic` holds the parsed words, and this build does not enable that crate's zeroize feature. Its normalization step may allocate as well. `bip32` chain codes are outside Arktos's control. The module comment in [`src/wallet_manager.rs`](../../src/wallet_manager.rs) states this.
- **Compiler and allocator copies.** Moves can leave copies on the stack, a reallocation can leave an old buffer behind, and nothing zeroizes freed allocator pages that a value previously occupied.
- **The process environment.** `MASTER_KEY`, `DATABASE_KEY` and `ADMIN_API_KEY` are environment variables, and they stay readable in the process environment for the lifetime of the process. The derived subkeys are held for the process lifetime by design. SQLCipher keeps its own page key inside the connection.
- **Client API keys in transit.** The `X-API-KEY` header value is copied into an ordinary `String` during authentication, and the HTTP stack's header buffers are not zeroized.
- **The operating system.** Swap, hibernation images, core dumps and `ptrace`/`/proc/<pid>/mem` access by a sufficiently privileged user can all see process memory.

So zeroization is **hygiene that shrinks windows**. It is not a boundary. The boundaries are process isolation, the operator's control of the host, and the fact that no server code path *sends* a wallet secret anywhere. To harden further you need deployment controls: disable core dumps, encrypt or disable swap, run as a dedicated user, use no debugger in production. Alternatively, move the secret into a separate process or device ([Challenge 4](CHALLENGES.md#challenge-4--hsmkms-backed-keys), [Challenge 6](CHALLENGES.md#challenge-6--separate-signing-service)).

## Experiments

### Trace a first-use call

Open [`src/wallet_services.rs`](../../src/wallet_services.rs) at `get_ethereum_address` and follow it into `account`, `crypto::open` and `wallet_manager::derive_account_keys`. On paper, mark every line where plaintext secret material is **live**. A value is live if it exists in memory, even when no code is currently using it. You should find at least: the decrypted byte buffer, the `RecoveryPhrase`, the parsed `Mnemonic`, the seed, each intermediate `XPrv`, and the final `XPrv`. Then mark the first line after which **none** of them exists.

### Observe: when is the secret touched?

With the [lab server](../CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md#lab-setup) running at `RUST_LOG=info`:

```bash
mcp "$A" create_wallet '{"wallet_name":"trace"}'
mcp "$A" get_ethereum_address '{"wallet_name":"trace","account_index":7}'
mcp "$A" get_ethereum_address '{"wallet_name":"trace","account_index":7}'
```

**Predict** how many times the server log will show `Deriving new account`. Then look. The answer is once. The second call finds the stored public `accounts` row and returns it **without decrypting anything**. Only the first use of each (wallet, network, index) enters the secret boundary. The log line itself contains the wallet id, chain, network and index. It contains nothing secret.

### Inspect: what is persisted

```bash
cargo test --test secret_storage_tests accounts_persist_only_public_data
cargo test --test secret_storage_tests responses_never_contain_seed_or_private_key
```

The first test reads every cell of every table straight from the SQLCipher file. It asserts that the publicly known private key of the BIP39 test vector's `m/44'/60'/0'/0/0`, and the plaintext test phrase, appear nowhere. The second test asserts that neither appears in responses or `Debug` output.

### Explain

**Could this code derive the address using only public material?** As implemented, no. Each first-use derivation starts from the recovery phrase, because Arktos stores no extended *public* key.

In principle the answer is "partly". The last two levels of both paths (`…/0/index`) are non-hardened. A stored account-level extended public key could therefore derive every receive address without touching the phrase. Arktos deliberately does not store one. An xpub reveals *every* address of the account, and if the xpub is combined with any single leaked child private key, the parent private key can be recovered. Storing it would reduce how often secrets are used, at the cost of a new, privacy-critical, quasi-secret value. Weigh that trade-off yourself. [Lesson 07](07-design-least-capability-tools.md#composition-can-exceed-the-sum-of-the-parts) returns to it.

**So does the agent need to receive the mnemonic?** No. The service needs to *use* the mnemonic. The agent needs only the *result*. Those are different operations, so they can and must have different exposure.

## Failure mode

- "We need the seed to compute the address, so the tool returns the seed." This conflates use with disclosure.
- Long-lived plaintext: caching decrypted phrases "for performance", or holding them in a request-scoped struct that outlives the derivation.
- Logging or `Debug`-printing secret-bearing structs.
- Believing zeroization makes memory disclosure harmless.

## Takeaway

> Secret use and secret disclosure are different operations.
>
> The safest private key is often the one you never persist.

## Related reference

- [Architecture — Key Hierarchy & Secret Storage](../architecture.md#key-hierarchy--secret-storage) (secret lifecycle table)
- [Secret lifecycle walkthrough](SECRET-LIFECYCLE-WALKTHROUGH.md), which traces this lesson step by step
- [Case study: why recovery phrases are stored but private account keys are not](CASE-STUDIES.md#why-recovery-phrases-are-stored-but-private-account-keys-are-not)

Previous: [C3](03-encryption-is-a-data-format.md) · Next: [C5 — Derive, don't invent](05-derive-dont-invent.md)
