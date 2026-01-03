# Story 1.4: Implement Initial Dockerization and Rust Project Structure

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want the Arktos Wallet system to be deployable via containerization and built with an idiomatic Rust project structure,
So that I can easily set up a development environment and deploy the system.

## Acceptance Criteria

1.  **Given** the core wallet and API key authentication functionalities are implemented
2.  **When** a developer wants to set up or deploy the system
3.  **Then** a `Dockerfile` shall exist to build a container image of the application
4.  **And** the application shall be implemented entirely in Rust with a clean, idiomatic project structure
5.  **And** the system shall be deployable via containerization (e.g., Docker).

## Tasks / Subtasks

- [x] Task 1: Initialize Rust project and manage dependencies (AC: #4)
  - [x] Create new Rust binary project using `cargo new arktos-wallet --bin`
  - [x] Add core dependencies to `Cargo.toml` (Axum, Tokio, Rusqlite with SQLCipher, RMCP, etc.)
- [x] Task 2: Define idiomatic Rust project structure (AC: #4)
  - [x] Create feature-based modules (`src/main.rs`, `src/config.rs`, `src/auth.rs`, `src/database.rs`, `src/wallet.rs`, `src/wallet_manager.rs`, `src/wallet_services.rs`, `src/wallet_store.rs`, `src/error.rs`, `src/utils.rs`, `src/telemetry.rs`)
  - [x] Organize tests into `src/.../tests` (unit) and `tests/integration_tests.rs` (integration)
- [ ] Task 3: Implement initial `Dockerfile` (AC: #3, #5)
  - [ ] Create multi-stage `Dockerfile` using `cargo-chef` for efficient builds
  - [ ] Use `distroless` base image for final stage
  - [ ] Configure Dockerfile for local deployment and testing

## Dev Notes

-   **Technical Stack**: Rust 2021 edition, `tokio` for async runtime, `axum` for web framework, `rusqlite` with SQLCipher, `rmcp` for MCP server. `bip39`, `bip32`, `rust-bitcoin`, `tiny-keccak`.
-   **Code Structure**: Standard Rust `src/` and `tests/` with feature-based modules (`src/main.rs`, `src/config.rs`, `src/auth.rs`, etc.).
-   **Testing Standards**: Standard Rust testing with `cargo test`.
-   **Deployment Patterns**: Multi-stage Docker builds using `cargo-chef` and `distroless` base image. Self-hosted (Docker Container).
-   **Naming Patterns**: Rust Idiomatic Conventions (`snake_case` for functions/variables, `PascalCase` for types, `snake_case` for modules/files, `SCREAMING_SNAKE_CASE` for constants).

### Project Structure Notes

- Alignment with unified project structure (paths, modules, naming)
- Detected conflicts or variances (with rationale)

### Architecture Compliance

-   **Selected Starter**: Standard Rust Project with Axum and Core Dependencies. Project initialized with `cargo new arktos-wallet --bin`.
-   **Language & Runtime**: Rust 2021 edition, `tokio` as the asynchronous runtime.
-   **Build Tooling**: `cargo` is the primary build tool. Docker multi-stage builds incorporating `lukemathwalker/cargo-chef` and `distroless` base images for efficient and lean production images.
-   **Code Organization**: Starts with `src/main.rs`. Modularization into logical concerns (e.g., `src/wallet_manager.rs`, `src/services.rs`) using Rust's module and crate system.
-   **Environment Configuration**: Environment Variables.
-   **Monitoring and Logging**: Structured Logging (using `tracing` crate).
-   **Scaling Strategy**: Horizontal Scaling (Stateless Instances).

### Previous Story Intelligence (Story 1.3: Implement API Key Authentication for MCP Client Requests)

-   **Key Takeaways**: Story 1.3 focused on implementing API key authentication, including secure storage of keys, middleware for validation, and MCP tools for management.
-   **Relevant Learnings**: The implementation established patterns for database interactions (`src/db.rs`, `src/models.rs`), authentication logic (`src/auth.rs`), and exposing functionalities via MCP (`src/services.rs`, `src/main.rs`). It also reinforced the use of RFC 7807 for error handling and the `tracing` crate for logging.
-   **Established Patterns**: Database-managed keys, ownership-based authorization, security middleware, robust input validation, Axum Extensions for state management, and Rust's `Result<T, E>` for error handling.
-   **File Changes**: `src/auth.rs`, `src/db.rs`, `src/models.rs`, `src/services.rs`, `src/main.rs`, `Cargo.toml`, `tests/api_key_auth_tests.rs`. These files provide examples of the project structure and coding conventions that should be extended in this story.

### Git Intelligence Summary

-   Recent commits (`Story 1.1`, `Story 1.2`, `Story 1.3`) indicate an iterative build-out of core wallet functionalities and API key authentication.
-   The current work on Story 1.4 will formalize and consolidate the project structure, Dockerization, and deployment aspects, building upon the established Rust codebase from previous stories.

### References

-   [Source: /Users/victornitu/Projects/CognoKratos/arktos-wallet/_bmad-output/project-context.md]
-   [Source: /Users/victornitu/Projects/CognoKratos/arktos-wallet/_bmad-output/prd.md]
-   [Source: /Users/victornitu/Projects/CognoKratos/arktos-wallet/_bmad-output/planning-artifacts/architecture.md]
-   [Source: /Users/victornitu/Projects/CognoKratos/arktos-wallet/_bmad-output/epics.md#Story 1.4: Implement Initial Dockerization and Rust Project Structure]
-   [Source: /Users/victornitu/Projects/CognoKratos/arktos-wallet/_bmad-output/implementation-artifacts/Story-1.3-implement-api-key-authentication-for-mcp-client-requests.md]

## Dev Agent Record

### Agent Model Used

gemini-1.5-flash

### Debug Log References

### Completion Notes List

-   Story title, objectives, and acceptance criteria extracted from epics.md.
-   Dev Notes populated with technical stack, code structure, testing standards, deployment patterns, and naming conventions from architecture.md.
-   Tasks and subtasks defined based on acceptance criteria and architectural guidance.
-   Previous story intelligence from Story 1.3 extracted, highlighting relevant learnings and established patterns.
-   Git intelligence summary provides context from recent commits.
-   References to PRD, Architecture, Epics, and Project Context documents included.
-   The generated story is ready for development, providing comprehensive context.

### File List

-   `Cargo.toml` (modified to add dependencies)
-   `src/main.rs` (initial setup for Axum server)
-   `src/config.rs` (for environment variable loading)
-   `Dockerfile` (newly created for multi-stage build)
-   Potentially new module files in `src/` for initial structuring based on architectural guidance (e.g., `src/auth.rs`, `src/database.rs`, `src/wallet.rs`, `src/wallet_manager.rs`, etc.)
-   `tests/integration_tests.rs` (initial setup)