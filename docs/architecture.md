# Arktos Wallet - Architecture

This document outlines the architecture of the Arktos Wallet application.

## 1. Executive Summary

Arktos Wallet is a backend HTTP server built with Rust. It is designed as a single, cohesive service (a monolith). The primary function is to act as an "HTTP MCP server", based on the project's dependencies and configuration. The architecture is API-centric, designed to handle HTTP requests and execute business logic on the server side.

## 2. Technology Stack

| Category          | Technology            | Version      | Justification                                |
|-------------------|-----------------------|--------------|----------------------------------------------|
| Language          | Rust                  | 2024 edition | `Cargo.toml` `[package]` section             |
| Framework         | Axum                  | 0.8          | Web server framework from `[dependencies]`     |
| SDK               | rmcp (MCP Rust SDK)   | 0.12         | Core dependency for MCP server functionality |
| Async Runtime     | Tokio                 | 1            | `[dependencies]`                             |
| Serialization     | Serde, Serde_json     | 1            | `[dependencies]`                             |
| Schema Generation | Schemars              | 1            | `[dependencies]` for JSON schema generation    |
| Logging           | Tracing               | 0.1, 0.3     | `[dependencies]`                             |

## 3. Architecture Pattern

The application follows a simple **API-centric (or Service-Oriented) Architecture**.

*   **Web Layer**: The `axum` framework is responsible for handling all incoming HTTP requests. It manages routing, request parsing, and response serialization.
*   **Business Logic**: All business logic appears to be contained within `src/main.rs`. Given the "quick scan" analysis, it's inferred that this file contains the core application logic, MCP functionalities, and request handlers.
*   **Asynchronous Processing**: The `tokio` runtime is used to handle operations asynchronously, allowing for efficient, non-blocking I/O. This is critical for a high-performance network server.

Due to the project's simple structure (a single `main.rs` file), a more granular layered architecture (e.g., distinct service, repository, and controller layers) is not apparent from the file structure alone.

## 4. Data Architecture

The application uses an encrypted **SQLite** database for persistence, managed via the `rusqlite` crate with the **SQLCipher** feature enabled. This ensures that all data at rest, including sensitive wallet passphrases and private keys, is secure.

The primary data models are `Wallet` and `Account`. For detailed schema information, refer to the [Data Models](./data-models.md) document.

## 5. API Design

The server exposes a minimal HTTP interface and provides its core functionality through Model Context Protocol (MCP) tools. This approach separates basic service management (health checks) from the primary business logic, which is accessible only to authorized MCP agents.

*   **HTTP Interface**: The only exposed HTTP endpoints are `/healthz` for liveness checks and `/mcp` for handling MCP transport.
*   **MCP Tools**: Wallet creation and address retrieval are exposed as MCP tools (e.g., `create_wallet`, `get_bitcoin_address`). This design ensures that wallet functionalities are accessed in a structured, secure, and context-aware manner via the MCP layer.

For a complete breakdown of all HTTP endpoints and MCP tools, see the [API Contracts](./api-contracts.md) document.

## 6. Source Tree

For a detailed analysis of the project's file and directory structure, refer to the [Source Tree Analysis](./source-tree-analysis.md).

## 7. Development & Deployment

*   **Development**: Instructions for setting up the local environment, building, and running the application can be found in the [Development Guide](./development-guide.md).
*   **Deployment**: No `Dockerfile`, CI/CD pipeline configurations, or other deployment artifacts were found in the project. The deployment strategy is currently undefined.
