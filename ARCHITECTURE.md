# System Architecture

## Overview

Multi-agent zero-knowledge proof system with universal verification:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Agent A (Consumer)                            │
│  Requests service → Gets proof → Verifies on-chain              │
└────────┬──────────────────────────────────────┬─────────────────┘
         │                                      │
         ▼                                      ▼
┌──────────────────────┐                ┌──────────────────────┐
│   Agent B Server     │                │  Sepolia Testnet     │
│  - Pricing service   │                │  Universal Verifier  │
│  - Booking service   │                │  0x53A9038dCB210...  │
│  - ELF registration  │                └──────────────────────┘
└──────────┬───────────┘                          ▲
           │                                      │
           ▼                                      │
┌──────────────────────────────────────────────────────────┐
│    ZK Attester Service (GPU-accelerated)                │
│ - Receives ELF from Agent B                             │
│ - STARK proof generation (GPU, 11-27 min)              │
│ - Groth16 proof generation (<1 min)                    │
│ - Returns: proof + vk_hash + public_values             │
└──────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Agent A (Consumer)

**Location**: `/agent-a/`

**Purpose**: Consumes Agent B services with cryptographic verification

**Flow**:
```
1. HTTP POST http://localhost:8001/price
   ├─ Request: { from: "NYC", to: "LON", vip: true }
   ├─ Response: { data: {"price":578.0}, program_id: uuid, elf_hash: 0x... }
   └─ Store: program_id

2. HTTP POST http://localhost:8001/zk-input
   ├─ Request: { endpoint: "price", input: {...} }
   ├─ Response: { input_bytes: [1,2,3...] }
   └─ Get properly formatted zkVM input

3. HTTP POST http://localhost:8000/attest
   ├─ Payload: { program_id, input_bytes, claimed_output, verify_locally: true }
   ├─ Wait: 11-27 minutes (STARK) + <1 min (Groth16)
   ├─ Response: { proof: 0x..., vk_hash: 0x..., public_values: 0x..., verified_output: 578.0 }
   └─ Verify: local verification passed

4. eth_call to Sepolia (JSON-RPC)
   ├─ Contract: SP1VerifierGroth16 at 0x53A9038dCB210D210A7C973fA066Fd2C50aa8847
   ├─ Method: verifyProof(bytes32 vkHash, bytes publicValues, bytes proof)
   ├─ Response: Success (no revert) or Error (revert with reason)
   └─ Success: cryptographically verified on-chain!
```

**Environment Variables**:
- `AGENT_B_URL`: Agent B endpoint (default: http://localhost:8001)
- `ATTESTER_URL`: Attester endpoint (default: http://localhost:8000)
- `SP1_VERIFIER_ADDRESS`: Universal verifier contract (default: 0x53A9038dCB210D210A7C973fA066Fd2C50aa8847)
- `RPC_URL`: Sepolia RPC endpoint
- `RPC_URL`: Libertas RPC endpoint (optional; if missing, skips on-chain verify)

**Key Code**:
- Main loop: waits for user input, calls Agent B, attester, contract
- `verify_on_chain()`: encodes proof + inputs, calls contract via JSON-RPC

**Dependencies**:
- reqwest (HTTP client)
- serde_json (JSON parsing)
- ethers::abi (ABI encoding)
- hex (hex encoding/decoding)

---

### 2. Agent B Server (Multi-function Provider)

**Location**: `/agent-b/`

**Purpose**: Multi-function service (pricing + booking) with zkVM proof support

**Startup Flow**:
```
1. On startup:
   ├─ Read ELF from target/elf-compilation/.../agent-b-program
   ├─ POST to attester at /register-elf
   │  ├─ File: ELF binary
   │  ├─ Response: { program_id: uuid, elf_hash: 0x... }
   └─ Store program_id

2. Start HTTP server on 0.0.0.0:8001
```

**Endpoints**:

**POST /price**
```json
Request: { "from": "NYC", "to": "LON", "vip": true }
Response: {
  "data": {"price": 578.0},
  "program_id": "89456604-93dd-4aa5-bf70-109367ef33ad",
  "elf_hash": "0x8e93c12ab6da873e..."
}
```

**POST /zk-input**
```json
Request: { "endpoint": "price", "input": {...} }
Response: { "input_bytes": [1, 2, 3, ...] }
Purpose: Returns properly formatted bincode bytes for zkVM
```

**POST /book** (future)
```json
Request: { "from": "NYC", "to": "LON", "date": "2025-12-20" }
Response: { "data": {"confirmation": "ABC123"}, "program_id": "...", "elf_hash": "..." }
```

**Environment Variables**:
- `ATTESTER_URL`: Attester location (default: http://localhost:8000)
- `BOOKING_API_URL`: External booking API (optional)

**Key Features**:
- Single ELF handles multiple RPC functions (pricing, booking)
- Agent A doesn't need to know internal zkVM structure
- Each function returns its own proof

---

### 3. ZK Attester Service (GPU-accelerated Proof Generator)

**Location**: `/zk-attestation-service/attester/`

**Purpose**: Generates SP1 v5.2.4 proofs with GPU acceleration

**Startup Flow**:
```
1. Initialize in-memory HashMap for ELF storage
2. Detect GPU (NVIDIA CUDA)
3. Start HTTP server on 0.0.0.0:8000
```

**Endpoints**:

**POST /register-elf** (multipart/form-data)
```
Request (multipart):
  - file: ELF binary
  - field: elf_name (optional)

Response:
{
  "program_id": "89456604-93dd-4aa5-bf70-109367ef33ad",
  "elf_hash": "0x8e93c12ab6da873e..."
}
```

**POST /attest** (application/json)
```
Request:
{
  "program_id": "89456604-93dd-4aa5-bf70-109367ef33ad",
  "input_bytes": [1, 2, 3, ...],
  "claimed_output": "{\"price\":578.0}",
  "verify_locally": true
}

Response:
{
  "success": true,
  "proof": "0xa4594c59bbc142f3...",  // 260 bytes (VERIFIER_HASH + Groth16)
  "public_values": "0x000000000000000000108240",  // 12 bytes
  "vk_hash": "0x003a20824d4b95530548ffa351cb96699dc3ed7386719ab90699d49dd910273c",
  "verified_output": "{\"price\":578.0}"
}
```

**Proof Generation Pipeline**:
```
1. Retrieve ELF from HashMap by program_id
2. Create SP1 ProverClient (GPU-accelerated)
3. Call prover.setup(&elf) → get proving key (PK) and verifying key (VK)
4. Compute vk_hash = vk.bytes32()
5. STARK Phase (GPU-accelerated, 11-27 minutes):
   ├─ Create stdin with input_bytes
   ├─ prover.prove(&pk, &stdin) → generates STARK proof
   └─ Uses CUDA for acceleration (1000-3000% CPU usage = multi-core + GPU)
6. Groth16 Phase (<1 minute):
   ├─ .groth16().run() → wraps STARK in Groth16
   ├─ Uses Docker container sp1-gnark
   └─ Result: 260-byte proof (4-byte VERIFIER_HASH + 256-byte Groth16)
7. Local Verification:
   ├─ prover.verify(&proof, &vk)
   └─ Ensures proof is valid before returning
8. Extract components:
   ├─ proof_bytes = proof.bytes()  // 260 bytes with VERIFIER_HASH
   ├─ public_values = proof.public_values.as_slice()
   └─ vk_hash (32 bytes)
9. Return AttestResponse
```

**No Environment Variables Required**
- GPU auto-detected via CUDA
- All computation local, no blockchain interaction

**Key Features**:
- GPU acceleration for STARK phase
- Docker-based Groth16 wrapping
- Universal proof format (works with any v5.2.4 verifier)
- Local verification before returning

**Dependencies**:
- sp1-sdk v5.2.4 (proof generation)
- axum (HTTP server)
- tokio (async runtime)
- serde/serde_json (serialization)
- hex (encoding)

---

### 4. Universal Verifier Contract (On-Chain Verification)

**Deployed**: Sepolia at `0x53A9038dCB210D210A7C973fA066Fd2C50aa8847`

**Purpose**: Verifies ALL SP1 v5.2.4 Groth16 proofs (program-agnostic, version-specific)

**Contract Interface**:
```solidity
function VERSION() external pure returns (string memory);
// Returns: "v5.0.0" (circuits unchanged from v5.0.0)

function VERIFIER_HASH() public pure returns (bytes32);
// Returns: 0xa4594c59bbc142f3b81c3ecb7f50a7c34bc9af7c4c444b5d48b795427e285913

function verifyProof(
    bytes32 programVKey,      // VK hash from zkVM program
    bytes calldata publicValues,  // Committed public values
    bytes calldata proofBytes     // [4-byte VERIFIER_HASH][256-byte Groth16]
) external view;
// Reverts if proof invalid
// Returns nothing if proof valid
```

**Verification Logic**:
1. Extract VERIFIER_HASH from proofBytes[:4]
2. Check: bytes4(proofBytes[:4]) == bytes4(VERIFIER_HASH())
   - If mismatch: revert WrongVerifierSelector
3. Hash public values: sha256(publicValues) & ((1 << 253) - 1)
4. Build inputs: [programVKey, publicValuesDigest]
5. Decode Groth16 proof: abi.decode(proofBytes[4:], (uint256[8]))
6. Call Groth16Verifier.Verify(proof, inputs)
   - If invalid: revert InvalidProof
7. Return (no revert = success)

**Key Properties**:
- **Universal**: Verifies ANY program using SP1 v5.2.4
- **Stateless**: No VK storage needed (passed as parameter)
- **Scalable**: One contract for unlimited programs
- **Gas Efficient**: ~250k gas per verification

**Source**: https://github.com/succinctlabs/sp1-contracts

---

## Data Flow: Complete Request

### Scenario: Agent A requests proof for pricing

**Step 1: Agent A → Agent B (POST /price)**
```
Agent A: "Price from NYC to LON, VIP?"
Agent B: {"data": {"price":578.0}, "program_id": "...", "elf_hash": "0x..."}
```

**Step 2: Agent B (startup) → Attester (POST /register-elf)**
```
Agent B: "Here's my ELF binary"
Attester: "Registered! program_id: <uuid>, elf_hash: 0x..."
```

**Step 3: Agent A → Agent B (POST /zk-input)**
```
Agent A: "Give me zkVM input for /price with these params"
Agent B: {"input_bytes": [1,2,3,...]}  // Bincode-encoded RpcCall
```

**Step 4: Agent A → Attester (POST /attest)**
```
Agent A: "Prove program <uuid> with input_bytes outputs {"price":578.0}"
Attester:
  1. Load ELF for <uuid>
  2. Setup: prover.setup(&elf) → PK, VK
  3. STARK phase (GPU, 11-27 min) → STARK proof
  4. Groth16 phase (<1 min) → wrap STARK in Groth16
  5. Local verify: prover.verify(&proof, &vk)
  6. Return: proof (260 bytes), vk_hash (32 bytes), public_values (12 bytes)
```

**Step 5: Agent A → Sepolia (eth_call verifyProof)**
```
Agent A: "Verify this proof at 0x53A9038dCB210D210A7C973fA066Fd2C50aa8847"
Contract:
  1. Check VERIFIER_HASH in proof matches contract
  2. Verify Groth16 proof cryptographically
  3. No revert = success ✓
Agent A: "Proof verified on-chain!"
```

## Key Design Decisions

### 1. Universal Verifier Pattern

- **Why**: One contract for all programs, no per-program deployment
- **Trade-off**: VK passed as parameter (32 bytes overhead)
- **Benefit**: Infinitely scalable, no contract management

### 2. Two-Phase Proving (STARK + Groth16)

- **Why**: STARK fast to generate, Groth16 cheap to verify on-chain
- **Trade-off**: Longer proof time (11-27 min vs instant)
- **Benefit**: ~100k gas verification vs millions for STARK

### 3. GPU Acceleration

- **Why**: STARK phase is parallelizable, 10-50x faster with GPU
- **Trade-off**: Requires CUDA GPU (RTX 4090)
- **Benefit**: 11-27 min instead of hours

### 4. Multi-Function Single ELF

- **Why**: One zkVM program handles pricing + booking + future functions
- **Trade-off**: Larger ELF binary
- **Benefit**: Agent A only needs one program_id for all functions

### 5. In-Memory ELF Storage

- **Why**: Fast, simple, good for PoC
- **Trade-off**: Lost on restart
- **Production**: Add database (PostgreSQL)

### 6. JSON-RPC eth_call for Verification

- **Why**: Read-only, no gas, simple
- **Trade-off**: No on-chain event/storage
- **Alternative**: eth_sendTransaction for permanent record (costs gas)

## Performance Profile

| Operation | Duration | Notes |
|-----------|----------|-------|
| ELF registration | <1s | One-time per program |
| STARK proof (GPU) | 11-27 min | Parallelized on RTX 4090 |
| Groth16 wrapping | <1 min | Docker-based |
| Local verification | <1s | Fast cryptographic check |
| On-chain verify call | 1-3s | Network latency |
| On-chain gas | ~250k | Cheap Groth16 verification |
| **Total flow** | **12-30 min** | Dominated by STARK phase |

## Scalability

- **Single attester**: 1 proof per 12-30 minutes (sequential)
- **Multiple attesters**: Horizontal scaling possible
- **Universal verifier**: Unlimited programs, one contract
- **Proof caching**: Cache proofs for identical inputs
- **Future**: Proof aggregation (combine multiple proofs)

## Future Enhancements

1. **Persistent Storage**: Move ELF storage to database with replication
2. **Async Proof Generation**: Queue attestation requests, return proof later
3. **Proof Caching**: Cache proofs for identical inputs across multiple programs
4. **Contract Optimization**: Embed real verification logic; use precompiles
5. **Token Economics**: Charge for attestation; distribute to provers
6. **Proof Markets**: Multiple attesters competing; cost/speed trade-offs
