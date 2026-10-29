# Build stage
FROM rust:slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Build context is the repository root (Cargo workspace).
# Copy manifests first for better dependency caching
COPY Cargo.toml Cargo.lock* ./
COPY backend/Cargo.toml backend/
COPY crates/aidb-types/Cargo.toml crates/aidb-types/
COPY crates/aidb-client/Cargo.toml crates/aidb-client/
COPY crates/aidb-cli/Cargo.toml crates/aidb-cli/

# Dummy sources for every workspace member, then build dependencies only
RUN mkdir -p backend/src crates/aidb-types/src crates/aidb-client/src crates/aidb-cli/src && \
    echo "fn main() {}" > backend/src/main.rs && \
    touch crates/aidb-types/src/lib.rs crates/aidb-client/src/lib.rs && \
    echo "fn main() {}" > crates/aidb-cli/src/main.rs && \
    cargo build --release -p ai-db-api && \
    rm -rf backend/src crates/aidb-types/src crates/aidb-client/src crates/aidb-cli/src

# Copy actual source code
COPY crates ./crates
COPY backend/src ./backend/src

# Build the application (touch so cargo rebuilds over the dummy artifacts)
RUN touch backend/src/main.rs crates/aidb-types/src/lib.rs && \
    cargo build --release -p ai-db-api

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false appuser

# Create data directory for job persistence (BEFORE switching to appuser)
RUN mkdir -p /data && chown appuser:appuser /data

# Copy binary from builder
COPY --from=builder /app/target/release/ai-db-api /app/ai-db-api

# Set ownership
RUN chown -R appuser:appuser /app

USER appuser

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/api/health || exit 1

# Run the binary
CMD ["./ai-db-api"]
