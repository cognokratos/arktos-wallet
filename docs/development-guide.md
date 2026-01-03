# Development Guide

This guide provides instructions for setting up the development environment, building, and running the `arktos-wallet` project locally.

## Prerequisites

- **Rust**: A recent version of the Rust toolchain is required. Install from [rust-lang.org](https://www.rust-lang.org/). The project is configured for the 2024 edition of Rust.
- **Cargo**: The Rust package manager (installed with Rust)
- **SQLite 3.x**: Required by SQLCipher for database operations
- **Make** (optional): For convenient command shortcuts via `Makefile`

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
rustc --version  # Should show Rust 1.75+
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
make test
# or
cargo test
```

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

Format all code according to Rust conventions & Check for code quality issues:

```shell
make format
```

This runs `fmt` + `check` + `clippy` on the entire codebase.
Or manually:

```shell
cargo fmt
cargo check
cargo clippy
```

Fix some issues automatically:

```shell
make fix
```

or manually:

```bash
cargo clippy --fix --allow-dirty
cargo fix --allow-dirty
```

### Security Auditing

Check dependencies for known vulnerabilities:

```bash
cargo audit
```

## Database Operations

### Running Migrations

Apply database migrations:

```bash
# Using refinery (if configured)
cargo run --bin migrate
```

### Database Management

Access the database directly:

```bash
make sql
# Or manually:
sqlcipher data/arktos.db
```

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

6. **Format and lint**:
   ```bash 
   make format
   ```

7. **Test the entire suite**:
   ```bash
   make test
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
| `cargo test` | Run all tests |
| `cargo fmt` | Format code |
| `cargo clippy` | Lint code |
| `cargo audit` | Check for vulnerabilities |
| `cargo doc --open` | Generate and open documentation |
| `make dev` | Run development server with debug logs |
| `make format` | Format code (shortcut) |

## Troubleshooting

### Build Issues

**`error: failed to parse manifest at ...`**
- Ensure `Cargo.toml` is valid TOML
- Check file encoding is UTF-8

**`error: linker cc not found`**
- Install C compiler: `apt-get install build-essential` (Linux) or Xcode (macOS)

### Runtime Issues

**`SQLCipher: database is locked`**
- Another process is using the database
- Check for running instances: `lsof arktos.db`
- Restart development server

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
