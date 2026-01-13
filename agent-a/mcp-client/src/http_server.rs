/// HTTP Server wrapper for Agent A MCP Client
/// Exposes the orchestration logic as REST API endpoints
/// Allows web interfaces to interact with the agent
/// Includes Visa Trusted Agent Protocol (TAP) signature support for RFC 9421 compliance

use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use mcp_client::{AgentConfig, BookingState, ClaudeMessage, process_user_query, tap_signature};
use tap_signature::{TapConfig, TapSignatureHeaders};

/// Session data storing conversation history and booking state
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    messages: Vec<ClaudeMessage>,
    state: BookingState,
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            state: BookingState::default(),
        }
    }
}

/// Session manager for storing conversation state across requests
type SessionManager = Arc<Mutex<HashMap<String, SessionData>>>;

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Chat request from frontend
#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    #[serde(default)]
    session_id: Option<String>,
}

/// Chat response to frontend
#[derive(Debug, Serialize)]
struct ChatResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
}

/// TAP Signature configuration wrapper
struct TapSignatureManager {
    config: Option<TapConfig>,
}

/// Health check endpoint
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: "0.1.0".to_string(),
    })
}

/// Generate TAP signature headers for a given URL
/// This should be called before making HTTP requests to authenticated endpoints
fn get_tap_signature_headers(url: &str, tap_config: &Option<TapConfig>) -> Option<TapSignatureHeaders> {
    match tap_config {
        Some(config) => {
            match tap_signature::parse_url_components(url) {
                Ok((authority, path)) => {
                    match tap_signature::create_tap_signature(config, &authority, &path) {
                        Ok(headers) => {
                            println!("[TAP] Generated signature headers for URL: {}", url);
                            Some(headers)
                        }
                        Err(e) => {
                            println!("[TAP ERROR] Failed to create signature: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    println!("[TAP ERROR] Failed to parse URL: {}", e);
                    None
                }
            }
        }
        None => {
            println!("[TAP] TAP signature generation is disabled");
            None
        }
    }
}

/// Main chat endpoint with session-based conversation management
async fn chat(
    axum::extract::Extension(sessions): axum::extract::Extension<SessionManager>,
    axum::extract::Extension(tap_config): axum::extract::Extension<Option<TapConfig>>,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    let session_id = payload.session_id.clone().unwrap_or_else(|| {
        format!("sess_{}", uuid::Uuid::new_v4())
    });
    
    println!("[CHAT] Incoming request - SessionID: {}, Message length: {}", 
             session_id, payload.message.len());
    
    // Log TAP signature status for this request
    if tap_config.is_some() {
        println!("[TAP] TAP signatures will be generated for outgoing requests from this session");
    }
    
    // Initialize config
    let config = match AgentConfig::from_env() {
        Ok(cfg) => {
            println!("[CONFIG] Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            println!("[ERROR] Configuration failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChatResponse {
                    success: false,
                    response: None,
                    error: Some(format!("Configuration error: {}", e)),
                    session_id: Some(session_id),
                }),
            );
        }
    };

    // Lock session manager and get/create session data
    let mut sessions_lock = sessions.lock().await;
    let mut session = sessions_lock
        .get(&session_id)
        .cloned()
        .unwrap_or_default();
    
    println!("[SESSION] Retrieved session - State: {}, Messages: {}", 
             session.state.step, session.messages.len());

    // Process the user query with full conversation history and state
    println!("[PROCESSING] Starting orchestration - User message: '{}'", &payload.message[..payload.message.len().min(100)]);
    
    match process_user_query(&config, &payload.message, &session.messages, &mut session.state, &session_id).await {
        Ok((response, updated_messages, updated_state)) => {
            // Update session with new messages and state
            let message_count = updated_messages.len();
            session.messages = updated_messages;
            session.state = updated_state.clone();
            sessions_lock.insert(session_id.clone(), session);
            
            println!("[SUCCESS] Request processed - SessionID: {}, New state: {}, Messages: {}", 
                     session_id, updated_state.step, message_count);
            
            (
                StatusCode::OK,
                Json(ChatResponse {
                    success: true,
                    response: Some(response),
                    error: None,
                    session_id: Some(session_id),
                }),
            )
        }
        Err(e) => {
            println!("[ERROR] Processing failed - SessionID: {}, Error: {}", session_id, e);
            (
                StatusCode::BAD_REQUEST,
                Json(ChatResponse {
                    success: false,
                    response: None,
                    error: Some(format!("Error processing request: {}", e)),
                    session_id: Some(session_id),
                }),
            )
        }
    }
}


#[tokio::main]
async fn main() {
    // Load .env file
    let _ = dotenv::dotenv();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║     Agent A - HTTP API Server (Claude-powered Agent)       ║");
    println!("║    With Session-based Conversation Management              ║");
    println!("║    With Visa TAP (RFC 9421) Signature Support              ║");
    println!("║          Connects to Agent A & B MCP Servers               ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Get port from environment or use default
    let port = std::env::var("AGENT_A_HTTP_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);
    
    println!("[INIT] Server configuration:");
    println!("  Port: {}", port);
    
    if let Ok(agent_b_url) = std::env::var("AGENT_B_MCP_URL") {
        println!("  Agent B URL: {}", agent_b_url);
    }
    
    if let Ok(payment_url) = std::env::var("PAYMENT_AGENT_URL") {
        println!("  Payment Agent URL: {}", payment_url);
    }

    // Initialize TAP configuration (optional, fails gracefully if not configured)
    let tap_config = TapConfig::from_env().ok();
    if tap_config.is_some() {
        println!("[TAP] ✓ TAP signature generation enabled");
        println!("  Algorithm: Ed25519");
        println!("  Key ID: {}", tap_config.as_ref().unwrap().key_id);
    } else {
        println!("[TAP] TAP signature generation disabled (no credentials in environment)");
    }

    // Create session manager
    let sessions: SessionManager = Arc::new(Mutex::new(HashMap::new()));
    println!("[SESSION] Session manager initialized");

    // Build router with session manager and TAP config extensions
    let app = Router::new()
        .route("/health", get(health))
        .route("/chat", post(chat))
        .layer(CorsLayer::permissive())
        .layer(axum::extract::Extension(sessions))
        .layer(axum::extract::Extension(tap_config));

    // Create listener
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Failed to bind listener");

    println!("[STARTUP] ✓ Agent A HTTP Server running on http://0.0.0.0:{}", port);
    println!("  POST /chat     — Send a message (session-based conversation)");
    println!("  GET  /health   — Check server health\n");

    // Run server
    if let Err(e) = axum::serve(listener, app).await {
        println!("[FATAL] Server failed: {}", e);
    }
}
