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
As of FEAT-037: highest is `023_encounter_turns.sql`. Next migration: `024_`.

### AppState
`AppState.llm` is `Arc<dyn LlmClient>` — use trait object dispatch, not concrete type.

## Spell Slot Tracker (FEAT-002)
- Migration: `009_spell_slots.sql` — adds `spell_slots TEXT NOT NULL DEFAULT '[]'` to characters
- Core types: `SpellSlot`, `SpendSlotRequest`, `RestoreSlotRequest` in `guide-core/src/models/character.rs`
- Repo methods: `spend_spell_slot(id, level)`, `restore_spell_slots(id, Option<level>)` in `guide-db/src/characters.rs`
- Routes: `POST .../spell-slots/spend`, `POST .../spell-slots/restore` in `guide-api/src/routes/characters.rs`

## Character Sheet PDF Import (FEAT-001)
- `guide_pdf::extractor::extract_text_sync` — pub fn, temp-file pattern: write bytes → pdfium text extract → delete
- `LlmTask::CharacterSheetParse`, `LlmTask::AudioTranscription` added to `guide-llm/src/client.rs`
- `ParsedSheetResult` in `guide-core/src/models/character.rs`
- Route: `POST /campaigns/{id}/characters/parse-sheet` — multipart `file` field, registered after other static routes
- In `relationships.rs`: import only `CreateRelationshipRequest` (not response type) to avoid unused-import warning

## NPC Relationship Web (FEAT-011)
- Migration: `022_relationships.sql` — `character_relationships` with UNIQUE(campaign_id, from, to)
- Core types: `CharacterRelationship`, `CreateRelationshipRequest` in `guide-core/src/models/relationship.rs`
- Repo: `guide-db/src/relationships.rs`, module in `guide-db/src/lib.rs`
- Routes: `GET/POST /campaigns/{id}/relationships`, `DELETE /{rel_id}` in `guide-api/src/routes/relationships.rs`
- `post` routing import not needed in relationships.rs — `.post()` is a method on axum MethodRouter

## Encounter Replay (FEAT-037)
- Migration: `023_encounter_turns.sql` — upsert on UNIQUE(encounter_id, turn_number)
- `EncounterTurnSnapshot` in `guide-core/src/models/encounter.rs`
- Repo: `record_turn_snapshot`, `count_turn_snapshots`, `list_turn_snapshots` on `EncounterRepository`
- Snapshot recording: count existing → use count as turn_number; errors warn-logged not propagated
- Route: `GET /campaigns/{id}/encounters/{id}/replay`

## Voice-to-Text Stub (FEAT-038)
- Route: `POST /campaigns/{id}/sessions/{id}/transcribe` — reads `audio` multipart field, returns `{"transcript": ""}`
- Real Whisper deferred; multipart field validation still enforced

## DM Prep Suite
- Migration: `021_dm_prep.sql` — adds `current_chapter TEXT` to campaigns; creates `dm_prep_results` table
- Core types: `PrepType`, `DmPrepResult`, `SessionRecapRequest`, `StoryContextRequest`, `CharacterRoadmapRequest` in `guide-core/src/models/prep.rs`
- Repo: `DmPrepRepository` in `guide-db/src/prep.rs` — upsert/get/list_by_campaign with (campaign_id, prep_type, character_id) key
- LlmTask variants added: `SessionRecap`, `StorySoFar`, `StoryAhead`, `CharacterRoadmap`
- Routes: `guide-api/src/routes/prep.rs` — GET/POST for session-recap, story-so-far, story-ahead, character-roadmap/{char_id}
- All endpoints cache results; `force_regenerate: true` bypasses cache
- story-so-far and story-ahead require Qdrant-indexed documents; return 422 if none found
- character-roadmap validates `character.campaign_id == campaign_id` before generating

## Test Status (at DM Prep Suite)
- `test_start_encounter` is a **pre-existing failure** unrelated to this feature — do not fix unless tasked
- All 55 tests pass (23 api + 12 db + 11 combat + 9 pdf)

## Frontend Notes
- Package manager: **bun** (not npm/yarn)
- `useApi` hook manages data/loading/error/refetch — no direct `setData` exposure
- Pre-existing TS build error: unused `CampaignSearch` import in `CampaignDetailPage.tsx`
