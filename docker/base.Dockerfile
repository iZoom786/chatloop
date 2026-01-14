# ChatLoop Base Image
#
# This is the base Docker image for ChatLoop components.
# It contains the runtime dependencies and CPU optimizations.

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

# Install CPU optimization libraries
RUN apt-get update && apt-get install -y \
    libmkl-dev \
    libmkl-rt \
    libopenblas-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Copy workspace members
COPY crates ./crates

# Build in release mode
ENV RUSTFLAGS="-C target-cpu=native -C target-feature=+avx2,+fma"
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    libmkl-rt \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 chatloop

# Set working directory
WORKDIR /home/chatloop

# Copy binaries from builder
COPY --from=builder /build/target/release/chatloop-worker /usr/local/bin/
COPY --from=builder /build/target/release/chatloop-coordinator /usr/local/bin/

# Create directories
RUN mkdir -p /home/chatloop/models /home/chatloop/configs /home/chatloop/logs

# Set permissions
RUN chown -R chatloop:chatloop /home/chatloop

# Switch to non-root user
USER chatloop

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD exit 0

# Default command
CMD ["/bin/bash"]
