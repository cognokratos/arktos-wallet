# Stage 1: Planner
# Use cargo-chef to cache dependencies separately from code
FROM rust:1-trixie AS planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Builder
FROM rust:1-trixie AS builder
WORKDIR /app

# Install build dependencies including OpenSSL dev
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json

# Cook dependencies (cache layer)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build
COPY . .
RUN cargo build --release --bin arktos-wallet

# Stage 3: Runtime
# Use distroless base image for minimal attack surface
FROM gcr.io/distroless/cc-debian13:nonroot

# Copy the compiled binary from builder stage
COPY --from=builder /app/target/release/arktos-wallet /usr/local/bin/

# Set up non-root user (distroless:nonroot already handles this)
USER nonroot:nonroot

# Expose default port (configurable via environment)
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/arktos-wallet", "--health-check"] || exit 1

# Run the application
ENTRYPOINT ["/usr/local/bin/arktos-wallet"]
