# Deployment Guide: Arktos Wallet

This guide outlines the deployment strategy for the Arktos Wallet HTTP MCP server, focusing on containerization using Docker.

## Containerization with Docker

The project utilizes a multi-stage Docker build process to create efficient and secure production-ready images. This approach separates the dependency caching and build steps from the final runtime image, resulting in smaller image sizes and improved security.

### Docker Strategy

The [`Dockerfile`](../Dockerfile) has three stages:

1.  **Planner**: Uses [`cargo-chef`](https://github.com/LukeMathWalker/cargo-chef) to compute a dependency recipe.
2.  **Builder**: Cooks (pre-builds) dependencies as a cached layer, then builds the `arktos-wallet` binary with `--locked`. The `rust:<version>` base image matches the toolchain pinned in `rust-toolchain.toml`.
3.  **Runtime**: Copies the binary into `gcr.io/distroless/cc-debian13:nonroot` and runs it as the non-root user (UID 65532), exposing port 8080.

The image has no shell and defines no Docker `HEALTHCHECK`; configure your orchestrator to probe `GET /healthz`.

CI builds the image and lints the Dockerfile with hadolint on every push and pull request (`make docker-build` / `make docker-lint` locally).

## Running with Docker

To build the Docker image:

```sh
docker build -t arktos-wallet .
```

To run the Docker container (example, adjust port mapping as needed):

```sh
docker run -p 8080:8080 arktos-wallet
```
