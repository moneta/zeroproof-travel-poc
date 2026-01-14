/// Proof storage and retrieval module
/// Stores cryptographic proofs in an in-memory database
/// In production, this should use a persistent database like PostgreSQL

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Stored proof record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProof {
    pub proof_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub timestamp: u64,
    pub request: serde_json::Value,
    pub response: serde_json::Value,
    pub proof: serde_json::Value,
    pub verified: bool,
    pub onchain_compatible: bool,
    pub submitted_by: Option<String>, // Which agent submitted this proof (agent-a, agent-b, payment-agent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u32>, // Order in the workflow (1, 2, 3, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_proof_id: Option<String>, // Reference to parent/related proof (for dependency tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_stage: Option<String>, // e.g., "pricing", "payment_enrollment", "payment", "booking"
}

/// In-memory proof database
pub struct ProofDatabase {
    proofs: Arc<RwLock<HashMap<String, Vec<StoredProof>>>>, // session_id -> proofs
}

impl ProofDatabase {
    pub fn new() -> Self {
        Self {
            proofs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a proof in the database
    pub async fn store_proof(&self, proof: StoredProof) -> Result<String, String> {
        let mut db = self.proofs.write().await;
        
        let proof_id = proof.proof_id.clone();
        let session_id = proof.session_id.clone();
        
        db.entry(session_id)
            .or_insert_with(Vec::new)
            .push(proof);
        
        Ok(proof_id)
    }

    /// Retrieve all proofs for a session, sorted by timestamp
    pub async fn get_proofs(&self, session_id: &str) -> Result<Vec<StoredProof>, String> {
        let db = self.proofs.read().await;
        
        let mut proofs = db
            .get(session_id)
            .cloned()
            .unwrap_or_default();
        
        // Sort by timestamp to maintain chronological order
        proofs.sort_by_key(|p| p.timestamp);
        
        Ok(proofs)
    }

    /// Retrieve a specific proof by ID
    pub async fn get_proof(&self, proof_id: &str) -> Result<Option<StoredProof>, String> {
        let db = self.proofs.read().await;
        
        // Search through all sessions to find the proof
        for proofs in db.values() {
            for proof in proofs {
                if proof.proof_id == proof_id {
                    return Ok(Some(proof.clone()));
                }
            }
        }
        
        Ok(None)
    }

    /// Get proof count for a session
    pub async fn get_proof_count(&self, session_id: &str) -> Result<usize, String> {
        let db = self.proofs.read().await;
        Ok(db.get(session_id).map(|p| p.len()).unwrap_or(0))
    }

    /// Clear proofs for a session
    pub async fn clear_proofs(&self, session_id: &str) -> Result<usize, String> {
        let mut db = self.proofs.write().await;
        Ok(db.remove(session_id).map(|p| p.len()).unwrap_or(0))
    }
}

impl Clone for ProofDatabase {
    fn clone(&self) -> Self {
        Self {
            proofs: self.proofs.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve_proof() {
        let db = ProofDatabase::new();
        
        let proof = StoredProof {
            proof_id: "proof_1".to_string(),
            session_id: "session_1".to_string(),
            tool_name: "get-ticket-price".to_string(),
            timestamp: 1234567890,
            request: serde_json::json!({ "from": "NYC" }),
            response: serde_json::json!({ "price": 450 }),
            proof: serde_json::json!({ "verified": true }),
            verified: true,
            onchain_compatible: true,
            submitted_by: Some("agent-a".to_string()),
        };
        
        db.store_proof(proof.clone()).await.unwrap();
        
        let proofs = db.get_proofs("session_1").await.unwrap();
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].proof_id, "proof_1");
    }
}
