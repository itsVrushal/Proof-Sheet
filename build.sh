#!/usr/bin/env bash
set -e

# 1. Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 2. Add WebAssembly target
rustup target add wasm32-unknown-unknown

# 3. Download prebuilt Trunk binary (v0.22.1)
curl -sSfL https://github.com/trunk-rs/trunk/releases/download/v0.22.1/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf- -C "$HOME/.cargo/bin"

# 4. Build release static files into dist/
trunk build --release
