#!/bin/bash
set -e

# -------------------------------
# Step 0: Define paths
# -------------------------------
SP1_HOME="$HOME/.sp1"
PROJECT_DIR="$HOME/zeroproof-travel-poc/agents"
SP1_REPO="$HOME/sp1"

# -------------------------------
# Step 1: Clean previous SP1 installs
# -------------------------------
echo "Cleaning old SP1 installs..."
rm -rf "$SP1_HOME"
rm -rf "$SP1_REPO"
rustup toolchain remove succinct 2>/dev/null || true
cd "$PROJECT_DIR"
rm -f rust-toolchain rust-toolchain.toml
rustup override unset || true

# -------------------------------
# Step 2: Install SP1-compatible Rust
# -------------------------------
echo "Installing Rust 1.91.1..."
rustup install 1.91.1

# Link it as 'succinct' toolchain
echo "Linking succinct to Rust 1.91.1..."
rustup toolchain link succinct ~/.rustup/toolchains/1.91.1-x86_64-unknown-linux-gnu
rustup default succinct

# Verify Rust version
echo "Rust version for succinct:"
rustc +succinct --version

# -------------------------------
# Step 3: Clone SP1 repo
# -------------------------------
echo "Cloning SP1 repo..."
git clone https://github.com/succinctlabs/sp1.git "$SP1_REPO"

# Checkout tag compatible with Rust 1.91.1
git -C "$SP1_REPO" checkout v5.2.3

# -------------------------------
# Step 4: Install SP1 CLI (cargo-prove)
# -------------------------------
echo "Installing SP1 CLI..."
cd "$SP1_REPO/crates/cli"
cargo install --locked --force --path .

# Ensure cargo-prove is on PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Verify cargo-prove
cargo-prove prove --version

# -------------------------------
# Step 5: Build SP1 toolchain
# -------------------------------
echo "Building SP1 toolchain..."
cargo-prove prove build-toolchain

# FIX
cd "$SP1_REPO"
rustup toolchain remove succinct
rustup toolchain link succinct ~/.rustup/toolchains/1.91.1-x86_64-unknown-linux-gnu

# END FIX

# -------------------------------
# Step 6: Configure project to use succinct
# -------------------------------
cd "$PROJECT_DIR"
echo "[toolchain]
channel = \"succinct\"
components = [\"llvm-tools\", \"rustc-dev\"]" > rust-toolchain

rustup override set succinct

# Verify everything
echo "Rust version in project:"
rustc --version
rustc +succinct --version

# -------------------------------
# Step 7: Build, prove, verify agent_b
# -------------------------------
echo "Building agent_b..."
cargo-prove prove build --bin agent_b
echo "Generating proof..."
cargo-prove prove prove --bin agent_b
echo "Verifying proof..."
cargo-prove prove verify --bin agent_b

echo "✅ SP1 environment is ready and agent_b successfully built, proved, and verified!"
