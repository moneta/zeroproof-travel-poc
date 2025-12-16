use sp1_sdk::{ProverClient, SP1Stdin};

fn main() {
    println!("Building Groth16 circuit files...");
    println!("This will take 30-60 minutes on first run.");
    
    let client = ProverClient::new();
    
    // Trigger circuit build by attempting a Groth16 proof setup
    // This will download and build the circuit artifacts
    sp1_sdk::install::install_circuit_artifacts();
    
    println!("✓ Groth16 circuit artifacts installed");
}
