# Case studies: why Arktos looks the way it does

Each case study is a real decision in the current code: what was decided, what the alternative was, and which principle the decision applies. They describe architectural choices. They are not incident reports.

---

## Why recovery phrases are stored but private account keys are not

**Decision.** `wallets.encrypted_passphrase` is the only wallet secret that is persisted. `accounts` rows hold public data only (path, public key, address). Private keys are re-derived from the phrase when needed and are never stored ([`migrations/V1__initial_schema.sql`](../../migrations/V1__initial_schema.sql) states this in its header).

**Alternative.** Store each account's encrypted private key next to its address, so that a future signing feature can read it directly.

**Why not.**

- **Recoverability.** The phrase *is* the wallet. Any BIP39/BIP32-compliant implementation can rebuild every account from it alone (`known_mnemonic_derives_published_addresses_end_to_end`). Stored private keys add no recoverability.
- **Deterministic derivation.** A private key at a path is a pure function of the phrase, so storing it would cache a value that can always be recomputed.
- **Minimizing secret proliferation.** Every stored private key is one more ciphertext to protect, migrate, rotate and audit. With N accounts there would be N+1 secrets in storage instead of 1, and N+1 places where a future bug could write plaintext.

**Principle.** *The safest private key is often the one you never persist.*

---

## Why `DATABASE_KEY` is independent from `MASTER_KEY`

**Decision.** `DATABASE_KEY` is generated separately and is not derived from the HKDF hierarchy. Startup rejects a `DATABASE_KEY` that equals `MASTER_KEY` ([`src/config.rs`](../../src/config.rs)).

**Alternative.** Derive a third HKDF subkey, `arktos/sqlcipher/v1`, so that there is one root secret to manage.

**Why not.** The two layers exist for **different threat scenarios**:

| Scenario | SQLCipher (`DATABASE_KEY`) | Field encryption (`MASTER_KEY`) |
|---|---|---|
| Stolen disk, volume snapshot or backup file | protects | protects |
| Someone with a SQL console, a dump, or `DATABASE_KEY` itself (a DBA, a backup job, a debugging session) | **does not protect** (they are inside) | protects |
| Someone with `MASTER_KEY` but no database access | n/a | n/a: there is nothing to decrypt |

If both keys came from one root, the holder of that root would defeat both layers at once, and the second row of the table would collapse into the first. Independence gives **two compromise domains**. The cost is one more secret to keep available ([lesson 08](08-recovery-is-part-of-security.md#recovery-combinations)). The layers do not "double" AES-256. They cover different exposures.

**Principle.** Separate keys exist to separate *people and systems*, not to stack bits.

---

## Why API keys are HMAC-hashed rather than encrypted

**Decision.** Client API keys are stored as `HMAC-SHA256(ApiKeyHmacKey, key)` hex ([`src/keys.rs`](../../src/keys.rs), [`src/key_store.rs`](../../src/key_store.rs)). Wallet phrases are stored as AES-256-GCM ciphertext.

**The question to ask of any stored secret:** *will the system ever need the plaintext back?*

```text
API key       → only ever needs:   is presented == issued?     → verification
wallet phrase → must later yield:  the phrase itself, to derive → recovery
```

- **Verification does not require recovery.** A keyed hash answers "does this match?" and can never produce the key. Even with `MASTER_KEY`, nobody can list the API keys. They are shown exactly once, at creation or rotation.
- **Why HMAC rather than a plain hash?** API keys are 256-bit random values, so a plain SHA-256 would already resist brute force. Keying the hash adds two things. Someone holding only the database cannot even *test* a guessed key offline. And the stored values are bound to this deployment's `MASTER_KEY`.
- **Why not a password hash such as Argon2?** Password hashes add cost to slow down brute force of *low-entropy* human secrets. These keys have full entropy, so the cost would buy nothing except slower requests.
- **Lookup by HMAC.** The database compares keyed hashes only (`KeyServices::lookup`), so neither an index nor a query log ever holds a usable key.
- **The consequence of the choice.** Because the HMAC key is derived from `MASTER_KEY`, losing `MASTER_KEY` invalidates every client key ([lesson 08](08-recovery-is-part-of-security.md#the-dangerous-partial-failure)), and rotating `MASTER_KEY` would require re-issuing them all.

**Principle.** The treatment of a secret follows from what the system must do with it later.

---

## Why API keys are not MCP parameters

**Decision.** The caller's identity arrives in the `X-API-KEY` HTTP header, is resolved by middleware into an `ApiKey`, and reaches tools through request `Parts` (`caller()` in [`src/mcp.rs`](../../src/mcp.rs)). No tool request type has an identity field.

**Alternative.** `create_wallet(wallet_name, api_key)`, where the model passes its credential along with the operation. This is simpler to wire into some agent frameworks.

**Why not.** A credential in tool arguments sits in the model's context window and in every transcript and log of it. It can be chosen or substituted by any prompt injection, echoed back in errors, and rotated only by re-prompting. Out-of-band identity makes "who is calling" a property of the **connection the host configured**, not of **text the model produced**. The model chooses an operation. It never chooses its authenticated identity. See the live demonstration in [lesson 06](06-bind-identity-to-capability.md#break-try-to-choose-an-identity-in-the-arguments).

**Principle.** *Credentials belong outside model-visible tool arguments.*

---

## Why encrypted envelopes are versioned

**Decision.** Ciphertext is stored as `{"v":1,"alg":"A256GCM","nonce":…,"ct":…}`, with version, algorithm and purpose bound into the AAD. Unknown versions and algorithms fail with explicit, non-secret errors ([`src/crypto.rs`](../../src/crypto.rs)).

**Alternative.** Store `base64(nonce ‖ ciphertext)`, the common minimal format. `non_envelope_values_are_rejected` in [`tests/secret_storage_tests.rs`](../../tests/secret_storage_tests.rs) shows that Arktos now refuses exactly that shape.

**Why.** A stored ciphertext outlives the code that wrote it. Without a version, the first change of algorithm, AAD, padding or key forces the decryptor to guess. With a version:

- old and new formats can coexist during a migration;
- a reader can refuse a format it does not understand, rather than misinterpret it;
- binding the version into the AAD means no edit to the `v` field can make one version's ciphertext pass as another's.

Lesson 03 lists [what v1 does not yet bind](03-encryption-is-a-data-format.md#what-v1-does-not-bind): row identity and plaintext length. Any change to either needs exactly this mechanism.

**Principle.** *Key names, purpose labels and envelope versions are persistent protocol design.*

---

## Why key purposes have Rust types

**Decision.** `ApiKeyHmacKey` and `WalletSeedKey` are distinct types with no public byte accessors. `KeyServices` receives only `ApiKeyKeys`, and `WalletServices` receives only `WalletKeys` ([`src/keys.rs`](../../src/keys.rs), [`src/main.rs`](../../src/main.rs)).

**Alternative.** Pass `[u8; 32]` or `&[u8]` everywhere. HKDF already makes the two keys cryptographically independent, so why add types?

**Why.** HKDF protects against *cryptographic* misuse. Types protect against *programmer* misuse: the right bytes handed to the wrong function, a refactor that swaps two arguments, a new module that "just needs" the master key. With types:

- `crypto::seal(&keyring.api_keys.hmac, …)` is a compile error (`E0308`), shown by the `compile_fail` doctest on `Keyring`;
- a service can only use the keys it was constructed with, so the dependency graph *is* the key-access graph;
- `Debug` is implemented once per type to print `[REDACTED]`, so the redaction cannot be forgotten at a call site.

**Principle.** Make the misuse unrepresentable, then you don't have to review for it.

---

## Why Ethereum chain ID is reported even though address derivation is unchanged

**Decision.** `ETHEREUM_CHAIN_ID` is validated at startup and returned with every Ethereum address. It is not stored, and it does not affect derivation. Accounts use the single network value `evm` ([`migrations/V2__account_network.sql`](../../migrations/V2__account_network.sql), `EthereumChainId` in [`src/domain.rs`](../../src/domain.rs)).

**The distinction:**

```text
account identity    = f(phrase, m/44'/60'/0'/0/i)        same on every EVM chain
transaction domain  = chain ID (EIP-155), part of what a signature commits to
```

The same address exists on Ethereum mainnet, Sepolia and every other EVM chain. What differs is where a *transaction* from that address is valid. Arktos signs nothing today, but reporting the configured chain ID makes the eventual transaction domain **explicit and server-controlled** from the start. An agent is told "this address, intended for chain 11155111" by configuration. It is never left to assume a chain. A future `sign_transaction` must take the chain ID from the same configuration, not from the model ([Challenge 1](CHALLENGES.md#challenge-1--design-safe-transaction-signing)).

The Bitcoin case is the opposite and instructive: there the network changes the **key** (coin type `0'` vs `1'`) and the **encoding** (`bc1p` vs `tb1p` vs `bcrt1p`), so `network` is part of each account's stored identity ([lesson 05](05-derive-dont-invent.md#inspect)).

**Principle.** Keep *who* (account identity) and *where* (transaction domain) separate, and keep both out of the model's hands.

---

## Why stateless MCP does not imply horizontally scalable persistence

**Decision.** `/mcp` implements MCP `2026-07-28` with no sessions (`NeverSessionManager`, [`src/mcp.rs`](../../src/mcp.rs)). Persistence is one SQLCipher file behind one connection and a mutex ([`src/database.rs`](../../src/database.rs)). The supported deployment is **one Arktos instance per database**.

**The tempting inference.** "No sessions, so any request can go to any instance, so we can run five replicas behind a load balancer."

**Why it is wrong.**

```text
protocol statelessness   ≠   storage statelessness
```

- The *protocol* layer is stateless: `independent_requests_need_no_session` in [`tests/mcp_protocol_tests.rs`](../../tests/mcp_protocol_tests.rs) creates a wallet in one request and reads it on a brand-new connection, linked only by the database.
- The *application* is stateful: wallets, accounts and API keys are persistent, and request B depends on what request A wrote.
- SQLite locking is not designed for several processes sharing one file over NFS or a shared volume. Arktos's single connection serializes all access on purpose and assumes it owns the file.
- Correctness depends on that: `UNIQUE` constraints plus `BEGIN IMMEDIATE` make concurrent `create_wallet` and `insert_account` race-safe *within one process*. Five processes on five copies of the file would each be internally consistent and mutually divergent.

Scaling out would need a different persistence architecture, and, once signing exists, a design for *which instance holds signing authority* ([Challenge 6](CHALLENGES.md#challenge-6--separate-signing-service)). The stateless transport removes only the protocol-level obstacle.

**Principle.** *Protocol statelessness does not imply application statelessness.*

---

## Why client errors are tool results but server faults are opaque

**Decision.** `invalid_argument`, `not_found` and `already_exists` are returned as tool results with `isError: true` and a JSON body the model can read and act on. Storage, crypto and derivation faults become JSON-RPC `-32603 "internal error"` with no data ([`src/error.rs`](../../src/error.rs)).

**Why.** A client error is information *about the caller's own request*. The model can fix it, for example by choosing another name. A server fault is information *about the system's internals*: which wallet failed to decrypt, which constraint fired, which key is wrong. Telling the model which of those happened helps nobody fix the request, and it could help an attacker probe. `server_faults_hide_details` pins this, and the full detail goes to the server log, which never contains secrets.

**Principle.** Errors are part of the capability surface. Return what the caller can act on and nothing more.

Back to the [lessons](README.md) · Next: [Challenges](CHALLENGES.md)
