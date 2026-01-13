# TAP Protocol - Code Examples

This document provides practical code examples for implementing and testing TAP signatures in Agent A.

## Table of Contents

1. [Environment Setup](#environment-setup)
2. [Basic Usage](#basic-usage)
3. [HTTP Server Integration](#http-server-integration)
4. [Custom HTTP Requests](#custom-http-requests)
5. [Error Handling](#error-handling)
6. [Testing](#testing)

## Environment Setup

### Load Credentials from Environment

```rust
use mcp_client::TapConfig;

// Automatic loading from environment
let tap_config = TapConfig::from_env().ok();

if let Some(config) = &tap_config {
    println!("✓ TAP enabled with key ID: {}", config.key_id);
} else {
    println!("✗ TAP disabled (no credentials in environment)");
}
```

### .env File Format

```env
# .env
ED25519_PRIVATE_KEY=-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg
[base64 encoded key content]
-----END PRIVATE KEY-----

TAP_KEY_ID=poqkLGiymh_W0uP6PZFw-dvez3QJT5SolqXBCW38r0U
TAP_ALGORITHM=Ed25519
```

### Load from File

```rust
use mcp_client::TapConfig;
use std::env;

fn load_tap_config() -> anyhow::Result<TapConfig> {
    let private_key = std::fs::read_to_string("private-key.pem")?;
    let key_id = env::var("TAP_KEY_ID")?;
    
    Ok(TapConfig::new(
        private_key,
        key_id,
        "Ed25519".to_string()
    ))
}
```

## Basic Usage

### 1. Parse URL and Extract Components

```rust
use mcp_client::parse_url_components;

let url = "https://dev.agentb.zeroproofai.com/api/products?id=123&size=M";

match parse_url_components(url) {
    Ok((authority, path)) => {
        println!("Authority: {}", authority);      // dev.agentb.zeroproofai.com
        println!("Path: {}", path);                // /api/products?id=123&size=M
    }
    Err(e) => {
        eprintln!("Failed to parse URL: {}", e);
    }
}
```

### 2. Create TAP Signature

```rust
use mcp_client::{TapConfig, create_tap_signature, parse_url_components};

async fn create_signature_example() -> anyhow::Result<()> {
    // Load config
    let config = TapConfig::from_env()?;
    
    // Parse URL
    let url = "https://dev.agentb.zeroproofai.com/api/products";
    let (authority, path) = parse_url_components(url)?;
    
    // Create signature
    let headers = create_tap_signature(&config, &authority, &path)?;
    
    println!("Signature-Input: {}", headers.signature_input);
    println!("Signature: {}", headers.signature);
    println!("Key ID: {}", headers.key_id);
    
    Ok(())
}
```

### 3. Add Headers to HTTP Request

```rust
use mcp_client::parse_url_components;
use mcp_client::create_tap_signature;
use reqwest::Client;

async fn make_signed_request(
    url: &str,
    tap_config: &Option<TapConfig>
) -> anyhow::Result<String> {
    let client = Client::new();
    
    // Build request
    let mut request = client.get(url);
    
    // Add TAP headers if configured
    if let Some(config) = tap_config {
        let (authority, path) = parse_url_components(url)?;
        let sig_headers = create_tap_signature(config, &authority, &path)?;
        
        request = request
            .header("signature-input", sig_headers.signature_input)
            .header("signature", sig_headers.signature)
            .header("key-id", sig_headers.key_id);
        
        println!("[TAP] Added signature headers to request");
    }
    
    // Send request
    let response = request.send().await?;
    let body = response.text().await?;
    
    Ok(body)
}
```

## HTTP Server Integration

### 1. HTTP Server with TAP Config Extension

```rust
use axum::{
    extract::Json,
    response::IntoResponse,
    routing::post,
    Router,
    http::StatusCode,
};
use mcp_client::TapConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    session_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    success: bool,
    response: Option<String>,
}

// Handler with TAP config extension
async fn chat(
    axum::extract::Extension(tap_config): axum::extract::Extension<Option<TapConfig>>,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    // TAP config available here for making signed requests
    
    if let Some(_) = &tap_config {
        println!("[TAP] TAP signatures will be generated for outgoing requests");
    }
    
    // Process chat message
    let response = format!("Received: {}", payload.message);
    
    (
        StatusCode::OK,
        Json(ChatResponse {
            success: true,
            response: Some(response),
        }),
    )
}

#[tokio::main]
async fn main() {
    // Load TAP config
    let tap_config = TapConfig::from_env().ok();
    
    if let Some(ref config) = tap_config {
        println!("[TAP] ✓ TAP enabled: {}", config.key_id);
    } else {
        println!("[TAP] TAP disabled");
    }
    
    // Build router
    let app = Router::new()
        .route("/chat", post(chat))
        .layer(axum::extract::Extension(tap_config));
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .unwrap();
    
    println!("Server running on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}
```

### 2. Handler Making Signed Requests

```rust
use mcp_client::{tap_get, TapConfig};

async fn product_handler(
    axum::extract::Extension(tap_config): axum::extract::Extension<Option<TapConfig>>,
) -> impl IntoResponse {
    // Make signed request to merchant endpoint
    let merchant_url = "https://dev.agentb.zeroproofai.com/api/products";
    
    match tap_get(merchant_url, &tap_config).await {
        Ok(response) => {
            println!("[SUCCESS] Got response from merchant: {} bytes", response.len());
            axum::response::Json(serde_json::json!({
                "success": true,
                "data": response
            }))
        }
        Err(e) => {
            println!("[ERROR] Request failed: {}", e);
            axum::response::Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            }))
        }
    }
}
```

## Custom HTTP Requests

### 1. Using TapHttpRequestBuilder

```rust
use mcp_client::{TapHttpRequestBuilder, TapConfig};

async fn custom_request_example(
    tap_config: &Option<TapConfig>
) -> anyhow::Result<()> {
    let builder = TapHttpRequestBuilder::new(tap_config.clone());
    
    // GET request
    let get_response = builder
        .get_with_signature("https://api.example.com/products")
        .await?;
    
    println!("GET Response: {}", get_response);
    
    // POST request
    let post_body = r#"{"name": "John", "email": "john@example.com"}"#.to_string();
    let post_response = builder
        .post_with_signature(
            "https://api.example.com/checkout",
            Some(post_body)
        )
        .await?;
    
    println!("POST Response: {}", post_response);
    
    Ok(())
}
```

### 2. Manual Request Construction

```rust
use mcp_client::{parse_url_components, create_tap_signature, TapConfig};
use reqwest::Client;

async fn manual_request(
    url: &str,
    tap_config: &Option<TapConfig>
) -> anyhow::Result<String> {
    let client = Client::new();
    
    // Parse URL
    let (authority, path) = parse_url_components(url)?;
    
    // Create signature
    let sig_headers = if let Some(config) = tap_config {
        Some(create_tap_signature(config, &authority, &path)?)
    } else {
        None
    };
    
    // Build request
    let mut request = client.get(url);
    
    // Add headers
    if let Some(sig) = sig_headers {
        request = request
            .header("signature-input", sig.signature_input)
            .header("signature", sig.signature)
            .header("key-id", sig.key_id)
            .header("user-agent", "Agent A/1.0");
    }
    
    // Send
    let response = request.send().await?;
    Ok(response.text().await?)
}
```

### 3. Browser Authentication vs. Payer Authentication

```rust
use mcp_client::{TapConfig, create_tap_signature, parse_url_components};

async fn browsing_request(
    url: &str,
    config: &TapConfig
) -> anyhow::Result<()> {
    // For browsing (product lookup, availability check)
    let mut config = config.clone();
    config.tag = "agent-browser-auth".to_string();
    
    let (auth, path) = parse_url_components(url)?;
    let headers = create_tap_signature(&config, &auth, &path)?;
    
    println!("Browsing request tag: {}", config.tag);
    // Make request with headers
    Ok(())
}

async fn payment_request(
    url: &str,
    config: &TapConfig
) -> anyhow::Result<()> {
    // For payments
    let mut config = config.clone();
    config.tag = "agent-payer-auth".to_string();
    
    let (auth, path) = parse_url_components(url)?;
    let headers = create_tap_signature(&config, &auth, &path)?;
    
    println!("Payment request tag: {}", config.tag);
    // Make request with headers
    Ok(())
}
```

## Error Handling

### 1. Graceful Fallback

```rust
use mcp_client::tap_get;

async fn make_request_with_fallback(url: &str) -> anyhow::Result<String> {
    // Try to load TAP config (optional)
    let tap_config = std::env::var("ED25519_PRIVATE_KEY").ok();
    
    let config = if tap_config.is_some() {
        TapConfig::from_env().ok()
    } else {
        None
    };
    
    if config.is_some() {
        println!("Making signed request...");
        tap_get(url, &config).await
    } else {
        println!("TAP not configured, making unsigned request...");
        
        // Fallback to unsigned request
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        Ok(response.text().await?)
    }
}
```

### 2. Detailed Error Messages

```rust
use anyhow::anyhow;

async fn request_with_diagnostics(
    url: &str,
    tap_config: &Option<TapConfig>
) -> anyhow::Result<String> {
    // Step 1: Parse URL
    let (authority, path) = parse_url_components(url)
        .map_err(|e| anyhow!("URL parsing failed: {}. URL was: {}", e, url))?;
    
    // Step 2: Create signature
    let sig_headers = if let Some(config) = tap_config {
        create_tap_signature(config, &authority, &path)
            .map_err(|e| anyhow!("Signature generation failed: {}. Config key ID: {}", e, config.key_id))?
    } else {
        return Err(anyhow!("TAP config not available"));
    };
    
    // Step 3: Make request
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("signature-input", sig_headers.signature_input)
        .header("signature", sig_headers.signature)
        .send()
        .await
        .map_err(|e| anyhow!("HTTP request failed for {}: {}", url, e))?;
    
    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(anyhow!(
            "HTTP request returned {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ))
    }
}
```

## Testing

### 1. Unit Test for URL Parsing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mcp_client::parse_url_components;
    
    #[test]
    fn test_url_parsing() {
        let url = "https://dev.agentb.zeroproofai.com/api/products?id=123&category=flights";
        let (authority, path) = parse_url_components(url).unwrap();
        
        assert_eq!(authority, "dev.agentb.zeroproofai.com");
        assert_eq!(path, "/api/products?id=123&category=flights");
    }
    
    #[test]
    fn test_url_parsing_with_port() {
        let url = "https://example.com:8443/api/checkout";
        let (authority, path) = parse_url_components(url).unwrap();
        
        assert_eq!(authority, "example.com:8443");
        assert_eq!(path, "/api/checkout");
    }
}
```

### 2. Integration Test

```rust
#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_signed_request() {
    let config = TapConfig::from_env().ok();
    
    // Skip test if no credentials
    if config.is_none() {
        println!("Skipping test: No TAP credentials available");
        return;
    }
    
    let response = tap_get(
        "https://httpbin.org/get",
        &config
    ).await;
    
    assert!(response.is_ok(), "Request should succeed");
    let body = response.unwrap();
    assert!(!body.is_empty(), "Response should not be empty");
}
```

### 3. Manual Testing with curl

```bash
#!/bin/bash

# Set up
export ED25519_PRIVATE_KEY=$(cat private-key.pem)
export TAP_KEY_ID="your-key-id"

# Start server
cargo run --bin mcp-client-http &
SERVER_PID=$!

sleep 2

# Test health endpoint
echo "Testing /health endpoint..."
curl -v http://localhost:3001/health

# Test chat endpoint
echo "Testing /chat endpoint..."
curl -X POST http://localhost:3001/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "What flights are available?",
    "session_id": "test-session-1"
  }'

# Cleanup
kill $SERVER_PID
```

### 4. Debug Logging

```bash
# Enable full debug output
RUST_LOG=debug cargo run --bin mcp-client-http

# Expected output:
# [TAP] Creating signature for:
#   Authority: dev.agentb.zeroproofai.com
#   Path: /api/products
#   Algorithm: Ed25519
#   Created: 1735689600
#   Expires: 1735693200
# [TAP] Signature created successfully
```

## Complete Example: Mini Agent

```rust
use mcp_client::{TapConfig, tap_post};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load TAP credentials
    let tap_config = TapConfig::from_env().ok();
    
    println!("🤖 Mini Agent starting...");
    println!("   TAP enabled: {}", tap_config.is_some());
    
    // Example: Make booking request
    let booking_url = "https://dev.agentb.zeroproofai.com/api/bookings";
    
    let booking_data = json!({
        "passenger": "John Doe",
        "origin": "NYC",
        "destination": "LAX",
        "date": "2025-01-15"
    }).to_string();
    
    println!("\n📤 Sending booking request...");
    match tap_post(booking_url, booking_data, &tap_config).await {
        Ok(response) => {
            println!("✓ Booking successful!");
            println!("Response: {}", response);
        }
        Err(e) => {
            println!("✗ Booking failed: {}", e);
        }
    }
    
    Ok(())
}
```

## Performance Testing

```bash
#!/bin/bash

# Signature generation performance test
time for i in {1..1000}; do
    cargo run --release << 'EOF'
        use mcp_client::{TapConfig, create_tap_signature, parse_url_components};
        
        #[tokio::main]
        async fn main() -> anyhow::Result<()> {
            let config = TapConfig::from_env()?;
            let (authority, path) = parse_url_components("https://example.com/api/test")?;
            let _sig = create_tap_signature(&config, &authority, &path)?;
            Ok(())
        }
EOF
done

# Expected: ~1-2ms per signature
```

This collection of examples covers all major usage patterns for implementing and testing TAP protocol support in Agent A.
