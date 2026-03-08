# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1-slim AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && rm -rf /var/lib/apt/lists/*
# Copy workspace manifest and strip the Tauri frontend member (not needed server-side)
COPY Cargo.toml Cargo.lock ./
RUN sed -i '/guide-frontend/d' Cargo.toml
COPY crates/ crates/
RUN cargo build --release -p guide-api

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/guide-api /app/guide-api
COPY crates/guide-db/migrations/ /app/migrations/

RUN mkdir -p data/uploads data/indexes
VOLUME ["/app/data"]
EXPOSE 8000

CMD ["/app/guide-api"]
