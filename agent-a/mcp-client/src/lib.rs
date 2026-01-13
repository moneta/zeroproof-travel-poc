/// Agent A MCP Client library
/// Exposes core orchestration logic for reuse in CLI and HTTP server modes

pub mod orchestration;
pub mod proxy_fetch;
pub mod tap_signature;
pub mod tap_http_client;

pub use orchestration::{AgentConfig, BookingState, ClaudeMessage, process_user_query};
pub use tap_signature::{TapConfig, TapSignatureHeaders, create_tap_signature, parse_url_components};
pub use tap_http_client::{TapHttpRequestBuilder, tap_get, tap_post};


