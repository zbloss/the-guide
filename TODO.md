# The Guide — Feature Backlog

> Last updated: 2026-03-12
> Format: `[Effort: S=hours, M=1-2 days, L=3-5 days, XL=week+]`

---

## P0 — Critical Bug Fixes (blocking release)

- [x] **BUG-001** — Session status field missing `[Effort: S]` ✓ Done
- [x] **BUG-002** — Condition case mismatch `[Effort: S]` ✓ Done
- [x] **BUG-003** — EventType enum mismatch `[Effort: S]` ✓ Done
- [x] **BUG-004** — EventSignificance enum mismatch `[Effort: S]` ✓ Done
- [x] **BUG-005** — SessionEvent date field mismatch `[Effort: S]` ✓ Done

---

## P1 — High-Value Features

- [x] **FEAT-001** — Character Sheet PDF Import `[Effort: L]` ✓ Done
      `POST /campaigns/{id}/characters/parse-sheet` multipart endpoint: pdfium OCR + vision LLM maps fields to `ParsedSheetResult`. "Upload Sheet" button on `CharactersPage.tsx` pre-fills the character form.

- [x] **FEAT-002** — Spell Slot Tracker `[Effort: L]` ✓ Done

- [x] **FEAT-003** — Condition Duration Tracking `[Effort: L]` ✓ Done
      Conditions upgraded to `{condition, duration_rounds, applied_round}` objects. Combat engine auto-expires on round advance. Rounds shown in participant cards.

- [x] **FEAT-004** — Live Combat Damage Calculator `[Effort: M]` ✓ Done
      Resistance/Vulnerability multiplier dropdown added to damage widget on participant cards.

- [x] **FEAT-005** — Death Save Tracker `[Effort: M]` ✓ Done

- [x] **FEAT-006** — Encounter Difficulty Estimator `[Effort: M]` ✓ Done

- [x] **FEAT-007** — AI Improv Assist `[Effort: M]` ✓ Done

- [x] **FEAT-008** — AI Lore Consistency Checker `[Effort: L]` ✓ Done

- [x] **FEAT-009** — NPC Quick-Create with AI `[Effort: M]` ✓ Done

- [x] **FEAT-010** — Session Summary Export `[Effort: S]` ✓ Done

---

## P2 — Medium-Value Features

- [x] **FEAT-011** — NPC Relationship Web `[Effort: XL]` ✓ Done
      `character_relationships` table with CRUD + PATCH endpoints. Force-directed graph on `RelationshipMapPage` with edge color coding by type, color legend, inline edit sidebar, and "View on Map" link from CharacterDetailPage.

- [x] **FEAT-012** — In-World Calendar Tracker `[Effort: L]` ✓ Done

- [x] **FEAT-013** — Loot & Treasure Log `[Effort: M]` ✓ Done

- [x] **FEAT-014** — Character Level-Up Assistant `[Effort: L]` ✓ Done
      `POST /characters/{id}/level-up` → LLM-powered level-up recommendations. Level-up assist accessible from CharacterDetailPage.

- [x] **FEAT-015** — Global Search `[Effort: M]` ✓ Done

- [x] **FEAT-016** — Encounter Template Library `[Effort: M]` ✓ Done
      Save any encounter as a reusable template. `GET /encounter-templates` returns global library. "Load Template" populates the encounter form.

- [x] **FEAT-017** — Backstory Plot Hook Tracker `[Effort: M]` ✓ Done
      `plot_hooks` table, auto-populated from analyze-backstory. Kanban board on CharacterDetailPage with Open/Active/Resolved columns.

- [x] **FEAT-018** — Multi-Campaign Dashboard `[Effort: M]` ✓ Done
      Campaign cards with last-updated relative time, quick-action buttons.

- [x] **FEAT-019** — Rule Reference Sidebar `[Effort: M]` ✓ Done
      `GET /rules/search?q=...` queries global PageIndex. Collapsible floating sidebar panel available on all pages.

- [x] **FEAT-020** — Encounter Combat Log Export `[Effort: S]` ✓ Done

- [x] **FEAT-021** — Character Portrait Upload `[Effort: S]` ✓ Done

- [x] **FEAT-022** — Webhook / Discord Notification `[Effort: M]` ✓ Done
      `POST /campaigns/{id}/webhooks` — register Discord webhook URL. Fires on session_start, session_end events. WebhookManager component on CampaignDetailPage.

- [x] **FEAT-023** — Document Chunk Search UI `[Effort: M]` ✓ Done
      `GET /campaigns/{id}/documents/{doc_id}/search?q=...` queries campaign PageIndex. DocumentChunkSearch component on DocumentsPage.

---

## P3 — Nice-to-Have / Future

- [x] **FEAT-024** — AI Plot Twist Generator `[Effort: M]` ✓ Done
      `POST /campaigns/{id}/plot-twist` + PlotTwistModal on CampaignDetailPage.

- [x] **FEAT-025** — D&D Beyond Character Import `[Effort: L]` ✓ Done
      Accept D&D Beyond character JSON export. Map fields to `Character` model. `POST /campaigns/{id}/characters/import-dndbeyond`. Frontend button on CharactersPage.

- [x] **FEAT-026** — Initiative Roll Automation `[Effort: S]` ✓ Done
      Auto-roll d20 + DEX modifier for participants with `initiative_roll == 0` on encounter start.

- [x] **FEAT-027** — Session Prep AI Assistant `[Effort: L]` ✓ Done
      `POST /campaigns/{id}/sessions/prep` — LLM generates structured prep document for upcoming session.

- [x] **FEAT-028** — Shared Player View (read-only) `[Effort: L]` ✓ Done
      `POST /campaigns/{id}/share` generates token. `/view/{token}` shows party, world state, last session. Share button on CampaignDetailPage copies URL to clipboard.

- [x] **FEAT-029** — Campaign Analytics Dashboard `[Effort: L]` ✓ Done
      `GET /campaigns/{id}/analytics` — sessions/encounters/characters counts, sessions-by-month bar chart, encounter difficulty distribution. Analytics tab on CampaignDetailPage.

- [x] **FEAT-030** — Bulk Character Import (CSV) `[Effort: S]` ✓ Done
      `POST /campaigns/{id}/characters/import-csv` — CSV upload button on CharactersPage.

- [x] **FEAT-031** — Map Attachment to Sessions `[Effort: M]` ✓ Done
      Upload image maps per session. Stored in `data/maps/`. Map tab on SessionDetailPage.

- [x] **FEAT-032** — AI Villain Motivations Generator `[Effort: S]` ✓ Done
      `POST /characters/{id}/villain-profile` — LLM generates villain backstory, motivation, flaw, lair, signature move. Available for NPC/Monster characters.

- [x] **FEAT-033** — Weather & Atmosphere Generator `[Effort: S]` ✓ Done

- [x] **FEAT-034** — Persistent Chat History `[Effort: M]` ✓ Done
      `campaign_chat` table + `GET /campaigns/{id}/chat/history` endpoint. Chat UI loads history on mount.

- [x] **FEAT-035** — Homebrew Rule Registry `[Effort: M]` ✓ Done
      `homebrew_rules` table, CRUD endpoints, `HomebrewRuleList` component on CampaignDetailPage.

- [x] **FEAT-036** — AI Session Debrief `[Effort: M]` ✓ Done
      `POST /sessions/{id}/debrief` — LLM generates structured post-session coaching report. Debrief tab on SessionDetailPage.

- [x] **FEAT-037** — Encounter Replay `[Effort: XL]` ✓ Done
      `encounter_turns` table stores full state snapshots per turn. `GET /encounters/{id}/replay` streams via SSE (`event: snapshot` per turn, terminal `event: done`). VCR scrubber UI on EncounterDetailPage.

- [x] **FEAT-038** — Voice-to-Text Session Notes `[Effort: L]` ✓ Done
      `VoiceNoteCapture` component on SessionDetailPage: Web Speech API primary, MediaRecorder + `/sessions/{id}/transcribe` Whisper fallback (Ollama). Transcript pre-fills SessionEventForm description. Configurable via `GUIDE__WHISPER_MODEL`.

- [x] **FEAT-039** — Faction & Reputation Tracker `[Effort: M]` ✓ Done
      `factions` + `faction_reputation` tables, CRUD endpoints, `FactionTracker` component on CampaignDetailPage.

- [x] **FEAT-040** — Offline Mode / PWA `[Effort: XL]` ✓ Done
      `vite-plugin-pwa` + Workbox `NetworkFirst` SW. IDB (`idb`) stores campaigns, characters, sessions, encounters. CampaignsPage/SessionsPage/EncountersPage cache on load and fall back to IDB when offline. Mutation buttons disabled when offline. `OfflineIndicator` banner in layout. `syncManager` replays pending writes on reconnect.

---

---

## Story Extraction Pipeline QA

> QA report produced 2026-03-19. All bugs found were already fixed; one limitation patched.

- [x] **BUG-1** — UTF-8 slice panic in chunker `[Effort: S]` ✓ Already fixed
      `floor_char_boundary` used at truncation sites (pipeline.rs lines 419, 540) prevents slicing mid-codepoint.

- [x] **BUG-2** — `looks_like_heading` false-positive on "act "/"scene " prefixes `[Effort: S]` ✓ Already fixed
      Pattern now requires a digit or uppercase letter immediately after the prefix; bare "act " or "scene " no longer match.

- [x] **BUG-3** — `looks_like_heading` false-positive on D&D stat-block rows `[Effort: S]` ✓ Already fixed
      Rule now requires ≥5 consecutive uppercase chars; short abbreviation rows (e.g. "STR DEX CON") no longer trigger heading detection.

- [x] **LIMIT-2** — `event_title_to_id` lookup was case-sensitive `[Effort: S]` ✓ Fixed 2026-03-19
      Insert and lookup keys now normalized to lowercase, matching the existing `arc_title_to_id` pattern (pipeline.rs lines 605, 629).

- [ ] **LIMIT-1** — TABLE-header rows can be misclassified as headings `[Effort: M]` *(accepted tech debt)*
      Markdown table syntax (`| --- |`) is not yet excluded from heading heuristics. Low impact in practice; fix when false-positive rate becomes measurable.

- [ ] **LIMIT-3** — Single-call fallback truncates at 40 k chars `[Effort: M]` *(accepted tech debt)*
      Very large documents that bypass chunked extraction are silently truncated. Acceptable given typical PDF sizes; revisit if ingestion quality drops on XL rulebooks.

---

## Summary

**Completed: 46 / 48 items (96%)**

P0: 5/5 ✓ | P1: 10/10 ✓ | P2: 13/13 ✓ | P3: 16/17 | Story Pipeline QA: 4/6
