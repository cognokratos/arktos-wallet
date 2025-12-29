---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
inputDocuments:
  - "_bmad-output/planning-artifacts/product-brief.md"
  - "docs/architecture.md"
  - "docs/api-contracts.md"
  - "docs/index.md"
  - "docs/deployment-guide.md"
  - "docs/data-models.md"
  - "docs/project-overview.md"
  - "docs/development-guide.md"
  - "docs/source-tree-analysis.md"
workflowType: 'prd'
lastStep: 11
briefCount: 1
researchCount: 0
brainstormingCount: 0
projectDocsCount: 8
---

# Product Requirements Document - Арктос Wallet

**Author:** Victor
**Date:** 2025-12-29

## Executive Summary

Arktos Wallet is a blueprint for a non-custodial, cold wallet server (Rust/MCP) enabling secure AI agent-controlled Bitcoin and Ethereum fund management. It addresses the complexity of key management and the need for programmatic AI access, empowering developers ('Alex') to build AI-powered financial applications and providing secure, intuitive experiences for end-users ('Ben').

### What Makes This Special

Its AI-First Design, Non-Custodial & Secure nature, role as an Educational Blueprint, and Multi-Chain Foundation are its core differentiators.

## Project Classification

**Technical Type:** API Backend
**Domain:** Fintech
**Complexity:** High
**Project Context:** Brownfield - extending existing system

Classified as an API Backend due to its server nature and MCP interface. The Fintech domain reflects its focus on cryptocurrency and fund management. The high complexity arises from the critical security and compliance requirements inherent in financial and blockchain applications.

## Success Criteria

### User Success

A developer like Alex must be able to "plug a cold wallet solution into her agentic system in minutes," enabling rapid integration and customization. Key indicators include:
-   Rapid Integration: Developers quickly set up the server and make successful API calls in a test environment.
-   Customization and Adoption: Developers fork the repository to adapt it for their specific, custom use case.

### Business Success

-   **Adoption:** Arktos Wallet becomes a widely recognized and used foundation for building AI-first, non-custodial wallets.
-   **Community:** Cultivate a vibrant and active community of builders who contribute to, ask questions about, and improve the project.
-   **Impact:** Establish Arktos Wallet as the *de facto* "blueprint" for this architectural pattern in the open-source community.

### Technical Success

-   The Rust-based MCP server maintains high robustness, security, and performance.
-   The project facilitates rapid and seamless integration for developers using the blueprint.
-   The blueprint remains clear, open-source, well-documented, and easily extensible.

### Measurable Outcomes

-   **Developer Speed & Ease-of-Use:** Short "Time to 'Hello, Wallet!'" (from `git clone` to first successful `create_wallet` API call).
-   **Community Adoption & Customization:** High number of GitHub Forks.
-   **Community Engagement:** High number of GitHub Stars, active Community Contributions (Pull Requests and Issues), and consistent Active Contributor Growth.

## Product Scope

### MVP - Minimum Viable Product

The MVP focuses on creating a robust, secure, and educational blueprint for an AI-controlled cold wallet, solving key generation, storage, and address derivation.

**Core Features:**
-   **Technology Stack:** Rust, encrypted SQLite (SQLCipher), `bip39`/`bip32`, `rust-bitcoin` and `tiny-keccak` for address generation, HTTP MCP Server (`rmcp`), multi-stage Docker build for deployment.
-   **Agent-Facing MCP API:** `create_wallet`, `get_bitcoin_address`, `get_ethereum_address`.
-   **Server-Side Logic:** Secure generation of private keys from passphrases, encrypted storage of wallet data (name, passphrase) and private keys in SQLite.
-   **Data Schema:** `Wallet` (`Name`, `Passphrase`), `Account` (`ID`, `WalletID`, `Private Key`).
-   **Core MVP Success Criteria:** Successful build/run via Docker, MCP client can create wallets and retrieve addresses, private key storage verification.

### Growth Features (Post-MVP)

Future enhancements beyond the MVP include:
-   **Transaction Signing & Broadcasting:** Capabilities to enable AI agents to securely sign and broadcast transactions, evolving Arktos into a "hot wallet" controller.
-   **Expanded Blockchain Support:** Integration with additional popular blockchain networks (e.g., Solana, Polygon).
-   **Alternative Interfaces:** Exploration of other communication interfaces beyond HTTP MCP (e.g., gRPC, direct CLI).

### Vision (Future)

The long-term vision for Arktos Wallet includes:
-   Becoming a full-fledged "hot wallet" controller with comprehensive transaction capabilities.
-   Integrating with Advanced Security Modules (e.g., Hardware Security Modules - HSMs) for enterprise-grade key protection.
-   Developing and open-sourcing example AI agents that leverage the wallet for various tasks, further simplifying adoption for builders.

## User Journeys

### Journey 1: Alex, The AI-Blockchain Builder - Empowering Ben's Autonomous Savings

Alex, a forward-thinking founder and lead developer, is building an innovative agentic system for her client, Ben. Ben, a "Curious but Cautious Adopter" of crypto, wants a seamless way for his AI agent to manage savings, receive funds from customers, and interact with the blockchain without the daunting complexity of key management. Alex understands the need for secure, non-custodial solutions but has been frustrated by existing options that don't allow for programmatic, AI-driven control. She worries about the security implications of managing Ben's funds and the development overhead of building such a system from scratch.

One day, while researching solutions for AI-driven blockchain interactions, Alex discovers Arktos Wallet – an open blueprint for a non-custodial, agent-controlled cold wallet. Intrigued by its "AI-First Design" and Rust-based security, she decides to fork the repository. The clear documentation and the MCP (Model Context Protocol) interface immediately resonate with her, offering a structured way for her AI agents to interact with the wallet.

She quickly integrates Arktos Wallet into Ben's agentic system. Using the MCP, her AI agent can now securely create new savings wallets for Ben, derive Bitcoin and Ethereum addresses for receiving funds from external customers, and manage these accounts without ever directly exposing private keys. This integration is surprisingly fast and smooth, validating Arktos Wallet's "plug a cold wallet solution into her agentic system in minutes" promise.

The breakthrough comes when Ben's AI successfully receives its first customer payment into a newly created Arktos-powered savings account. Alex realizes she has not only delivered a critical, secure feature for Ben but has also significantly accelerated her development timeline. She can now confidently customize Arktos Wallet further to meet Ben's evolving needs, focusing on building more sophisticated AI features rather than wrestling with low-level blockchain infrastructure. Arktos Wallet has empowered her to deliver true financial autonomy to Ben's AI agent, securely and efficiently.

### Journey 2: Ben, The Curious but Cautious Adopter - Effortless Financial Autonomy

Ben has always been intrigued by the potential of cryptocurrency but intimidated by its complexity. He’s a "Curious but Cautious Adopter," wary of the technical hurdles and security risks. While he understands the concept of self-custody, the thought of managing private keys, seed phrases, and navigating various blockchain interfaces felt overwhelming. He relies on Alex's agentic system for many aspects of his digital life, and he trusts her to simplify complex tasks.

When Alex introduces him to the new capabilities of his agentic system, powered by Arktos Wallet, Ben is initially skeptical but quickly becomes impressed. Alex explains that his AI agent can now manage multiple blockchain "savings accounts" – each with its own public address – securely and autonomously, without him needing to touch any complex crypto infrastructure. Instead of sharing an IBAN for payments, he can now simply provide a public blockchain address generated by his AI, knowing the funds will be securely received into a non-custodial wallet.

The "aha!" moment for Ben comes when he sees his AI effortlessly managing incoming funds across various blockchain accounts, all transparently reported within Alex's familiar application interface. He realizes he has gained true financial autonomy and the flexibility of unlimited accounts, previously only possible with banks, but now without relying on a centralized provider. The fear of losing keys or navigating complex transactions is gone. Arktos Wallet, working seamlessly behind the scenes, has transformed his cautious curiosity into confident participation in the decentralized economy, all thanks to the intelligent automation provided by Alex's system.

### Journey Requirements Summary

These journeys highlight the need for the following capabilities within Arktos Wallet and its integration into agentic systems:
-   **Secure, Programmatic Wallet Creation:** The ability for AI agents to securely create new non-custodial wallets.
-   **Secure Address Derivation:** The ability for AI agents to derive and retrieve Bitcoin and Ethereum public addresses for specific wallets.
-   **Non-Custodial Key Management:** Ensuring that private keys are generated, stored, and managed securely without direct user exposure, empowering true self-custody via the agent.
-   **Multi-Account Management:** Supporting the creation and management of multiple distinct blockchain accounts per user/agent.
-   **Seamless Agent Integration:** Providing a clear and robust interface (MCP) for AI agents to interact with wallet functionalities.
-   **Blockchain Complexity Abstraction:** Hiding the underlying technical complexities of blockchain transactions and key management from the end-user.
-   **Transparent Reporting:** Enabling the agentic system to provide clear and user-friendly reports of AI-managed funds and activities to the end-user.

## Domain-Specific Requirements

### Fintech Compliance & Regulatory Overview

Given Arktos Wallet's focus on non-custodial cryptocurrency management and its high complexity within the Fintech domain, specific attention to compliance, security, and data protection is paramount. The design principles emphasize local control and customizability to assist builders in meeting diverse regional and industry-specific regulations.

### Key Domain Concerns

-   **Regional Compliance:** The minimal and customizable nature of Arktos Wallet empowers builders to adapt the system to various regional compliance requirements. This architectural flexibility is crucial in a globally fragmented regulatory landscape.
-   **Security Standards:** Adherence to robust security standards is maintained by exposing only public data through the APIs. Sensitive data, such as private keys and passphrases, are stored encrypted within the local database and are only accessible by the system owner.
-   **Data Protection:** All operational data, including sensitive user information, is stored locally and encrypted. This design choice eliminates reliance on external third parties for data storage, ensuring that the system owner maintains full control and ownership of their data.
-   **Audit Requirements & Fraud Prevention:** (To be further detailed in later stages, but implied by the focus on security and data protection.)

### Compliance Requirements

-   The blueprint's adaptability allows builders (like Alex) to implement specific regional and industry-specific compliance measures required for their deployments.
-   Full control over data residency and encryption empowers system owners to meet local data protection laws (e.g., GDPR, CCPA implications handled by the system owner's deployment strategy).

### Industry Standards & Best Practices

-   Leverages established cryptographic standards (`bip39`, `bip32`) for key generation and address derivation.
-   Employs database encryption (SQLCipher) for sensitive data at rest.
-   The MCP provides a structured and secure interface for AI agents, adhering to modern API interaction paradigms.

### Required Expertise & Validation

-   Builders integrating Arktos Wallet will require expertise in deploying secure backend services and understanding the specific compliance landscape of their target regions.
-   Validation will involve rigorous security audits of the deployed system and continuous monitoring for compliance.

### Implementation Considerations

-   The customizable nature necessitates that builders carefully configure and secure their deployments to meet their specific compliance obligations.
-   Ongoing vigilance regarding evolving regulatory frameworks in the Fintech and blockchain spaces is essential for long-term product viability.

## Innovation & Novel Patterns

### Detected Innovation Areas

-   **AI-Native Blockchain UX:** Arktos Wallet significantly improves the user experience of blockchain interactions by abstracting complexity through AI agents, enabling wider adoption.
-   **Native Financial Transactions for AI Agents:** It provides a secure and programmatic framework for AI agents to perform financial transactions on behalf of users, a critical and underserved capability.
-   **Educational Open Blueprint:** Its lean, customizable, and open-source nature serves as a flexible first step for builders, fostering adaptation and evolution to real-world needs.
-   **Non-Custodial AI Control:** Uniquely combines non-custodial key management with AI-driven control, empowering users with self-sovereignty in an AI-driven economy.

### Market Context & Competitive Landscape

The market currently lacks accessible, secure, and AI-native non-custodial solutions tailored for developers. Existing options are either overly complex for end-users or not designed for programmatic AI interaction. Arktos Wallet addresses this gap by offering an open and flexible foundation.

### Validation Approach

-   **Alpha/Beta Program with Builders (Alex Persona):** Recruit early-adopter developers to integrate Arktos Wallet into their AI systems, gathering feedback on ease of integration and customization. Track "Time to Hello, Wallet!" (from `git clone` to first successful API call).
-   **Security Audits & Penetration Testing:** Engage independent security firms to conduct white-box and black-box penetration testing on the core wallet and MCP server. Review code for cryptographic best practices and common vulnerabilities.
-   **User Scenario Testing (Ben Persona):** Conduct user testing with non-technical users interacting with AI agents powered by Arktos Wallet. Observe their comfort level, understanding of transactions, and overall satisfaction.
-   **Community Engagement & Feedback Loops:** Actively monitor GitHub issues, discussions, and community contributions. Implement a clear feedback channel for feature requests and usability concerns.

### Risk Mitigation

-   **AI-Driven Financial Transaction Risk:**
    -   **Explicit User Consent:** Ensure Alex's system implements explicit, clear, and granular user consent mechanisms for all AI-driven financial actions.
    -   **Transaction Limits/Safeguards:** Provide configuration options within Arktos Wallet for Alex's system to set transaction limits, whitelist addresses, or require multi-factor authentication/human override for high-value transactions.
    *   **Audit Trails:** Implement robust logging and audit trails within Arktos Wallet so that all AI-initiated actions can be reviewed and verified by Alex and Ben.
-   **Security of Customization/Developer Implementation Risk:**
    -   **Comprehensive Documentation & Best Practices:** Provide extensive documentation, tutorials, and security best practices specifically for customizing and deploying Arktos Wallet securely. Highlight common pitfalls.
    -   **Security-Focused Examples/Templates:** Offer secure example implementations and templates for common use cases to guide developers.
    -   **Community Review/Support:** Encourage and facilitate community peer review of customized implementations. Offer guidance for secure modifications.
-   **Educational Blueprint Effectiveness Risk:**
    -   **Clear Learning Paths:** Develop structured learning paths, tutorials, and workshops to guide developers through integrating and customizing Arktos Wallet.
    -   **Active Community Management:** Foster a supportive community where developers can ask questions and receive assistance.
    -   **Iterative Feedback:** Continuously gather feedback on the clarity and utility of the blueprint and evolve it based on developer needs.

## API Backend Specific Requirements

### Project-Type Overview

Arktos Wallet is implemented as a lean, API-centric backend service, designed to function as an MCP (Model Context Protocol) server. Its primary purpose is to expose core wallet functionalities programmatically, specifically to be consumed by AI agents within a trusted system environment. The focus is on secure, robust, and efficient machine-to-machine communication rather than traditional human-facing web interactions.

### Technical Architecture Considerations

-   **API Endpoints:**
  -   Core functionalities (e.g., `create_wallet`, `get_bitcoin_address`, `get_ethereum_address`) will be exposed as MCP tools via a single HTTP MCP endpoint (e.g., `/mcp`).
  -   A standard health check endpoint (e.g., `/healthz`) will also be available.
-   **Authentication Model:** Authentication between the MCP client and the Arktos Wallet server will be handled using API keys. This mechanism is suitable given that both client and server are owned by the same system owner, ensuring a secure and controlled environment.
-   **Data Exchange Formats:** Client and server will primarily exchange data as text, likely in JSON format, aligning with standard HTTP and MCP communication patterns. The MCP itself handles serialization and deserialization of structured data.
-   **Rate Limiting:** No explicit rate limiting will be implemented as the client and server are assumed to operate within the same trusted system boundary and under the control of a single system owner.
-   **API Versioning:** A strategy for API versioning should be considered for future compatibility, potentially integrated into the MCP tool definition or handled via URL prefixes (e.g., `/v1/mcp`).
-   **API Documentation:** Comprehensive API documentation will be essential, detailing each MCP tool, its arguments, return types, and potential error codes.

### Implementation Considerations

-   The server will be implemented in Rust, leveraging its performance and security features.
-   The design prioritizes a minimal footprint and ease of customization, allowing system owners to adapt the wallet's functionality to their specific needs without unnecessary overhead.
-   Deployment will likely leverage containerization (e.g., Docker) as outlined in the Deployment Guide, ensuring a consistent and isolated runtime environment.

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**MVP Approach:** A hybrid approach combining a **Platform MVP** (building a robust, extensible foundation) and a **Problem-Solving MVP** (addressing the core problem of AI-controlled non-custodial key management).
**Resource Requirements:** A lean, expert team including Rust Developers (with cryptography/blockchain experience), Security Engineers, Technical Writers, and DevOps specialists.

### MVP Feature Set (Phase 1)

**Core User Journeys Supported:**
-   **Alex, The AI-Blockchain Builder:** Enabling her to securely integrate an AI-controlled non-custodial wallet into her agentic system, create wallets, and retrieve public addresses for Bitcoin and Ethereum.
-   **Ben, The Curious but Cautious Adopter:** Experiencing effortless, secure management of blockchain savings accounts through Alex's AI-powered system, abstracting away complexity.

**Must-Have Capabilities:**
-   **Technology Stack:** Rust, encrypted SQLite (SQLCipher), `bip39`/`bip32` for key derivation, `rust-bitcoin` and `tiny-keccak` for address generation, HTTP MCP Server (`rmcp`), multi-stage Docker build for deployment.
-   **Agent-Facing MCP API:** `create_wallet`, `get_bitcoin_address`, `get_ethereum_address`.
-   **Server-Side Logic:** Secure generation of private keys from passphrases, encrypted storage of wallet data (name, passphrase) and private keys in SQLite.
-   **Data Schema:** `Wallet` (`Name`, `Passphrase`), `Account` (`ID`, `WalletID`, `Private Key`).
-   **Core MVP Success Criteria:** Successful build/run via Docker, MCP client can create wallets and retrieve addresses, private key storage verification.

### Post-MVP Features

**Phase 2 (Growth):**
-   **Transaction Signing & Broadcasting:** Capabilities to enable AI agents to securely sign and broadcast transactions, evolving Arktos into a "hot wallet" controller.
-   **Expanded Blockchain Support:** Integration with additional popular blockchain networks (e.g., Solana, Polygon).
-   **Alternative Interfaces:** Exploration of other communication interfaces beyond HTTP MCP (e.g., gRPC, direct CLI).

**Phase 3 (Expansion/Vision):**
-   **Advanced Security Modules Integration:** Support for Hardware Security Modules (HSMs) or other enterprise-grade key protection mechanisms.
-   **Reference AI Agents:** Development and open-sourcing of example AI agents to demonstrate practical use cases and further lower the barrier to entry for builders.

### Risk Mitigation Strategy

-   **Technical Risks:** Mitigation through rigorous security audits, comprehensive testing, and adherence to Rust's safety features and cryptographic best practices. The modularity of the blueprint allows for isolated updates and fixes.
-   **Market Risks:** Addressed by the "educational blueprint" approach, which fosters community-driven adoption and validation. Early feedback from builders in alpha/beta programs will inform necessary pivots.
-   **Resource Risks:** Managed by maintaining a lean MVP scope, prioritizing core value, and leveraging the open-source community for contributions and support. Emphasis on robust documentation to reduce ongoing support burden.

## Functional Requirements

### Wallet Management

- FR1: The Arktos Wallet system shall allow an AI agent to securely create a new non-custodial wallet.
- FR2: The Arktos Wallet system shall securely generate a unique recovery passphrase for each new wallet.
- FR3: The Arktos Wallet system shall securely store the wallet's recovery passphrase in an encrypted format.
- FR4: The Arktos Wallet system shall enable the AI agent to manage multiple distinct blockchain accounts for a system owner.

### Address Management

- FR5: The Arktos Wallet system shall allow an AI agent to derive and retrieve a Bitcoin public address for a specified wallet ID.
- FR6: The Arktos Wallet system shall allow an AI agent to derive and retrieve an Ethereum public address for a specified wallet ID.
- FR7: The Arktos Wallet system shall leverage established cryptographic standards (e.g., BIP39/BIP32) for address derivation.

### Security & Data Handling

- FR8: The Arktos Wallet system shall expose only public data through its external APIs.
- FR9: The Arktos Wallet system shall store all sensitive data (e.g., private keys, passphrases) in an encrypted format locally.
- FR10: The Arktos Wallet system shall ensure sensitive data is only accessible by the system owner via direct database access.
- FR11: The Arktos Wallet system shall employ database encryption (e.g., SQLCipher) for data at rest.

### API & Integration

- FR12: The Arktos Wallet system shall expose core wallet functionalities as MCP tools via a single HTTP endpoint.
- FR13: The Arktos Wallet system shall provide an HTTP endpoint for system health checks (e.g., `/healthz`).
- FR14: The Arktos Wallet system shall authenticate MCP client requests using API keys.
- FR15: The Arktos Wallet system shall support data exchange in text-based formats (e.g., JSON) for MCP communication.
- FR16: The Arktos Wallet system shall provide comprehensive API documentation detailing MCP tools, arguments, return types, and potential error codes.

### System Operations

- FR17: The Arktos Wallet system shall be deployable via containerization (e.g., Docker).
- FR18: The Arktos Wallet system shall be implemented in Rust.

### Documentation & Extensibility

- FR19: The Arktos Wallet system shall serve as an open-source, educational blueprint for AI-controlled non-custodial wallets.
- FR20: The Arktos Wallet system shall be designed for customization by system owners to adapt to specific needs.
- FR21: The Arktos Wallet system shall provide mechanisms for builders to adapt to regional compliance requirements.
- FR22: The Arktos Wallet system shall abstract blockchain complexities from the end-user (via the AI agent).
- FR23: The Arktos Wallet system shall enable the AI agent to provide transparent reporting of managed funds to the end-user.

## Non-Functional Requirements

### Performance

-   NFR1: The `create_wallet` MCP API call shall respond within 500 milliseconds 95% of the time under normal load.
-   NFR2: The `get_bitcoin_address` and `get_ethereum_address` MCP API calls shall respond within 100 milliseconds 95% of the time under normal load.
-   NFR3: The system shall support up to 100 concurrent MCP API requests per second with no more than 5% degradation in response times.
-   NFR4: Wallet generation and address derivation processes shall consume minimal CPU and memory resources to enable efficient deployment within resource-constrained environments (e.g., small edge devices or microservices).

### Security

-   NFR5: All sensitive data, including recovery passphrases and private keys, shall be encrypted at rest using industry-standard AES-256 encryption.
-   NFR6: All communication to and from the MCP HTTP endpoint shall be secured using TLS 1.2 or higher.
-   NFR7: API keys used for authentication shall be unique per client and stored securely (e.g., hashed) by Arktos Wallet.
-   NFR8: The system shall prevent unauthorized access to sensitive data and functionalities, ensuring that only authenticated AI agents with valid API keys can perform authorized actions.
-   NFR9: The system shall be hardened against common web vulnerabilities (e.g., SQL injection, cross-site scripting) even though its primary interface is API-based.
-   NFR10: The system shall provide an audit log of all critical wallet management actions (e.g., wallet creation, address retrieval requests) accessible to the system owner.

### Scalability

-   NFR11: Arktos Wallet shall be deployable as a stateless microservice instance (at the processing layer) to facilitate horizontal scaling by the system owner.
-   NFR12: The system shall maintain consistent performance (as defined in NFR1-NFR3) when deployed across multiple instances, assuming an appropriate load balancing strategy.
-   NFR13: The local encrypted SQLite database shall support up to 10,000 wallets and 50,000 accounts without significant performance degradation on standard hardware.

### Integration

-   NFR14: The MCP API shall conform to the Model Context Protocol specification for tool exposure and interaction.
-   NFR15: The API documentation shall be sufficient for a developer to integrate an AI agent for basic wallet creation and address retrieval within 2 hours.

### Reliability

-   NFR16: The system shall achieve 99.9% uptime for its MCP API endpoint when deployed in a production environment with appropriate monitoring and infrastructure.
-   NFR17: In the event of an unexpected system shutdown or crash, all persisted data shall remain consistent and recoverable.
-   NFR18: The system shall provide clear error messages and status codes for API failures to assist client-side error handling.
