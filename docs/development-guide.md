# Development Guide

This guide provides instructions for setting up the development environment, building, and running the `arktos-wallet` project locally.

## Prerequisites

- **Rust**: Install [rustup](https://rustup.rs/). The exact toolchain (currently Rust 1.97.1, 2024 edition, with `rustfmt` and `clippy`) is pinned in [`rust-toolchain.toml`](../rust-toolchain.toml) and installed automatically by the first `cargo` command. CI and the Docker build use the same version.
- **C toolchain, `make` and `perl`**: SQLCipher and OpenSSL are compiled from source by `rusqlite` (`bundled-sqlcipher-vendored-openssl`), so no system SQLite is required.
- **Check tools** (for `make ci`): `cargo install --locked cargo-nextest cargo-audit cargo-deny`
- **Optional**: Docker and [hadolint](https://github.com/hadolint/hadolint) for `make docker-build` / `make docker-lint`; `sqlcipher` for `make sql`

## Installation

Project dependencies are managed by `cargo` and will be downloaded automatically when you build or run the project for the first time.

### Setting Up

1. Clone the repository:
```bash
git clone <repository-url>
cd arktos-wallet
```

2. Verify Rust installation:
```bash
rustc --version  # Picks up the version pinned in rust-toolchain.toml
cargo --version
```

3. Build the project:
```bash
cargo build
```

## Development

### Running the Development Server

To start the development server with debug logging enabled:

```sh
make dev
```

This is a shortcut for:

```sh
RUST_LOG=debug cargo run --bin arktos-wallet
```

The server will start on `http://localhost:8080` by default.

## Building

### Debug Build

To build the project without running it (faster builds):

```sh
cargo build
```

Output binary: `target/debug/arktos-wallet`

### Release Build

For an optimized release version (slower build, faster runtime):

```sh
cargo build --release
```

Output binary: `target/release/arktos-wallet`

Use release builds for performance testing and production deployment.

## Testing

### Running Tests

To run the project's full test suite:

```sh
make test      # cargo test --all-features
make nextest   # cargo-nextest + doctests, as run in CI
```

Tests do not require Docker or network access.

`tests/mcp_protocol_tests.rs` starts the real router on an ephemeral port and
exercises `/mcp` with the official `rmcp` client (MCP `2026-07-28`, discover
lifecycle): discovery, `tools/list`, all tools, API-key authentication,
statelessness and per-key wallet isolation. Run it alone with:

```sh
cargo test --test mcp_protocol_tests
``` Docker image builds and Dockerfile linting run in CI (`make docker-build` / `make docker-lint` locally).

### Test Modes

Run specific tests:
```bash
cargo test <test_name>
cargo test -- --test-threads=1  # Single-threaded tests
```

### Writing Tests

Tests are located in:
- `src/` - Unit tests (inline with modules)
- `tests/` - Integration tests (end-to-end)

Example unit test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        // Your test here
    }
}
```

## Code Quality

### Formatting & Linting

```shell
make fmt        # cargo fmt --all
make fmt-check  # cargo fmt --all --check
make check      # cargo check --all-targets --all-features
make lint       # cargo clippy --all-targets --all-features -- -D warnings
```

Before opening a pull request, run everything CI runs (except the Docker jobs):

```shell
make ci
```

Fix some issues automatically:

```shell
make fix
```

### Security Auditing

```bash
make audit  # cargo audit: known vulnerabilities (config: .cargo/audit.toml)
make deny   # cargo deny check: advisories, licenses, bans, sources (config: deny.toml)
```

## Database Operations

The database is created at `DATABASE_PATH` (default `data/arktos.db`, relative
to the working directory) the first time Arktos starts. Pending migrations are
applied automatically at startup.

| Command | Purpose |
|---------|---------|
| `make migrate` | Apply pending migrations and exit (`arktos-wallet migrate`) |
| `make db-info` | Schema version, SQLCipher version, journal mode, pragmas — no secrets |
| `make sql` | Open the database in the `sqlcipher` shell (key passed via a temporary owner-only init file, not the command line) |

### Adding a Migration

1. Add `migrations/V<N>__<description>.sql` (next number, never edit an applied file).
2. Append it to `MIGRATIONS` in `src/database.rs`.
3. Run `cargo test`: `migrations_are_valid` applies all migrations to an empty
   database, and `tests/persistence_tests.rs` checks the resulting schema.

Migrations run inside a transaction and are tracked in `PRAGMA user_version`.
Arktos refuses to open databases created before migrations existed — delete
such development databases and let Arktos recreate them.

### Persistence Code

SQL lives only in `src/key_store.rs` and `src/wallet_store.rs`. They call
`Database::read` / `Database::write`, which run on Tokio's blocking pool
(`write` wraps the closure in a transaction). Do not call `rusqlite` directly
from async services.

## Customization & Extension

Before modifying Arktos, review the extension patterns:

- **Adding Blockchains**: See [Customization Guide - Blockchain Support](../docs/customization-guide.md#1-adding-blockchain-support)
- **Custom Authentication**: See [Customization Guide - Authentication](../docs/customization-guide.md#2-custom-authentication)
- **Storage Backend**: See [Customization Guide - Storage](../docs/customization-guide.md#3-storage-backend-customization)
- **API Extension**: See [Customization Guide - API Design](../docs/customization-guide.md#4-api-customization)

## Compliance & Testing for Regulations

If implementing compliance features:

- **GDPR**: See [Regional Compliance - GDPR](../docs/regional-compliance.md#1-gdpr-compliance-europe)
- **HIPAA**: See [Regional Compliance - HIPAA](../docs/regional-compliance.md#2-hipaa-compliance-healthcare---usa)
- **PCI DSS**: See [Regional Compliance - PCI DSS](../docs/regional-compliance.md#3-pci-dss-compliance-payment-card-industry)
- **SOC 2**: See [Regional Compliance - SOC 2](../docs/regional-compliance.md#4-soc-2-compliance-service-organization-control)

## Development Workflows

### Adding a New Feature

1. **Create a new module**:
   ```rust
   // src/my_feature.rs
   pub fn my_function() { }
   
   #[cfg(test)]
   mod tests {
       #[test]
       fn test_my_function() { }
   }
   ```

2. **Declare module in lib.rs**:
   ```rust
   mod my_feature;
   ```

3. **Write tests first** (TDD):
   ```bash
   cargo test  # Tests fail
   ```

4. **Implement feature** to make tests pass

5. **Refactor** while keeping tests green

6. **Format**:
   ```bash
   make fmt
   ```

7. **Run all checks**:
   ```bash
   make ci
   ```

### Debugging

#### Logging

Enable structured logging:

```bash
RUST_LOG=debug cargo run
RUST_LOG=arktos_wallet::wallet=trace cargo run
```

## Common Commands

| Command | Purpose |
|---------|---------|
| `cargo build` | Build debug binary |
| `cargo build --release` | Build optimized binary |
| `cargo run` | Build and run |
| `cargo doc --open` | Generate and open documentation |
| `make dev` | Run development server with debug logs |
| `make test` / `make nextest` | Run all tests |
| `make fmt` / `make fmt-check` | Format code / check formatting |
| `make lint` | Clippy with warnings denied |
| `make audit` / `make deny` | Dependency security and license checks |
| `make ci` | All local CI checks |
| `make help` | List all targets |

## Troubleshooting

### Build Issues

**`error: failed to parse manifest at ...`**
- Ensure `Cargo.toml` is valid TOML
- Check file encoding is UTF-8

**`error: linker cc not found`**
- Install C compiler: `apt-get install build-essential` (Linux) or Xcode (macOS)

### Runtime Issues

**`database unavailable: … database is locked`**
- Another connection (e.g. an open `make sql` session in a transaction) held the write lock longer than the 5 s busy timeout
- Check for other processes: `lsof data/arktos.db`

**`cannot read database: DATABASE_KEY is wrong …`**
- The database was created with a different `DATABASE_KEY`

**`database was created by a pre-migration version of Arktos`**
- Delete the old development database; Arktos recreates it with the current schema

**`Connection refused on port 8080`**
- Port already in use
- Change port: `PORT=8081 cargo run`
- Or kill existing process: `kill $(lsof -t -i:3000)`

### Test Issues

**Tests hanging**
- Add timeout: `timeout 30 cargo test`
- Run single-threaded: `cargo test -- --test-threads=1`

## Next Steps

- Read [Architecture](../docs/architecture.md) to understand system design
- Check [API Contracts](../docs/api-contracts.md) for API details
- Review [Customization Guide](../docs/customization-guide.md) for extension patterns
- See [Deployment Guide](../docs/deployment-guide.md) for production setup

## Additional Resources

- **Rust Book**: https://doc.rust-lang.org/book/
- **Axum Documentation**: https://docs.rs/axum/latest/
- **Tokio Guide**: https://tokio.rs/tokio/tutorial
- **SQLCipher**: https://www.zetetic.net/sqlcipher/
