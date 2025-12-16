use axum::{
    extract::{Multipart, DefaultBodyLimit},
    routing::post,
    Json, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use serde::Serialize;
use sp1_sdk::{ProverClient, SP1ProvingKey, SP1VerifyingKey, SP1Stdin, HashableKey};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use uuid::Uuid;
use zk_protocol::{AttestRequest, AttestResponse};

type ElfStore = HashMap<String, Vec<u8>>; // program_id → ELF bytes
type KeyCache = HashMap<String, (SP1ProvingKey, SP1VerifyingKey)>; // program_id → (pk, vk)

static STORE: Lazy<Arc<RwLock<ElfStore>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
static KEY_CACHE: Lazy<Arc<RwLock<KeyCache>>> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

// Simple error wrapper for better error responses
struct AppError(String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.0).into_response()
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError(err)
    }
}

#[derive(Serialize)]
struct RegisterResponse {
    program_id: String,
    registered_at: String,
}

// POST /register-elf  ← called by Agent B on startup
async fn register_elf(mut multipart: Multipart) -> Result<Json<RegisterResponse>, AppError> {
    let mut elf_bytes: Option<Vec<u8>> = None;

    // Read all multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        eprintln!("✗ Multipart next_field error: {}", e);
        AppError(format!("Multipart error: {}", e))
    })? {
        let field_name = field.name().map(|s| s.to_string());
        let file_name = field.file_name().map(|s| s.to_string());
        
        println!("📦 Received field: {:?}, filename: {:?}", field_name, file_name);
        
        if field_name.as_deref() == Some("elf") {
            // Read the entire field as bytes
            let bytes = field.bytes().await.map_err(|e| {
                eprintln!("✗ Failed to read field bytes: {}", e);
                AppError(format!("Failed to read ELF bytes: {}", e))
            })?;
            
            println!("✓ Read ELF file: {} bytes", bytes.len());
            elf_bytes = Some(bytes.to_vec());
            break; // Got what we need, stop reading
        }
    }

    let elf = elf_bytes.ok_or_else(|| {
        eprintln!("✗ No ELF file found in multipart request");
        AppError("ELF file required but not found in request".to_string())
    })?;
    
    let program_id = Uuid::new_v4().to_string();

    {
        let mut store = STORE.write().unwrap();
        store.insert(program_id.clone(), elf);
    }

    println!("✓ ELF registered with program_id: {}", program_id);

    Ok(Json(RegisterResponse {
        program_id: program_id.clone(),
        registered_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }))
}

// POST /attest  ← called by Agent A
async fn attest(
    Json(payload): Json<AttestRequest>,
) -> Json<AttestResponse> {
    let prover = ProverClient::from_env();
    let program_id = &payload.program_id;

    // 1. Fetch the pre-registered ELF
    let elf = {
        let store = STORE.read().unwrap();
        store.get(program_id)
            .expect("Unknown program_id")
            .clone()
    };

    // 2. Get or compute pk and vk (cached after first setup)
    let (pk, vk) = {
        let mut cache = KEY_CACHE.write().unwrap();
        
        if let Some((cached_pk, cached_vk)) = cache.get(program_id) {
            // Cache hit: use cached keys
            println!("✓ Using cached keys for program_id: {}", program_id);
            (cached_pk.clone(), cached_vk.clone())
        } else {
            // Cache miss: compute keys and store in cache
            println!("⚙ Computing keys for program_id: {} (will be cached)", program_id);
            let (new_pk, new_vk) = prover.setup(&elf);
            cache.insert(program_id.clone(), (new_pk.clone(), new_vk.clone()));
            (new_pk, new_vk)
        }
    };

    // 3. Compute VK hash for on-chain verification (stateless universal verifier pattern)
    // SP1 uses bytes32() to hash the VK, which is passed to verifyProof() each time
    // NO storage on-chain needed - contracts are stateless!
    let vk_hash = vk.bytes32();  // 32-byte hash of the VK (already has 0x prefix)
    let vk_hash_str = vk_hash.to_string();

    println!("✓ Verifying Key Hash: {}", vk_hash_str);
    println!("  (Pass this to SP1VerifierGroth16.verifyProof() on-chain)");

    // 4. Create stdin with the input
    // Input is already bincode-serialized by the agent
    let mut stdin = SP1Stdin::new();
    stdin.write_vec(payload.input_bytes.clone());

    // 5. Generate Groth16 proof (SNARK-wrapped for on-chain compatibility)
    // Groth16: (~100k gas on-chain, uses GPU acceleration if available)
    // Alternative: .plonk() (~300k gas, const-size proof)
    let proof = prover
        .prove(&pk, &stdin)
        .groth16()  // Wraps STARK in Groth16 for on-chain verification
        .run()
        .expect("Proving failed");

    // 6. Optional: Verify proof locally before returning
    // - If verify_locally=true (default): Verify proof in attester (safe, adds 2-3s)
    // - If verify_locally=false: Skip verification (fast, Agent A verifies on-chain)
    if payload.verify_locally {
        println!("⚙ Verifying proof locally in attester...");
        prover.verify(&proof, &vk)
            .expect("Verification failed");
        println!("✓ Local verification passed");
    } else {
        println!("⊘ Skipping local verification (Agent A will verify on-chain)");
    }

    // 7. Extract public values and proof bytes
    let actual_output = payload.claimed_output.unwrap_or_else(|| serde_json::json!({}));
    let public_values_bytes = proof.public_values.as_slice();

    // proof.bytes() returns [vkey_hash[..4], proof_bytes]
    // The contract expects proofBytes to START with the first 4 bytes of the verifier hash
    // So we use proof.bytes() as-is (it already has the correct format)
    let proof_bytes = proof.bytes();

    Json(AttestResponse {
        proof: hex::encode(proof_bytes),
        public_values: hex::encode(public_values_bytes),
        vk_hash: vk_hash_str,  // Include VK hash for on-chain verification
        verified_output: actual_output,
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/register-elf", post(register_elf))
        .route("/attest", post(attest))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024)); // 20MB limit for ELF files

    println!("ZK Attester running → http://0.0.0.0:8000");
    println!("   POST /register-elf   ← Agent B calls this once");
    println!("   POST /attest        ← Agent A calls this");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("Failed to bind to 0.0.0.0:8000");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}