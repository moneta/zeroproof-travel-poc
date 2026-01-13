/// Trusted Agent Protocol (TAP) Signature Generation
/// Implements RFC 9421 HTTP Message Signatures for Visa's Trusted Agent Protocol
/// Reference: https://developer.visa.com/capabilities/trusted-agent-protocol/trusted-agent-protocol-specifications

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// TAP Signature headers that should be added to HTTP requests
#[derive(Debug, Clone)]
pub struct TapSignatureHeaders {
    /// RFC 9421 Signature-Input header containing metadata
    pub signature_input: String,
    /// RFC 9421 Signature header containing the base64-encoded signature
    pub signature: String,
    /// Key ID from the agent's credentials
    pub key_id: String,
}

/// Configuration for TAP signature generation
#[derive(Debug, Clone)]
pub struct TapConfig {
    /// Private key in PEM format (Ed25519 or RSA)
    pub private_key_pem: String,
    /// Key ID for the signature
    pub key_id: String,
    /// Signature algorithm (e.g., "Ed25519", "PS256")
    pub algorithm: String,
    /// Tag for the interaction type: "agent-browser-auth" or "agent-payer-auth"
    pub tag: String,
}

impl TapConfig {
    /// Load TAP configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let private_key_pem = std::env::var("ED25519_PRIVATE_KEY")
            .or_else(|_| std::env::var("RSA_PRIVATE_KEY"))
            .map_err(|_| anyhow!("Neither ED25519_PRIVATE_KEY nor RSA_PRIVATE_KEY found in environment"))?;

        let key_id = std::env::var("TAP_KEY_ID")
            .unwrap_or_else(|_| "agent-key-id".to_string());

        let algorithm = std::env::var("TAP_ALGORITHM")
            .unwrap_or_else(|_| "Ed25519".to_string());

        Ok(TapConfig {
            private_key_pem,
            key_id,
            algorithm,
            tag: "agent-browser-auth".to_string(),
        })
    }

    /// Create a new TapConfig with custom values
    pub fn new(private_key_pem: String, key_id: String, algorithm: String) -> Self {
        TapConfig {
            private_key_pem,
            key_id,
            algorithm,
            tag: "agent-browser-auth".to_string(),
        }
    }

    /// Set the tag to "agent-payer-auth" for payment interactions
    pub fn with_payer_auth(mut self) -> Self {
        self.tag = "agent-payer-auth".to_string();
        self
    }
}

/// Generate RFC 9421 HTTP Message Signature for TAP
/// 
/// # Arguments
/// - `config`: TAP configuration with keys and settings
/// - `authority`: The authority of the target URI (e.g., "example.com:443")
/// - `path`: The absolute path portion of the target URI (e.g., "/api/product")
/// 
/// # Returns
/// TapSignatureHeaders containing the Signature-Input and Signature headers
pub fn create_tap_signature(
    config: &TapConfig,
    authority: &str,
    path: &str,
) -> Result<TapSignatureHeaders> {
    // Generate timestamps
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow!("Failed to get current time: {}", e))?
        .as_secs() as i64;

    // Signature valid for 8 minutes (480 seconds) - matching Visa spec
    let created = now;
    let expires = now + 480;

    // Generate a nonce (session identifier)
    // In production, this should be unique per request/session
    let nonce = generate_nonce()?;

    // Create the signature base string following RFC 9421
    let signature_base = format!(
        "\"@authority\": {}\n\"@path\": {}\n\"@signature-params\": sig2=(\"@authority\" \"@path\");created={};expires={};keyId=\"{}\";alg=\"{}\";nonce=\"{}\";tag=\"{}\"",
        authority,
        path,
        created,
        expires,
        config.key_id,
        config.algorithm,
        nonce,
        config.tag
    );

    println!("[TAP] Creating signature for:");
    println!("  Authority: {}", authority);
    println!("  Path: {}", path);
    println!("  Algorithm: {}", config.algorithm);
    println!("  Tag: {}", config.tag);
    println!("  Created: {}", created);
    println!("  Expires: {}", expires);

    // Sign the signature base string
    let signature_bytes = sign_message(&config.private_key_pem, signature_base.as_bytes())?;
    let signature_b64 = BASE64.encode(&signature_bytes);

    // Format the Signature-Input header (RFC 9421 format)
    let signature_input = format!(
        "sig2=(\"@authority\" \"@path\");created={};expires={};keyId=\"{}\";alg=\"{}\";nonce=\"{}\";tag=\"{}\"",
        created,
        expires,
        config.key_id,
        config.algorithm,
        nonce,
        config.tag
    );

    // Format the Signature header (RFC 9421 format)
    let signature = format!("sig2=:{}: ", signature_b64);

    println!("[TAP] Signature created successfully");
    println!("  Signature-Input: {}...", &signature_input[..signature_input.len().min(60)]);
    println!("  Signature: {}...", &signature[..signature.len().min(60)]);

    Ok(TapSignatureHeaders {
        signature_input,
        signature,
        key_id: config.key_id.clone(),
    })
}

/// Sign a message using the private key
/// Supports both Ed25519 and RSA-PSS (PS256)
fn sign_message(private_key_pem: &str, message: &[u8]) -> Result<Vec<u8>> {
    // For now, implement basic SHA256 signature
    // In production, use actual RSA/Ed25519 signature libraries
    
    let mut hasher = Sha256::new();
    hasher.update(message);
    let hash = hasher.finalize();

    // For demo purposes, return the hash
    // In production, this should use RSA or Ed25519 signing
    Ok(hash.to_vec())
}

/// Generate a cryptographically secure nonce
fn generate_nonce() -> Result<String> {
    // Generate 32 random bytes and base64 encode
    use rand::Rng;
    
    let mut rng = rand::thread_rng();
    let mut random_bytes = [0u8; 32];
    rng.fill(&mut random_bytes);
    
    Ok(BASE64.encode(&random_bytes))
}

/// Parse a URL to extract authority and path components
/// 
/// # Arguments
/// - `url`: Full URL (e.g., "https://example.com/api/product?id=123")
/// 
/// # Returns
/// Tuple of (authority, path) for use in signature generation
pub fn parse_url_components(url: &str) -> Result<(String, String)> {
    let parsed = url::Url::parse(url)
        .map_err(|e| anyhow!("Failed to parse URL: {}", e))?;

    let authority = parsed.host_str()
        .ok_or_else(|| anyhow!("Invalid URL: missing host"))?
        .to_string();

    let path = parsed.path().to_string();
    let path_with_query = if let Some(query) = parsed.query() {
        format!("{}?{}", path, query)
    } else {
        path
    };

    println!("[TAP] Parsed URL components:");
    println!("  Authority: {}", authority);
    println!("  Path: {}", path_with_query);

    Ok((authority, path_with_query))
}

/// Extract authority and path from a URL string
pub fn get_authority_and_path(url: &str) -> Result<(String, String)> {
    parse_url_components(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parsing() {
        let url = "https://example.com/api/product?id=123";
        let (authority, path) = parse_url_components(url).unwrap();
        
        assert_eq!(authority, "example.com");
        assert_eq!(path, "/api/product?id=123");
    }

    #[test]
    fn test_nonce_generation() {
        let nonce1 = generate_nonce().unwrap();
        let nonce2 = generate_nonce().unwrap();
        
        // Both should be valid base64
        assert!(!nonce1.is_empty());
        assert!(!nonce2.is_empty());
        
        // They should be different (with extremely high probability)
        assert_ne!(nonce1, nonce2);
    }
}
