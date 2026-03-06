# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**The Guide** is an AI-powered assistant for Dungeon Masters running D&D campaigns. The backend has been fully migrated from Rust to Python (Phases 0–7 archived; Python implementation complete with 33 passing tests).

## Tech Stack & Architectural Decisions

- **Language:** Python 3.12+
- **Dependency management:** `uv` + `pyproject.toml`
- **HTTP framework:** FastAPI + uvicorn
- **Models:** Pydantic v2 `BaseModel`
- **Config:** `pydantic-settings` `BaseSettings` (`GUIDE__` prefix)
- **Database:** `aiosqlite` (SQLite, same schema as Rust era)
- **LLM SDK:** `openai.AsyncOpenAI` pointed at Ollama `/v1` endpoint
- **Default model:** `nanbeige4.1:3b` via Ollama
- **PDF Processing:** Docling (`DocumentConverter`) — no vision model required for well-formatted PDFs
- **RAG:** PageIndex (vectorless, LLM-reasoning retrieval) — disk-based JSON indexes at `data/indexes/`
- **No Qdrant** — replaced by PageIndex

## Key Constraints

- Use `openai.AsyncOpenAI` for all LLM interactions (including Ollama) — no provider-specific SDKs.
- PDF ingestion uses Docling, not vision models or pdfium.
- PageIndex indexes stored at `data/indexes/{campaign_id}/{doc_id}.json` (campaign) or `data/indexes/global/{doc_id}.json` (rulebooks).
- All DB access via `aiosqlite` repositories — no SQLAlchemy ORM.

## Commands

```bash
uv sync --extra dev       # install all dependencies
uv run pytest             # run all 33 tests
uv run pytest tests/test_combat.py -v    # run single test file
uv run uvicorn guide.api.main:app --reload  # start dev server (port 8000)
uv run ruff check src/    # lint
```

## Source Structure

```
src/guide/
  config.py          AppConfig (pydantic-settings, GUIDE__ prefix)
  errors.py          GuideError, NotFoundError, InvalidInputError, LlmError
  models/            Pydantic domain models (campaign, character, session, encounter, document, playstyle, shared)
  db/                aiosqlite repositories + migrations/001_initial.sql, 002_document_kind.sql
  llm/               LlmClient ABC, OllamaProvider, CloudProvider, LlmRouter, prompts
  pdf/               Docling extractor + PageIndex pipeline
  combat/            CombatEngine, initiative helpers
  api/               FastAPI app factory (main.py), AppState (state.py), routes/
tests/
  conftest.py        in-memory SQLite, mock LLM, AsyncClient fixture
  test_combat.py     11 combat unit tests
  test_repositories.py  12 DB integration tests
  test_api.py        HTTP layer tests
```

## Feature Areas

1. Combat Management (initiative, action economy, state tracking)
2. Campaign Intelligent Assistant (context-aware Q&A, spoiler prevention via PageIndex)
3. Backstory & Character Integration (LLM-extracted plot hooks)
4. Meaningful Encounter Generation (RAG-grounded, party-aware)
5. Rulebook & Mechanics Reference (D&D 5e core)
6. PDF Parsing & Campaign Portability (Docling → PageIndex)
7. Playstyle Personalization (PlaystyleProfile)
8. World Building & Narrative Tools
9. Session Summaries (tiered: player recaps vs. DM master logs)

## Agent Files

- `.claude/agents/python-backend-architect.md` — infrastructure, DB, LLM routing
- `.claude/agents/dnd-pipeline-engineer.md` — Docling + PageIndex pipeline
- `.claude/agents/dnd5e-systems-engineer.md` — combat engine, D&D mechanics
- `.claude/agents/partner-dm-narrator.md` — RAG Q&A, narrative generation, session summaries

The old Rust `crates/` directory remains as reference — delete once fully validated.
