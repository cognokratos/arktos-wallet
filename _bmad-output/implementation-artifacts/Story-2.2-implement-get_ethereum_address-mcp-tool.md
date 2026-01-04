# Story 2.2: Implement `get_ethereum_address` MCP Tool

Status: ready-for-dev

## Story

As an AI agent,
I want to derive and retrieve an Ethereum public address for a specified wallet name,
So that I can receive Ethereum funds into a managed wallet.

## Acceptance Criteria

**Given** a wallet with associated accounts exists in the system
**When** an MCP client sends a `get_ethereum_address` request with a valid wallet name
**Then** the system shall derive and return a valid Ethereum public address for that wallet
**And** the `get_ethereum_address` MCP API call shall respond within 100 milliseconds 95% of the time.

## Tasks / Subtasks

- [ ] Implement `get_ethereum_address` MCP tool endpoint (AC: 1, 2)
  - [ ] Add new MCP tool to `src/wallet_services.rs`
  - [ ] Implement logic in `src/wallet_manager.rs` to derive Ethereum address
  - [ ] Utilize `secp256k1` and `tiny-keccak` for derivation
  - [ ] Ensure performance NFR (NFR2) is met
- [ ] Add unit and integration tests for `get_ethereum_address` (AC: 1, 2)
- [ ] Update API documentation for `get_ethereum_address` (FR16)

## Dev Notes

- **Relevant Architecture Patterns and Constraints:**
    - **Functional Requirements:** FR6 (derive and retrieve an Ethereum public address for a specified wallet ID). FR7 (leverage BIP39/BIP32 for address derivation).
    - **Non-Functional Requirements:** NFR2 (`get_ethereum_address` MCP API calls shall respond within 100 milliseconds 95% of the time). NFR4 (minimal CPU and memory resources).
    - **Technical Stack:** Rust, `secp256k1`, `bip39`, `bip32`, `tiny-keccak`.
    - **API Patterns:** MCP Tool Definition (`get_ethereum_address`), Semantic Naming, Problem Details for HTTP APIs (RFC 7807) for error handling.
    - **Security:** Data encryption at rest (NFR5), secure communication (NFR6), input validation (NFR8), audit logging (NFR10).
    - **Testing Standards:** Standard Rust testing with `cargo test`.
    - **Shared State:** Axum Extensions + `Arc<T>` for database connection.

- **Source Tree Components to Touch:**
    - `src/main.rs`: Potentially update router if needed, ensure `tracing` setup is active.
    - `src/wallet_services.rs`: Add new MCP tool endpoint and validation logic.
    - `src/wallet_manager.rs`: Implement the core Ethereum address derivation logic.
    - `src/database.rs`: Interface for retrieving wallet/account data needed for derivation.
    - `src/wallet.rs`: Potentially add or update data structures if needed for Ethereum specifics.
    - `src/error.rs`: Ensure proper error handling for derivation failures.
    - `src/auth.rs`: Authentication for MCP client requests.
    - `Cargo.toml`: Add `tiny-keccak` and `hex` dependencies.
    - `tests/integration_tests.rs`: Add new integration tests.
    - `docs/api-contracts.md`: Update API documentation.

### Technical Requirements

-   **FR6:** The Arktos Wallet system shall allow an AI agent to derive and retrieve an Ethereum public address for a specified wallet name.
-   **FR7:** The Arktos Wallet system shall leverage established cryptographic standards (e.g., BIP39/BIP32) for address derivation.
-   **NFR2:** The `get_ethereum_address` MCP API call shall respond within 100 milliseconds 95% of the time under normal load.
-   **NFR4:** Wallet generation and address derivation processes shall consume minimal CPU and memory resources.
-   **NFR14:** The MCP API shall conform to the Model Context Protocol specification for tool exposure and interaction.
-   **NFR15:** The API documentation shall be sufficient for a developer to integrate an AI agent for basic wallet creation and address retrieval within 2 hours.

### Architecture Compliance

-   **Data Modeling:** Utilize Manual SQL with `rusqlite` for any necessary database interactions (e.g., retrieving wallet/account data).
-   **Data Validation:** Implement Manual Validation for the input `wallet_name`.
-   **API Key Management:** Rely on Database-managed Keys for authentication.
-   **Authorization:** Apply Ownership-Based Authorization to ensure the agent is authorized for the specified wallet.
-   **Security Middleware:** Ensure TLS Configuration, `TraceLayer` (for audit logging), and Security Headers Middleware are active.
-   **API Security Strategy:** Implement Robust Input Validation/Sanitization, Secure Error Handling (RFC 7807), and Comprehensive Logging/Monitoring.
-   **API Design Patterns:** Adhere to Strict MCP Tool Definition (`get_ethereum_address`) and Semantic Naming.
-   **Error Handling Standards:** Use Problem Details for HTTP APIs (RFC 7807) for error responses.
-   **Monitoring and Logging:** Utilize Structured Logging (using `tracing` crate).

### Library and Framework Requirements

-   **Language:** Rust 2021 edition.
-   **Asynchronous Runtime:** `tokio`.
-   **Web Framework:** `axum` (for the MCP endpoint).
-   **Database:** `rusqlite` (with SQLCipher for encryption).
-   **Cryptographic Libraries:**
    -   `secp256k1` (for elliptic curve operations, public key handling).
    -   `tiny-keccak` (for Keccak-256 hashing for Ethereum address derivation).
    -   `bip39`, `bip32` (for HD wallet functionality).
-   **Serialization/Deserialization:** `serde` (for JSON handling).
-   **Hexadecimal Encoding:** `hex` (for formatting addresses).
-   **Logging:** `tracing` (for structured logging).
-   **MCP Server:** `rmcp` SDK.

### File Structure Requirements

-   **Project Organization:** Standard Rust `src/` and `tests/` structure with feature-based modules.
    -   `src/main.rs`: Application entry point.
    -   `src/wallet_services.rs`: Will house the `get_ethereum_address` MCP tool definition and request handling.
    -   `src/wallet_manager.rs`: Will contain the core logic for Ethereum address derivation.
    -   `src/database.rs`: For database interactions to retrieve wallet/account data.
    -   `src/error.rs`: For custom error types and RFC 7807 conversions.
    -   `src/auth.rs`: For API key authentication.
    -   `src/wallet.rs`: For data structures related to wallets and accounts.
-   **Conventional Locations:**
    -   Unit tests co-located with modules (`#[cfg(test)]`).
    -   Integration tests in top-level `tests/` directory (`tests/integration_tests.rs`).

### Testing Requirements

-   **Testing Framework:** Standard Rust testing with `cargo test`.
-   **Unit Tests:** Implement unit tests for the Ethereum address derivation logic within `src/wallet_manager.rs`.
-   **Integration Tests:** Add integration tests in `tests/integration_tests.rs` to verify the `get_ethereum_address` MCP tool endpoint, including authentication, correct address derivation, and adherence to performance NFRs (NFR2).

### Previous Story Intelligence

-   **Story 2.1 (`get_bitcoin_address` MCP Tool):**
    -   No dedicated story file found for detailed learnings.
    -   However, Story 2.1 (implementing `get_bitcoin_address`) serves as a direct precedent and architectural pattern for this story. The core logic of retrieving a wallet/account from the database, deriving an address using cryptographic standards (BIP39/BIP32), and exposing it via an MCP tool will be highly similar.
    -   Learnings from Story 2.1's implementation regarding database interaction, error handling, performance optimization, and MCP tool integration should be directly applied here.

### Git Intelligence Summary

-   **Note:** Git analysis for previous work patterns was not performed as a dedicated story file for Story 2.1 was not found. If such a file exists (e.g., as part of a commit history not reflected in the file system), manual review of relevant commits for `get_bitcoin_address` implementation is recommended to understand established patterns, modified files, and testing approaches.

### Latest Technical Information

-   **Ethereum Address Derivation from `secp256k1` Public Key (Rust):**
    1.  **Public Key:** Obtain the `secp256k1` public key (typically 65 bytes, uncompressed, starting with `0x04`). Remove the `0x04` prefix to get the raw 64-byte public key (x and y coordinates concatenated).
    2.  **Keccak-256 Hash:** Compute the Keccak-256 hash of these 64 bytes. Use `tiny_keccak::Keccak::v256()`.
    3.  **Extract Address:** Take the last 20 bytes of the resulting 32-byte Keccak-256 hash.
    4.  **Format:** Format the address as a hexadecimal string, prefixed with `0x`. Use the `hex` crate for encoding.

-   **Dependencies to add (to `Cargo.toml`):**
    ```toml
    secp256k1 = { version = "0.28", features = ["rand-std"] } # Or specific version used in project
    tiny-keccak = { version = "2.0", features = ["keccak"] } # Or specific version used in project
    hex = "0.4" # Or specific version used in project
    ```

### Project Context Reference

-   For overall project vision, goals, and high-level context, refer to:
    -   `_bmad-output/project-context.md`

## Dev Agent Record

### Agent Model Used

gemini-1.5-flash

### Completion Notes List

-   Story requirements and acceptance criteria extracted from `epics.md`.
-   Architectural constraints, patterns, and relevant decisions for implementation extracted from `architecture.md`.
-   Specific technical guidance for Ethereum address derivation in Rust gathered from web search.
-   Dependencies required (`secp256k1`, `tiny-keccak`, `hex`) identified.
-   Project structure and file organization patterns reinforced.
-   Testing requirements outlined for unit and integration tests.
-   Cross-reference to previous story (2.1) made for architectural consistency, despite absence of a dedicated story file.

## Project Structure Notes

- **Alignment with unified project structure (paths, modules, naming):**
    - The new functionality will align with the existing modular structure: `wallet_services` for API, `wallet_manager` for business logic, `database` for persistence.
    - Naming conventions (`snake_case` for functions/tools, `camelCase` for JSON fields) must be strictly followed.
    - Error handling will use the `src/error.rs` module and `Result<T, E>` pattern.

- **Detected conflicts or variances (with rationale):** None expected if adhering to the established architecture.

### References

- **Ethereum Address Derivation:**
    - Steps: 1) `secp256k1` public key (uncompressed, 64 bytes after removing `0x04` prefix). 2) Keccak-256 hash of these 64 bytes. 3) Last 20 bytes of hash = Ethereum address. 4) Prefix with `0x`.
    - **Dependencies:** `secp256k1`, `tiny-keccak` (for `Keccak::v256`), `hex`.
    - [Source: Web Search - Rust Ethereum Address Derivation]

- [Source: PRD.md#Functional Requirements]
- [Source: PRD.md#Non-Functional Requirements]
- [Source: architecture.md#Core Architectural Decisions]
- [Source: architecture.md#Implementation Patterns & Consistency Rules]
