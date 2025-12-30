---
name: "project-context"
description: "Project Context for Arktos Wallet"
date: "Tuesday, December 30, 2025"
---

# Project Context: Arktos Wallet

This document provides a concise overview of the project's technical landscape, critical implementation rules, and architectural guidelines that AI agents **MUST** follow to ensure consistent and high-quality code generation.

## 1. Project Overview

-   **Project Name:** Arktos Wallet
-   **Description:** A non-custodial, AI agent-controlled cold wallet server written in Rust, enabling secure Bitcoin and Ethereum fund management. Designed as an educational blueprint.
-   **Primary Domain:** API/Backend (Fintech)
-   **Complexity:** High (due to security, cryptography, and performance requirements)
-   **Target Audience (Blueprint Users):** Tech entrepreneurs building AI-driven financial applications.

## 2. Technology Stack & Key Dependencies

**AI agents MUST use the following technologies and specified versions:**

-   **Language:** Rust (2024 Edition)
    -   **Coding Style:** Prefer Functional Programming.
    -   **Code Naming Conventions:** Follow Rust Idiomatic Conventions:
        -   `snake_case` for function/variable/module/file names.
        -   `PascalCase` for type names (structs, enums, traits).
        -   `SCREAMING_SNAKE_CASE` for constants.
-   **Web Framework:** Axum (`0.8`)
-   **Async Runtime:** Tokio (`1`, with `full` features)
-   **MCP Server:** RMCP (`0.12`, with `server`, `macros`, `transport-streamable-http-server` features)
-   **Database:** SQLite with SQLCipher
    -   **Crate:** `rusqlite` (`0.38`, with `bundled-sqlcipher-vendored-openssl` features)
    -   **Data Modeling:** Manual SQL.
    -   **Migrations:** `refinery` crate (must be added to `Cargo.toml`).
    -   **Database Naming:** `snake_case` for all elements; plural table names.
-   **Serialization:** `serde` (`1`, with `derive` feature), `serde_json` (`1`), `schemars` (`1`).
-   **Logging:** `tracing` (`0.1`), `tracing-subscriber` (`0.3`, with `env-filter` feature).
    -   **Approach:** Structured Logging to stdout/stderr.
-   **Error Handling (Internal):** `Result<T, E>` with custom error types (using `thiserror` or `anyhow`).
    -   **Crate:** `anyhow` (`1.0.100`) is already included; `thiserror` to be added for custom errors.
-   **Cryptography/Blockchain Libraries:**
    -   `bip39` (`2`)
    -   `bitcoin` (`0.32`, with `std` feature)
    -   `tiny-keccak` (`2`)
-   **Docker:**
    -   **Multi-stage Build:** `lukemathwalker/cargo-chef` for build stage.
    -   **Base Runtime Image:** `gcr.io/distroless/static-debian12:nonroot` for lean runtime.
-   **Build Tooling:** `cargo` (standard Rust build system).
-   **Testing:** `cargo test` (standard Rust testing).
-   **Linting/Formatting:** `cargo fmt`, `cargo clippy`.

## 3. Core Architectural Decisions & Implementation Patterns

**AI agents MUST adhere to these architectural decisions and implementation patterns:**

### Data Architecture
-   **Data Validation:** Manual Validation, performed **Early Validation (at API Boundary)**.
-   **Caching Strategy:** Database-Level Optimizations (rely on SQLite's internal mechanisms, query optimization).

### Authentication & Security
-   **API Key Management:** Database-managed Keys (hashed and stored securely).
-   **Authorization:** Ownership-Based Authorization (agent can only manage owned/assigned resources).
-   **Security Middleware:** TLS Configuration, `TraceLayer`, and Security Headers Middleware.
-   **Data Encryption:** Rely **only** on SQLCipher (at rest) and TLS (in transit). No additional application-level encryption.
-   **API Security Strategy:** Robust Input Validation/Sanitization, Secure Error Handling (RFC 7807), Comprehensive Logging/Monitoring.

### API & Communication
-   **API Design Patterns:** Strict MCP Tool Definition and Semantic Naming.
-   **API Naming Conventions:** `snake_case` for MCP tool names and arguments; `camelCase` for JSON fields in requests/responses.
-   **API Documentation:** Code-generated Documentation (`rustdoc`).
-   **Error Handling Standards (API):** Problem Details for HTTP APIs (RFC 7807).
-   **Rate Limiting:** **No Rate Limiting** (adhering to NFR for trusted, single-system-owner environment).
-   **Internal Communication:** Direct function calls (no explicit event system).
-   **State Management:** Axum Extensions + `Arc<T>` (and `Mutex` or `RwLock` for mutability) for shared application state.

### Infrastructure & Deployment
-   **Hosting Strategy:** Self-hosted (Docker Container).
-   **CI/CD Pipeline:** Basic CI (Build & Test Only, e.g., `.github/workflows/ci.yml`) coupled with Manual CI/CD Instructions (`Makefile`).
-   **Environment Configuration:** Environment Variables.
-   **Monitoring and Logging:** Structured Logging (`tracing`) to stdout/stderr.
-   **Scaling Strategy:** Horizontal Scaling (Stateless Instances).

## 4. Project Structure & File Organization

**AI agents MUST respect the defined project structure:**

-   **Project Organization:** Standard Rust `src/` and `tests/` structure with feature-based modules.
    -   `src/main.rs`: Application entry point.
    -   Modules within `src/` organized by concern: `config.rs`, `wallet_manager.rs`, `db.rs`, `auth.rs`, `api_handlers.rs`, `error.rs`, `models.rs`, `utils.rs`, `telemetry.rs`.
    -   Unit tests: Co-located (`#[cfg(test)]`) within `src/` modules.
    -   Integration tests: Top-level `tests/` directory (`tests/integration_tests.rs`, `tests/common/`).
    -   Shared utilities: `src/utils/`.
-   **File Structure (Conventional Locations):**
    -   `Cargo.toml`, `Cargo.lock`, `Makefile`, `.env.example`, `.gitignore`, `Dockerfile`, `README.md`.
    -   `.github/workflows/ci.yml` for basic CI.
    -   `db/migrations/` for `refinery` SQL migration scripts.
    -   `docs/` for manual project documentation.

## 5. Anti-Patterns & Pitfalls to Avoid

**AI agents MUST avoid the following anti-patterns:**

-   **Inconsistent Naming:** Mixing `snake_case` and `camelCase` where a single convention is specified.
-   **Bypassing Validation:** Accepting unvalidated input into business logic.
-   **Direct Panics for Recoverable Errors:** Using `panic!` instead of `Result` for expected error conditions.
-   **Information Leakage:** Exposing internal error details to API clients.
-   **Hardcoding Configuration:** Embedding sensitive values directly in code instead of using environment variables.
-   **Uncontrolled State:** Managing shared mutable state without `Arc<Mutex<T>>` or `Arc<RwLock<T>>` in concurrent contexts.
-   **Ignoring Code Style:** Deviating from `rustfmt` or `clippy` suggestions.
-   **Modifying `Cargo.toml` without clear architectural reason.**
