use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub from: String,
    pub to: String,
    pub passenger_name: String,
    pub passenger_email: String,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub booking_id: String,
    pub status: String,
    pub confirmation_code: String,
}

/// Booking logic that runs both on server and inside SP1
/// NOTE: Inside SP1, external HTTP calls are not possible, so this will
/// return a deterministic result based on input. The server implementation
/// can override this to make real HTTP calls.
pub fn handle(req: Request) -> Response {
    // Deterministic booking logic for ZK proof
    // In SP1: generates deterministic booking based on inputs
    // On server: this can be overridden to call real booking API
    
    // Generate deterministic booking ID from request data
    let booking_data = alloc::format!(
        "{}-{}-{}-{}",
        req.from, req.to, req.passenger_name, req.passenger_email
    );
    
    // Simple hash-like transformation (deterministic)
    let booking_id = alloc::format!("BK{:08X}", booking_data.len() * 12345);
    let confirmation_code = alloc::format!("CONF{:06X}", booking_data.len() * 67890);

    Response {
        booking_id,
        status: String::from("confirmed"),
        confirmation_code,
    }
}
