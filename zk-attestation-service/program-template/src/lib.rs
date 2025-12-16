#![no_main]
sp1_zkvm::entrypoint!(main);

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct PriceRequest {
    pub from: String,
    pub to: String,
}

#[derive(Serialize)]
pub struct PriceResponse {
    pub price: f64,
    // they can add anything here
}

pub fn main() {
    let request: PriceRequest = sp1_zkvm::io::read();

    // ←←← THIS IS THEIR ORIGINAL CODE (they just paste it here) ←←←
    // Example: they can keep their full existing logic, even using std!
    let price = if request.from == "NYC" && request.to == "LON" {
        682.50
    } else {
        450.0
    };
    // ←←← END OF THEIR CODE ←←←

    let response = PriceResponse { price };
    sp1_zkvm::io::commit(&response);
}