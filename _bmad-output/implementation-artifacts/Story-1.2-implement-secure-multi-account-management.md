# Story 1.2: Implement Secure Multi-Account Management

Status: ready-for-dev

## Story

As a system owner,
I want my AI agent to manage multiple distinct blockchain accounts within a single wallet,
So that I can organize and isolate my funds effectively.

## Acceptance Criteria

1.  **Given** a wallet has been created via `create_wallet`
2.  **When** the AI agent needs to create or manage a new account within that wallet
3.  **Then** the system shall securely store and associate this new account with the parent wallet in the database.
4.  **And** the system shall leverage established cryptographic standards (BIP32) for hierarchical account derivation from the wallet's master key. (FR7)
5.  **And** sensitive data for these accounts (e.g., derived private keys) shall be encrypted at rest and accessible only by the system owner. (FR9, FR10, NFR5)

## Tasks / Subtasks

- [ ] **Task 1: Update Database Schema and Models for Accounts**
  - [ ] Subtask 1.1: Create a new database migration script in `db/migrations/` to add the `accounts` table. It should include columns like `id`, `wallet_id` (foreign key to `wallets`), `account_index`, `private_key` (encrypted), `public_key`, and `chain_type` (e.g., Bitcoin, Ethereum).
  - [ ] Subtask 1.2: Update the `Account` struct in `src/models.rs` to reflect the new table structure. Ensure it derives `serde::Serialize` and `schemars::JsonSchema`.
  - [ ] Subtask 1.3: Update `src/db.rs` with functions to insert and retrieve accounts associated with a wallet.

- [ ] **Task 2: Implement Hierarchical Key Derivation Logic**
  - [ ] Subtask 2.1: In `src/wallet_manager.rs`, create a function that takes a wallet's mnemonic (retrieved and decrypted from the DB) and an account index.
  - [ ] Subtask 2.2: Use the `bip39` and `bip32` crates to derive the correct child private key based on the standard derivation paths for Bitcoin (m/44'/0'/0'/0/index) and Ethereum (m/44'/60'/0'/0/index).
  - [ ] Subtask 2.3: The derived private key must be encrypted using the same mechanism as the mnemonic before being stored in the database.

- [ ] **Task 3: Integrate Account Creation into Address Derivation**
  - [ ] Subtask 3.1: **Account Creation as Side Effect**: The `create_account` function will be an internal helper, called by `get_bitcoin_address` or `get_ethereum_address` (from Epic 2) if an account for a given chain and wallet does not already exist. It will not be exposed as a direct MCP tool.
  - [ ] Subtask 3.2: Ensure that when an address is requested, the system first checks for an existing account for that wallet and chain. If none exists, a new account is created, stored, and then the address is derived.

- [ ] **Task 4: Implement Unit and Integration Tests**
  - [ ] Subtask 4.1: Add unit tests in `src/wallet_manager.rs` to verify correct BIP32 key derivation for both Bitcoin and Ethereum.
  - [ ] Subtask 4.2: Add unit tests in `src/db.rs` to verify the creation and retrieval of account records.
  - [ ] Subtask 4.3: Extend the integration tests in `tests/integration_tests.rs` to create a wallet, then derive an address (which implicitly creates an account), and verify the account record is created correctly in the database.

## Dev Notes

-   **CRITICAL CONCEPT**: This story enables stories 2.1 and 2.2. The core is implementing the **BIP32 Hierarchical Derivation**. The wallet created in Story 1.1 represents the **master seed**. This story creates the logic to derive child accounts from that seed.

-   **Relevant architecture patterns and constraints**:
    -   **Cryptography**: You **MUST** use the `bip32` crate for key derivation. Follow its documentation closely. The master seed is derived from the mnemonic using `bip39`.
    -   **Derivation Paths**: Use standard BIP44 derivation paths.
        -   **Bitcoin:** `m/44'/0'/0'/0/{account_index}`
        -   **Ethereum:** `m/44'/60'/0'/0/{account_index}`
    -   **Database**: A new `accounts` table is required. You MUST use `refinery` to create a new migration file. The `wallet_id` column MUST be a foreign key that references the `wallets` table to enforce relational integrity.
    -   **Encryption**: Derived private keys are sensitive and **MUST** be encrypted before being stored in the database, using the existing cryptographic utilities in `src/crypto.rs`.
    -   **Modularity**: All key derivation logic should be encapsulated within `src/wallet_manager.rs`. All database interactions should be in `src/db.rs`.

-   **Source tree components to touch**:
    -   `db/migrations/`: To add the new `V002_add_accounts_table.sql` migration script.
    -   `src/models.rs`: To update the `Account` struct.
    -   `src/db.rs`: To add `insert_account`, `get_account` functions.
    -   `src/wallet_manager.rs`: To add the core `derive_account_key` logic.
    -   `src/error.rs`: Potentially add new error variants for derivation or account-related failures.
    -   `tests/integration_tests.rs`: To add tests for the full account creation lifecycle.

-   **Testing standards summary**:
    -   Verify that derived keys are correct for known test vectors if possible.
    -   Ensure that attempting to create an account for a non-existent wallet fails with a specific error.
    -   Confirm that database lookups for accounts by wallet ID are successful.

### References

-   [Source: architecture.md#Data-Architecture](docs/architecture.md#Data-Architecture)
-   [Source: architecture.md#Project-Structure-&-Boundaries](docs/architecture.md#Project-Structure-&-Boundaries)
-   [Source: _bmad-output/epics.md#Story-1.2](_bmad-output/epics.md#Story-1.2)
-   [Source: Previous Story 1.1](_bmad-output/implementation-artifacts/Story-1.1-create_wallet.md)
