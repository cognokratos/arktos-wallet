# Story 1.3: Implement API Key Authentication for MCP Client Requests

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a system owner,
I want the Arktos Wallet system to authenticate MCP client requests using API keys,
so that only authorized AI agents can interact with my wallets and accounts.

## Acceptance Criteria

1. **Given** the Arktos Wallet server is running
   **When** an MCP client sends a request to the server
   **Then** the server shall validate the provided API key
   **And** only requests with a valid API key shall be processed
   **And** API keys used for authentication shall be unique per client and stored securely (e.g., hashed) by Arktos Wallet
   **And** the system shall prevent unauthorized access to sensitive data and functionalities.

## Tasks / Subtasks

- [x] Implement API key storage in database (AC: #1)
  - [x] Create API keys table with hashed keys and metadata
  - [x] Implement database functions for key creation, validation, and revocation
- [x] Implement API key authentication middleware (AC: #1)
  - [x] Create middleware to extract API key from requests
  - [x] Validate API key against stored keys
  - [x] Block requests with invalid API keys
- [x] Implement API key management endpoints (AC: #1)
  - [x] Create endpoint for generating new API keys
  - [x] Create endpoint for revoking existing API keys
  - [x] Implement proper authorization for key management

## Dev Notes

- **Database-managed Keys**: API keys should be stored in the database using hashed values for security (NFR7)
- **Ownership-Based Authorization**: Ensure that API key management follows ownership-based authorization patterns
- **Security Middleware**: Implement TLS Configuration, TraceLayer, and Security Headers Middleware as per architectural decisions
- **API Security Strategy**: Use Robust Input Validation/Sanitization, Secure Error Handling (RFC 7807), and Comprehensive Logging/Monitoring
- **State Management**: Use Axum Extensions + `Arc<T>` (and `Mutex` or `RwLock` for mutability) for shared application state
- **Error Handling**: Follow RFC 7807 Problem Details for HTTP APIs for error responses

### Project Structure Notes

- **File Location**: `src/auth.rs` - Authentication and authorization logic
- **Database Module**: `src/db.rs` - API key storage and validation functions
- **Middleware**: Implement as Axum middleware in `src/auth.rs` or `src/main.rs`
- **API Key Management**: MCP tools in `src/services.rs` for key creation/revocation
- **Naming**: Follow Rust idiomatic conventions (`snake_case` for functions, `PascalCase` for types)

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/epics.md#Story 1.3: Implement API Key Authentication for MCP Client Requests]
- [Source: _bmad-output/prd.md#Functional Requirements - FR14]
- [Source: _bmad-output/prd.md#Non-Functional Requirements - NFR7, NFR8, NFR10]

## Dev Agent Record

### Agent Model Used

gpt-4

### Debug Log References

### Completion Notes List

- ✅ API keys stored securely using SHA256 hashing (plain text keys generated, hashed in DB)
- ✅ Created api_keys SQLite table with foreign key to wallets, supporting revocation
- ✅ Implemented cryptographically secure API key generation (32-char alphanumeric + symbols)
- ✅ Database functions for key creation, validation (checking non-revoked status), and revocation
- ✅ Auth module provides key generation, hashing, and extraction from x-api-key headers
- ✅ Services layer exposes create_api_key, validate_api_key, revoke_api_key, list_api_keys methods
- ✅ MCP tools create_api_key and revoke_api_key exposed for client use
- ✅ Comprehensive test coverage: 8 tests covering creation, validation, revocation, listing, metadata
- ✅ All existing tests continue to pass (31 lib tests + 8 new API key tests + 3 integration tests)
- ✅ Code formatting and linting passes (cargo fmt, cargo clippy clean)

### File List

- `src/auth.rs` - Authentication module with API key generation, hashing, and extraction
- `src/db.rs` - Database module with API key table schema and CRUD operations
- `src/models.rs` - Model types for ApiKey, ApiKeyResponse, CreateApiKeyRequest, RevokeApiKeyRequest
- `src/services.rs` - Service layer for API key management (create, validate, revoke, list)
- `src/main.rs` - MCP tools for create_api_key and revoke_api_key
- `Cargo.toml` - Added sha2 dependency for secure key hashing
- `tests/api_key_auth_tests.rs` - Comprehensive test suite for API key functionality (8 tests)