#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== GlobalTelco Local Build & Verify ==="
echo ""
echo "Note: Fly.io and Vercel deploy automatically on push to main."
echo "This script builds everything locally to verify before pushing."
echo ""

# 1. Build and test Rust
echo "[1/4] Building Rust workspace..."
cargo build --release
cargo build --release --bin gt-server --features postgres
echo "       Running tests..."
cargo test

# 2. Build WASM module
echo "[2/4] Building WASM module..."
wasm-pack build "$ROOT/crates/gt-wasm" --target web --out-dir ../../web/src/lib/wasm/pkg

# 3. Build frontend
echo "[3/4] Building frontend..."
cd "$ROOT/web" && bun install && bun run build
cd "$ROOT"

# 4. Docker image (optional, verifies Fly.io build will work)
echo "[4/4] Building Docker image..."
docker build -f crates/gt-server/Dockerfile -t globaltelco-server .

echo ""
echo "=== All builds passed ==="
echo "Push to main to deploy: git push origin main"
echo "Tag for desktop release: git tag v0.x.x && git push --tags"
