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

- [ ] **FEAT-001** — Character Sheet PDF Import `[Effort: L]`
  `POST /campaigns/{id}/characters/parse-sheet` multipart endpoint: Docling extracts text from uploaded PDF, structured LLM prompt maps fields to `Character` model, returns pre-filled JSON. Add "Upload Sheet" button to `CharactersPage.tsx` that opens a file picker and populates `CharacterForm` fields automatically.

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

- [ ] **FEAT-011** — NPC Relationship Web `[Effort: XL]`
  New `character_relationships` table: `(from_id, to_id, relationship_type, notes)`. CRUD endpoints under `/campaigns/{id}/relationships`. Force-directed graph visualization using D3 + React on a new `RelationshipMapPage`. Clicking a node opens the character sidebar.

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

- [ ] **FEAT-037** — Encounter Replay `[Effort: XL]`
  Store full turn-by-turn state snapshots in `encounter_turns` table. `GET /encounters/{id}/replay` streams the sequence.

- [ ] **FEAT-038** — Voice-to-Text Session Notes `[Effort: L]`
  Browser-based speech recognition (Web Speech API) on `SessionDetailPage`.

- [x] **FEAT-039** — Faction & Reputation Tracker `[Effort: M]` ✓ Done
  `factions` + `faction_reputation` tables, CRUD endpoints, `FactionTracker` component on CampaignDetailPage.

- [ ] **FEAT-040** — Offline Mode / PWA `[Effort: XL]`
  Service worker + IndexedDB cache for critical campaign data.

---

## Summary

**Completed: 40 / 45 items (89%)**

P0: 5/5 ✓ | P1: 9/10 | P2: 13/13 ✓ | P3: 13/17
