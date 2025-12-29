# Development Guide

This guide provides instructions for setting up the development environment, building, and running the `arktos-wallet` project.

## Prerequisites

*   **Rust**: A recent version of the Rust toolchain is required. You can install it from [rust-lang.org](https://www.rust-lang.org/). The project is configured for the 2024 edition of Rust.

## Installation

Project dependencies are managed by `cargo`, the Rust build tool and package manager. They will be downloaded automatically when you build or run the project for the first time.

## Development

To start the development server with debug logging enabled, run the following command from the `Makefile`:

```sh
make dev
```

This is a shortcut for:

```sh
RUST_LOG=debug cargo run
```

## Building

To build the project without running it, use:

```sh
cargo build
```

To build an optimized release version, use:

```sh
cargo build --release
```

## Testing

To run the project's test suite, use the standard cargo command:

```sh
cargo test
```

## Formatting

To format the code according to the project's style guidelines, run:

```sh
make format
```

This uses `cargo fmt` to format the entire codebase.
