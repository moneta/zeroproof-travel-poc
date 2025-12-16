use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub from: String,
    pub to: String,
    pub vip: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub price: f64,
}

/// This function runs both on your server and inside SP1
/// → Zero duplication, 100% guaranteed correctness
pub fn handle(req: Request) -> Response {
    // ←←← YOUR REAL SECRET PRICING LOGIC (edit only here!) ←←←
    let base = if req.from == "NYC" && req.to == "LON" {
        680.0
    } else if req.from == "LON" && req.to == "NYC" {
        675.0
    } else {
        450.0
    };

    let price = if req.vip {
        base * 0.85
    } else {
        base
    };

    // You can add arbitrage checks, signature verification, etc.
    // As long as it uses only no_std-compatible code

    Response { price }
}
