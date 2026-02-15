# ══════════════════════════════════════════════════════════
#  Multi-stage Dockerfile for Slack Incident Bot
# ══════════════════════════════════════════════════════════

# ── Stage 1: Build Dependencies ──
FROM rust:1.75-slim as planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 2: Build Cached Dependencies ──
FROM rust:1.75-slim as cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# ── Stage 3: Build Application ──
FROM rust:1.75-slim as builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release

# ── Stage 4: Runtime ──
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash incident-bot

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/incident-bot /app/incident-bot

# Copy migrations (needed for startup)
COPY --from=builder /app/migrations /app/migrations

# Set ownership
RUN chown -R incident-bot:incident-bot /app

# Switch to non-root user
USER incident-bot

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["/usr/bin/curl", "-f", "http://localhost:3000/health"]

# Run the binary
CMD ["/app/incident-bot"]
