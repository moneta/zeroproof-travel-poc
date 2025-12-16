# Deploy SP1 v5.2.4 Universal Groth16 Verifier to Sepolia

## Prerequisites

1. **Sepolia ETH**: ~0.01 ETH for deployment (~3-5M gas)
2. **Private Key**: Deployer wallet private key
3. **RPC URL**: Sepolia RPC endpoint (Alchemy/Infura)
4. **Etherscan API Key** (optional, for verification)

## Setup

1. **Copy environment file**:
```bash
cd /home/revolution/zeroproof-travel-poc/sp1-verifier-deploy
cp .env.example .env
```

2. **Edit `.env` file**:
```bash
nano .env
```

Add your keys:
```
RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_ALCHEMY_KEY
PRIVATE_KEY=0x1234567890abcdef...  # Your deployer private key
ETHERSCAN_API_KEY=ABC123...         # Your Etherscan API key
```

## Deploy

```bash
# Load environment variables
source .env

# Deploy to Sepolia
forge script script/DeploySP1Verifier.s.sol:DeploySP1Verifier \
    --rpc-url $RPC_URL \
    --broadcast \
    --verify \
    --etherscan-api-key $ETHERSCAN_API_KEY \
    -vvvv
```

**Output will show**:
```
SP1 Groth16 Verifier deployed to: 0x... 
VERIFIER_HASH: 0xa4594c59bbc142f3b81c3ecb7f50a7c34bc9af7c4c444b5d48b795427e285913
VERSION: v5.0.0
```

## Verify Contract (if --verify fails)

```bash
forge verify-contract \
    <DEPLOYED_ADDRESS> \
    src/SP1VerifierGroth16.sol:SP1Verifier \
    --chain sepolia \
    --etherscan-api-key $ETHERSCAN_API_KEY
```

## Update Agent A

After deployment, update the contract address in `/home/revolution/zeroproof-travel-poc/agent-a/src/main.rs`:

```rust
let sp1_verifier_addr = "0x...";  // Your deployed address
```

## Test On-Chain Verification

```bash
cd /home/revolution/zeroproof-travel-poc/agent-a
cargo run
```

Should now see:
```
✅ Proof verified on-chain successfully!
```

## Contract Details

- **VERIFIER_HASH**: `0xa4594c59bbc142f3b81c3ecb7f50a7c34bc9af7c4c444b5d48b795427e285913`
- **Version**: v5.0.0 (circuits compatible with SP1 SDK v5.2.4)
- **Function**: `verifyProof(bytes32 programVKey, bytes calldata publicValues, bytes calldata proofBytes)`
- **Universal**: Works for ALL programs using SP1 v5.2.4

## Why This Works

The Groth16 circuits (and VERIFIER_HASH) are the same between v5.0.0 and v5.2.4. Only the SDK version changed, not the underlying proving system. This single verifier contract will verify proofs from:
- Agent B (pricing)
- Agent B (booking)
- Any other zkVM program using SP1 v5.2.4

**One contract = unlimited programs** ✅
