# TODO.md — The Guide: Agent Progress Tracker

> This file is the single source of truth for AI agent coordination across
> context resets. Update **Current Focus** at the start/end of every session.

---

## Current Focus

- **Phase:** 7 — Code Quality & Security Hardening (in progress)
- **Last session:** 2026-03-05
- **Next action:** Continue Phase 7 fixes — PlaystyleProfile persistence (migration 002), API integration tests, rate-limiting middleware.

---

## Phase Status

- [x] **Phase 0** — Skeleton (workspace, stubs, guide-core models, /health)
- [x] **Phase 1** — Database Foundation (repository tests, character CRUD, session CRUD, session events)
- [x] **Phase 2** — LLM Layer (`/chat` RAG endpoint with Qdrant + perspective filter + LLM)
- [x] **Phase 3** — Combat System (EncounterRepository, full combat lifecycle, 11 tests)
- [x] **Phase 4** — PDF Ingestion Pipeline (PDF bytes → GLM-OCR → Qdrant; no pdfium needed)
- [x] **Phase 5** — RAG + Intelligence (backstory analysis, session summaries, LocalWithFallback, OpenAI/Gemini cloud)
- [x] **Phase 6** — Encounter Generation + Personalization (RAG-grounded, party-aware, PlaystyleProfile)
- [ ] **Phase 7** — Code Quality & Security Hardening (input validation, error handling, config, serialization fixes)

---

## Completed Tasks

| Date       | Task                                                                                                         |
| ---------- | ------------------------------------------------------------------------------------------------------------ |
| 2026-03-05 | Created workspace Cargo.toml with 6 members                                                                  |
| 2026-03-05 | Implemented guide-core: AppConfig, all models, GuideError                                                    |
| 2026-03-05 | Implemented guide-llm: LlmClient trait, OllamaProvider, LlmRouter                                            |
| 2026-03-05 | Implemented guide-db: SQLite pool init, migrations, CampaignRepository                                       |
| 2026-03-05 | Implemented guide-db Qdrant: try_connect, create/delete collection helpers                                   |
| 2026-03-05 | Implemented guide-combat: CombatEngine, initiative rolling/sorting                                           |
| 2026-03-05 | Implemented guide-pdf: stub (Phase 4 placeholder)                                                            |
| 2026-03-05 | Implemented guide-api: main.rs, AppState, /health, /version, /campaigns CRUD                                 |
| 2026-03-05 | Created .env.example, docker/Dockerfile, docker/docker-compose.yml                                           |
| 2026-03-05 | Created 001_initial.sql schema (all tables)                                                                  |
| 2026-03-05 | Fixed: sqlx macro → query(), create_if_missing(true), axum {id} syntax, CompletionUsage, VectorParamsBuilder |
| 2026-03-05 | Verified: cargo build --release ✅, /health ✅, /version ✅, /campaigns CRUD ✅                              |
| 2026-03-05 | Phase 1: CharacterRepository CRUD + /campaigns/{id}/characters routes                                        |
| 2026-03-05 | Phase 1: SessionRepository + SessionEventRepository + /campaigns/{id}/sessions routes                        |
| 2026-03-05 | Phase 1: 12 integration tests pass (campaigns, characters, sessions, events, spoiler filter)                 |
| 2026-03-05 | Changed default port from 3000 → 8000                                                                        |
| 2026-03-05 | Fixed config loading: set_default() in builder so partial env-var overrides work                             |
| 2026-03-05 | Phase 2: /campaigns/{id}/chat — embed → Qdrant RAG (is_player_visible filter) → LLM                          |
| 2026-03-05 | Phase 3: EncounterRepository + encounter/combat routes (start/next-turn/update/end)                          |
| 2026-03-05 | Phase 3: 11 combat unit tests (initiative sort, HP damage, defeat, conditions, rounds)                       |
| 2026-03-05 | Changed default port to 8000                                                                                 |
| 2026-03-05 | Phase 5: analyze-backstory — LLM extracts PlotHooks/motivations/secrets as JSON                              |
| 2026-03-05 | Phase 5: session summary — tiered DM/player generation via LLM                                               |
| 2026-03-05 | Phase 5: document upload stubs (multipart) + ingest stub (202 Accepted, Phase 4 pending)                     |
| 2026-03-05 | Phase 5: OpenAICloudProvider + LocalWithFallback + LlmRouter::from_config                                    |
| 2026-03-05 | Phase 5: prompts.rs with system prompt templates for all LLM tasks                                           |
| 2026-03-05 | Phase 6: PlaystyleProfile model + GeneratedEncounter model                                                   |
| 2026-03-05 | Phase 6: POST /campaigns/{id}/encounters/generate — RAG + party state + LLM                                  |
| 2026-03-05 | cargo build --release ✅ — all 23 tests pass                                                                 |
| 2026-03-05 | Phase 7: Full codebase code review — 28 issues identified across all crates                                  |
| 2026-03-05 | Phase 7: Added max_upload_bytes to AppConfig + input validation on file upload                               |
| 2026-03-05 | Phase 7: Fixed silent data loss in document upload (unwrap_or_default → proper error)                        |
| 2026-03-05 | Phase 7: Added ingestion timeout (5 min) to background tokio::spawn task                                     |
| 2026-03-05 | Phase 7: Fixed silent participant skip in create_encounter (validate before insert)                          |
| 2026-03-05 | Phase 7: Added message length validation to /chat endpoint                                                   |
| 2026-03-05 | Phase 7: Fixed openai_cloud embed to use self.model as fallback instead of hardcoded string                  |
| 2026-03-05 | Phase 7: Updated TODO.md file map to reflect Phases 1-6 actual file structure                                |

---

## Blocked / Decisions Needed

_(none currently)_

---

## Architecture Decisions Log

| Decision                           | Rationale                                                                                          |
| ---------------------------------- | -------------------------------------------------------------------------------------------------- |
| `async-openai` for all LLM calls   | Single SDK covers Ollama, OpenAI, and Gemini (OpenAI-compatible); keeps LLM abstraction thin       |
| Qdrant optional at startup         | `guide_db::qdrant::try_connect` returns `Option<Qdrant>`; routes needing Qdrant return 503 if None |
| SQLite for relational state        | Simple deployment (single file), sufficient for game state scale                                   |
| One Qdrant collection per campaign | Clean isolation; delete campaign → drop collection                                                 |
| Spoiler prevention: dual-layer     | Qdrant filter (`is_player_visible: true`) + LLM system prompt — defense in depth                   |
| PDF ingestion via vision API       | No pdfium-free text extraction; GLM-OCR handles layout/tables better than text-strip libs          |
| `config` crate + env vars          | `GUIDE__` prefix; `config.toml` optional; graceful default fallback                                |

---

## Known Issues / Tech Debt

### Critical

- **Race condition on campaign creation** (`campaigns.rs:47`): Qdrant collection creation is fire-and-forget; a document upload immediately after create will get a 503. Fix: sync creation or add `qdrant_status` field + retry loop.
- **No campaign authorization checks**: Any caller knowing a campaign UUID can read/modify/delete it. All routes extract `_campaign_id` from path but never validate ownership.

### High

- **PlaystyleProfile not persisted** — uses in-memory defaults. Needs migration 002 + `PlaystyleProfileRepository`. `sessions_sampled` never incremented.
- **Encounter generation**: PlaystyleProfile not loaded from past session data yet.
- **No API-level integration tests** — 23 unit/repository tests pass, but all 11 route modules are untested at the HTTP layer.

### Medium

- **Enum serialization hacks in `sessions.rs`**: `EventType` and `EventSignificance` are stored as bare strings using `serde_json::to_string` + `trim_matches('"')`. Deserialization re-adds quotes. Fragile if variant names change.
- **N+1 query in `encounters.rs:59,75`**: `list_by_session` calls `list_participants` once per encounter. Replace with a batch JOIN or `IN (...)` query.
- **Silent `.unwrap_or_default()` on corrupt DB data**: `characters.rs:190`, `encounters.rs:237`, `campaigns.rs:144` — masks data corruption instead of surfacing errors.
- **`LocalWithFallback` falls back on all errors**: Should only fall back on connection/timeout errors, not logic errors (bad model name, invalid prompt).
- **No rate limiting or request size validation middleware** (`guide-api/src/main.rs`).
- **Hardcoded magic numbers**: RAG context limit (5), HNSW ef (64/128), max DB connections (10), default character speed (30) should be in `AppConfig`.
- **Anthropic cloud provider not implemented** — only OpenAI/Gemini (both OpenAI-compatible).

### Low

- **Vision message format** in `ollama.rs` and `openai_cloud.rs` hand-rolls JSON content array. Update when `async-openai` exposes `ChatCompletionRequestUserMessageContent` enum properly.
- **`OpenAICloudProvider` embed** uses hardcoded `"text-embedding-3-small"` fallback instead of `self.model`.
- **`_campaign_id` extracted but unused** in many handlers — document as future authorization hook or validate against loaded entity.
- **Qdrant point ID collision risk** (`qdrant.rs:94`): chunk UUIDs used directly as point IDs; should encode campaign ID.
- **No OpenAPI/Swagger schema** exported.
- **No soft delete** on campaigns/characters — deleted data is unrecoverable.
- **`TODO.md` file map was stale** (shown as Phase 0 skeleton) — updated below.

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
    │   ├── config.rs                       AppConfig (server/db/qdrant/llm + max_upload_bytes)
    │   ├── error.rs                        GuideError + Result alias
    │   └── models/
    │       ├── mod.rs                      Shared enums (GameSystem, Condition, Perspective, etc.)
    │       ├── campaign.rs                 Campaign, WorldState, Create/UpdateCampaignRequest
    │       ├── character.rs                Character, AbilityScores, Backstory, PlotHook
    │       ├── encounter.rs                Encounter, CombatParticipant, ActionBudget
    │       ├── session.rs                  Session, SessionEvent, SessionSummary
    │       ├── document.rs                 CampaignDocument, ExtractedLore, IngestionStatus
    │       └── playstyle.rs               PlaystyleProfile, GeneratedEncounter
    ├── guide-llm/src/
    │   ├── lib.rs
    │   ├── client.rs                       LlmClient trait, CompletionRequest/Response, LlmTask
    │   ├── ollama.rs                       OllamaProvider (async-openai + custom base_url)
    │   ├── openai_cloud.rs                 OpenAICloudProvider (OpenAI + Gemini)
    │   ├── prompts.rs                      System prompt templates (backstory, summary, etc.)
    │   └── router.rs                       LlmRouter + RoutingStrategy (LocalOnly/Cloud/Fallback)
    ├── guide-db/src/
    │   ├── lib.rs                          init_sqlite(), run_migrations()
    │   ├── campaigns.rs                    CampaignRepository CRUD
    │   ├── characters.rs                   CharacterRepository CRUD + set_backstory
    │   ├── sessions.rs                     SessionRepository + SessionEventRepository
    │   ├── encounters.rs                   EncounterRepository + participant persistence
    │   ├── documents.rs                    DocumentRepository (insert/get/list/update_status)
    │   ├── qdrant.rs                       try_connect, create/delete collection, upsert_lore_chunk
    │   ├── tests/repository_tests.rs       12 integration tests
    │   └── migrations/001_initial.sql      Full schema (all tables)
    ├── guide-pdf/src/lib.rs                PDF → GLM-OCR → Qdrant ingestion pipeline
    ├── guide-combat/src/
    │   ├── lib.rs                          CombatEngine, build_participant, HP/condition helpers
    │   ├── initiative.rs                   roll_d20, roll_initiative, sort_initiative
    │   └── tests/combat_tests.rs           11 unit tests
    └── guide-api/src/
        ├── main.rs                         tokio::main, AppState init, server bind
        ├── state.rs                        AppState (config, llm: Arc<dyn LlmClient>, db, qdrant)
        └── routes/
            ├── mod.rs                      all_routes() merger
            ├── health.rs                   GET /health, GET /version
            ├── campaigns.rs                CRUD /campaigns, /campaigns/{id}
            ├── characters.rs               CRUD + POST analyze-backstory (LLM)
            ├── sessions.rs                 CRUD + start/end + events + GET summary (LLM)
            ├── encounters.rs               create/start/next-turn/update-participant/end
            ├── generate.rs                 POST /campaigns/{id}/encounters/generate (RAG+LLM)
            ├── chat.rs                     POST /campaigns/{id}/chat (RAG Q&A)
            └── documents.rs               upload + list + get + POST ingest (async OCR, 202)
```
