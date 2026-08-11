# =============================================================================
# DataGen Server — Dockerfile
# Multi-stage build: Rust builder → slim Debian runtime
# =============================================================================

# ---------------------------------------------------------------------------
# Stage 1: Builder
# ---------------------------------------------------------------------------
FROM rust:1.86-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies for native crates (OpenSSL, pkg-config, etc.)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml ./core/
COPY native/Cargo.toml ./native/
COPY server/Cargo.toml ./server/

# Create dummy src files to build dependencies (cached layer)
RUN mkdir -p core/src native/src server/src && \
    echo "fn main() {}" > server/src/main.rs && \
    for d in core native; do \
        echo "// dummy" > $d/src/lib.rs; \
    done

# Build dependencies (this layer is cached unless Cargo.toml/lock changes)
RUN cargo build --release -p datagen-server 2>/dev/null || true

# Remove dummy files and copy real source
RUN rm -rf core/src native/src server/src
COPY core/src ./core/src
COPY native/src ./native/src
COPY server/src ./server/src

# Touch main.rs to force rebuild of the actual binary
RUN touch server/src/main.rs

# Build the actual server binary
RUN cargo build --release -p datagen-server

# ---------------------------------------------------------------------------
# Stage 2: Runtime
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies:
#   - g++ / gcc  : compile C/C++ judge solutions
#   - python3    : run Python data-generation scripts & judge solutions
#   - ca-certificates: HTTPS support
RUN apt-get update && apt-get install -y --no-install-recommends \
    g++ \
    gcc \
    python3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled server binary from the builder stage
COPY --from=builder /app/target/release/datagen-server /app/datagen-server

# Expose the default server port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/app/datagen-server", "--health"] || curl -f http://localhost:3000/health || exit 1

# Run the server
ENTRYPOINT ["/app/datagen-server"]