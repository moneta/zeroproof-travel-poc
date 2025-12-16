/// General protocol for ZK attestation between agents
/// This library provides common types and serialization helpers
/// that any agent can use without depending on other agents' code.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request to the attester service to generate a ZK proof
#[derive(Serialize, Deserialize, Debug)]
pub struct AttestRequest {
    pub program_id: String,
    /// Input data as raw bytes (bincode-serialized)
    /// Will be passed to the zkVM program via stdin
    pub input_bytes: Vec<u8>,
    /// Expected output for verification (optional, format defined by agent)
    pub claimed_output: Option<Value>,
    /// Whether to verify the proof locally before returning
    #[serde(default = "default_verify")]
    pub verify_locally: bool,
}

fn default_verify() -> bool {
    true
}

/// Response from the attester service
#[derive(Serialize, Deserialize, Debug)]
pub struct AttestResponse {
    /// Hex-encoded Groth16 proof for on-chain verification
    pub proof: String,
    /// Public values committed by the zkVM program (hex-encoded)
    pub public_values: String,
    /// VK hash for on-chain verifier (bytes32)
    pub vk_hash: String,
    /// Output from the zkVM program
    pub verified_output: Value,
}

/// Response from an agent's pricing/booking endpoint
#[derive(Serialize, Deserialize, Debug)]
pub struct AgentResponse {
    /// Agent-specific response data (price, booking ID, etc.)
    #[serde(flatten)]
    pub data: Value,
    /// Program ID for ZK verification
    pub program_id: String,
    /// ELF hash of the zkVM program
    pub elf_hash: String,
}

/// Helper to serialize any serde-compatible type to bincode bytes
pub fn serialize_input<T: Serialize>(input: &T) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(input)
}

/// Helper to deserialize bincode bytes to any serde-compatible type
pub fn deserialize_output<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, bincode::Error> {
    bincode::deserialize(bytes)
}

/// Convert bincode bytes to JSON array format for HTTP transport
pub fn bytes_to_json_array(bytes: &[u8]) -> Value {
    Value::Array(bytes.iter().map(|b| Value::Number((*b).into())).collect())
}

/// Extract bytes from JSON array format
pub fn json_array_to_bytes(value: &Value) -> Option<Vec<u8>> {
    if let Value::Array(arr) = value {
        Some(arr.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
    } else {
        None
    }
}
