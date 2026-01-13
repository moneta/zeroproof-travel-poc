/// TAP-enabled HTTP Client
/// Provides utilities for making HTTP requests with Visa TAP signature headers

use crate::tap_signature::{TapConfig, create_tap_signature, parse_url_components};
use anyhow::{anyhow, Result};
use reqwest::{Client, Request};

/// Builder for TAP-signed HTTP requests
pub struct TapHttpRequestBuilder {
    client: Client,
    tap_config: Option<TapConfig>,
}

impl TapHttpRequestBuilder {
    /// Create a new TAP HTTP request builder
    pub fn new(tap_config: Option<TapConfig>) -> Self {
        Self {
            client: Client::new(),
            tap_config,
        }
    }

    /// Add TAP signature headers to a request URL
    pub async fn get_with_signature(&self, url: &str) -> Result<String> {
        self.request_with_signature("GET", url, None).await
    }

    /// Add TAP signature headers to a POST request
    pub async fn post_with_signature(&self, url: &str, body: Option<String>) -> Result<String> {
        self.request_with_signature("POST", url, body).await
    }

    /// Generic request with TAP signature
    pub async fn request_with_signature(
        &self,
        method: &str,
        url: &str,
        body: Option<String>,
    ) -> Result<String> {
        // Parse URL components
        let (authority, path) = parse_url_components(url)?;

        // Create TAP signature headers if configured
        let mut request_builder = match method {
            "GET" => self.client.get(url),
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "DELETE" => self.client.delete(url),
            _ => return Err(anyhow!("Unsupported HTTP method: {}", method)),
        };

        // Add TAP signature headers if available
        if let Some(config) = &self.tap_config {
            match create_tap_signature(config, &authority, &path) {
                Ok(sig_headers) => {
                    request_builder = request_builder
                        .header("signature-input", &sig_headers.signature_input)
                        .header("signature", &sig_headers.signature)
                        .header("key-id", &sig_headers.key_id);
                    
                    println!("[TAP] Added signature headers to {} request to {}", method, url);
                }
                Err(e) => {
                    println!("[TAP WARN] Failed to create signature for {}: {}", url, e);
                    // Continue without signature
                }
            }
        } else {
            println!("[TAP] No TAP config available, sending unsigned request to {}", url);
        }

        // Add body if present
        if let Some(body_content) = body {
            request_builder = request_builder.body(body_content);
        }

        // Send request and get response
        let response = request_builder
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;

        println!(
            "[TAP] Response received - Status: {}, Length: {} bytes",
            status,
            response_text.len()
        );

        Ok(response_text)
    }
}

/// Helper function to make a TAP-signed GET request
pub async fn tap_get(url: &str, tap_config: &Option<TapConfig>) -> Result<String> {
    let builder = TapHttpRequestBuilder::new(tap_config.clone());
    builder.get_with_signature(url).await
}

/// Helper function to make a TAP-signed POST request
pub async fn tap_post(
    url: &str,
    body: String,
    tap_config: &Option<TapConfig>,
) -> Result<String> {
    let builder = TapHttpRequestBuilder::new(tap_config.clone());
    builder.post_with_signature(url, Some(body)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires valid credentials and external endpoint
    async fn test_tap_signed_request() {
        let config = TapConfig::from_env().ok();
        let response = tap_get("https://httpbin.org/get", &config).await;
        
        match response {
            Ok(body) => {
                println!("Got response: {}", body);
                assert!(!body.is_empty());
            }
            Err(e) => {
                println!("Request failed (expected if no TAP config): {}", e);
            }
        }
    }
}
