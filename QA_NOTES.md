# QA Notes — Working Log

## 2026-03-10 Session

### Method
- Browser extension unavailable; used static code review + curl API calls
- Verified all 47 frontend source files
- Made direct API calls to running backend (http://localhost:8000)
- Backend confirmed running: `GET /health` → `{"status":"ok"}`, version `0.1.0`
- Existing live data: 1 campaign ("Land of Vampires"), 3 characters, 1 session, 2 events, 0 encounters

### Critical Findings

**Session status** — completely broken. Backend `Session` Rust struct has no `status` field. Frontend reads `session.status` which is always `undefined`. Verified via live API call.

**Condition case mismatch** — backend uses `snake_case` serde (`"blinded"`) but frontend uses PascalCase (`'Blinded'`). Adding/removing/displaying conditions is broken. Also: frontend has `'Exhausted'` not in backend enum.

**EventType mismatch** — only 3 of 9 frontend types match backend: `combat`, `exploration`, `item_found`. The rest (`roleplay`, `skill_challenge`, `npc_introduced`, `quest_update`, `revelation`, `other`) will cause 422 errors.

**EventSignificance mismatch** — `moderate` and `critical` don't exist in backend enum (which has `minor`, `major`, `milestone`).

**SessionEvent field** — backend returns `occurred_at`, frontend reads `created_at` → "Invalid Date" in event table.

### KI-6 Debunked
The DELETE /events/{event_id} endpoint DOES exist in the Rust backend (`crates/guide-api/src/routes/sessions.rs:44-45`).

### Live Data Observed
- Character "Dale Dug" has `current_hp: 20, max_hp: 10` (HP overflow — backend accepts over-max HP)
- Global doc "2024_DnD_SRD.pdf" stuck in `"processing"` status (may be a long-running ingest)
- Session "Chapter 1" has 2 events (combat, exploration)

### Files Reviewed
- All pages: CampaignsPage, CampaignDetailPage, CharacterDetailPage, SessionDetailPage, EncounterDetailPage, GlobalDocumentsPage, ChatPage, PlaystylePage, HealthPage
- Key components: ChatPanel, CombatTracker, ParticipantRow, SummaryView, BackstoryPanel, SessionEventList, EncounterForm, IngestButton, UploadForm, ConfirmButton, WorldStateEditor, Sidebar
- All API files: sessions.ts, encounters.ts, characters.ts, documents.ts, types.ts
- All hooks: useChat.ts
- Rust models: session.rs, encounter.rs, shared.rs

### Not Tested (Browser Required)
- SSE streaming chat
- File upload (PDF)
- Document ingest polling lifecycle
- Keyboard shortcuts (Spacebar for next turn)
- Drag-to-reorder sidebar campaigns
- localStorage persistence (playstyle, chat history, campaign order)
- Direct URL access to sub-routes
- Browser back/forward navigation
