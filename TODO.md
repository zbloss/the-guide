# TODO.md — The Guide: Agent Progress Tracker

> This file is the single source of truth for AI agent coordination across
> context resets. Update **Current Focus** at the start/end of every session.

---

## Current Focus

- **Phase:** 4 — PDF Ingestion ✅ — ALL PLANNED PHASES COMPLETE
- **Last session:** 2026-03-05
- **Next action:** Secondary tasks — persist PlaystyleProfile to SQLite (migration 002), load & update from session events. Add `config.toml.example`.

---

## Phase Status

- [x] **Phase 0** — Skeleton (workspace, stubs, guide-core models, /health)
- [x] **Phase 1** — Database Foundation (repository tests, character CRUD, session CRUD, session events)
- [x] **Phase 2** — LLM Layer (`/chat` RAG endpoint with Qdrant + perspective filter + LLM)
- [x] **Phase 3** — Combat System (EncounterRepository, full combat lifecycle, 11 tests)
- [x] **Phase 4** — PDF Ingestion Pipeline (PDF bytes → GLM-OCR → Qdrant; no pdfium needed)
- [x] **Phase 5** — RAG + Intelligence (backstory analysis, session summaries, LocalWithFallback, OpenAI/Gemini cloud)
- [x] **Phase 6** — Encounter Generation + Personalization (RAG-grounded, party-aware, PlaystyleProfile)

---

## Completed Tasks

| Date       | Task |
|------------|------|
| 2026-03-05 | Created workspace Cargo.toml with 6 members |
| 2026-03-05 | Implemented guide-core: AppConfig, all models, GuideError |
| 2026-03-05 | Implemented guide-llm: LlmClient trait, OllamaProvider, LlmRouter |
| 2026-03-05 | Implemented guide-db: SQLite pool init, migrations, CampaignRepository |
| 2026-03-05 | Implemented guide-db Qdrant: try_connect, create/delete collection helpers |
| 2026-03-05 | Implemented guide-combat: CombatEngine, initiative rolling/sorting |
| 2026-03-05 | Implemented guide-pdf: stub (Phase 4 placeholder) |
| 2026-03-05 | Implemented guide-api: main.rs, AppState, /health, /version, /campaigns CRUD |
| 2026-03-05 | Created .env.example, docker/Dockerfile, docker/docker-compose.yml |
| 2026-03-05 | Created 001_initial.sql schema (all tables) |
| 2026-03-05 | Fixed: sqlx macro → query(), create_if_missing(true), axum {id} syntax, CompletionUsage, VectorParamsBuilder |
| 2026-03-05 | Verified: cargo build --release ✅, /health ✅, /version ✅, /campaigns CRUD ✅ |
| 2026-03-05 | Phase 1: CharacterRepository CRUD + /campaigns/{id}/characters routes |
| 2026-03-05 | Phase 1: SessionRepository + SessionEventRepository + /campaigns/{id}/sessions routes |
| 2026-03-05 | Phase 1: 12 integration tests pass (campaigns, characters, sessions, events, spoiler filter) |
| 2026-03-05 | Changed default port from 3000 → 8000 |
| 2026-03-05 | Fixed config loading: set_default() in builder so partial env-var overrides work |
| 2026-03-05 | Phase 2: /campaigns/{id}/chat — embed → Qdrant RAG (is_player_visible filter) → LLM |
| 2026-03-05 | Phase 3: EncounterRepository + encounter/combat routes (start/next-turn/update/end) |
| 2026-03-05 | Phase 3: 11 combat unit tests (initiative sort, HP damage, defeat, conditions, rounds) |
| 2026-03-05 | Changed default port to 8000 |
| 2026-03-05 | Phase 5: analyze-backstory — LLM extracts PlotHooks/motivations/secrets as JSON |
| 2026-03-05 | Phase 5: session summary — tiered DM/player generation via LLM |
| 2026-03-05 | Phase 5: document upload stubs (multipart) + ingest stub (202 Accepted, Phase 4 pending) |
| 2026-03-05 | Phase 5: OpenAICloudProvider + LocalWithFallback + LlmRouter::from_config |
| 2026-03-05 | Phase 5: prompts.rs with system prompt templates for all LLM tasks |
| 2026-03-05 | Phase 6: PlaystyleProfile model + GeneratedEncounter model |
| 2026-03-05 | Phase 6: POST /campaigns/{id}/encounters/generate — RAG + party state + LLM |
| 2026-03-05 | cargo build --release ✅ — all 23 tests pass |

---

## Blocked / Decisions Needed

_(none currently)_

---

## Architecture Decisions Log

| Decision | Rationale |
|----------|-----------|
| `async-openai` for all LLM calls | Single SDK covers Ollama, OpenAI, and Gemini (OpenAI-compatible); keeps LLM abstraction thin |
| Qdrant optional at startup | `guide_db::qdrant::try_connect` returns `Option<Qdrant>`; routes needing Qdrant return 503 if None |
| SQLite for relational state | Simple deployment (single file), sufficient for game state scale |
| One Qdrant collection per campaign | Clean isolation; delete campaign → drop collection |
| Spoiler prevention: dual-layer | Qdrant filter (`is_player_visible: true`) + LLM system prompt — defense in depth |
| PDF ingestion via vision API | No pdfium-free text extraction; GLM-OCR handles layout/tables better than text-strip libs |
| `config` crate + env vars | `GUIDE__` prefix; `config.toml` optional; graceful default fallback |

---

## Known Issues / Tech Debt

- PlaystyleProfile not persisted to SQLite yet — uses in-memory defaults. Needs migration 002 + repository.
- Encounter generation: PlaystyleProfile not loaded from past session data yet.
- Anthropic cloud provider not implemented — only OpenAI/Gemini (both OpenAI-compatible).
- PDF ingestion (Phase 4): pdfium native binary needed; download from https://github.com/bblanchon/pdfium-binaries/releases.
- No auth/rate-limiting middleware yet.
- Vision message format in `ollama.rs` uses a JSON string for content rather than structured `ChatCompletionRequestUserMessageContent` — should be updated when async-openai exposes the enum properly.
- `pdfium-render` is commented out in guide-pdf until Phase 4 and the native binary is available.
- No authentication/authorization layer yet.
- No rate limiting or request validation middleware.

---

## File Map

```
the-guide/
├── Cargo.toml                              Workspace root
├── .env.example                            Environment variable template
├── TODO.md                                 This file
├── docker/
│   ├── Dockerfile
│   └── docker-compose.yml
└── crates/
    ├── guide-core/src/
    │   ├── lib.rs
    │   ├── config.rs                       AppConfig (server/db/qdrant/llm)
    │   ├── error.rs                        GuideError + Result alias
    │   └── models/
    │       ├── mod.rs                      Shared enums (GameSystem, Condition, etc.)
    │       ├── campaign.rs                 Campaign, WorldState
    │       ├── character.rs                Character, AbilityScores, Backstory, PlotHook
    │       ├── encounter.rs                Encounter, CombatParticipant, ActionBudget
    │       ├── session.rs                  Session, SessionEvent, SessionSummary
    │       └── document.rs                 CampaignDocument, ExtractedLore
    ├── guide-llm/src/
    │   ├── lib.rs
    │   ├── client.rs                       LlmClient trait, request/response types, LlmTask
    │   ├── ollama.rs                       OllamaProvider (async-openai, custom base_url)
    │   └── router.rs                       LlmRouter + RoutingStrategy
    ├── guide-db/src/
    │   ├── lib.rs                          init_sqlite(), run_migrations()
    │   ├── campaigns.rs                    CampaignRepository CRUD
    │   ├── characters.rs                   STUB (Phase 3)
    │   ├── qdrant.rs                       try_connect, create/delete campaign collection
    │   └── migrations/001_initial.sql      Full schema
    ├── guide-pdf/src/lib.rs                STUB (Phase 4)
    ├── guide-combat/src/
    │   ├── lib.rs                          CombatEngine, build_participant
    │   └── initiative.rs                   roll_initiative, sort_initiative
    └── guide-api/src/
        ├── main.rs                         tokio::main, config load, server bind
        ├── state.rs                        AppState (config, llm, db, qdrant)
        └── routes/
            ├── mod.rs                      all_routes() merger
            ├── health.rs                   GET /health, GET /version
            └── campaigns.rs               CRUD /campaigns, /campaigns/:id
```
