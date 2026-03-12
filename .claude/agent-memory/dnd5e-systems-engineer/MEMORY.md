# D&D 5e Systems Engineer — Agent Memory

## Project: The Guide (Rust/axum + React/TypeScript)

### Key Files
- `crates/guide-core/src/models/encounter.rs` — `CombatParticipant`, `UpdateParticipantRequest`, `EncounterDifficulty`
- `crates/guide-combat/src/lib.rs` — `CombatEngine`, `build_participant()`
- `crates/guide-db/src/encounters.rs` — DB repos; `row_to_participant`, `save_state`, `add_participant`
- `crates/guide-api/src/routes/encounters.rs` — All encounter HTTP handlers + XP difficulty logic
- `crates/guide-db/migrations/` — SQL migration files (currently up to 008)
- `guide-frontend/src/api/types.ts` — TS interfaces mirroring all Rust models
- `guide-frontend/src/api/encounters.ts` — API call functions
- `guide-frontend/src/components/encounters/ParticipantRow.tsx` — Combat row UI
- `guide-frontend/src/pages/EncounterDetailPage.tsx` — Main encounter page

### D&D 5e Rules Implemented
- Death saves: 3 successes = stabilize (hp→1, clear unconscious, reset counters); 3 failures = `is_defeated = true`
- XP thresholds: per-player-level (Easy/Medium/Hard/Deadly) from DMG p.82
- Difficulty endpoint: GET `/campaigns/{id}/encounters/{enc_id}/difficulty` — only counts `CharacterType::Pc` participants
- Initiative sort: DESC by `initiative_total`, then `initiative_modifier`, then UUID (tiebreak)
- HP clamps to `[0, max_hp]`; reaching 0 sets `is_defeated = true` and adds `Condition::Unconscious`
- Condition duration tracking (FEAT-003): conditions stored as `ConditionEntry { condition, duration_rounds, applied_round }`. Auto-expired in `next_turn()` on round wrap when `applied_round + duration_rounds <= current_round`. `None` duration = permanent.

### Architecture Decisions
- `CombatParticipant` has `death_saves_success: i32` and `death_saves_failure: i32` (defaults 0)
- Migration `008_death_saves.sql` — `ALTER TABLE combat_participants ADD COLUMN` (two columns)
- Death saves only processed when `current_hp == 0` (guard in route handler)
- `EncounterDifficulty` struct added to `guide-core/src/models/encounter.rs`
- Difficulty handler looks up character records to get level; skips non-PC participants
- `ConditionEntry` struct in `encounter.rs` wraps `Condition` with `duration_rounds: Option<i32>` and `applied_round: Option<i32>`. No DB migration needed — conditions column is a JSON blob that automatically serializes the new struct. `unwrap_or_default()` handles old bare-enum JSON (produces empty vec).

### Known Pre-existing Bugs (not introduced by this work)
- `test_start_encounter` API test fails — expects `EncounterSummary` wrapper (`body["encounter"]`), but handler returns flat `Encounter`. This was failing before any FEAT-004/005/006 changes.
- Adding `spell_slots` field to `CreateCharacterRequest` (by another engineer) broke multiple call sites: `characters.rs` route (fixed with `spell_slots: None`), `repository_tests.rs` (fixed with `spell_slots: None`)
- `campaigns.rs` referenced `search_campaign` and `generate_atmosphere` functions that didn't exist (fixed by linter auto-generating full implementations)

### Test Count
- `cargo test --workspace`: 55 pass, 0 failures (as of FEAT-003)
- `cargo clippy --workspace -- -D warnings`: zero warnings/errors

### DB Column Pattern
SQLite stores JSON blobs for: `conditions` (JSON array), `action_budget` (JSON object).
Death saves are plain `INTEGER NOT NULL DEFAULT 0` columns — NOT JSON.
`row_to_participant` uses `.unwrap_or(0)` for the new columns for backward compatibility.

### Frontend Death Save UI
- Shows only when `participant.current_hp === 0 && !participant.is_defeated`
- 3 circles for successes (`+` when filled), 3 for failures (`x` when filled)
- CSS classes: `.death-saves`, `.death-save-circle`, `.death-save-success`, `.death-save-failure`
- Filled state: add class `.filled`
