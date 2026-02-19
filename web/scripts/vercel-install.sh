#!/bin/sh
set -e
export RUSTUP_HOME=/rust CARGO_HOME=/rust

rustup target add wasm32-unknown-unknown

# Install wasm-pack 0.14.0 (prebuilt binary)
WP_VERSION=0.14.0
WP_URL="https://github.com/rustwasm/wasm-pack/releases/download/v${WP_VERSION}/wasm-pack-v${WP_VERSION}-x86_64-unknown-linux-musl.tar.gz"
if curl -fsSL "$WP_URL" -o /tmp/wasm-pack.tar.gz; then
  tar xzf /tmp/wasm-pack.tar.gz -C /tmp
  cp "/tmp/wasm-pack-v${WP_VERSION}-x86_64-unknown-linux-musl/wasm-pack" /rust/bin/wasm-pack
  chmod +x /rust/bin/wasm-pack
  rm -rf /tmp/wasm-pack*
else
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

bun install
