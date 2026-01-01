# Story 1.3: Implement API Key Authentication for MCP Client Requests

Status: ready-for-dev

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

- [ ] Implement API key storage in database (AC: #1)
  - [ ] Create API keys table with hashed keys and metadata
  - [ ] Implement database functions for key creation, validation, and revocation
- [ ] Implement API key authentication middleware (AC: #1)
  - [ ] Create middleware to extract API key from requests
  - [ ] Validate API key against stored keys
  - [ ] Block requests with invalid API keys
- [ ] Implement API key management endpoints (AC: #1)
  - [ ] Create endpoint for generating new API keys
  - [ ] Create endpoint for revoking existing API keys
  - [ ] Implement proper authorization for key management

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

- API keys must be stored securely using hashing (not plain text)
- Authentication middleware should be applied to all MCP endpoints except health checks
- Consider rate limiting for API key validation attempts to prevent brute force attacks
- Ensure audit logging captures authentication attempts (success/failure)
- API key generation should follow cryptographically secure random generation practices

### File List

- `src/auth.rs` - Authentication module with API key validation logic
- `src/db.rs` - Database functions for API key storage and retrieval
- `src/main.rs` - Middleware integration
- `src/services.rs` - API key management MCP tools
- `db/migrations/V003_add_api_keys.sql` - Database migration for API keys table