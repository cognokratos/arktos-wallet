# Story 2.1: Implement `get_bitcoin_address` MCP Tool

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an AI agent,
I want to derive and retrieve a Bitcoin public address for a specified wallet ID,
So that I can receive Bitcoin funds into a managed wallet.

## Acceptance Criteria

1. **Given** a wallet with associated accounts exists in the system
2. **When** an MCP client sends a `get_bitcoin_address` request with a valid wallet ID
3. **Then** the system shall derive and return a valid Bitcoin public address for that wallet
4. **And** the `get_bitcoin_address` MCP API call shall respond within 100 milliseconds 95% of the time.

## Tasks / Subtasks

- [ ] Implement `get_bitcoin_address` MCP tool (`src/wallet_services.rs`)
  - [ ] Define the `get_bitcoin_address` MCP tool and its input/output structures.
  - [ ] Implement input validation for the `wallet_id`.
  - [ ] Call `wallet_manager` to derive the Bitcoin address.
- [ ] Implement Bitcoin address derivation logic (`src/wallet_manager.rs`)
  - [ ] Retrieve the wallet's extended public key (or necessary components) using `wallet_store`.
  - [ ] Use `rust-bitcoin` and `bip32` (or similar) to derive a Bitcoin public address based on standard derivation paths (e.g., BIP44).
  - [ ] Ensure only public data (address) is returned.
- [ ] Update `src/wallet_store.rs` (if necessary)
  - [ ] Ensure efficient retrieval of wallet/account data required for address derivation.
- [ ] Add unit tests for address derivation logic (`src/wallet_manager.rs`).
- [ ] Add integration tests for the `get_bitcoin_address` MCP endpoint (`tests/integration_tests.rs`).
  - [ ] Verify successful address derivation for valid wallet IDs.
  - [ ] Verify error handling for invalid or non-existent wallet IDs.
  - [ ] Verify performance meets NFR2 (100ms response time).
- [ ] Update API documentation (rustdoc) for `get_bitcoin_address` tool.

## Dev Notes

- Relevant architecture patterns and constraints:
    - All incoming data from API requests will be validated immediately, with robust sanitization to prevent injection issues.
    - Internal error handling will use idiomatic Rust `Result` and custom error types, converting to RFC 7807 Problem Details for HTTP API responses.
    - All AI Agents MUST adhere to defined Naming Conventions, Structure Patterns, API Response/Data Exchange Formats, Communication Patterns, Error Handling, and Validation Timing.
    - Successful API responses will use a wrapped `data` field (e.g., `{"data": {"walletId": "uuid"}}`) and JSON fields will use `camelCase`.
- Source tree components to touch:
    - `src/wallet_services.rs`: Define MCP tool and handle API request.
    - `src/wallet_manager.rs`: Implement core derivation logic.
    - `src/wallet_store.rs`: Potentially modify for efficient data access.
    - `src/utils.rs`: For cryptographic helpers (e.g., hash functions used in address derivation).
    - `src/error.rs`: For custom error types and RFC 7807 problem details.
- Testing standards summary:
    - Standard Rust testing with `cargo test`. Unit tests co-located, integration tests in top-level `tests/`.
    - Performance test to ensure `get_bitcoin_address` meets NFR2 (100ms).

### Project Structure Notes

- Alignment with unified project structure (paths, modules, naming):
    - Modules like `wallet_manager`, `wallet_services`, `database`, `auth`, `error`, `utils` should be distinct `.rs` files under `src/`.
    - Unit tests within `#[cfg(test)]` blocks in module files.
    - Integration tests in `tests/integration_tests.rs`.
- Detected conflicts or variances (with rationale): None anticipated based on current understanding. Adhere strictly to the defined project structure from `architecture.md`.

### References

- [Source: _bmad-output/epics.md#Epic 2: Address Generation and Retrieval]
- [Source: _bmad-output/prd.md#Functional Requirements] (FR5, FR7)
- [Source: _bmad-output/prd.md#NonFunctional Requirements] (NFR2)
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
- `src/wallet_services.rs` (new/modified)
- `src/wallet_manager.rs` (new/modified)
- `src/wallet_store.rs` (modified, if needed)
- `src/utils.rs` (modified, if cryptographic helpers are added)
- `src/error.rs` (modified, if new error types are introduced)
- `tests/integration_tests.rs` (new/modified)