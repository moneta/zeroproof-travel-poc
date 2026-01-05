# Agent A Rust MCP Server

A high-performance Rust implementation of the Model Context Protocol (MCP) server that wraps Agent A's ZK proof operations, enabling integration with Claude Desktop and other MCP clients.

## Overview

This server exposes Agent A's core functionality as MCP tools:

- **call_agent_b**: Get pricing and program info from Agent B
- **format_zk_input**: Format input for zkVM computation  
- **request_attestation**: Request ZK proof from attester (11-27 min for STARK)
- **verify_on_chain**: Verify proofs on Sepolia testnet via JSON-RPC

## Architecture

```
MCP Client (Claude, Claude Desktop, etc.)
    ↓ (MCP Protocol - stdio or HTTP)
Rust MCP Server (rmcp SDK)
    ↓ (Function calls)
Agent A Library (src/lib.rs)
    ↓ (HTTP requests)
Agent B, Attester Service, Blockchain RPC
```

## Benefits

✅ **Performance**: Rust implementation handles crypto and HTTP efficiently  
✅ **Type Safety**: Schemars for JSON schema validation of tool parameters  
✅ **Integration**: Works with Claude and any MCP-compatible client  
✅ **Reusability**: Original Rust logic preserved, no rewrites  
✅ **Flexibility**: Can be used standalone or with Claude orchestration  

## Building

### Prerequisites

- Rust 1.70+ (use `rustup` to install)
- Clone the repo with zk-protocol dependency

### Build Release Binary

```bash
cd agent-a/mcp-server
cargo build --release
```

Binary will be at: `target/release/agent-a-mcp`

### Run Locally

```bash
# Set environment variables
export AGENT_B_URL=http://localhost:8001
export ATTESTER_URL=http://localhost:8000
export ZEROPROOF_ADDRESS=0x9C33252D29B41Fe2706704a8Ca99E8731B58af41
export RPC_URL=https://sepolia-rpc.com
export RUST_LOG=info

# Run the server (listens on stdin/stdout)
./target/release/agent-a-mcp
```

## Usage

### With MCP Inspector (Testing)

```bash
# Terminal 1: Start the server in the inspector
npx @modelcontextprotocol/inspector

# Enter command:
/path/to/agent-a-mcp

# Then call tools interactively
```

### With Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "agent-a": {
      "command": "/path/to/agent-a-mcp",
      "args": [],
      "env": {
        "AGENT_B_URL": "http://localhost:8001",
        "ATTESTER_URL": "http://localhost:8000",
        "ZEROPROOF_ADDRESS": "0x9C33252D29B41Fe2706704a8Ca99E8731B58af41",
        "RPC_URL": "https://sepolia-rpc.com",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### With Python Client

```bash
cd agent-service/mcp_client/agent_a
python main.py
```

## Tool Definitions

### call_agent_b

Get pricing from Agent B service.

**Input Schema:**
```json
{
  "from": "string",  // Source location (e.g., "NYC")
  "to": "string",    // Destination location (e.g., "LON")
  "vip": "boolean"   // VIP customer status
}
```

**Output:**
```json
{
  "price": 578.0,
  "program_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
  "elf_hash": "0x1234567890abcdef..."
}
```

### format_zk_input

Format input for zkVM computation.

**Input Schema:**
```json
{
  "endpoint": "string",   // E.g., "price", "booking"
  "input": "object"       // JSON object with input data
}
```

**Output:**
```json
{
  "input_hex": "0x48656c6c6f...",
  "input_array": [72, 101, 108, 108, 111],
  "length": 5
}
```

### request_attestation

Request ZK proof from attester service. **⏱️ Takes 11-27 minutes!**

**Input Schema:**
```json
{
  "program_id": "string",           // From call_agent_b
  "input_hex": "string",            // From format_zk_input
  "claimed_output": "string|null",  // Expected output (optional)
  "verify_locally": "boolean"       // Always true for now
}
```

**Output:**
```json
{
  "verified_output": "578.0",
  "vk_hash": "0xabcdef1234567890...",
  "proof": "0x48656c6c6f20576f726c6421...",
  "public_values": "0x0000000000000000000002660..."
}
```

### verify_on_chain

Verify proof on Sepolia testnet.

**Input Schema:**
```json
{
  "proof": "string",           // From request_attestation
  "public_values": "string",   // From request_attestation
  "vk_hash": "string"          // From request_attestation
}
```

**Output:**
```json
{
  "verified": true,
  "error": null,
  "details": "Proof cryptographically verified on-chain"
}
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AGENT_B_URL` | `http://localhost:8001` | Agent B service endpoint |
| `ATTESTER_URL` | `http://localhost:8000` | Attester service endpoint |
| `ZEROPROOF_ADDRESS` | `0x9C33...` | Sepolia ZeroProof contract address |
| `RPC_URL` | (Google RPC) | Sepolia JSON-RPC endpoint |
| `RUST_LOG` | `info` | Log level (debug, info, warn, error) |

## Docker

### Build Image

```bash
cd ../..  # Go to zeroproof-travel-poc root
docker build -f agent-a/mcp-server/Dockerfile -t agent-a-mcp:latest .
```

### Run Container

```bash
docker run \
  -e AGENT_B_URL=http://agent-b:8001 \
  -e ATTESTER_URL=http://attester:8000 \
  -e ZEROPROOF_ADDRESS=0x9C33... \
  -e RPC_URL=https://sepolia-rpc.com \
  agent-a-mcp:latest
```

### With Docker Compose

See [docker-compose.yaml](../../docker-compose.yaml):

```bash
docker compose up agent-a-mcp
```

## File Structure

```
agent-a/mcp-server/
├── Cargo.toml              # Dependencies
├── Dockerfile              # Container image
├── src/
│   ├── lib.rs             # Library with core functions
│   │   ├── verify_on_chain()        # Proof verification
│   │   ├── call_agent_b()           # Agent B call
│   │   ├── format_zk_input()        # Input formatting
│   │   └── request_attestation()    # Attestation request
│   └── main.rs            # MCP server implementation
│       ├── AgentAMcp struct         # Server handler
│       ├── Tool handlers (4 tools)
│       └── main() entrypoint
└── README.md              # This file
```

## Development

### Project Structure

- **lib.rs**: Pure functions for ZK operations (testable, reusable)
- **main.rs**: MCP server boilerplate and tool wrappers

### Key Dependencies

- `rmcp 0.1`: Rust MCP SDK with `#[tool_router]` macros
- `tokio`: Async runtime
- `ethers`: Ethereum ABI encoding
- `reqwest`: HTTP client
- `schemars`: JSON schema generation

### Adding New Tools

1. Add function to `lib.rs` with clear parameters
2. Create parameter struct in `main.rs` with `#[derive(JsonSchema)]`
3. Add `#[tool(...)]` method to `AgentAMcp` impl block
4. Use `Parameters(params)` wrapper to extract args

Example:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MyToolParams {
    pub arg1: String,
    pub arg2: i32,
}

#[tool(description = "My new tool description")]
async fn my_tool(&self, Parameters(params): Parameters<MyToolParams>) -> Result<CallToolResult> {
    match my_lib_function(&params.arg1, params.arg2).await {
        Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
        Err(e) => Ok(CallToolResult::error(vec![Content::text(format!("Error: {}", e))])),
    }
}
```

## Logging

Logs go to stderr with configurable levels:

```bash
RUST_LOG=debug ./target/release/agent-a-mcp  # Verbose
RUST_LOG=info ./target/release/agent-a-mcp   # Normal
RUST_LOG=warn ./target/release/agent-a-mcp   # Warnings only
```

Example log output:
```
2024-01-02T10:15:30.123Z INFO agent_a_mcp: Starting Agent A MCP Server
2024-01-02T10:15:30.125Z INFO agent_a_mcp:   Agent B URL: http://localhost:8001
2024-01-02T10:15:30.126Z INFO agent_a_mcp:   Attester URL: http://localhost:8000
2024-01-02T10:15:30.128Z INFO agent_a_mcp: Agent A MCP Server ready. Waiting for connections...
```

## Troubleshooting

### "command not found"

Ensure the binary is in PATH or use full path:

```bash
/path/to/target/release/agent-a-mcp
```

### "Connection refused" from Agent B or Attester

Check that services are running:

```bash
curl http://localhost:8001/health  # Agent B
curl http://localhost:8000/health  # Attester
```

### Timeout on attestation request

This is normal - proofs take 11-27 minutes. The server has a 2-hour timeout. Be patient!

### Invalid RPC endpoint

Verify the endpoint is correct and accessible:

```bash
curl -X POST $RPC_URL -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

## Integration with Agent Service

This MCP server is designed to work with the `agent-service` ecosystem:

- Use via Python client: `agent-service/mcp_client/agent_a/`
- Use via Claude Desktop: Add to `claude_desktop_config.json`
- Use via MCP Inspector: Run the binary in inspector UI

See [Agent A Client README](../agent_a/README.md) for usage examples.

## Performance Notes

- Agent B call: <1 second
- ZK input formatting: <1 second  
- Attestation request: 11-27 minutes (STARK proof generation on GPU)
- On-chain verification: 1-2 seconds (JSON-RPC call)

**Total end-to-end time: ~15 minutes for first proof**

## Security Considerations

- Proof verification happens both locally (by attester) and on-chain (by ZeroProof contract)
- VK hash must match the contract's verification key
- All data is hex-encoded for safety
- Environment variables store sensitive URLs - keep secure in production

## License

Same as parent project

## References

- [Model Context Protocol](https://modelcontextprotocol.io/)
- [Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [ZK Protocol](../../zk-protocol)
- [Agent B Service](../agent-b)
