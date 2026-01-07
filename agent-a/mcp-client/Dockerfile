# Multi-stage build for Agent A MCP Client (Rust)
# Stage 1: Builder
FROM rust:latest AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace and build files
COPY agent-a/Cargo.toml agent-a/Cargo.lock ./

# Copy workspace members
COPY agent-a/mcp-server/ ./mcp-server/
COPY agent-a/mcp-client/ ./mcp-client/
COPY agent-a/src/ ./src/

# Copy shared protocol
COPY zk-protocol/ ../zk-protocol/

# Build the MCP client in release mode
RUN cargo build --release -p mcp-client

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the built binary from builder
COPY --from=builder /app/target/release/mcp-client /app/mcp-client

# Set environment variables with defaults
ENV AGENT_A_URL=http://localhost:3029
ENV RUST_LOG=info

# Expose the MCP client port
EXPOSE 3030

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3030/health || exit 1

ENTRYPOINT ["/app/mcp-client"]
