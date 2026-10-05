# C7 — Design least-capability tools

> **A narrow API is a security boundary.**

**Question:** given a cryptographic capability you want an agent to have, what is the *narrowest* tool that delivers it? How do you tell when a tool's real authority is wider than its name?

**Prerequisite:** guardrails filter what goes in and out of a model, as covered in [simple-agent-template stage 5](https://github.com/cognokratos/simple-agent-template/blob/main/docs/LEARNING-PATH.md#stage-5-guardrails-and-untrusted-data). This lesson is about the layer under the guardrails. A guardrail can be bypassed by one cleverly worded input. A capability that was never built cannot be invoked by any input.

## Mental model

The authority of a tool is the **set of effects reachable through its arguments**, not what its name promises. Three properties make that set small:

1. **Specific operations.** The tool's name and its schema fix *what* happens. The arguments select only *which* resource, within a range the server enforces.
2. **Typed, closed contracts.** Inputs and outputs are typed, validated and schema-described. The server decides the output shape, never the caller.
3. **Public-only results.** Nothing value-bearing comes back, whatever the arguments.

## How Arktos tools are built

| Property | Where | Effect |
|---|---|---|
| Typed requests | `CreateWalletRequest`, `GetBitcoinAddressRequest`, `GetEthereumAddressRequest` in [`src/wallet_services.rs`](../../src/wallet_services.rs) | Two fields at most: `wallet_name` (validated `WalletName`) and `account_index` (`0..=2^31-1`, enforced by `DerivationIndex` and declared in the schema) |
| Typed responses | `CreateWalletResponse`, `BitcoinAddressResponse`, `EthereumAddressResponse`, marked `PUBLIC-ONLY` | No secret-bearing field exists to fill. `deny_unknown_fields` makes a typed client reject anything beyond the contract |
| Generated JSON Schema | `schemars` derives, published by `rmcp` as `inputSchema` and `outputSchema` | The model sees exactly the contract the server enforces. There is no hand-written schema to drift from the code |
| Structured content | `Json<…>` returns in [`src/mcp.rs`](../../src/mcp.rs) | Results are data, not prose for the model to parse |
| Honest descriptions | `#[tool(description = …)]` | Side effects are stated ("Creates state", "the account is recorded on first use") along with what is *not* returned ("the recovery phrase is never returned") |
| Two error channels | `AppError` in [`src/error.rs`](../../src/error.rs) | Client errors (`invalid_argument`, `not_found`, `already_exists`) are tool results the model can act on. Server faults are an opaque `-32603` |
| Configuration is not an argument | `ChainConfig` from `BITCOIN_NETWORK` and `ETHEREUM_CHAIN_ID` | The model cannot select a network |
| Minimal set | Four tools | The whole surface fits in [one table](01-model-cryptographic-authority.md#the-current-authority-surface) |

## A spectrum of tools

None of the hypothetical tools below exists in Arktos. They are **future design** comparisons only.

| Tool | Can it reveal value-bearing secret material? | Abusable by prompt injection? | Authority broader than its name? | Constrainable structurally? | Should a model call it? | Human approval? |
|---|---|---|---|---|---|---|
| **Safer:** `get_bitcoin_address(wallet_name, account_index)` (implemented) | No | Only to fetch the caller's own public data | No | Already is: typed, ranged, owner-scoped, network from config | Yes | No |
| **Dangerous:** `wallet_execute(operation, payload)` | Whatever any `operation` can do, including operations added later | Yes, because the attacker picks the operation | **Yes, by construction.** Its real authority is the union of everything it dispatches to, and that set grows silently | Not without turning it back into separate tools | No. Split it into specific tools | Cannot be decided per call, because the tool has no fixed meaning |
| **Very dangerous:** `export_seed(wallet_name)` | **Yes**, all of it, permanently | Catastrophically | It *is* total authority | No. The output is the secret | **No** | No amount of approval makes model-context disclosure safe |
| **High authority:** `sign_arbitrary_bytes(wallet_name, bytes)` | Not the key, but it is a **signing oracle** | Yes: the attacker supplies the bytes | **Yes.** "Arbitrary bytes" includes serialized transactions, login challenges for other services, and attestations | Only by replacing "bytes" with typed, domain-separated structures | Not in this form | Yes, but a human cannot meaningfully approve opaque bytes either |

### Composition can exceed the sum of the parts

Authority must be evaluated **across** tools, not one tool at a time:

- `get_account_xpub` (privacy loss: every address becomes linkable) plus `derive_private_key(index)` (one account) gives you the **parent account private key**, and with it every account under it. Non-hardened BIP32 derivation lets anyone holding the parent xpub and *any* non-hardened child private key solve for the parent private key. Arktos's last two path levels are non-hardened.
- `sign_arbitrary_bytes` plus any public transaction builder is `sign_transaction` with no policy attached.
- `create_wallet` plus a future `sign_transaction` that is missing a per-wallet allowlist lets an agent create a fresh wallet *and then* spend from wallets it was never meant to touch, if wallet selection is just a name.

### The operator has capabilities the agent does not

Arktos ships an operator tool, `src/bin/secret.rs` (`make encrypt`, `make decrypt`, `make hash`). `make decrypt` **prints a recovery phrase**. That is not a contradiction of this lesson. It is the lesson applied:

- It is not reachable over MCP or HTTP. It is a separate binary.
- It requires `MASTER_KEY` in the operator's own environment, plus a ciphertext the operator extracted from the database with `DATABASE_KEY`.
- It exists for ceremonies such as disaster recovery or migrating a wallet to another implementation, which are run by a human who already holds the root secrets.

Capabilities are assigned **by principal**: the operator can do things the agent cannot, because the operator already holds the authority those things require. The learning labs never use `make decrypt`.

## Lab

### Inspect the published contract

```bash
cargo test --test mcp_protocol_tests tools_publish_input_and_output_schemas
cargo test --test mcp_protocol_tests domain_errors_are_tool_errors_with_codes
```

Read the first test and list every property it asserts about the schemas. Then compare `GetEthereumAddressRequest` with the schema you would write by hand. Is there anything in the generated schema that you would have left out?

### Exercise: prove ownership of an Ethereum address

A user asks the agent: *"prove to this website that I control address `0x…`"*. Design the tool. Compare:

```text
export_private_key(wallet_name, account_index)      → the website verifies by deriving the address
sign_challenge(wallet_name, account_index, challenge) → the website verifies the signature
```

`export_private_key` proves ownership by **transferring** it. After one call, the website, the model, the transcript and every log hold the key. Reject it.

`sign_challenge` keeps the key inside the service. Now go further: **is `sign_challenge` safe as written?** Work through each of the following:

| Concern | If missing | What a safe design pins down |
|---|---|---|
| **Domain separation** | The "challenge" could be a valid transaction, or a challenge for a different site | A fixed, recognisable prefix or type tag (compare EIP-191's `"\x19Ethereum Signed Message:\n"` prefix, or EIP-712 typed data with a domain separator), so the signature can never be valid as anything else |
| **Message structure** | The attacker chooses free text that the model will happily pass along | A typed message: audience (domain or URI), statement, address, chain ID, nonce, issued-at, expiry. EIP-4361, Sign-In with Ethereum, is one existing structure of this kind |
| **Replay protection** | One captured signature logs in forever | A verifier-issued nonce, a short expiry, and an audience the verifier checks |
| **Purpose restriction** | The same tool becomes a general signing oracle | The tool signs *only* this structure, and only with a dedicated derivation path or key if possible, never "any bytes in this shape" |
| **Who can request it** | Any injected instruction can trigger a signature for any site | Possibly a human confirmation that shows the **parsed** structure, never raw bytes |

Write the request and response types for your `sign_challenge` as Rust structs, in the style of [`src/wallet_services.rs`](../../src/wallet_services.rs). Then say what *the type itself* now makes impossible.

**Do not implement it in Arktos.** This is a design exercise, and [Challenge 2](CHALLENGES.md#challenge-2--design-safe-message-signing) extends it.

### Explain

Why does this lesson claim that capability design matters more than prompt design? Give a concrete prompt-injection input that no system prompt reliably stops, and show which property of the *tool* contains it.

## Failure mode

- A generic dispatcher tool (`execute`, `run`, `call`) whose authority grows with every new operation.
- "Flexible" byte-level signing APIs.
- Evaluating each tool's risk on its own, when the risk lives in the combination.
- Using prompt instructions ("never export the seed unless…") as the control, when the capability should not exist.

## Takeaway

> Capability design is more important than prompt design when agents can act.

## Related reference

- [API Contracts — Conventions](../api-contracts.md#conventions), [Errors](../api-contracts.md#errors)
- [Architecture — Error Model](../architecture.md#error-model)
- [Customization Guide — Extending MCP Tools](../customization-guide.md#extending-mcp-tools): before adding a tool, put it through this lesson's table

Previous: [C6](06-bind-identity-to-capability.md) · Next: [C8 — Recovery is part of security](08-recovery-is-part-of-security.md)
