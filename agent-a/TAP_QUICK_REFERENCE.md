# TAP (Trusted Agent Protocol) Quick Reference

## What is TAP?

TAP is Visa's protocol for authenticating AI agents making payments on behalf of consumers. It uses RFC 9421 HTTP Message Signatures to cryptographically prove that requests come from a trusted agent.

## Quick Setup

### 1. Set Environment Variables

```bash
# For Ed25519 (recommended)
export ED25519_PRIVATE_KEY="$(cat /path/to/private-key.pem)"
export TAP_KEY_ID="your-key-id-from-visa"

# For RSA (alternative)
# export RSA_PRIVATE_KEY="$(cat /path/to/rsa-key.pem)"
```

### 2. Start Agent A

```bash
cargo run --bin mcp-client-http --release
```

Server will log:
```
[TAP] ✓ TAP signature generation enabled
  Algorithm: Ed25519
  Key ID: your-key-id-from-visa
```

### 3. Make Signed Requests

The agent automatically adds TAP headers to all HTTP requests:

```
Signature-Input: sig2=("@authority" "@path");created=...;expires=...;keyId="...";alg="Ed25519";nonce="...";tag="agent-browser-auth"
Signature: sig2=:base64_encoded_signature:
```

## Using TAP in Code

### Basic GET Request with Signature

```rust
use mcp_client::tap_get;

let response = tap_get(
    "https://dev.agentb.zeroproofai.com/api/products",
    &tap_config
).await?;
```

### POST Request with Signature

```rust
use mcp_client::tap_post;

let response = tap_post(
    "https://dev.agentb.zeroproofai.com/api/checkout",
    r#"{"item": "flight", "date": "2025-01-15"}"#.to_string(),
    &tap_config
).await?;
```

### Manual Signature Generation

```rust
use mcp_client::{TapConfig, create_tap_signature, parse_url_components};

let config = TapConfig::from_env()?;
let (authority, path) = parse_url_components("https://example.com/api/products")?;
let headers = create_tap_signature(&config, &authority, &path)?;

println!("Signature-Input: {}", headers.signature_input);
println!("Signature: {}", headers.signature);
```

## Request Flow

```
Agent A Application
        │
        ├─ Load ED25519_PRIVATE_KEY from environment
        │
        ├─ Make HTTP request to merchant endpoint
        │
        ├─ Generate TAP signature headers:
        │  ├─ Extract authority: example.com
        │  ├─ Extract path: /api/products
        │  ├─ Create timestamp window (now to now+8min)
        │  ├─ Generate random nonce
        │  ├─ Build RFC 9421 signature base string
        │  ├─ Sign with Ed25519 private key
        │  └─ Base64 encode signature
        │
        ├─ Add headers to request:
        │  ├─ Signature-Input: sig2=(...)
        │  └─ Signature: sig2=:...base64...:
        │
        └─ Send request
            │
            ▼
        Merchant Edge (Cloudflare)
            │
            ├─ Receive request with TAP headers
            │
            ├─ Validate signature:
            │  ├─ Check timestamp window (8min max)
            │  ├─ Retrieve public key from Visa
            │  ├─ Reconstruct signature base string
            │  ├─ Verify signature with public key
            │  └─ Check nonce not used before
            │
            ├─ If valid:
            │  ├─ Forward to origin server
            │  └─ Add "X-Trust-Headers: agent-verified"
            │
            └─ If invalid:
               ├─ Return 403 Forbidden, or
               └─ Return 429 Too Many Requests
```

## Header Format

### Signature-Input Header

```
sig2=("@authority" "@path");
created=1735689600;
expires=1735693200;
keyId="poqkLGiymh_W0uP6PZFw-dvez3QJT5SolqXBCW38r0U";
alg="Ed25519";
nonce="e8N7S2MFd/qrd6T2R3tdfAuuANngKI7LFtKYI/vowzk4lAZYadIX6wW25MwG7DCT9RUKAJ0qVkU0mEeLElW1qg==";
tag="agent-browser-auth"
```

### Signature Header

```
sig2=:jdq0SqOwHdyHr9+r5jw3iYZH6aNGKijYp/EstF4RQTQdi5N5YYKrD+mCT1HA1nZDsi6nJKuHxUi/5Syp3rLWBA==:
```

## Key Components

| Component | Example | Purpose |
|-----------|---------|---------|
| **Authority** | `dev.agentb.zeroproofai.com` | Host portion of URL |
| **Path** | `/api/products?id=123` | Path + query parameters |
| **Created** | `1735689600` | Unix timestamp (seconds) |
| **Expires** | `1735693200` | created + 480 seconds |
| **KeyId** | `poqkLGiymh_...` | Public key identifier |
| **Algorithm** | `Ed25519` or `PS256` | Crypto algorithm |
| **Nonce** | `e8N7S2MFd/qr...` | Random session ID |
| **Tag** | `agent-browser-auth` | Interaction type |

## Interaction Types

| Tag | Use Case | Example |
|-----|----------|---------|
| `agent-browser-auth` | Browsing for product info | GET /api/products |
| `agent-payer-auth` | Making a payment | POST /api/checkout |

## Environment Variables

| Variable | Required | Example |
|----------|----------|---------|
| `ED25519_PRIVATE_KEY` | Yes (or RSA_PRIVATE_KEY) | `-----BEGIN PRIVATE KEY-----...` |
| `TAP_KEY_ID` | No | `poqkLGiymh_W0uP6PZFw-...` |
| `TAP_ALGORITHM` | No | `Ed25519` (default) or `PS256` |

## Common Issues

| Issue | Solution |
|-------|----------|
| "TAP signature generation disabled" | Set `ED25519_PRIVATE_KEY` environment variable |
| "Failed to parse URL" | Ensure URL includes scheme (https://) and host |
| "Signature verification failed" (merchant) | Check clock synchronization, ensure same URL parsing |
| Permission denied errors | Verify private key has correct format and not corrupted |

## Verification (Merchant Side)

Merchants should validate:

1. **Headers present**: `Signature-Input` and `Signature` exist
2. **Timestamps valid**: `created ≤ now ≤ expires`, gap ≤ 480 seconds
3. **Nonce unique**: No duplicate nonces within 8-minute window
4. **Public key available**: Retrieve from `https://mcp.visa.com/.well-known/jwks`
5. **Signature verifies**: Reconstruct base string and verify signature

## Testing

### Enable Debug Logging

```bash
RUST_LOG=debug ./target/release/mcp-client-http
```

### Test Without TAP (No Credentials)

```bash
./target/release/mcp-client-http
# [TAP] TAP signature generation disabled (no credentials in environment)
```

### Test With TAP (Credentials Set)

```bash
export ED25519_PRIVATE_KEY="..."
export TAP_KEY_ID="..."
./target/release/mcp-client-http
# [TAP] ✓ TAP signature generation enabled
```

## References

- **RFC 9421**: https://datatracker.ietf.org/doc/html/rfc9421
- **Visa TAP Specs**: https://developer.visa.com/capabilities/trusted-agent-protocol/trusted-agent-protocol-specifications
- **Ed25519**: https://ed25519.cr.yp.to/
- **Visa Key Store**: https://mcp.visa.com/.well-known/jwks

## Architecture

```
┌─────────────────────────────────────────────┐
│          Agent A (agent-a)                  │
│                                             │
│  ┌──────────────────────────────────────┐   │
│  │  HTTP Server (axum)                  │   │
│  │  - POST /chat endpoint               │   │
│  │  - GET /health endpoint              │   │
│  └──────────────────────────────────────┘   │
│                    │                        │
│                    ├─ TapConfig             │
│                    │  └─ Extension Layer    │
│                    │                        │
│                    ├─ TapSignature Module   │
│                    │  ├─ create_signature() │
│                    │  ├─ parse_url()        │
│                    │  └─ sign_message()     │
│                    │                        │
│                    ├─ TapHttpClient         │
│                    │  ├─ tap_get()          │
│                    │  ├─ tap_post()         │
│                    │  └─ Builder            │
│                    │                        │
│                    └─ Environment Variables │
│                       ├─ ED25519_PRIVATE_KEY
│                       ├─ TAP_KEY_ID         │
│                       └─ TAP_ALGORITHM      │
└─────────────────────────────────────────────┘
         │
         │ HTTP Requests with TAP Headers
         ▼
┌─────────────────────────────────────────────┐
│  Merchant Edge/CDN (Cloudflare)             │
│  - Validate Signature-Input                 │
│  - Verify signature with public key         │
│  - Check timestamp and nonce                │
│  - Forward if valid (403 if invalid)        │
└─────────────────────────────────────────────┘
         │
         │ Trusted request
         ▼
┌─────────────────────────────────────────────┐
│  Agent B / Merchant Server                  │
│  - Process authenticated request            │
│  - Fulfill booking, process payment, etc.   │
└─────────────────────────────────────────────┘
```

## Next Steps

1. ✅ Load TAP credentials in environment
2. ✅ Start HTTP server
3. ✅ Verify TAP signatures in logs
4. ⏳ Test with merchant endpoint
5. ⏳ Configure Cloudflare validation rules
6. ⏳ Deploy to production

For detailed instructions, see [TAP_INTEGRATION_GUIDE.md](./TAP_INTEGRATION_GUIDE.md)
