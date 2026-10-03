# Keep the Rust version in sync with rust-toolchain.toml.
ARG RUST_IMAGE=rust:1.97.1-trixie
ARG CARGO_CHEF_VERSION=0.1.78

# Stage 1: Planner
# Use cargo-chef to cache dependencies separately from code
FROM ${RUST_IMAGE} AS planner
ARG CARGO_CHEF_VERSION
WORKDIR /app
RUN cargo install --locked cargo-chef --version "${CARGO_CHEF_VERSION}"
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Builder
FROM ${RUST_IMAGE} AS builder
ARG CARGO_CHEF_VERSION
WORKDIR /app

# Install build dependencies including OpenSSL dev
# Package versions are intentionally not pinned (hadolint DL3008, see .hadolint.yaml).
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install --locked cargo-chef --version "${CARGO_CHEF_VERSION}"
COPY rust-toolchain.toml ./
COPY --from=planner /app/recipe.json recipe.json

# Cook dependencies (cache layer)
RUN cargo chef cook --release --locked --recipe-path recipe.json

# Copy source code and build
COPY . .
RUN cargo build --release --locked --bin arktos-wallet

# Stage 3: Runtime
# Use distroless base image for minimal attack surface
FROM gcr.io/distroless/cc-debian13:nonroot

# Copy the compiled binary from builder stage
COPY --from=builder /app/target/release/arktos-wallet /usr/local/bin/

# Run as the distroless "nonroot" user (numeric so runAsNonRoot can verify it)
USER 65532:65532

# Expose default port (configurable via environment)
EXPOSE 8080

# No HEALTHCHECK: the previous one invoked an unimplemented `--health-check`
# flag through a shell, which distroless does not provide, so it could never
# succeed. Probe GET /healthz from the orchestrator until a self-check exists.

# Run the application
ENTRYPOINT ["/usr/local/bin/arktos-wallet"]
