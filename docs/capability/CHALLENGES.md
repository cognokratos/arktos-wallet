# Challenges

These are advanced competency challenges. They are open-ended design problems, not tutorials, and **no solutions are published**. A good answer is a design document that someone else could review and implement. It names the trust boundaries, states the threat model, gives the types and data flows, and argues honestly about what remains unsafe.

Every capability in these challenges is **future design**. Arktos implements none of them, and the challenges do not ask you to add them to this repository. If you prototype one, do it on a fork, with synthetic keys and test networks only.

Before you start, re-read the [current authority surface](01-model-cryptographic-authority.md#the-current-authority-surface) and the [secret lifecycle walkthrough](SECRET-LIFECYCLE-WALKTHROUGH.md). Most challenges come down to the question: *which row of those tables changes, and what must be added so that the change is safe?*

---

## Challenge 1 — Design safe transaction signing

Design a `sign_transaction` capability for Arktos. Do not implement it.

Your design must answer each of the following explicitly:

| Concern | Questions you must answer |
|---|---|
| **Caller identity** | How is the caller identified? Can you keep [lesson 06](06-bind-identity-to-capability.md) intact, so that nothing in the request names an identity? |
| **Wallet ownership** | How is the signing wallet resolved? Is a wallet *name* still enough, given that an agent can also `create_wallet`? |
| **Chain ID** | Where does it come from: request, configuration or policy? What happens if the request and the configuration disagree? |
| **Transaction type** | Which transaction types are allowed (legacy, EIP-1559, contract calls, Bitcoin PSBT)? Is "arbitrary calldata" allowed? Why? |
| **Destination** | Allowlist, denylist, address book, or anything? Who maintains the list, and can the agent edit it? |
| **Amount / value** | Per-transaction, per-period and per-wallet limits? In which unit? What about fee-only drain? |
| **Nonce** | Who chooses the account nonce (Ethereum) or the inputs (Bitcoin)? What happens with concurrent signing requests? |
| **Gas / fees** | Who sets them? What bounds stop a fee-burning attack? |
| **Human approval** | Which transactions need it? What does the human see: the parsed transaction, never raw bytes? How is approval bound to *exactly* this transaction? (Mechanics: [template stage 9](https://github.com/cognokratos/simple-agent-template/blob/main/docs/LEARNING-PATH.md#stage-9-human-in-the-loop-and-controlled-mutations); authority versus recommendation: [etf-research-agent A5](https://github.com/cognokratos/etf-research-agent/blob/main/docs/applied/05-recommendation-authority-and-consent.md).) |
| **Replay protection** | Chain-level (EIP-155, account nonce) *and* request-level: what if the same tool call is delivered twice? (Idempotent side effects in general: [sophos-agent R7](https://github.com/cognokratos/sophos-agent/blob/main/docs/runtime/07-side-effects-and-idempotency.md).) |
| **Audit** | What is recorded, where, and is it tamper-evident? What must *never* be recorded? |
| **Private-key lifecycle** | Rewrite the [walkthrough](SECRET-LIFECYCLE-WALKTHROUGH.md) for signing. Which step now uses the private scalar, and for how long? Does the key ever leave `wallet_manager`? |
| **Result format** | What does the tool return: a signed transaction, a hash, an approval request? Is a signed-but-unbroadcast transaction itself a bearer instrument that must be treated as sensitive? |
| **Broadcast separation** | See below. |

**Then answer: should signing and broadcasting be one capability or two?** Justify your answer with at least: who can broadcast a leaked signed transaction; where idempotency lives; what an approval covers; and what the agent sees between the two steps.

**Hard mode:** show that your design survives a fully prompt-injected agent that is *also* allowed to call `create_wallet` and `get_*_address`.

---

## Challenge 2 — Design safe message signing

Compare two designs:

```text
sign_arbitrary_bytes(wallet_name, account_index, bytes)
```

versus a **domain-separated structured challenge**, for example a typed message with a fixed type tag.

Your structured design must specify:

- **Purpose domain.** A fixed prefix or type that can never also be a valid transaction or another protocol's message. Compare EIP-191 and EIP-712 domain separators, and say what each prevents.
- **Expiry.** Who sets it, its maximum lifetime, and how the verifier enforces it.
- **Nonce.** Who issues it (the verifier, not the agent), and its single-use semantics.
- **Audience.** Which party the signature is valid for. How does a phishing site that relays another site's challenge fail?
- **Replay semantics.** Exactly when a captured signature becomes worthless.

Then show a concrete `bytes` value for which `sign_arbitrary_bytes` produces something dangerous that your structured design can never produce. Finally, decide whether challenge signing should use the same key as the funds or a dedicated derivation path, and justify the choice.

---

## Challenge 3 — Rotate wallet encryption keys

Design the migration from `arktos/wallet-seed-encryption/v1` to a `v2`. The new version might change the key, the algorithm, or the envelope, for example to bind `wallet_id` and `key_id` into the AAD (see [what v1 does not bind](03-encryption-is-a-data-format.md#what-v1-does-not-bind)) or to pad the plaintext to a fixed length.

Answer:

- **Detecting the version.** How does `open` choose v1 or v2? What does the AAD look like for v2, and how does it prevent a v1 ciphertext from being accepted as v2, and the reverse?
- **Decrypting old data.** Which keys must be loaded during the transition? Does `WalletKeys` gain a second key type? How do you keep [purpose types](CASE-STUDIES.md#why-key-purposes-have-rust-types) meaningful?
- **When to re-encrypt.** Lazily on read, eagerly in a batch, or both? What does each mean for how long v1 must remain decryptable?
- **Partial migration.** Some rows are v1 and some v2. How do you know when it is *safe to delete the v1 key*? What evidence do you require?
- **Interruption.** The batch dies halfway. Show that no row can be lost or double-encrypted. Consider `BEGIN IMMEDIATE`, per-row atomicity, and backups taken mid-migration.
- **The API-key side.** If the change is a new `MASTER_KEY` and not just a new label, every API-key HMAC changes too. Design the client re-issuing process.

Arktos implements none of this, and should not until a real requirement forces it.

---

## Challenge 4 — HSM/KMS-backed keys

Replace the process-environment `MASTER_KEY` with external key custody: an HSM, a cloud KMS, or a TPM-sealed key.

Design for:

- **Startup.** Does Arktos unwrap a data key at boot, or call the KMS for every operation? What does it hold in memory either way, and how does that change the [summary table](SECRET-LIFECYCLE-WALKTHROUGH.md#summary)?
- **Latency.** A first-use derivation becomes a network call. What is the new p95, and does the current target in the [Architecture](../architecture.md#performance-targets-nfr-compliant) still hold?
- **Outage.** The KMS is unreachable. Which tools keep working? (Hint: think about [step 14](SECRET-LIFECYCLE-WALKTHROUGH.md#14-an-already-derived-account-short-circuits).) Should the service report not-ready?
- **Caching.** What may be cached, for how long, and how is the cache invalidated on rotation or revocation?
- **Rotation.** How do KMS key versions map onto envelope versions and HKDF labels?
- **Deployment.** How does the process authenticate to the KMS without reintroducing a long-lived secret in the environment?
- **Audit.** The KMS now logs every unwrap. What does that audit trail reveal, and to whom?

State explicitly which threats this design defeats that the current design does not (for example a memory dump of a stopped process, or a leaked environment), and which it does not defeat (for example a live attacker inside the Arktos process).

---

## Challenge 5 — Multi-tenant authorization

Replace "one API key owns its wallets" with organizations, users, roles, and wallet-level permissions such as *may derive addresses*, *may create wallets*, and the future *may request signatures*.

Constraints:

- **The model never chooses its own identity.** No organization, user or role appears in a tool argument.
- **Scoped lookups.** Wherever practical, keep the [unaddressable-resource pattern](06-bind-identity-to-capability.md#mental-model): the query itself is scoped, not followed by a check.
- **No cross-tenant oracles.** Errors must not reveal what exists in another organization.

Design the schema, the request-to-principal resolution, how an agent acting *for* a user gets a narrower authority than the user has (delegation), and how revocation propagates. Show what happens to `UNIQUE (key_id, name)` and to the `not_found` semantics. Explain how you would test that no tool can reach a wallet outside its principal's scope.

---

## Challenge 6 — Separate signing service

Imagine that signing exists and has been moved into a separate, hardened process. It might be a different host, a different user, an enclave or an HSM front-end. Arktos keeps the MCP surface.

Answer:

- **Which keys move?** `WalletSeedKey`? The encrypted phrases? `ApiKeyHmacKey`? What does the MCP-facing process still hold, and what can an attacker who owns that process now do?
- **Which APIs remain?** Define the API between the two processes. Is it `sign(wallet_id, structured_tx)`, `derive_public(wallet_id, path)`, or both? Who resolves wallet names?
- **Where is authorization enforced?** In the MCP process, in the signer, or in both? What must the signer verify for itself, without trusting the caller?
- **Where is policy enforced?** Limits, allowlists, approvals. Can the MCP-facing process bypass them?
- **What gets logged, and where?** Which side keeps the authoritative audit?
- **What network boundary is introduced?** Transport authentication between the processes, replay across that boundary, and behavior when the signer is down.

Finally, revisit [why stateless MCP does not imply horizontally scalable persistence](CASE-STUDIES.md#why-stateless-mcp-does-not-imply-horizontally-scalable-persistence). Does a separate signer make horizontal scaling of the MCP layer easier, harder, or neither?

---

> A future signing tool changes the threat model much more than it changes the API surface.

Back to the [learning path](../CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md)
