# Dependency Advisory Inventory (security baseline, PR 0)

Inventory of the RustSec findings that blocked `cargo audit` and `cargo deny` on
`main` in November 2026, with dependency paths, exposure, remediation and the
evidence that behavior did not change. Tracked by
[issue #36](https://github.com/cognokratos/arktos-wallet/issues/36).

- **Baseline:** `main` at `3dfdf4f`. `cargo audit` 0.22.2 and `cargo deny`
  0.20.2 (the CI-pinned versions) against advisory-db `4ef2df8` (2026-11-13).
- **Scope of exposure:** "runtime" means code in the shipped server binary;
  "build" means code that runs only while compiling; "dev" means code used only
  by the test suite.

The original report (CI run of PR #35, 2026-11-07) listed six vulnerabilities
and one unsound advisory. By 2026-11-14 the database also reported
`libsqlite3-sys` RUSTSEC-2026-0459, three `rmcp` advisories, and `rustix` and
`pin-utils` warnings. The `ctutils` path in the original report was incomplete:
it reaches Arktos through the AES-GCM/HMAC stack, not through `bip32`.

## Findings

| Advisory | Crate (version) | Path | Exposure | Resolution |
|---|---|---|---|---|
| RUSTSEC-2026-0387 timing variability in constant-time ops | `crypto-bigint` 0.5.5 | `bip32` 0.5 → `k256` 0.13 → `elliptic-curve` 0.13 | runtime: BIP32 child-key arithmetic on secret scalars | **Removed.** BIP32 now uses `bitcoin::bip32` over libsecp256k1 |
| RUSTSEC-2026-0385 non-constant-time masks / `u128` compares | `ctutils` 0.4.2 | `aes-gcm` 0.11, `ghash`/`polyval`, `hmac` 0.13, `hkdf` 0.13, `sha2` 0.11 | runtime: seed-envelope AES-256-GCM and API-key HMAC | **Fixed:** 0.4.8 (lockfile only) |
| RUSTSEC-2026-0367, -0368, -0369, -0429 unsafe ZIP extraction | `zip` 3.0.0 | build-dependency of `utoipa-swagger-ui` 9 | build: unpacks the Swagger UI archive downloaded at compile time | **Removed** with the embedded Swagger UI |
| RUSTSEC-2026-0461 unbounded tool-schema cache (server DoS) | `rmcp` 3.5.0 | direct (`transport-streamable-http-server`) | runtime: `/mcp`, limited to holders of a client API key by `api_key_auth` | **Fixed:** 3.9.0 |
| RUSTSEC-2026-0457 OAuth client SSRF | `rmcp` 3.5.0 | dev-dependency client | dev only; the server does not enable client or auth features | **Fixed:** 3.9.0 |
| RUSTSEC-2026-0462 unbounded client response buffering | `rmcp` 3.5.0 | dev-dependency client (`transport-streamable-http-client-reqwest`) | dev only | **Fixed:** 3.9.0 |
| RUSTSEC-2026-0386 `AsyncFd` I/O-safety (unsound) | `tokio` 1.48.0 | direct, and via axum/hyper/rmcp | runtime crate, but Arktos never constructs an `AsyncFd` | **Fixed:** 1.51.6 |
| RUSTSEC-2026-0486 `xattr` functions (unsound, warning) | `rustix` 1.1.3 | `tempfile` (dev-dependency) | dev only; no xattr calls | **Fixed:** 1.1.6 |
| RUSTSEC-2026-0441 unmaintained (warning) | `pin-utils` 0.1.0 | `futures-util` (axum, rmcp) | runtime crate, informational | **Open, not failing.** No patched release; needs `futures-util` upstream. `deny.toml` fails unmaintained crates only when they are direct dependencies |
| RUSTSEC-2026-0459 SQLite uninitialized page-cache memory via mismatched `-wal` | `libsqlite3-sys` 0.36.0 (SQLCipher 4.10.0, SQLite 3.50.4) | `rusqlite` 0.38 (`bundled-sqlcipher-vendored-openssl`) | runtime, local: an attacker must place a crafted `-wal` beside `DATABASE_PATH` | **Open: needs a maintainer decision.** See below |

## Decisions and rejected alternatives

### BIP32: `bitcoin::bip32` instead of `bip32`/`k256`

`crypto-bigint` has no patched 0.5 release. Fixing it in place means moving to
`bip32` 0.6 → `k256` 0.14 → `elliptic-curve` 0.14 → `crypto-bigint` 0.7, a new
major RustCrypto EC stack. `bitcoin` 0.32 was already a direct dependency for
Taproot addresses and already linked libsecp256k1, so `bitcoin::bip32` removes a
second secp256k1 implementation and 20 crates instead of adding new majors.

**Evidence that behavior is unchanged:**

- Golden vectors were generated with the old implementation *before* the swap
  and pass unmodified after it (`golden_derivation_table` in
  [`src/wallet_manager.rs`](../src/wallet_manager.rs)): every supported
  network, indices 0, 1, 2, 19, 1000 and 2³¹−1, two BIP39 reference mnemonics,
  covering path, compressed public key and address.
- BIP86 specification internal keys and BIP341 tweaked output keys
  (`bip86_internal_and_output_keys_match_the_specification`) match, so
  internal-key versus output-key semantics are unchanged.
- A differential run of `bip32` 0.5.3 against `bitcoin` 0.32.8 over 3000 random
  seeds (16, 32 and 64 bytes) and 10407 hardened/normal steps, including indices
  0 and 2³¹−1, produced identical secret scalars, chain codes, public keys,
  parent fingerprints, depths and serialized xprvs. This was a one-off scratch
  run and is not part of the test suite.

**Differences, accepted:**

- `bitcoin::bip32::Xpriv` is `Copy` and does not wipe itself. `bip32::XPrv`
  zeroized its `k256` scalar on drop. Arktos now holds every extended key in
  `ErasedXpriv`, which overwrites the scalar and chain code when the key is
  replaced or dropped (`extended_private_keys_are_erased`). Temporaries inside
  rust-bitcoin's child derivation (HMAC state, the untweaked child scalar) are
  not wiped. The [secret-lifetime lesson](capability/04-minimize-secret-lifetimes.md)
  and the [walkthrough](capability/SECRET-LIFECYCLE-WALKTHROUGH.md) say so.
- On an invalid child key (I_L ≥ n or a zero key, probability below 2⁻¹²⁷ per
  step), rust-bitcoin panics where `bip32` returned an error.

### Swagger UI: removed, `/openapi.json` kept

`utoipa-swagger-ui` 10.0.2 depends on `zip` ^9, but it requires `utoipa` 6, a
major release that changes OpenAPI generation; Dependabot PRs #14 and #17 do not
compile. It also keeps the compile-time download of the Swagger UI archive.
Removing the embedded UI is smaller and also removes a network fetch from the
build. `/openapi.json` is now a plain handler for the same `ApiDoc::openapi()`.
The served document is byte-identical to the previous build (8274 bytes,
`application/json`). The `/swagger-ui` page is gone; any OpenAPI viewer can load
the document.

Removing the merged Swagger router exposed a latent routing detail: the router
fallback would inherit the `/mcp` API-key layer and answer 401 for unknown
paths. An explicit 404 fallback keeps the previous behavior, and protocol tests
pin it.

### rmcp 3.9.0

3.9.0 is the lowest release that fixes all three advisories. A raw HTTP
comparison with the previous build found these byte-identical:
`server/discover`, `tools/list` (every tool's input and output schema), `ping`,
the rejection of a legacy `initialize`, and every SEP-2243 header rejection
(-32020, HTTP 400). One observable change comes from the 0461 fix itself:
`tools/call` for an unknown tool returns the same JSON-RPC error (-32602
"tool not found") with **HTTP 200 instead of 400**, because the transport no
longer looks tools up while validating headers. The test
`routing_headers_are_validated_and_unknown_tools_are_errors` pins this.

### tokio 1.51.6, ctutils 0.4.8, rustix 1.1.6

These are lockfile-only patch updates within existing manifest ranges. For
tokio, 1.51.6 is the closest patched release (the patched ranges are
`>=1.51.6, <1.52` and `>=1.53.3`). The pinned AES-GCM envelope and API-key HMAC
known answers (`previously_stored_values_remain_readable` in
[`src/keys.rs`](../src/keys.rs)) cover the `ctutils` path.

## Open: RUSTSEC-2026-0459 (`libsqlite3-sys`)

**Why it is not fixed here.** The advisory marks `libsqlite3-sys` ≥ 0.39.0 as
patched. However, its own text states that the `bundled-sqlcipher` build in
0.39.0 and 0.39.1 uses `sqlcipher-src` 419.0.6, which bundles SQLite 3.53.4 and
**is still affected**. The only real fix for an SQLCipher build is
`libsqlite3-sys` 0.40.0 (SQLCipher 4.20.0, SQLite 3.54.1), which ships with
`rusqlite` 0.42 (released 2026-11-11). No `rusqlite_migration` release supports
`rusqlite` 0.42 yet: 2.7.0 requires `^0.41`. Options:

1. `rusqlite` 0.41 with `rusqlite_migration` 2.7: turns the scanners green but
   leaves the bundled SQLite affected. **Rejected** as a false remediation.
2. `rusqlite` 0.42 with `rusqlite_migration` replaced by an in-house
   `user_version` runner: a real fix, but it rewrites migration semantics for
   the encrypted store inside a dependency-baseline change. Deferred.
3. Keep `rusqlite` 0.38 and accept a narrow, time-boxed exception until
   `rusqlite_migration` supports 0.42. **Proposed**, pending maintainer
   approval. No ignore entry has been added.

**Exposure.** The attacker must be able to write a crafted `-wal` file next to
`DATABASE_PATH` that the service then opens. Arktos runs in WAL mode. Such an
attacker can already delete or replace the database. SQLCipher encrypts and
HMAC-authenticates every page with a key derived from `DATABASE_KEY`, which
should turn forged or stale page content into an error. The WAL and frame
headers are plaintext, however, and it has not been shown that the
uninitialized read happens only after page authentication. Treat this as
"local precondition, impact not excluded": process memory can transiently hold
a decrypted recovery phrase.

**Proposed exception (requires maintainer approval):**

```toml
# deny.toml [advisories]
ignore = [
    { id = "RUSTSEC-2026-0459", reason = "bundled SQLCipher; real fix needs rusqlite 0.42 (libsqlite3-sys 0.40, SQLite 3.54.1), unsupported by rusqlite_migration as of 2026-11-14. Local precondition: crafted -wal beside DATABASE_PATH. Approved by <maintainer> on <date>. Revisit by 2026-12-15 or on a rusqlite_migration release supporting rusqlite 0.42." },
]
```

```toml
# .cargo/audit.toml [advisories]
ignore = ["RUSTSEC-2026-0459"] # see deny.toml for reason, approval and revisit date
```

**Interim mitigation.** Keep the database directory writable only by the
service user. Do not open database files obtained from untrusted sources.

**Revisit trigger.** A `rusqlite_migration` release that accepts `rusqlite`
0.42, or 2026-12-15, whichever comes first. Then move to `rusqlite` 0.42 and
remove the exception. Verify the result with `db-info` (expect SQLite ≥ 3.54.0)
and by opening an existing encrypted database.

## Dependabot triage

None of the open Dependabot PRs should be merged as-is.

| PR | Proposal | Recommendation |
|---|---|---|
| #14 | `utoipa` 5 → 6 | Defer. Fails to compile and is no longer needed for a security fix. Upgrade later as an API-documentation change with a diff of `/openapi.json` |
| #15 | `bip39` 2 → 3 | Separate, security-sensitive PR. The golden vectors above now make it reviewable |
| #16 | `base64` 0.22 → 0.23 | Separate PR. The stored-envelope known answer covers envelope decoding |
| #17 | `utoipa-swagger-ui` 9 → 10 | Close after this lands: the dependency is removed |
| #23 | Rust toolchain 1.97.1 → 1.99.1 | Separate PR after this lands. Its audit and deny failures were these advisories; recheck Clippy |
| #24 | 14 compatible updates | Rebase after this lands. `tokio` and `rmcp` are superseded. `bitcoin` 0.32.8 → 0.32.102 now provides BIP32 and `zeroize` 1.8 → 1.9 touches secret hygiene, so take them as separate, reviewed commits |
| #35 | GitHub Actions bumps | Re-run CI after this lands; its failures were these advisories |

## Branch protection on `main`

As of 2026-11-14, `main` has no classic branch protection. The repository
ruleset "main" (id 24495277) exists but has enforcement **disabled** and an
**empty branch condition**, so it applies to no branch. It contains only the
deletion and non-fast-forward rules. Nothing was changed. Applying protection
is a repository-admin action for an authorized maintainer.

**Recommended ruleset** (target the default branch, enforcement active):

- Block deletion and force pushes. These rules already exist.
- Require a pull request: 1 approval, dismiss stale approvals on push, require
  approval of the most recent push, and require conversation resolution. With
  a single maintainer, a bypass actor (repository admin, pull requests only) is
  needed, or self-authored PRs cannot merge.
- Require these status checks to pass, with the branch up to date (job names
  from `.github/workflows/ci.yml`): `Formatting`, `Documentation links`,
  `Check & Clippy`, `Tests (nextest)`, `cargo audit`, `cargo deny`,
  `Dockerfile lint (hadolint)`, `Docker image build`.
- Keep auto-merge disabled. It is currently off.
