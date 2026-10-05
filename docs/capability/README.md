# Cryptographic Capability Engineering — lessons

This directory is the **educational layer** of Arktos. The reference documentation in [`docs/`](../index.md#reference) stays canonical for *what the system does*. These lessons teach *why it is shaped that way*, and what changes when an agent is the caller.

Start at the [learning path](../CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md) for positioning, prerequisites and the [lab setup](../CRYPTOGRAPHIC-CAPABILITY-LEARNING-PATH.md#lab-setup).

## Lessons

| # | Lesson | Core lesson |
|---|---|---|
| C1 | [Model cryptographic authority](01-model-cryptographic-authority.md) | Every tool is an authority grant |
| C2 | [Design key hierarchies](02-design-key-hierarchies.md) | Cryptographic naming is part of persistent protocol design |
| C3 | [Encryption is a data format](03-encryption-is-a-data-format.md) | Ciphertext is long-lived structured data |
| C4 | [Minimize secret lifetimes](04-minimize-secret-lifetimes.md) | Secret use and secret disclosure are different operations |
| C5 | [Derive, don't invent](05-derive-dont-invent.md) | Deterministic cryptography should remain deterministic |
| C6 | [Bind identity to capability](06-bind-identity-to-capability.md) | The model chooses an operation, not its authenticated identity |
| C7 | [Design least-capability tools](07-design-least-capability-tools.md) | Capability design beats prompt design when agents can act |
| C8 | [Recovery is part of security](08-recovery-is-part-of-security.md) | Encryption without recoverability can become data loss |

## Centerpiece and practice

- [Secret lifecycle walkthrough](SECRET-LIFECYCLE-WALKTHROUGH.md): follows one wallet secret from OS entropy to a public address. The other tracks each follow one unit through their system: simple-agent-template follows one request, sophos-agent one run, and etf-research-agent one decision. Arktos follows **one secret**.
- [Case studies](CASE-STUDIES.md): the real architectural decisions behind the current code.
- [Challenges](CHALLENGES.md): open-ended design problems with no published solutions, including safe transaction signing.

## How every lesson is built

Each lesson follows the same loop, and each experiment points at a real test, a real file or a real local server:

```text
Observe   what the system does today
Predict   what a change will do before you apply it
Break     apply the change (in a test or a lab instance, never in production)
Inspect   the result, the error class, the stored bytes
Explain   why the design produces exactly that behavior
```

## Ground rules for the material

- Every claim about the implementation maps to current code. Every capability Arktos does not have (signing, broadcasting, message signing, key or seed export) is labelled **future design**.
- No lesson, lab or helper prints a recovery phrase, a seed or a private key. The only mnemonic that appears anywhere is the public BIP39 test vector (`abandon … about`) that the existing tests already use. It is synthetic and controls no funds.
- Lessons link to the other CognoKratos tracks for agent loops, MCP basics, guardrails, durable runtimes, HITL and decision governance, and do not repeat them.
