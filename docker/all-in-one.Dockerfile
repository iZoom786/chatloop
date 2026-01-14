# ChatLoop All-in-One Dockerfile
# Builds both Coordinator and Worker in a single image
# Set CHATLOOP_ROLE environment variable to choose which service to run

# ============================================
# Stage 1: Builder - Compile Rust code
# ============================================
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    gcc \
    g++ \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy source code
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates

# Build all crates in release mode
ENV RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2"
RUN cargo build --release

# ============================================
# Stage 2: Runtime - Minimal image
# ============================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 chatloop

# Set working directory
WORKDIR /home/chatloop

# Copy compiled binaries from builder
COPY --from=builder /build/target/release/chatloop-worker /usr/local/bin/
COPY --from=builder /build/target/release/chatloop-coordinator /usr/local/bin/

# Copy configuration templates
COPY configs /home/chatloop/configs

# Create directories
RUN mkdir -p /home/chatloop/models /home/chatloop/logs && \
    chown -R chatloop:chatloop /home/chatloop

# Switch to non-root user
USER chatloop

# Set environment variables
ENV CHATLOOP_BIND_ADDRESS=0.0.0.0
ENV CHATLOOP_CONFIG=/home/chatloop/configs

# Expose ports (both coordinator and workers can use these ranges)
# Coordinator typically uses 50050
# Workers typically use 50051-50054
EXPOSE 50050 50051 50052 50053 50054

# Metrics port
EXPOSE 9091

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD exit 0 || exit 1

# Startup script - chooses which service to run based on CHATLOOP_ROLE
CMD ["/bin/bash", "-c", "\
    if [ \"$CHATLOOP_ROLE\" = \"coordinator\" ]; then \
        echo \"Starting ChatLoop Coordinator...\"; \
        export CHATLOOP_MODE=coordinator; \
        export CHATLOOP_CONFIG=/home/chatloop/configs/coordinator-config.yaml; \
        exec chatloop-coordinator; \
    elif [ \"$CHATLOOP_ROLE\" = \"worker\" ]; then \
        echo \"Starting ChatLoop Worker...\"; \
        export CHATLOOP_MODE=worker; \
        export CHATLOOP_CONFIG=/home/chatloop/configs/worker-config.yaml; \
        exec chatloop-worker; \
    else \
        echo \"Error: CHATLOOP_ROLE must be set to 'coordinator' or 'worker'\"; \
        echo \"Usage: docker run -e CHATLOOP_ROLE=coordinator ...\"; \
        echo \"       docker run -e CHATLOOP_ROLE=worker ...\"; \
        exit 1; \
    fi \
"]
