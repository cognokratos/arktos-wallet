# Contributing to Arktos Wallet

Thanks for your interest in contributing! For security issues, follow
[SECURITY.md](./SECURITY.md) instead of opening a public issue.

## Development setup

1. Install [rustup](https://rustup.rs/). The toolchain version and components
   are pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) and installed
   automatically on the first `cargo` invocation.
2. Install a C toolchain, `make` and `perl` (SQLCipher and OpenSSL are compiled
   from source by `rusqlite`).
3. Install the check tools:

   ```bash
   cargo install --locked cargo-nextest cargo-audit cargo-deny
   ```

   Optionally install [hadolint](https://github.com/hadolint/hadolint) and
   Docker for the Dockerfile checks.
4. Copy `.env.example` to `.env` and fill in the secrets to run the server
   (`make dev`). Generate `MASTER_KEY` and `DATABASE_KEY` separately
   with `make secret`.

Run `make help` to list all targets.

## Checks

| Task | Command |
|------|---------|
| Format | `make fmt` |
| Check formatting | `make fmt-check` |
| Type-check | `make check` |
| Lint (Clippy, warnings denied) | `make lint` |
| Tests | `make test` (cargo test) or `make nextest` (as CI) |
| Vulnerability audit | `make audit` |
| Licenses / bans / sources / advisories | `make deny` |
| **Everything CI runs, except Docker** | `make ci` |
| Dockerfile lint / image build | `make docker-lint` / `make docker-build` |

Tests must not depend on Docker, the network, or external services. Docker
image builds and Dockerfile linting run in CI only.

## Pull requests

- Open pull requests against `main` or `dev`.
- Keep PRs focused; avoid unrelated refactors.
- Use [Conventional Commits](https://www.conventionalcommits.org/) for titles
  (`feat:`, `fix:`, `chore:`, `docs:`, `ci:` …).
- `make ci` must pass locally, and all CI jobs must be green.
- Add or update tests for behavior changes, and update docs affected by the change.
- Fix Clippy findings rather than silencing them; a scoped `#[allow(...)]`
  needs a comment explaining why.
- New dependencies must pass `cargo deny` (allowed licenses, crates.io only).
- Changes touching cryptography, key handling, or authentication need extra
  review. Describe the security impact in the PR description.
