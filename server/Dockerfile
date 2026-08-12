# =============================================================================
# DataGen Server — Dockerfile
# Multi-stage build: Rust builder → slim Debian runtime
# =============================================================================

# ---------------------------------------------------------------------------
# Stage 1: Builder
# ---------------------------------------------------------------------------
FROM rust:slim-bookworm AS builder

WORKDIR /app

# Install build dependencies for native crates:
#   - pkg-config / libssl-dev            : core native-crate deps
#   - libwebkit2gtk-4.1-dev              : tauri v2 -> wry -> webkit2gtk-sys (Linux webview)
#   - libgtk-3-dev                       : tauri -> gtk-sys
#   - libayatana-appindicator3-dev       : tauri -> libappindicator-sys (tray)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libxdo-dev \
    patchelf \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml ./core/
COPY native/Cargo.toml ./native/
COPY wasm/Cargo.toml ./wasm/
COPY server/Cargo.toml ./server/

# Remove src-tauri from workspace members (not needed for server build)
RUN sed -i 's/ "src-tauri"//g' Cargo.toml

# Create dummy src files to build dependencies (cached layer)
RUN mkdir -p core/src native/src wasm/src server/src && \
    echo "fn main() {}" > server/src/main.rs && \
    for d in core native wasm; do \
        echo "// dummy" > $d/src/lib.rs; \
    done

# Build dependencies (this layer is cached unless Cargo.toml/lock changes).
# Fail loudly if the dependency build breaks (no hidden `|| true`).
RUN cargo build --release -p datagen-server

# Remove dummy files and copy real source
RUN rm -rf core/src native/src wasm/src server/src
COPY core/src ./core/src
COPY native/src ./native/src
COPY wasm/src ./wasm/src
COPY server/src ./server/src

# Force Cargo to rebuild every crate whose dummy source was used for the cache layer.
RUN touch core/src/lib.rs native/src/lib.rs wasm/src/lib.rs server/src/main.rs

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

# Health check: probe GET /health with the runtime's python3 (curl is not installed),
# honoring the PORT environment variable (server defaults to 3000).
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD python3 -c "import os, urllib.request; urllib.request.urlopen('http://localhost:' + os.environ.get('PORT', '3000') + '/health', timeout=5)"

# Run the server
ENTRYPOINT ["/app/datagen-server"]
