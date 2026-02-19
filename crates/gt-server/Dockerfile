# Multi-stage build for gt-server
FROM rust:1.83-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests first for Docker layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build the server binary with postgres support
RUN cargo build --release --bin gt-server --features postgres

# ── Runtime stage ─────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/gt-server /usr/local/bin/gt-server

ENV GT_HOST=0.0.0.0
ENV GT_PORT=3001
ENV RUST_LOG=gt_server=info,tower_http=info

EXPOSE 3001

CMD ["gt-server"]
