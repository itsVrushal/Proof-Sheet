#!/usr/bin/env bash
set -e

# 1. Ensure Rust is in PATH (Vercel build image already has Rust installed at /rust/bin)
export PATH="/rust/bin:$HOME/.cargo/bin:$PATH"

echo "Using Rust:"
rustc --version
cargo --version

# 2. Add WebAssembly target
rustup target add wasm32-unknown-unknown

# 3. Download statically linked Trunk binary (musl) into /tmp/bin
# Using musl avoids glibc version mismatches on Vercel's Amazon Linux container
mkdir -p /tmp/bin
curl -sSfL https://github.com/trunk-rs/trunk/releases/download/v0.22.1/trunk-x86_64-unknown-linux-musl.tar.gz | tar -xzf- -C /tmp/bin
export PATH="/tmp/bin:$PATH"

echo "Using Trunk:"
trunk --version

# 4. Build release static files into dist/
trunk build --release
