# Visa Trusted Agent Protocol (TAP) Integration Guide for Agent A

## Overview

Agent A now supports Visa's Trusted Agent Protocol (TAP) for RFC 9421 HTTP Message Signatures. This allows agent-a to cryptographically sign all outgoing HTTP requests to merchant endpoints, proving the request originated from a trusted, verified agent.

## Protocol Details

### RFC 9421 HTTP Message Signatures

TAP uses RFC 9421 standard for creating cryptographic signatures of HTTP requests. The signature covers critical components:

- **@authority**: The host portion of the request URL (e.g., `dev.agentb.zeroproofai.com`)
- **@path**: The path portion of the URL including query parameters
- **Timestamps**: `created` and `expires` for replay attack prevention (8-minute window)
- **Nonce**: Unique session identifier for additional security
- **Key ID**: Identifier of the cryptographic key used for signing
- **Algorithm**: Cryptographic algorithm (Ed25519 or RSA-PSS-SHA256)
- **Tag**: Interaction type (`agent-browser-auth` for browsing, `agent-payer-auth` for payments)

### Signature Flow

```
Agent A (agent-a)
    │
    ├─ Loads ED25519_PRIVATE_KEY or RSA_PRIVATE_KEY from environment
    │
    ├─ For each outgoing HTTP request:
    │  ├─ Parse URL to extract authority and path
    │  ├─ Create signature base string (RFC 9421 format)
    │  ├─ Sign with private key
    │  ├─ Encode signature in base64
    │  └─ Add headers:
    │     ├─ Signature-Input: sig2=(...signature metadata...)
    │     └─ Signature: sig2=:base64_encoded_signature:
    │
    └─ Send HTTP request with TAP headers
        │
        ▼
    Merchant Edge/CDN (Cloudflare)
        │
        ├─ Retrieve public key from https://mcp.visa.com/.well-known/jwks
        │
        ├─ Validate signature:
        │  ├─ Check timestamps (created < now < expires, gap ≤ 8 min)
        │  ├─ Check nonce uniqueness
        │  ├─ Reconstruct signature base string
        │  ├─ Verify signature with public key
        │  └─ Block if invalid (403/429 response)
        │
        └─ If valid: Forward request to Agent B with trust headers
            │
            ▼
        Agent B (MCP Server)
            │
            └─ Process authenticated request
```

## Environment Configuration

### Required Environment Variables

```bash
# TAP Signature Configuration
ED25519_PRIVATE_KEY="-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgZ...
-----END PRIVATE KEY-----"

# Optional: RSA Private Key (if not using Ed25519)
# RSA_PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----..."

# Key identifiers and configuration
TAP_KEY_ID="poqkLGiymh_W0uP6PZFw-dvez3QJT5SolqXBCW38r0U"
TAP_ALGORITHM="Ed25519"  # or "PS256" for RSA
```

### How to Set Environment Variables

**Option 1: .env file**
```bash
# .env
ED25519_PRIVATE_KEY="-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgZ...
-----END PRIVATE KEY-----"
TAP_KEY_ID="your-key-id"
TAP_ALGORITHM="Ed25519"
```

**Option 2: Docker/System Environment**
```bash
docker run \
  -e ED25519_PRIVATE_KEY="$YOUR_PRIVATE_KEY" \
  -e TAP_KEY_ID="your-key-id" \
  agent-a
```

**Option 3: Kubernetes Secret**
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: tap-credentials
type: Opaque
stringData:
  ED25519_PRIVATE_KEY: |
    -----BEGIN PRIVATE KEY-----
    MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgZ...
    -----END PRIVATE KEY-----
  TAP_KEY_ID: "poqkLGiymh_W0uP6PZFw-dvez3QJT5SolqXBCW38r0U"
  TAP_ALGORITHM: "Ed25519"
```

## Integration Points

### 1. HTTP Server Initialization

The HTTP server automatically:
- Loads TAP credentials from environment on startup
- Enables TAP signature generation if credentials are available
- Falls back gracefully if credentials are missing (warnings logged)
- Attaches TAP config to each request via Axum extensions

### 2. Using TAP Signatures in HTTP Clients

When making HTTP requests from agent-a to agent-b or merchant endpoints:

```rust
// Example: Making a signed HTTP request
use mcp_client::tap_signature;

async fn make_authenticated_request(url: &str, tap_config: &Option<TapConfig>) -> Result<()> {
    // Generate TAP signature headers
    let sig_headers = tap_signature::parse_url_components(url)
        .and_then(|(authority, path)| {
            tap_signature::create_tap_signature(
                &tap_config.as_ref().unwrap(),
                &authority,
                &path
            )
        })?;

    // Create HTTP client and add signature headers
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("signature-input", sig_headers.signature_input)
        .header("signature", sig_headers.signature)
        .header("key-id", sig_headers.key_id)
        .send()
        .await?;

    Ok(())
}
```

### 3. Endpoint Handler Example

For endpoints that need to make outgoing requests:

```rust
// In http_server.rs handlers
async fn some_handler(
    axum::extract::Extension(tap_config): axum::extract::Extension<Option<TapConfig>>,
    // ... other parameters
) -> impl IntoResponse {
    // When making requests to authenticated endpoints
    if let Some(headers) = get_tap_signature_headers(
        "https://dev.agentb.zeroproofai.com/api/products",
        &tap_config
    ) {
        // Add headers to your HTTP request:
        // - Signature-Input: headers.signature_input
        // - Signature: headers.signature
        // - Key-Id: headers.key_id
    }
}
```

## Signature Generation Details

### Creating the Signature Base String

The signature base string is created in RFC 9421 format:

```
"@authority": dev.agentb.zeroproofai.com
"@path": /api/products?id=123
"@signature-params": sig2=("@authority" "@path");created=1735689600;expires=1735693200;keyId="poqkLGiymh_W0uP6PZFw-dvez3QJT5SolqXBCW38r0U";alg="Ed25519";nonce="e8N7S2MFd/qrd6T2R3tdfAuuANngKI7LFtKYI/vowzk4lAZYadIX6wW25MwG7DCT9RUKAJ0qVkU0mEeLElW1qg==";tag="agent-browser-auth"
```

### Timestamps

- **created**: Unix timestamp when signature was created (seconds since epoch)
- **expires**: Unix timestamp when signature expires (created + 480 seconds)
- **Window**: Must be exactly 480 seconds (8 minutes) as per Visa spec

The receiving endpoint validates:
- `created` ≤ now
- `expires` ≥ now
- `expires - created` ≤ 480 seconds

### Nonce Generation

Nonces are cryptographically random 32-byte values, base64-encoded:
- Unique per request/session
- Prevents replay attacks
- Server must track nonces within the 8-minute validity window

### Algorithm Support

**Supported:**
- `Ed25519`: Preferred (faster, smaller keys)
- `PS256` / `RSA-PSS-SHA256`: Supported for RSA keys

**Selection:**
- Automatically detected from environment variable `TAP_ALGORITHM`
- Falls back to Ed25519 if not specified

## Merchant-Side Verification

Merchants receiving signed requests from agent-a should:

### Step 1: Extract Headers
```
Signature-Input: sig2=("@authority" "@path");created=1735689600;...
Signature: sig2=:jdq0SqOwHdyHr9+r5jw3iYZH6aNGKijYp/EstF4RQTQdi5N5YYKrD+mCT1HA1nZDsi6nJKuHxUi/5Syp3rLWBA==:
```

### Step 2: Validate Timestamps
- Check `created` and `expires` are within valid range
- Window should be exactly 480 seconds

### Step 3: Retrieve Public Key
```
GET https://mcp.visa.com/.well-known/jwks?kid=poqkLGiymh_W0uP6PZFw-dvez3QJT5SolqXBCW38r0U
```

### Step 4: Reconstruct Signature Base
```
"@authority": dev.agentb.zeroproofai.com
"@path": /api/products
"@signature-params": sig2=("@authority" "@path");created=1735689600;expires=1735693200;keyId="...";alg="Ed25519";nonce="...";tag="agent-browser-auth"
```

### Step 5: Verify Signature
Using the public key, verify the base64-decoded signature against the signature base string.

## Error Handling

### TAP Configuration Errors

If TAP credentials are missing, the server logs a warning but continues:
```
[TAP] TAP signature generation disabled (no credentials in environment)
```

### Signature Generation Failures

If signature generation fails for a specific request:
```
[TAP ERROR] Failed to create signature: [reason]
```

Requests can continue without signatures, or you can implement fallback behavior.

### Validation Errors (Merchant Side)

Merchants should implement these checks:

| Error | Cause | Action |
|-------|-------|--------|
| Missing headers | No Signature-Input header | Block (403) |
| Invalid timestamp | Outside valid window | Block (403) |
| Replay detected | Nonce already used | Block (429) |
| Invalid signature | Signature doesn't verify | Block (403) |
| Missing public key | Key retrieval failed | Block (503) |

## Security Considerations

### Key Management
- Keep `ED25519_PRIVATE_KEY` and `RSA_PRIVATE_KEY` secure
- Rotate keys periodically
- Use separate keys for different environments (dev/staging/prod)
- Never commit keys to version control

### Nonce Tracking
- Server must track nonces within the 8-minute window
- Use in-memory cache or Redis for nonce tracking
- Periodically clean up expired nonces

### Timestamp Validation
- Use consistent timezone (UTC/GMT required)
- Account for clock skew between systems (±30 seconds tolerance recommended)
- Monitor for time synchronization issues

### Request Integrity
- Signatures cover `@authority` and `@path`
- Optional: Extend signature to cover request body/method
- Consider additional headers for sensitive operations

## Testing

### Enable Debug Logging

```bash
RUST_LOG=debug ./target/release/mcp-client-http
```

This will output:
```
[TAP] Creating signature for:
  Authority: dev.agentb.zeroproofai.com
  Path: /api/products
  Algorithm: Ed25519
  Tag: agent-browser-auth
  Created: 1735689600
  Expires: 1735693200

[TAP] Signature created successfully
  Signature-Input: sig2=(...trimmed...)
  Signature: sig2=(...trimmed...)
```

### Test Endpoints

**Health Check** (no authentication needed):
```bash
curl http://localhost:3001/health
```

**Chat Endpoint** (TAP headers added automatically if configured):
```bash
curl -X POST http://localhost:3001/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "What flights are available to NYC on January 15?",
    "session_id": "sess_test_001"
  }'
```

### Verification with Commercial Tools

**Using httpie with signature headers:**
```bash
http --headers POST http://localhost:3001/chat \
  message="Test message" \
  session_id="sess_001"
```

## Troubleshooting

### Issue: "TAP signature generation disabled"
**Cause**: `ED25519_PRIVATE_KEY` or `RSA_PRIVATE_KEY` not in environment
**Solution**: Set the environment variable before starting the server

### Issue: "Failed to parse URL"
**Cause**: Invalid URL format
**Solution**: Verify URL is absolute (includes scheme and host)

### Issue: "Signature base string mismatch" (merchant side)
**Cause**: URL components extracted differently on merchant
**Solution**: Ensure identical URL parsing (include query parameters in path)

### Issue: Clock skew errors
**Cause**: Agent and merchant servers have different times
**Solution**: Synchronize clocks using NTP, or increase validation window tolerance

## References

- [RFC 9421 - HTTP Message Signatures](https://datatracker.ietf.org/doc/html/rfc9421)
- [Visa TAP Specifications](https://developer.visa.com/capabilities/trusted-agent-protocol/trusted-agent-protocol-specifications)
- [NIST FIPS 186-5 - Digital Signature Standards](https://csrc.nist.gov/publications/detail/fips/186/5/final)
- [Ed25519 Cryptography](https://ed25519.cr.yp.to/)

## Support

For TAP implementation questions:
1. Check the logs: Enable `RUST_LOG=debug` for detailed output
2. Validate configuration: Verify all environment variables are set
3. Test signature generation: Use test endpoints to verify signatures
4. Contact Visa TAP support for protocol clarifications
