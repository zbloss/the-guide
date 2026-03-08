# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**The Guide** is an AI-powered assistant for Dungeon Masters running D&D campaigns. The backend is written in Rust with a fully validated test suite (55 tests, zero clippy lints).

## Tech Stack & Architectural Decisions

- **Language:** Rust stable 1.82+
- **Build:** `cargo build`, `cargo test`, `cargo run -p guide-api`
- **HTTP framework:** axum 0.8 + tokio 1
- **Models:** `serde::{Serialize, Deserialize}` derived structs in `guide-core`
- **Config:** `config` crate + `dotenvy`, `GUIDE__` prefix (flat `AppConfig`)
- **Database:** sqlx 0.8 SQLite with `sqlx::migrate!()`, WAL mode, FK enforcement
- **LLM SDK:** `async-openai` 0.27 pointed at Ollama `/v1` endpoint
- **Default model:** configurable via `GUIDE__DEFAULT_MODEL` (Ollama)
- **PDF Processing:** pdfium-render 0.8 + GLM-OCR vision, lopdf text fallback
- **RAG:** Qdrant (optional) + PageIndex disk JSON fallback at `data/indexes/`
- **Embeddings:** `nomic-embed-text` via Ollama, 768-dim, `guide_chunks` collection

## Key Constraints

- All LLM interactions via `async-openai` Client pointing at Ollama or cloud endpoint.
- PDF ingestion uses pdfium-render (requires `pdfium.dll`/`libpdfium.so` at runtime) + GLM-OCR.
- PageIndex trees stored at `data/indexes/{campaign_id}/{doc_id}.json` (campaign) or `data/indexes/global/{doc_id}.json` (rulebooks).
- All DB access via sqlx repositories (no ORM). Repositories use `&SqlitePool` references.
- Qdrant is optional: set `GUIDE__QDRANT_URL` to connect; omit to disable vector search.
- Tests use in-memory SQLite via `guide_db::init_sqlite(":memory:")` — no server required.
- `AppState.llm` is `Arc<dyn LlmClient>` (not `Arc<LlmRouter>`) for trait method dispatch.

## Commands

```bash
cargo build                                  # build all crates
cargo test --workspace                       # run all 55 tests
cargo test -p guide-combat                   # run single crate tests
cargo clippy --workspace -- -D warnings      # lint (must be zero errors)
cargo run -p guide-api                       # start dev server (port 8000)
```

## Workspace Structure

```
crates/
  guide-core/     domain models, config, errors (no I/O deps)
  guide-combat/   CombatEngine, initiative helpers
  guide-llm/      LlmClient trait, OllamaProvider, LlmRouter, prompts
  guide-db/       sqlx repositories + Qdrant helpers + migrations/
  guide-pdf/      pdfium extractor, chunker, ingest pipeline
  guide-api/      axum HTTP server (lib + bin), routes, AppState
crates/guide-db/migrations/   001–006 SQL schema files
data/indexes/                 PageIndex JSON trees (runtime)
```

## Test Files

```
crates/guide-combat/tests/combat_tests.rs       11 tests — CombatEngine
crates/guide-db/tests/repository_tests.rs       12 tests — sqlx::test macro
crates/guide-pdf/tests/chunker_tests.rs          9 tests — chunker unit tests
crates/guide-api/tests/api_tests.rs             23 tests — tower::oneshot + MockLlm
```

## Feature Areas

1. Combat Management (initiative, action economy, state tracking)
2. Campaign Intelligent Assistant (context-aware Q&A, SSE streaming)
3. Backstory & Character Integration (LLM-extracted plot hooks)
4. Meaningful Encounter Generation (RAG-grounded, party-aware)
5. Rulebook & Mechanics Reference (D&D 5e core)
6. PDF Parsing & Campaign Portability (pdfium → PageIndex)
7. Playstyle Personalization (PlaystyleProfile)
8. World Building & Narrative Tools
9. Session Summaries (tiered: player recaps vs. DM master logs)

## Agent Files

- `.claude/agents/rust-backend-architect.md` — infrastructure, DB, LLM routing
- `.claude/agents/dnd-pipeline-engineer.md` — pdfium + PageIndex pipeline
- `.claude/agents/dnd5e-systems-engineer.md` — combat engine, D&D mechanics
- `.claude/agents/partner-dm-narrator.md` — RAG Q&A, SSE chat, session summaries

The Python `src/` directory remains for reference — delete once E2E validation is complete.
