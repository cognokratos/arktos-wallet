---
stepsCompleted: [step-01-validate-prerequisites, step-02-design-epics, step-03-create-stories, step-04-final-validation]
inputDocuments:
  - /Users/victornitu/Projects/CognoKratos/arktos-wallet/_bmad-output/planning-artifacts/prd.md
  - /Users/victornitu/Projects/CognoKratos/arktos-wallet/_bmad-output/planning-artifacts/architecture.md
status: 'complete'
completedAt: 'Tuesday, December 30, 2025'
---

# Epic Breakdown - Арктос Wallet

## Overview

This document provides the complete epic and story breakdown for arktos-wallet, decomposing the requirements from the PRD, UX Design if it exists, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: The Arktos Wallet system shall allow an AI agent to securely create a new non-custodial wallet.
FR2: The Arktos Wallet system shall securely generate a unique recovery passphrase for each new wallet.
FR3: The Arktos Wallet system shall securely store the wallet's recovery passphrase in an encrypted format.
FR4: The Arktos Wallet system shall enable the AI agent to manage multiple distinct blockchain accounts for a system owner.
FR5: The Arktos Wallet system shall allow an AI agent to derive and retrieve a Bitcoin public address for a specified wallet ID.
FR6: The Arktos Wallet system shall allow an AI agent to derive and retrieve an Ethereum public address for a specified wallet ID.
FR7: The Arktos Wallet system shall leverage established cryptographic standards (e.g., BIP39/BIP32) for address derivation.
FR8: The Arktos Wallet system shall expose only public data through its external APIs.
FR9: The Arktos Wallet system shall store all sensitive data (e.g., private keys, passphrases) in an encrypted format locally.
FR10: The Arktos Wallet system shall ensure sensitive data is only accessible by the system owner via direct database access.
FR11: The Arktos Wallet system shall employ database encryption (e.g., SQLCipher) for data at rest.
FR12: The Arktos Wallet system shall expose core wallet functionalities as MCP tools via a single HTTP endpoint.
FR13: The Arktos Wallet system shall provide an HTTP endpoint for system health checks (e.g., `/healthz`).
FR14: The Arktos Wallet system shall authenticate MCP client requests using API keys.
FR15: The Arktos Wallet system shall support data exchange in text-based formats (e.g., JSON) for MCP communication.
FR16: The Arktos Wallet system shall provide comprehensive API documentation detailing MCP tools, arguments, return types, and potential error codes.
FR17: The Arktos Wallet system shall be deployable via containerization (e.g., Docker).
FR18: The Arktos Wallet system shall be implemented in Rust.
FR19: The Arktos Wallet system shall serve as an open-source, educational blueprint for AI-controlled non-custodial wallets.
FR20: The Arktos Wallet system shall be designed for customization by system owners to adapt to specific needs.
FR21: The Arktos Wallet system shall provide mechanisms for builders to adapt to regional compliance requirements.
FR22: The Arktos Wallet system shall abstract blockchain complexities from the end-user (via the AI agent).
FR23: The Arktos Wallet system shall enable the AI agent to provide transparent reporting of managed funds to the end-user.

### NonFunctional Requirements

NFR1: The `create_wallet` MCP API call shall respond within 500 milliseconds 95% of the time under normal load.
NFR2: The `get_bitcoin_address` and `get_ethereum_address` MCP API calls shall respond within 100 milliseconds 95% of the time under normal load.
NFR3: The system shall support up to 100 concurrent MCP API requests per second with no more than 5% degradation in response times.
NFR4: Wallet generation and address derivation processes shall consume minimal CPU and memory resources to enable efficient deployment within resource-constrained environments (e.g., small edge devices or microservices).
NFR5: All sensitive data, including recovery passphrases and private keys, shall be encrypted at rest using industry-standard AES-256 encryption.
NFR6: All communication to and from the MCP HTTP endpoint shall be secured using TLS 1.2 or higher.
NFR7: API keys used for authentication shall be unique per client and stored securely (e.g., hashed) by Arktos Wallet.
NFR8: The system shall prevent unauthorized access to sensitive data and functionalities, ensuring that only authenticated AI agents with valid API keys can perform authorized actions.
NFR9: The system shall be hardened against common web vulnerabilities (e.g., SQL injection, cross-site scripting) even though its primary interface is API-based.
NFR10: The system shall provide an audit log of all critical wallet management actions (e.g., wallet creation, address retrieval requests) accessible to the system owner.
NFR11: Arktos Wallet shall be deployable as a stateless microservice instance (at the processing layer) to facilitate horizontal scaling by the system owner.
NFR12: The system shall maintain consistent performance (as defined in NFR1-NFR3) when deployed across multiple instances, assuming an appropriate load balancing strategy.
NFR13: The local encrypted SQLite database shall support up to 10,000 wallets and 50,000 accounts without significant performance degradation on standard hardware.
NFR14: The MCP API shall conform to the Model Context Protocol specification for tool exposure and interaction.
NFR15: The API documentation shall be sufficient for a developer to integrate an AI agent for basic wallet creation and address retrieval within 2 hours.
NFR16: The system shall achieve 99.9% uptime for its MCP API endpoint when deployed in a production environment with appropriate monitoring and infrastructure.
NFR17: In the event of an unexpected system shutdown or crash, all persisted data shall remain consistent and recoverable.
NFR18: The system shall provide clear error messages and status codes for API failures to assist client-side error handling.

### Additional Requirements

- Starter Template: Project initialization using `cargo new arktos-wallet --bin` and adding specified dependencies to `Cargo.toml` should be the first implementation story.
- Infrastructure and deployment:
    - Deployment will leverage containerization (e.g., Docker).
    - Hosting strategy is self-hosted (Docker Container).
    - CI/CD Pipeline approach: Basic CI (Build & Test Only) + Manual CI/CD Instructions.
    - Environment Configuration: Environment Variables.
    - Scaling Strategy: Horizontal Scaling (Stateless Instances).
    - `Dockerfile` provides a multi-stage build process leveraging `cargo-chef`.
- Integration requirements:
    - MCP client interacts via the HTTP MCP endpoint (`/mcp`).
    - `src/db.rs` manages interaction with the SQLite database file.
- Data migration or setup:
    - Migration Approach: `refinery` migration crate.
    - SQL migration scripts are located at `db/migrations/` at project root.
- Monitoring and logging:
    - Structured Logging (using `tracing` crate).
    - `src/telemetry.rs` for `tracing` setup.
    - Provide an audit log of all critical wallet management actions.
- API versioning or compatibility: A strategy for API versioning should be considered for future compatibility.
- Security implementation:
    - Authentication Model: API keys.
    - Data Encryption: No additional encryption beyond SQLCipher and TLS.
    - API Security Strategy: Robust Input Validation and Sanitization, Secure Error Handling, and Comprehensive Logging and Monitoring.
    - Security Middleware: TLS Configuration, `TraceLayer`, Security Headers Middleware.
    - Authorization Pattern: Ownership-Based Authorization.
    - Sensitive data (passphrases, private keys) shall be encrypted at rest using AES-256.
    - Communication to/from MCP HTTP endpoint shall be secured using TLS 1.2 or higher.
    - API keys shall be unique per client and stored securely (e.g., hashed).
- **CRITICAL**: The very first implementation story (Epic 1 Story 1) is to initialize the project using `cargo new arktos-wallet --bin` and add specified dependencies to `Cargo.toml`.

### FR Coverage Map

### FR Coverage Map

FR1: Epic 1 - The Arktos Wallet system shall allow an AI agent to securely create a new non-custodial wallet.
FR2: Epic 1 - The Arktos Wallet system shall securely generate a unique recovery passphrase for each new wallet.
FR3: Epic 1 - The Arktos Wallet system shall securely store the wallet's recovery passphrase in an encrypted format.
FR4: Epic 1 - The Arktos Wallet system shall enable the AI agent to manage multiple distinct blockchain accounts for a system owner.
FR5: Epic 2 - The Arktos Wallet system shall allow an AI agent to derive and retrieve a Bitcoin public address for a specified wallet ID.
FR6: Epic 2 - The Arktos Wallet system shall allow an AI agent to derive and retrieve an Ethereum public address for a specified wallet ID.
FR7: Epic 1 - The Arktos Wallet system shall leverage established cryptographic standards (e.g., BIP39/BIP32) for address derivation.
FR8: Epic 1 - The Arktos Wallet system shall expose only public data through its external APIs.
FR9: Epic 1 - The Arktos Wallet system shall store all sensitive data (e.g., private keys, passphrases) in an encrypted format locally.
FR10: Epic 1 - The Arktos Wallet system shall ensure sensitive data is only accessible by the system owner via direct database access.
FR11: Epic 1 - The Arktos Wallet system shall employ database encryption (e.g., SQLCipher) for data at rest.
FR12: Epic 1 - The Arktos Wallet system shall expose core wallet functionalities as MCP tools via a single HTTP endpoint.
FR13: Epic 2 - The Arktos Wallet system shall provide an HTTP endpoint for system health checks (e.g., `/healthz`).
FR14: Epic 1 - The Arktos Wallet system shall authenticate MCP client requests using API keys.
FR15: Epic 1 - The Arktos Wallet system shall support data exchange in text-based formats (e.g., JSON) for MCP communication.
FR16: Epic 2 - The Arktos Wallet system shall provide comprehensive API documentation detailing MCP tools, arguments, return types, and potential error codes.
FR17: Epic 1 - The Arktos Wallet system shall be deployable via containerization (e.g., Docker).
FR18: Epic 1 - The Arktos Wallet system shall be implemented in Rust.
FR19: Epic 1 - The Arktos Wallet system shall serve as an open-source, educational blueprint for AI-controlled non-custodial wallets.
FR20: Epic 1 - The Arktos Wallet system shall be designed for customization by system owners to adapt to specific needs.
FR21: Epic 1 - The Arktos Wallet system shall provide mechanisms for builders to adapt to regional compliance requirements.
FR22: Epic 2 - The Arktos Wallet system shall abstract blockchain complexities from the end-user (via the AI agent).
FR23: Epic 2 - The Arktos Wallet system shall enable the AI agent to provide transparent reporting of managed funds to the end-user.

## Epic List

### Epic 1: Foundational Setup & Wallet Creation
An AI agent can securely create a new non-custodial wallet with an encrypted recovery passphrase, and a developer can initialize the project.
**FRs covered:** FR1, FR2, FR3, FR4, FR7, FR8, FR9, FR10, FR11, FR12, FR14, FR15, FR17, FR18, FR19, FR20, FR21

### Story 1.1: Implement `create_wallet` MCP Tool

As an AI agent,
I want to securely create a new non-custodial wallet with an encrypted recovery passphrase,
So that I can manage funds for a system owner.

**Acceptance Criteria:**

**Given** the Arktos Wallet server is running and accessible via `/mcp`
**When** an MCP client sends a `create_wallet` request with a unique wallet name
**Then** a new wallet is securely created in the system
**And** a unique recovery passphrase is generated and securely stored (encrypted)
**And** the `create_wallet` MCP API call responds within 500 milliseconds 95% of the time
**And** sensitive data (passphrase) is encrypted at rest using industry-standard AES-256 encryption.

### Story 1.2: Implement Secure Multi-Account Management

As a system owner,
I want my AI agent to manage multiple distinct blockchain accounts within a single wallet,
So that I can organize and isolate my funds effectively.

**Acceptance Criteria:**

**Given** a wallet has been created via `create_wallet`
**When** the AI agent needs to create or manage a new account within that wallet
**Then** the system shall securely store and associate this account with the parent wallet
**And** the system shall leverage established cryptographic standards (e.g., BIP39/BIP32) for account derivation and management
**And** sensitive data for these accounts (e.g., private keys, passphrases) shall be encrypted at rest and accessible only by the system owner.

### Story 1.3: Implement API Key Authentication for MCP Client Requests

As a system owner,
I want the Arktos Wallet system to authenticate MCP client requests using API keys,
So that only authorized AI agents can interact with my wallets and accounts.

**Acceptance Criteria:**

**Given** the Arktos Wallet server is running
**When** an MCP client sends a request to the server
**Then** the server shall validate the provided API key
**And** only requests with a valid API key shall be processed
**And** API keys used for authentication shall be unique per client and stored securely (e.g., hashed) by Arktos Wallet
**And** the system shall prevent unauthorized access to sensitive data and functionalities.

### Story 1.4: Implement Initial Dockerization and Rust Project Structure

As a developer,
I want the Arktos Wallet system to be deployable via containerization and built with an idiomatic Rust project structure,
So that I can easily set up a development environment and deploy the system.

**Acceptance Criteria:**
*   **Given** the core wallet and API key authentication functionalities are implemented
*   **When** a developer wants to set up or deploy the system
*   **Then** a `Dockerfile` shall exist to build a container image of the application
*   **And** the application shall be implemented entirely in Rust with a clean, idiomatic project structure
*   **And** the system shall be deployable via containerization (e.g., Docker).

### Story 1.5: Document Project as an Open-Source, Customizable Blueprint

As a developer,
I want the Arktos Wallet system to be thoroughly documented as an open-source and customizable blueprint,
So that I can easily understand, extend, and adapt it for regional compliance.

**Acceptance Criteria:**
*   **Given** the core functionalities of wallet management and authentication are implemented
*   **When** a developer explores the project
*   **Then** the project documentation shall clearly position Arktos Wallet as an open-source, educational blueprint for AI-controlled non-custodial wallets (FR19)
*   **And** the project structure and provided examples shall demonstrate its design for customization by system owners to adapt to specific needs (FR20)
*   **And** the project documentation shall outline mechanisms and considerations for builders to adapt to regional compliance requirements (FR21).

### Epic 2: Address Generation and Retrieval
An AI agent can derive and retrieve Bitcoin and Ethereum public addresses for a specific wallet, and developers can implement health checks and documentation.
**FRs covered:** FR5, FR6, FR13, FR16, FR22, FR23

### Story 2.1: Implement `get_bitcoin_address` MCP Tool

As an AI agent,
I want to derive and retrieve a Bitcoin public address for a specified wallet ID,
So that I can receive Bitcoin funds into a managed wallet.

**Acceptance Criteria:**

**Given** a wallet with associated accounts exists in the system
**When** an MCP client sends a `get_bitcoin_address` request with a valid wallet ID
**Then** the system shall derive and return a valid Bitcoin public address for that wallet
**And** the `get_bitcoin_address` MCP API call shall respond within 100 milliseconds 95% of the time.

### Story 2.2: Implement `get_ethereum_address` MCP Tool

As an AI agent,
I want to derive and retrieve an Ethereum public address for a specified wallet ID,
So that I can receive Ethereum funds into a managed wallet.

**Acceptance Criteria:**

**Given** a wallet with associated accounts exists in the system
**When** an MCP client sends a `get_ethereum_address` request with a valid wallet ID
**Then** the system shall derive and return a valid Ethereum public address for that wallet
**And** the `get_ethereum_address` MCP API call shall respond within 100 milliseconds 95% of the time.

### Story 2.3: Implement Operational Endpoints and Documentation for Developer and User Experience

As a developer or system owner,
I want the Arktos Wallet system to provide an operational health check endpoint, comprehensive API documentation, and abstract blockchain complexities for the end-user,
So that I can monitor its status, easily integrate, and the AI agent can provide transparent financial reporting.

**Acceptance Criteria:**
*   **Given** the Arktos Wallet server is running
*   **When** a developer accesses the system's operational endpoints or documentation
*   **Then** the system shall provide an HTTP endpoint for system health checks (e.g., `/healthz`) (FR13)
*   **And** comprehensive API documentation shall be available detailing MCP tools, arguments, return types, and potential error codes (FR16)
*   **And** the system shall effectively abstract blockchain complexities from the end-user (via the AI agent) (FR22)
*   **And** the AI agent shall be enabled to provide transparent reporting of managed funds to the end-user (FR23).


