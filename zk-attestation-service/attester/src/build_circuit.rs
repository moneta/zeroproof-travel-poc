use sp1_sdk::install;
use std::path::PathBuf;

fn main() {
    println!("🔧 Downloading Groth16 circuit files...");
    println!("⏱  This will download circuits (~13GB) on first use.");
    println!("");
    
    let circuit_dir = PathBuf::from(std::env::var("HOME").unwrap())
        .join(".sp1/circuits");
    
    println!("📁 Installing to: {}", circuit_dir.display());
    println!("⏳ Downloading circuit files...");
    println!("");
    
    install::install_circuit_artifacts(circuit_dir.clone(), "groth16");
    
    println!("");
    println!("✓ Groth16 circuit artifacts installed successfully!");
    println!("  Location: {}", circuit_dir.display());
}
