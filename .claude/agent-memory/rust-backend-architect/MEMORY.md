# Persistent Memory — Rust Backend Architect

## Project Identity
- Workspace root: `C:\Users\altoz\Projects\the-guide`
- Rust stable, axum 0.8, sqlx 0.8 SQLite, async-openai pointing at Ollama `/v1`
- Config prefix: `GUIDE__`

## Workspace Crate Layout
```
crates/guide-core/   domain models + config + errors (no I/O)
crates/guide-db/     sqlx repos + migrations (./migrations/)
crates/guide-api/    axum HTTP server (lib + bin)
crates/guide-llm/    LlmClient trait + OllamaProvider + prompts
crates/guide-combat/ CombatEngine
crates/guide-pdf/    pdfium extractor + PageIndex
```

## Key Patterns

### JSON columns in SQLite
All non-scalar fields (conditions, ability_scores, spell_slots, backstory) stored as JSON TEXT.
Row-to-struct mapping uses `row.try_get("col").ok()` with `.unwrap_or_default()` to tolerate missing columns from migrations applied to existing data.

### Repository pattern
- Repos take `&'a SqlitePool` reference — no ownership
- Each method is async, returns `Result<T>` using `guide_core::{GuideError, Result}`
- Error variants: `GuideError::NotFound`, `GuideError::InvalidInput`, `GuideError::Internal`

### Migration numbering
Migrations in `crates/guide-db/migrations/` are numbered sequentially `001`–`NNN`.
As of FEAT-002: highest is `011_loot.sql`. Next migration: `012_`.

### AppState
`AppState.llm` is `Arc<dyn LlmClient>` — use trait object dispatch, not concrete type.

## Spell Slot Tracker (FEAT-002)
- Migration: `009_spell_slots.sql` — adds `spell_slots TEXT NOT NULL DEFAULT '[]'` to characters
- Core types: `SpellSlot`, `SpendSlotRequest`, `RestoreSlotRequest` in `guide-core/src/models/character.rs`
- Repo methods: `spend_spell_slot(id, level)`, `restore_spell_slots(id, Option<level>)` in `guide-db/src/characters.rs`
- Routes: `POST .../spell-slots/spend`, `POST .../spell-slots/restore` in `guide-api/src/routes/characters.rs`

## Test Status (at FEAT-002 implementation)
- `test_start_encounter` is a **pre-existing failure** unrelated to spell slots — do not fix unless tasked
- All other 54 tests pass

## Frontend Notes
- Package manager: **bun** (not npm/yarn)
- `useApi` hook manages data/loading/error/refetch — no direct `setData` exposure
- Pre-existing TS build error: unused `CampaignSearch` import in `CampaignDetailPage.tsx`
