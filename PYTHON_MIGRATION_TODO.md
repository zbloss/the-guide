# Python Migration TODO

Tracks completion status of the Rust → Python rewrite.

## Phase Status

| Phase | Description                                                   | Status  |
| ----- | ------------------------------------------------------------- | ------- |
| 1     | Project scaffolding (pyproject.toml, UV, directory structure) | ✅ Done |
| 2     | Core models (Pydantic v2)                                     | ✅ Done |
| 3     | Config (pydantic-settings)                                    | ✅ Done |
| 4     | Database layer (aiosqlite repositories)                       | ✅ Done |
| 5     | LLM layer (OllamaProvider, CloudProvider, LlmRouter, prompts) | ✅ Done |
| 6     | PDF ingestion (Docling + PageIndex)                           | ✅ Done |
| 7     | Combat engine (CombatEngine, initiative)                      | ✅ Done |
| 8     | FastAPI routes (all endpoints)                                | ✅ Done |
| 9     | Tests (combat, repositories, API layer)                       | ✅ Done |
| 10    | Agent file updates                                            | ✅ Done |

## To Do Before Production

- [ ] Install dependencies: `uv sync`
- [ ] Run tests: `uv run pytest` — must pass 100%
- [ ] Start server: `uv run uvicorn guide.api.main:app --reload`
- [ ] Verify `GET /health` returns 200
- [ ] Upload a PDF and trigger ingestion
- [ ] Test `POST /campaigns/{id}/chat`
- [ ] Remove Rust `crates/` directory once all tests pass

## Key Decisions

| Decision                        | Rationale                                                   |
| ------------------------------- | ----------------------------------------------------------- |
| PageIndex replaces Qdrant       | Vectorless reasoning-based retrieval, no embedding overhead |
| Docling replaces GLM-OCR        | Native Python, structured output, no vision model required  |
| tomng/nanbeige4.1:3b as default | Strong reasoning in 3B params, Ollama-deployable            |
| aiosqlite over SQLAlchemy       | Minimal overhead, same SQL as Rust migration files          |
| FastAPI over Flask/Django       | Native async, Pydantic v2 native, automatic OpenAPI         |
| PageIndex indexes on disk       | No external vector DB dependency                            |

## File Map (Python)

```
src/guide/
  config.py               AppConfig (pydantic-settings)
  errors.py               GuideError hierarchy
  models/shared.py        All shared enums
  models/campaign.py      Campaign, Create/Update requests
  models/character.py     Character, Backstory, PlotHook
  models/session.py       Session, SessionEvent, Summary
  models/encounter.py     Encounter, CombatParticipant, ActionBudget
  models/document.py      CampaignDocument, GlobalDocument
  models/playstyle.py     PlaystyleProfile, GeneratedEncounter
  db/pool.py              init_db(), migrations
  db/campaigns.py         CampaignRepository
  db/characters.py        CharacterRepository
  db/sessions.py          SessionRepository, SessionEventRepository
  db/encounters.py        EncounterRepository
  db/documents.py         DocumentRepository, GlobalDocumentRepository
  db/migrations/          001_initial.sql, 002_document_kind.sql
  llm/client.py           LlmClient ABC, dataclasses
  llm/ollama.py           OllamaProvider
  llm/cloud.py            CloudProvider
  llm/router.py           LlmRouter
  llm/prompts.py          System prompt templates
  pdf/extractor.py        Docling extraction
  pdf/pipeline.py         PageIndex ingestion + query
  combat/initiative.py    roll_d20, roll_initiative, sort_initiative
  combat/engine.py        CombatEngine, build_participant
  api/state.py            AppState dataclass
  api/main.py             create_app() factory
  api/routes/health.py    GET /health, /version
  api/routes/campaigns.py Campaign CRUD
  api/routes/characters.py Character CRUD + backstory analysis
  api/routes/sessions.py  Session CRUD + events + summary
  api/routes/encounters.py Encounter lifecycle
  api/routes/generate.py  POST /encounters/generate
  api/routes/chat.py      POST /chat (RAG Q&A)
  api/routes/documents.py Document upload + ingestion
tests/
  conftest.py             in-memory DB, mock LLM, AsyncClient
  test_combat.py          11 combat unit tests
  test_repositories.py    12 DB integration tests
  test_api.py             HTTP layer tests
```
