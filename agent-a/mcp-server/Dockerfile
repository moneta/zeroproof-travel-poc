# Build stage
FROM rust:latest as builder

WORKDIR /build

# Copy the MCP server code
COPY agent-a/mcp-server ./agent-a/mcp-server
COPY zk-protocol ../zk-protocol

# Build the MCP server
RUN cd agent-a/mcp-server && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/agent-a/mcp-server/target/release/agent-a-mcp /usr/local/bin/agent-a-mcp

# Set environment variables with defaults
ENV AGENT_B_URL=http://localhost:8001
ENV ATTESTER_URL=http://localhost:8000
ENV ZEROPROOF_ADDRESS=0x9C33252D29B41Fe2706704a8Ca99E8731B58af41
ENV RPC_URL=http://json-rpc.9lmur1sx205wod4wavn42kr8r.blockchainnodeengine.com/?key=AIzaSyBanzf369uxM4kL0EHhXh5HSZpk3J8_4nA
ENV RUST_LOG=info

ENTRYPOINT ["/usr/local/bin/agent-a-mcp"]
