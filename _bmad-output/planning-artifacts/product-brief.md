---
stepsCompleted: [1, 2, 3, 4, 5]
inputDocuments:
  - "_bmad-output/planning-artifacts/user_provided_context.md"
  - "docs/architecture.md"
  - "docs/api-contracts.md"
  - "docs/data-models.md"
  - "docs/project-overview.md"
  - "docs/development-guide.md"
  - "docs/deployment-guide.md"
date: 2025-12-29
author: Victor
---

# Product Brief: Арктос Wallet

## Executive Summary

The adoption of blockchain technology is hindered by its technical complexity, particularly the management of private and public keys. This forces most users into custodial solutions where they sacrifice self-sovereignty, or they must rely on third-party wallets with their own inherent risks. As AI agents become more prevalent, they will require secure, programmatic access to user keys to perform transactions, a challenge existing solutions are not built to address. Arktos Wallet provides an open blueprint for a non-custodial, agent-controlled wallet, aiming to solve the UX problem for humans while enabling the future of AI-driven blockchain interactions.

---

## Core Vision

### Problem Statement

To truly "own your crypto," users must manage their own cryptographic keys, a process that is technically complex, intimidating, and fraught with risk for newcomers. This fundamental usability challenge is the single greatest barrier to mainstream blockchain adoption.

### Problem Impact

This complexity forces a difficult choice upon users:

1.  **Use Custodial Services:** Entrusting assets to centralized third parties (like exchanges), which completely defeats the purpose of decentralization and self-sovereignty.
2.  **Use Third-Party Wallets:** Relying on software like Metamask, which still involves third-party risk and presents a steep learning curve for non-technical individuals.

Furthermore, the emerging paradigm of AI agents acting on a user's behalf lacks a secure standard for key management. How does an agent access a user's funds without compromising their private keys?

### Why Existing Solutions Fall Short

-   **Custodial solutions** trade the core value of blockchain (self-ownership) for convenience, which is antithetical to the blockchain ethos.
-   **Existing non-custodial wallets** are built for manual human interaction, not for secure, programmatic control by an autonomous AI agent.
-   There is a lack of clear, educational "blueprints" for developers to build secure, non-custodial wallets specifically designed for this emerging AI-agent economy.

### Proposed Solution

Arktos Wallet is a blueprint for a **Model Context Protocol (MCP) server written in Rust** that functions as a non-custodial cold wallet. It is designed to empower developers and entrepreneurs to build applications where AI agents can securely manage user funds on Bitcoin and Ethereum. By interacting with the wallet via MCP, the AI agent handles the complexity of key generation and address derivation, abstracting it away from the end-user and providing a seamless, secure experience.

### Key Differentiators

-   **AI-First Design:** Built from the ground up to be securely controlled by AI agents via the MCP standard.
-   **Non-Custodial & Secure:** Empowers users with true ownership of their assets, with private keys encrypted and stored locally.
-   **Educational Blueprint:** Serves as a clear, robust, and open-source example for building secure, Rust-based blockchain services.
-   **Multi-Chain Foundation:** Designed to support both Bitcoin and Ethereum, establishing a pattern for future expansion.

---

## Target Users

### Primary Users

Our primary user is the **innovative developer or tech-savvy entrepreneur**. We'll call her **"Alex"**.

*   **Persona: Alex, The AI-Blockchain Builder**
    *   **Role:** A founder or lead developer at a tech startup.
    *   **Context:** Alex is highly skilled in building applications, likely with AI/ML experience, but is not a deep expert in blockchain cryptography. She sees the potential of combining AI with decentralized finance but is wary of the security and complexity.
    *   **Goals:** She wants to build a product that offers a seamless, AI-driven experience for her own customers (e.g., an automated savings tool, a smart contract manager). Her goal is to get to market quickly and focus on her application's unique value, not on reinventing the low-level plumbing of wallet security.
    *   **Problem Experience:** Alex understands *why* self-custody is important but is daunted by the "how." The thought of building a secure, multi-chain wallet from scratch is a major roadblock. She knows a single mistake in key management could be catastrophic for her users and her company's reputation. Existing libraries are too low-level, and off-the-shelf wallets aren't designed for the programmatic, server-side control an AI agent requires.

### Secondary Users

The secondary users are the **end-users of the products Alex and other builders create**. We'll call this persona **"Ben"**.

*   **Persona: Ben, The Curious but Cautious Adopter**
    *   **Role:** An everyday person with disposable income who is interested in crypto but intimidated by the technology.
    *   **Context:** Ben hears about the potential of blockchain but is put off by horror stories of lost funds and confusing interfaces.
    *   **Goals:** He wants to participate in the upside of crypto without having to become a security expert. He is willing to trust an application that feels as simple and secure as a modern fintech app he already uses.
    *   **Problem Experience:** Ben has looked at wallets like Metamask and found them overwhelming. He doesn't understand seed phrases or gas fees. He likely uses a centralized exchange for convenience but is vaguely aware of the "not your keys, not your coins" risk.

### User Journey (for Alex, our Primary User)

1.  **Discovery:** Alex is researching how to build her AI-powered finance app. She discovers the Arktos Wallet project on GitHub or a technical blog, described as a "Rust blueprint for AI-first crypto wallets."
2.  **Onboarding:** She clones the repository. She's impressed by the clear documentation, the multi-stage Dockerfile using `cargo-chef`, and the use of the RMCP standard, which gives her a clear API for her AI agent to target.
3.  **Core Usage:** Alex integrates the Arktos Wallet server into her application's backend. She uses it as a microservice to create new, encrypted wallets for her users. Her AI agent can now call simple MCP endpoints like `create_wallet` or `get_address(wallet_id, 'bitcoin')` without ever directly touching a private key.
4.  **"Aha!" Moment:** The "aha!" moment strikes when she successfully has her AI agent generate a new wallet and retrieve an address in a test environment. She realizes she has a secure, non-custodial backend for her users without having to spend months building and auditing it herself.
5.  **Long-term:** The Arktos Wallet becomes a core, trusted component of her production infrastructure. She can now focus entirely on building more sophisticated AI-driven features for her customers, knowing the wallet foundation is solid and secure.

---

## Success Metrics

Success for Arktos Wallet is defined by how effectively we empower our primary user, Alex (the builder), and foster a thriving open-source community that validates the project's value.

### User Success: Empowering the Builder

The ultimate measure of success is the speed and ease with which a developer can integrate a secure, non-custodial wallet into their own application.

*   **Primary User Outcome:** A developer like Alex must be able to **"plug a cold wallet solution into her agentic system in minutes."** This measures the time-to-value from discovery to functional integration.
*   **Key User Behaviors Indicating Success:**
    *   **Rapid Integration:** The developer can quickly set up the server and make successful API calls from their application in a test environment.
    *   **Customization and Adoption:** The developer forks the repository to adapt it for their specific, custom use case.

### Project Objectives: Fostering a Go-To Blueprint

As an open-source educational project, our objectives are centered on adoption, community, and impact.

*   **Adoption:** Become a widely recognized and used foundation for building AI-first, non-custodial wallets.
*   **Community:** Cultivate a vibrant and active community of builders who contribute to, ask questions about, and improve the project.
*   **Impact:** Establish Arktos Wallet as the *de facto* "blueprint" for this architectural pattern in the open-source community.

### Key Performance Indicators (KPIs)

We will track progress towards our goals by monitoring the following KPIs, primarily centered on our GitHub repository:

*   **Developer Speed & Ease-of-Use:**
    *   **Time to "Hello, Wallet!":** The average time measured from `git clone` to a developer making their first successful API call (e.g., `create_wallet`). Our goal is to make this as short as possible.
*   **Community Adoption & Customization:**
    *   **GitHub Forks:** A direct proxy for how many developers are using Arktos as a starting point for their own projects.
*   **Community Engagement:**
    *   **GitHub Stars:** The primary leading indicator of developer interest and approval.
    *   **Community Contributions:** The number of meaningful pull requests and issues submitted by the community.
    *   **Active Contributor Growth:** The number of unique developers contributing code or documentation to the project over time.

---

## MVP Scope

This MVP is focused on creating a robust, secure, and educational blueprint for an AI-controlled cold wallet. The core goal is to solve the key generation, storage, and address derivation problem for our primary user, "Alex," the builder.

### Core Features

The MVP will consist of the following components and features:

*   **Technology Stack:**
    *   **Language:** Rust, written with a functional programming preference.
    *   **Database:** Encrypted SQLite using `rusqlite` with the `bundled-sqlcipher-vendored-openssl` feature for secure, self-contained storage.
    *   **Blockchain Logic:**
        *   `bip39` and `bip32` for hierarchical deterministic key generation from a passphrase.
        *   `rust-bitcoin` for deriving Bitcoin Bech32 addresses.
        *   `tiny-keccak` for deriving Ethereum addresses.
    *   **API:** An HTTP MCP Server built with the `rmcp` crate (`server`, `macros`, `transport-streamable-http-server` features).
    *   **Containerization:** A multi-stage Docker build using `lukemathwalker/cargo-chef` for optimized dependency caching and a lean `gcr.io/distroless/static-debian12:nonroot` runtime image.

*   **Agent-Facing Features (via MCP API):**
    1.  **Create Wallet:** An agent can request the creation of a new `Wallet`. The server will generate a random recovery `Passphrase` and return it.
    2.  **Read Bitcoin Address:** An agent can request the Bitcoin public address for an existing `Wallet` by providing its ID.
    3.  **Read Ethereum Address:** An agent can request the Ethereum public address for an existing `Wallet` by providing its ID.

*   **Server-Side Logic:**
    *   The server handles the generation of the private key from the passphrase.
    *   The server securely encrypts and saves the `Wallet` (including its name and passphrase) and the derived `Private Key` into the SQLite database.

*   **Data Schema:**
    *   `Wallet`: { Name: String, Passphrase: Text }
    *   `Account`: { ID: Integer, WalletID: Integer, Private_Key: Text }

### Out of Scope for MVP

To ensure we can deliver a high-quality, secure core, the following are **explicitly not** part of the MVP:

*   **Transaction Signing & Broadcasting:** The MVP is a *cold wallet* blueprint. It generates keys and addresses but does not sign or send transactions.
*   **Additional Blockchains:** Support is strictly limited to Bitcoin and Ethereum.
*   **Other Interfaces:** The only supported interface is the HTTP MCP Server. No gRPC, direct CLI, or other APIs will be built.
*   **User-Facing UI:** This is a backend service for developers; no graphical user interface will be created.

### MVP Success Criteria

The MVP will be considered successful when our user, "Alex," can:
1.  Successfully build and run the application using the provided Dockerfile.
2.  Use an MCP client to hit the API and successfully create a new wallet, receiving a valid passphrase.
3.  Use that wallet's ID to successfully retrieve the corresponding Bitcoin and Ethereum addresses.
4.  Verify that the private key is stored in an encrypted state within the generated SQLite database file.

### Future Vision

If the MVP is successful, the Arktos Wallet could evolve by:
*   **Adding Transaction Capabilities:** Introducing endpoints for an AI agent to securely sign and broadcast transactions, turning it into a "hot wallet" controller.
*   **Expanding Blockchain Support:** Incrementally adding support for other popular chains like Solana, Polygon, etc.
*   **Advanced Security Modules:** Integrating with Hardware Security Modules (HSMs) or other enterprise-grade key protection mechanisms.
*   **Building Reference Agents:** Creating and open-sourcing example AI agents that use the wallet to perform tasks, further lowering the barrier to entry for other builders.
