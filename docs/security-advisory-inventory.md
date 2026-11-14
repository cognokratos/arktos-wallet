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
| RUSTSEC-2026-0459 SQLite uninitialized page-cache memory via mismatched `-wal` | `libsqlite3-sys` 0.36.0 (SQLCipher 4.10.0, SQLite 3.50.4) | `rusqlite` 0.38 (`bundled-sqlcipher-vendored-openssl`) | runtime, local: an attacker must place a crafted `-wal` beside `DATABASE_PATH` | **Fixed:** `rusqlite` 0.42 / `libsqlite3-sys` 0.40 (SQLCipher 4.20.0, SQLite 3.54.1), using a vendored `rusqlite_migration` 2.7.0 with a one-line patch. See below |

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
  parent fingerprints, depths and serialized xprvs. It is not part of the test
  suite, because it needs the removed crates. [Reproduce it](#reproducing-the-evidence).

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
`application/json`). Removing the `/swagger-ui` page is an **intentional
compatibility change**: that path now returns 404, as documented in
[API Contracts](./api-contracts.md#3-admin-api). Any OpenAPI viewer can load
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

## RUSTSEC-2026-0459 (`libsqlite3-sys`): SQLCipher 4.20.0 / SQLite 3.54.1

**Why the obvious upgrade is not enough.** The advisory marks `libsqlite3-sys`
≥ 0.39.0 as patched. Its own text, however, states that the
`bundled-sqlcipher` build in 0.39.0 and 0.39.1 uses `sqlcipher-src` 419.0.6,
which bundles SQLite 3.53.4 and **is still affected**. `rusqlite` 0.41 would
turn the scanners green without fixing anything, so it was rejected. The real
fix for an SQLCipher build is `libsqlite3-sys` 0.40.0 (`sqlcipher-src`
420.0.0: SQLCipher 4.20.0, SQLite 3.54.1), released with `rusqlite` 0.42.

**The obstacle.** No `rusqlite_migration` release accepts `rusqlite` 0.42
yet: 2.7.0, the latest, requires `^0.41`.

**The fix.** `vendor/rusqlite_migration` holds the unmodified 2.7.0 release
plus one change: its `rusqlite` requirement goes from 0.41.0 to 0.42.0.
`[patch.crates-io]` substitutes it, and `Cargo.toml` pins `=2.7.0` exactly.
Evidence that the patch is sufficient and safe:

- 2.7.0 compiles unchanged against `rusqlite` 0.42; it uses only
  `Connection`, `Transaction`, `Statement`, `params` and a few `Error`
  variants.
- The 2.4.1 → 2.7.0 library diff (Arktos previously ran 2.4.1) is one
  debug-log line, so migration semantics are unchanged.
- Upstream's own test suite passes 48 of 51 tests on both `rusqlite` 0.41 and
  0.42. The same three `fk_check` tests fail on both versions, because
  `libsqlite3-sys` compiles with `SQLITE_DEFAULT_FOREIGN_KEYS=1` and those
  tests insert an orphan row while assuming foreign keys are off. A probe
  showed that `pragma_foreign_key_check` returns identical results on
  `rusqlite` 0.38, 0.41 and 0.42, bundled and SQLCipher.
- `make vendor-check` (run in `make ci` and in the CI `cargo deny` job)
  downloads the crates.io archive and checks it against the index checksum
  (`cb11df4f…`). It then applies the recorded patch and requires the vendored
  tree to match byte for byte, so any unrecorded edit fails the build.

**Behavior preserved** (tests in [`tests/persistence_tests.rs`](../tests/persistence_tests.rs)
and the [cross-version check](#reproducing-the-evidence)):

- SQLCipher is linked, and SQLite is ≥ 3.54.0
  (`bundled_sqlcipher_includes_the_cve_2026_119375_fix`).
- Fresh install (`empty_database_is_migrated_and_configured`), idempotent
  reopen (`reopening_is_idempotent_and_keeps_data`), v1 → v2 upgrade, and
  rejection of newer (`newer_schema_version_is_rejected`) and unmanaged
  schemas.
- A failed migration rolls back completely: version, schema and rows are
  unchanged, even after DDL ran (`failed_migration_rolls_back_completely`).
- A database created by the previous release (SQLCipher 4.10.0 / SQLite
  3.50.4) opens with the new binary. Stored accounts are unchanged, the stored
  recovery phrase decrypts to identical fresh addresses, repeated `migrate` is
  a no-op, and the previous release can still open the database afterwards.
  The on-disk format and encryption settings are unchanged.

**Licensing.** `sqlcipher-src` now carries the SQLite, SQLCipher and
libtomcrypt sources that `libsqlite3-sys` used to embed under its MIT label,
and declares their licenses accurately (`MIT AND BSD-3-Clause AND blessing
AND WTFPL`). `deny.toml` allows `blessing` and `WTFPL` for that crate only.

**Maintenance and removal.** The vendored copy is a temporary bridge:

- **Remove it** when a `rusqlite_migration` release accepts `rusqlite` 0.42.
  Delete `vendor/`, the `[patch]` section, the `vendor-check` target and its CI
  step, and the `.dockerignore` exception, then depend on that release. Track
  this in a dedicated follow-up issue.
- **Tripwires:** Dependabot proposes the next `rusqlite_migration` release
  against the `=2.7.0` pin, and `make vendor-check` blocks silent edits.
- **Residual risk:** `cargo audit` and `cargo deny` match advisories only
  against registry packages, so an advisory filed against
  `rusqlite_migration` 2.7.0 would not flag the vendored copy. Check
  <https://rustsec.org/packages/rusqlite_migration.html> when reviewing this
  bridge, and no later than 2026-12-15.

**Exception not needed.** The temporary exception drafted earlier was not
applied and is withdrawn.

**Database directory.** Fixing the CVE does not remove the trust assumption
behind its precondition. Whoever can write to the directory holding
`DATABASE_PATH` can still replace the database or plant `-wal`/`-shm` files.
Arktos creates missing directories with mode `0700`, but it uses an existing
directory or mount as-is. The [deployment guide](./deployment-guide.md)
documents this; a startup check is a proposed follow-up.

## Reproducing the evidence

**Cross-version database check.** Build the previous release in a separate
worktree, then compare it with the current build:

```bash
git worktree add ../arktos-old 3dfdf4f && (cd ../arktos-old && cargo build)
cargo build
scripts/cross_version_check.sh ../arktos-old/target/debug/arktos-wallet target/debug/arktos-wallet
```

**BIP32 differential** (`bip32` 0.5.3 vs `bitcoin` 0.32.8). Run it in a
scratch directory outside the repository. It deliberately depends on the
removed, vulnerable crates, so do not commit it.

```toml
# Cargo.toml
[package]
name = "bip32-diff"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
bip32 = "=0.5.3"
bitcoin = "=0.32.8"
getrandom = "0.2"
```

```rust
// src/main.rs: run with `cargo run --release`
use bitcoin::bip32::{ChildNumber, Xpriv};
use bitcoin::secp256k1::Secp256k1;

fn rnd<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::getrandom(&mut b).unwrap();
    b
}

fn main() {
    let secp = Secp256k1::new();
    let mut steps = 0;
    for round in 0..3000 {
        let seed: Vec<u8> = match round % 10 {
            0 => rnd::<16>().to_vec(),
            1 => rnd::<32>().to_vec(),
            _ => rnd::<64>().to_vec(),
        };
        let mut a = bip32::XPrv::new(&seed).unwrap();
        let mut b = Xpriv::new_master(bitcoin::NetworkKind::Main, &seed).unwrap();
        for _ in 0..1 + rnd::<1>()[0] % 6 {
            let r = u32::from_le_bytes(rnd::<4>());
            let idx = match r % 5 { 0 => 0, 1 => (1 << 31) - 1, _ => r >> 1 };
            let hardened = r & 1 == 1;
            a = a.derive_child(bip32::ChildNumber::new(idx, hardened).unwrap()).unwrap();
            let c = if hardened { ChildNumber::from_hardened_idx(idx) } else { ChildNumber::from_normal_idx(idx) };
            b = b.derive_priv(&secp, &[c.unwrap()]).unwrap();
            steps += 1;
            assert_eq!(a.private_key().to_bytes().as_slice(), &b.private_key.secret_bytes()[..]);
            assert_eq!(a.attrs().chain_code, b.chain_code.to_bytes());
            assert_eq!(a.public_key().to_bytes().as_slice(), &b.private_key.public_key(&secp).serialize()[..]);
            assert_eq!(a.attrs().parent_fingerprint, b.parent_fingerprint.to_bytes());
            assert_eq!(a.attrs().depth, b.depth);
        }
        assert_eq!(a.to_string(bip32::Prefix::XPRV).as_str(), b.to_string());
    }
    println!("OK: 3000 seeds, {steps} steps identical");
}
```

**Raw MCP comparison.** Send `server/discover`, `tools/list` and `tools/call`
requests, as built by `raw_request` in
[`tests/mcp_protocol_tests.rs`](../tests/mcp_protocol_tests.rs), to both
binaries and compare the bodies and status codes.

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
