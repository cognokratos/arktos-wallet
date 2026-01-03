# Deployment Guide: Arktos Wallet

This guide outlines the deployment strategy for the Arktos Wallet HTTP MCP server, focusing on containerization using Docker.

## Containerization with Docker

The project utilizes a multi-stage Docker build process to create efficient and secure production-ready images. This approach separates the dependency caching and build steps from the final runtime image, resulting in smaller image sizes and improved security.

### Docker Strategy

1.  **Dependency Installation & Caching (Builder Stage)**: Leverages `lukemathwalker/cargo-chef` to cache Rust dependencies. This significantly speeds up subsequent builds by only rebuilding changed dependencies.
2.  **Isolated Build (Builder Stage)**: The application is built within a dedicated builder stage.
3.  **Lean Runtime (Final Stage)**: The final image uses `gcr.io/distroless/static-debian13:nonroot` as a base. This provides a minimal, secure runtime environment with only the necessary components, reducing the attack surface and image size.

### Example Dockerfile (Conceptual)

A `Dockerfile` based on the provided strategy would look something like this:

```dockerfile
# Stage 1: Dependency Caching with cargo-chef
FROM lukemathwalker/cargo-chef:latest-rust-1.74.0 AS chef
WORKDIR /app
COPY . .
RUN cargo chef prepare --workspace

# Stage 2: Build the application
FROM chef AS builder
COPY . .
RUN cargo chef build --release --workspace

# Stage 3: Final lean runtime image
FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=builder /app/target/release/arktos-wallet .
# Optionally copy other assets if needed, e.g., configuration files
# COPY config/ /app/config/
EXPOSE 8080 # Example port, adjust as necessary
CMD ["./arktos-wallet"]
```

**Note:** This is a conceptual `Dockerfile`. Actual implementation might require adjustments for specific project layout, port exposure, and additional runtime dependencies if any.

## Running with Docker

To build the Docker image:

```sh
docker build -t arktos-wallet .
```

To run the Docker container (example, adjust port mapping as needed):

```sh
docker run -p 8080:8080 arktos-wallet
```
