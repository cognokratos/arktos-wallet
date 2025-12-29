# Project Overview: Arktos Wallet

## Executive Summary

Arktos Wallet is a backend HTTP server developed in Rust. It functions as a monolithic service designed to act as an "HTTP MCP server." The technology stack is modern and asynchronous, built on the Tokio runtime and the Axum web framework.

The project is currently in a simple state, with the core logic contained in a single `src/main.rs` file. This suggests it is either in an early stage of development or is intended to be a lightweight microservice.

## Technology Stack Summary

| Category          | Technology            |
|-------------------|-----------------------|
| Language          | Rust (2024 Edition)   |
| Web Framework     | Axum                  |
| Async Runtime     | Tokio                 |
| Core SDK          | rmcp (MCP Rust SDK)   |

## Repository Structure

The project is a **monolith**, with a single, cohesive codebase. There is one primary part: the backend service itself.

For a detailed breakdown of the file and directory structure, please see the [Source Tree Analysis](./source-tree-analysis.md).

## Architecture

The application follows an **API-centric architecture**. The Axum framework handles incoming HTTP requests, which are then processed by the core application logic.

For a complete architectural breakdown, refer to the [Architecture Document](./architecture.md).

## Key Documentation

*   [Architecture](./architecture.md)
*   [Source Tree Analysis](./source-tree-analysis.md)
*   [Development Guide](./development-guide.md)
