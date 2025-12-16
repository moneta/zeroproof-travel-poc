# Deployment Guide: ZeroProofVerifier

## Quick Start

### 1. Set Environment Variables

```bash
# Find the pre-deployed SP1 universal verifier address on your network.
# For testnet/mainnet documentation, check SP1 docs.
export UNIVERSAL_VERIFIER="0x..."   # Pre-deployed SP1 verifier address

# Your deployer account private key
export PRIVATE_KEY="0x..."           # Keep this secure!

# Network RPC endpoint
export RPC_URL="https://..."         # e.g., Libertas RPC
```

### 2. Dry-Run (Simulate Deployment)

```bash
cd /home/revolution/zeroproof-travel-poc/zk-attestation-service

forge script script/Deploy.s.sol:DeployZeroProofVerifier \
  --rpc-url $RPC_URL \
  --private-key $PRIVATE_KEY
```

This will show you the deployment without sending any transaction.

### 3. Deploy (Actual Broadcast)

```bash
forge script script/Deploy.s.sol:DeployZeroProofVerifier \
  --rpc-url $RPC_URL \
  --private-key $PRIVATE_KEY \
  --broadcast
```

Output will show:
```
Wrapper address:       0x1234...
Universal verifier:    0x5678...
Deployer:              0xabcd...
```

**Save the Wrapper address** — this is what Agent A will call.

## Verification

After deployment, verify the contract:

```bash
# Check the bytecode
cast code $WRAPPER_ADDRESS --rpc-url $RPC_URL

# Check owner
cast call $WRAPPER_ADDRESS "getOwner()" --rpc-url $RPC_URL

# Check universal verifier reference
cast call $WRAPPER_ADDRESS "getVerifier()" --rpc-url $RPC_URL
```

## Testing the Wrapper

Once deployed, test with a real proof from the attester:

```bash
# From Agent A or a test client:
cast call $WRAPPER_ADDRESS \
  "verifyProof(bytes32,bytes,bytes)" \
  $VK_HASH \
  $PUBLIC_VALUES_HEX \
  $PROOF_HEX \
  --rpc-url $RPC_URL
```

Returns `true` (0x01) if proof is valid, reverts with "ProofVerificationFailed" otherwise.

## Contract Details

- **Contract**: `ZeroProofVerifier`
- **Location**: `contracts/ZeroProofVerifier.sol`
- **Deploy Script**: `script/Deploy.s.sol`
- **Main function**: `verifyProof(bytes32, bytes, bytes) -> bool`
- **Helper function**: `verifyProofWithDecoding(bytes32, bytes, bytes) -> (bool, uint256, uint256)`

## Next Steps

1. Update Agent A to call `ZeroProofVerifier.verifyProof(...)` instead of the universal verifier directly.
2. (Optional) Add app-specific validation logic (e.g., check public values constraints).
3. (Optional) Set up event listeners to track `ProofVerified` events.
