# C3 — Encryption is a data format

> **Ciphertext is long-lived structured data. Design it like a versioned protocol.**

**Question:** a recovery phrase encrypted today may need to be decrypted in ten years, by a binary that does not exist yet, after the algorithm, the key or the library has changed. What has to be written down next to the ciphertext for that to work, and what must never vary?

## Mental model

An `encrypt(key, plaintext) → bytes` function is not a storage format. Stored ciphertext needs:

| Need | Why | Arktos v1 |
|---|---|---|
| A **version** | The decryptor has to know which rules produced these bytes before it tries anything | `"v": 1` |
| An **algorithm identifier** | Crypto agility: an algorithm can be replaced without guessing at old data | `"alg": "A256GCM"` |
| A **fresh nonce** per encryption | AES-GCM's confidentiality *and* integrity collapse if a (key, nonce) pair is ever reused | 12 random bytes from the OS RNG, `"nonce"` |
| **Authenticated** ciphertext | Tampering must be detected, not silently decrypted into garbage | AES-256-GCM: `"ct"` is ciphertext followed by a 16-byte tag |
| **Associated data** (AAD) | Binds the context the ciphertext is valid in, without storing it secretly | `arktos:v1:A256GCM:wallet-seed` |
| **Uniform failure** on authentication | The error must not tell an attacker *why* decryption failed | `CryptoError::DecryptionFailed` for every authentication failure |

The stored value in `wallets.encrypted_passphrase` is compact JSON:

```json
{"v":1,"alg":"A256GCM","nonce":"<base64url, 12 bytes>","ct":"<base64url ciphertext‖16-byte tag>"}
```

The AAD is never stored. Both sides compute it from code (`AeadKey::aad` in [`src/crypto.rs`](../../src/crypto.rs)):

```text
arktos:v1:A256GCM:<purpose>        e.g. arktos:v1:A256GCM:wallet-seed
```

Because the version and the algorithm are inside the AAD, a ciphertext produced under v1 rules cannot be accepted under some future v2 rules, and the reverse is also true, even if someone edits the `v` field. Because the purpose is inside the AAD, a ciphertext sealed for `wallet-seed` will not open under a key whose purpose is anything else, even if the key bytes happen to be identical.

## In the code

[`src/crypto.rs`](../../src/crypto.rs) is short enough to read in full. `open` checks things in a deliberate order:

```text
1. parse only {"v"}                → MalformedEnvelope        (not JSON / no v)
2. v != 1                          → UnsupportedVersion(v)    (explicit, before anything else)
3. parse full EnvelopeV1, no extra fields → MalformedEnvelope
4. alg != "A256GCM"                → UnsupportedAlgorithm
5. nonce not 12 bytes              → InvalidNonce
6. ct shorter than the 16-byte tag → MalformedEnvelope
7. AES-256-GCM decrypt with AAD    → DecryptionFailed         (wrong key, wrong purpose,
                                                               tampered nonce, ct or tag)
```

Steps 1–6 look only at **public structure**: anyone who can read the envelope already knows the answers, so naming the problem leaks nothing. Step 7 is the only step that depends on the **secret key**, and all of its failures collapse into one variant. A decryptor that reported "tag mismatch" separately from "wrong key", or that returned partially decrypted bytes, would be handing an attacker an oracle.

The collapse does not stop at `CryptoError`. Over MCP, *every* crypto, derivation and storage fault becomes the same JSON-RPC `-32603 "internal error"`, with no data attached (`AppError` in [`src/error.rs`](../../src/error.rs)). Details such as `crypto failure: wallet 7: failed to decrypt secret` go only to the server log, and they contain no secret.

## Lab

These are tests over synthetic keys (`[1; 32]`, `[2; 32]`) and a public test vector. Nothing real is decrypted.

```bash
cargo test --lib crypto::tests
```

Before reading each test, predict which `CryptoError` the change produces. Then check:

| # | Change | Test | Error class |
|---|---|---|---|
| 1 | Encrypt the same plaintext repeatedly | `nonces_and_ciphertexts_are_unique` | n/a: every nonce and every ciphertext differs |
| 2 | Flip a bit in the ciphertext (or in the tag at its end) | `modified_ciphertext_fails` | `DecryptionFailed` |
| 3 | Flip a bit in the nonce | `modified_nonce_fails` | `DecryptionFailed` |
| 4 | Truncate the nonce to 8 bytes | `wrong_nonce_length_fails` | `InvalidNonce` |
| 5 | Change `alg` to `A128GCM` | `unknown_algorithm_fails_explicitly` | `UnsupportedAlgorithm` |
| 6 | Change `v` to `2` | `unknown_version_fails_explicitly` | `UnsupportedVersion(2)` |
| 7 | Same key bytes, different purpose | `purpose_is_bound_to_ciphertext` | `DecryptionFailed` |
| 8 | Different key | `wrong_key_fails` | `DecryptionFailed` |
| 9 | Truncated JSON, bare base64, extra fields | `truncated_envelopes_fail_cleanly`, `invalid_encoding_fails_cleanly` | `MalformedEnvelope` |
| 10 | Render errors and keys with `{}` and `{:?}` | `errors_and_debug_do_not_leak_secrets` | no plaintext and no key bytes in any output |

Changes 3, 7 and 8 are three different mistakes, but they produce the same error. Ask yourself why that is the *correct* behavior, and what you would lose if the three were distinguishable.

Then follow a broken envelope end to end:

```bash
cargo test --test secret_storage_tests non_envelope_values_are_rejected
cargo test --test secret_storage_tests wallet_is_unreadable_with_a_different_master_key
cargo test --lib error::tests::server_faults_hide_details
```

### Inspect live: ciphertext without plaintext

With the [lab server](../CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md#lab-setup) running, create a wallet and look at what is stored. You only ever read **metadata**, never the decrypted value:

```bash
mcp "$A" create_wallet '{"wallet_name":"main"}'
mcp "$A" create_wallet '{"wallet_name":"savings"}'
labsql "SELECT id, name,
               json_extract(encrypted_passphrase, '$.v')   AS v,
               json_extract(encrypted_passphrase, '$.alg') AS alg,
               length(json_extract(encrypted_passphrase, '$.ct')) AS ct_b64_len
        FROM wallets;"
```

Notice that `ct_b64_len` **differs between wallets**. AES-GCM hides content but not length: the ciphertext is exactly as long as the plaintext plus the 16-byte tag. A 12-word English phrase is between 47 and 107 characters (BIP39 English words have 3–8 letters), so the length reveals something about which words were chosen. The leak is small, but it is real. Fixed-length padding before sealing would remove it, and adding padding would be an envelope-format change: it would need a new version.

## What v1 does not bind

The AAD binds **version, algorithm and purpose**. It does **not** bind the envelope to the row it is stored in: neither the wallet id nor the owning API key is part of it. In practice this means:

- Someone who can **write** to the database (who has `DATABASE_KEY` and file access) but lacks `MASTER_KEY` cannot read any phrase. Confidentiality holds.
- That same person *can* copy one wallet's envelope into another wallet's row. It would open cleanly, and future derivations for the victim's wallet would produce the *source* wallet's addresses. The same person could also edit the public `accounts` rows directly, because stored accounts are served without being re-derived.

So field encryption in v1 gives **confidentiality against database-level access**. It does not give integrity of the wallet-to-owner binding, or of stored public data, against someone who can write to the database. Defending against a writer would mean binding the row identity, for example `wallet_id` and `key_id`, into the AAD, which is a new envelope version. It would also mean authenticating or re-deriving the `accounts` rows. Both belong to [Challenge 3](CHALLENGES.md#challenge-3--rotate-wallet-encryption-keys). Arktos's threat model treats database write access as operator-level, and this section exists so that you know exactly where that line sits.

## Failure mode

- Unversioned ciphertext. The first migration then has to *guess* how each value was produced.
- Nonce reuse, for example from a counter that resets on restart, or from deriving the nonce from the plaintext.
- Unauthenticated encryption (CBC or CTR without a MAC), which turns tampering into silent corruption.
- Distinguishable authentication errors, which give an attacker an oracle.
- Assuming that "encrypted" also means "bound to its context". It does only if the context is in the AAD.

## Takeaway

> Ciphertext is long-lived structured data. Design it like a versioned protocol.

## Related reference

- [Architecture — Key Hierarchy & Secret Storage](../architecture.md#key-hierarchy--secret-storage) (envelope v1)
- [Data Models — Encryption](../data-models.md#encryption)
- [Case study: why encrypted envelopes are versioned](CASE-STUDIES.md#why-encrypted-envelopes-are-versioned)

Previous: [C2](02-design-key-hierarchies.md) · Next: [C4 — Minimize secret lifetimes](04-minimize-secret-lifetimes.md)
