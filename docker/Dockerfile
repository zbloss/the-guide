# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:1.75-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release --bin guide-api

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/guide-api /usr/local/bin/guide-api

# Data directory for SQLite
RUN mkdir -p /app/data

EXPOSE 8000

ENV GUIDE__SERVER__HOST=0.0.0.0
ENV GUIDE__SERVER__PORT=8000
ENV GUIDE__DATABASE__URL=sqlite:///app/data/guide.db
ENV GUIDE__QDRANT__URL=http://qdrant:6334
ENV GUIDE__LLM__OLLAMA_BASE_URL=http://ollama:11434/v1

CMD ["guide-api"]
