# The Guide — Frontend Implementation TODO

## Project Summary

**The Guide** is an AI-powered D&D DM assistant.
Backend: Rust/Axum on port 8000, 38 API endpoints, SQLite + Qdrant (optional), Ollama LLM.
Frontend: Tauri v2 desktop app at `guide-frontend/` using React 19 + TypeScript 5.8 + Bun + Vite 7.

## Key File Paths

| Purpose | Path |
|---------|------|
| Frontend root | `guide-frontend/` |
| Tauri config | `guide-frontend/src-tauri/tauri.conf.json` |
| Main entry | `guide-frontend/src/main.tsx` |
| App router | `guide-frontend/src/App.tsx` |
| Global CSS | `guide-frontend/src/App.css` |
| API types | `guide-frontend/src/api/types.ts` |
| API client | `guide-frontend/src/api/client.ts` |
| Hooks | `guide-frontend/src/hooks/` |
| Components | `guide-frontend/src/components/` |
| Pages | `guide-frontend/src/pages/` |
| Backend test | `cargo test --workspace` (55 tests) |
| Frontend build | `cd guide-frontend && /home/zbloss/.bun/bin/bun run build` |

> **Note:** `bun` is at `/home/zbloss/.bun/bin/bun` — not on PATH, use full path in scripts.

## All 38 Backend API Endpoints

### Health
1. `GET /health` — health status
2. `GET /version` — version info

### Campaigns (5)
3. `GET /campaigns` — list all
4. `POST /campaigns` — create
5. `GET /campaigns/:id` — get one
6. `PUT /campaigns/:id` — update (incl. world state)
7. `DELETE /campaigns/:id` — delete

### Characters (6)
8. `GET /campaigns/:id/characters` — list
9. `POST /campaigns/:id/characters` — create
10. `GET /campaigns/:id/characters/:cid` — get one
11. `PUT /campaigns/:id/characters/:cid` — update
12. `DELETE /campaigns/:id/characters/:cid` — delete
13. `POST /campaigns/:id/characters/:cid/analyze-backstory` — AI backstory analysis

### Sessions (9)
14. `GET /campaigns/:id/sessions` — list
15. `POST /campaigns/:id/sessions` — create
16. `GET /campaigns/:id/sessions/:sid` — get one
17. `DELETE /campaigns/:id/sessions/:sid` — delete
18. `POST /campaigns/:id/sessions/:sid/start` — start session
19. `POST /campaigns/:id/sessions/:sid/end` — end session
20. `GET /campaigns/:id/sessions/:sid/events` — list events
21. `POST /campaigns/:id/sessions/:sid/events` — create event
22. `GET /campaigns/:id/sessions/:sid/summary?perspective=...` — AI summary

### Encounters (9)
23. `GET /campaigns/:id/encounters` — list
24. `POST /campaigns/:id/encounters` — create
25. `GET /campaigns/:id/encounters/:eid` — get one
26. `DELETE /campaigns/:id/encounters/:eid` — delete
27. `POST /campaigns/:id/encounters/:eid/start` — start combat
28. `POST /campaigns/:id/encounters/:eid/next-turn` — advance turn
29. `POST /campaigns/:id/encounters/:eid/end` — end combat
30. `PUT /campaigns/:id/encounters/:eid/participants/:pid` — update participant
31. `POST /campaigns/:id/encounters/generate` — AI encounter generation

### Documents - Campaign (4)
32. `GET /campaigns/:id/documents` — list
33. `POST /campaigns/:id/documents` — upload (multipart)
34. `GET /campaigns/:id/documents/:did` — get one
35. `POST /campaigns/:id/documents/:did/ingest` — trigger ingest

### Documents - Global (4)
36. `GET /documents` — list global
37. `POST /documents` — upload global (multipart)
38. `GET /documents/:did` — get global one
39. `POST /documents/:did/ingest` — trigger global ingest

### Chat (1)
40. `POST /campaigns/:id/chat` — SSE streaming chat (fetch + ReadableStream)

---

## Implementation Tasks

### Step 0 — TODO.md
- [x] Write comprehensive TODO.md at project root

### Step 1 — Dependencies + Skeleton
- [x] `bun add react-router-dom` (installed react-router-dom@7.13.1)
- [x] `bun add -D @types/react-router-dom` (installed @types/react-router-dom@5.3.3)
- [x] Create `src/api/` — client.ts, types.ts, campaigns.ts, characters.ts, sessions.ts, encounters.ts, documents.ts, chat.ts, health.ts
- [x] Create `src/hooks/` — useApi.ts, useCampaign.ts, useChat.ts
- [x] Create `src/components/layout/` — Sidebar.tsx, Header.tsx, Layout.tsx
- [x] Create `src/components/common/` — LoadingSpinner.tsx, ErrorBanner.tsx, ConfirmButton.tsx, Badge.tsx, Modal.tsx, FormField.tsx
- [x] Create `src/components/campaigns/` — CampaignCard.tsx, CampaignForm.tsx, WorldStateEditor.tsx
- [x] Create `src/components/characters/` — CharacterList.tsx, CharacterCard.tsx, CharacterForm.tsx, BackstoryPanel.tsx, ConditionBadge.tsx
- [x] Create `src/components/sessions/` — SessionList.tsx, SessionCard.tsx, SessionForm.tsx, SessionEventList.tsx, SessionEventForm.tsx, SummaryView.tsx
- [x] Create `src/components/encounters/` — EncounterList.tsx, EncounterCard.tsx, EncounterForm.tsx, CombatTracker.tsx, ParticipantRow.tsx, GenerateEncounterPanel.tsx
- [x] Create `src/components/documents/` — DocumentList.tsx, UploadForm.tsx, IngestButton.tsx
- [x] Create `src/components/chat/` — ChatPanel.tsx, MessageBubble.tsx, PerspectiveSelector.tsx
- [x] Create `src/pages/` — all 13 page files
- [x] Update `tauri.conf.json` — width 1280, height 800, minWidth 1024, minHeight 700

### Step 2 — API Foundation
- [x] `src/api/types.ts` — all TypeScript interfaces + enums
- [x] `src/api/client.ts` — BASE_URL, ApiError, apiFetch, apiGet, apiPost, apiPut, apiDelete, apiMultipart
- [x] `src/api/campaigns.ts` — listCampaigns, createCampaign, getCampaign, updateCampaign, deleteCampaign
- [x] `src/api/characters.ts` — CRUD + analyzeBackstory
- [x] `src/api/sessions.ts` — CRUD + start/end + events + summary
- [x] `src/api/encounters.ts` — CRUD + start/nextTurn/end + updateParticipant + generateEncounter
- [x] `src/api/documents.ts` — campaign docs + global docs
- [x] `src/api/chat.ts` — raw fetch for SSE streaming
- [x] `src/api/health.ts` — getHealth, getVersion

### Step 3 — Routing + Layout
- [x] `src/main.tsx` — BrowserRouter wrapper added
- [x] `src/App.tsx` — full route tree with Layout parent, all 13 routes
- [x] `src/components/layout/Layout.tsx` — Sidebar + main content + Outlet
- [x] `src/components/layout/Sidebar.tsx` — title, live campaign list, global links
- [x] `src/components/layout/Header.tsx` — campaign name + 30s health polling status pill
- [x] `src/App.css` — dark theme reset, CSS variables, layout tokens, HP bar classes

### Step 4 — useApi Hook + CampaignsPage
- [x] `src/hooks/useApi.ts` — generic data fetcher with deps + refetch()
- [x] `src/components/common/LoadingSpinner.tsx`
- [x] `src/components/common/ErrorBanner.tsx`
- [x] `src/components/common/Modal.tsx` — ReactDOM.createPortal to document.body
- [x] `src/components/common/ConfirmButton.tsx` — inline yes/no confirm state
- [x] `src/components/common/FormField.tsx`
- [x] `src/components/common/Badge.tsx` + StatusBadge
- [x] `src/components/campaigns/CampaignCard.tsx`
- [x] `src/components/campaigns/CampaignForm.tsx`
- [x] `src/pages/CampaignsPage.tsx`

### Step 5 — CampaignDetailPage + WorldStateEditor
- [x] `src/components/campaigns/WorldStateEditor.tsx` — editable world state with tag lists
- [x] `src/pages/CampaignDetailPage.tsx` — detail + tab nav (Characters/Sessions/Encounters/Documents/Chat) + WorldStateEditor

### Step 6 — Characters
- [x] `src/components/characters/ConditionBadge.tsx` — with emoji icons per condition
- [x] `src/components/characters/CharacterForm.tsx` — all fields incl. ability scores + backstory
- [x] `src/components/characters/CharacterCard.tsx` — HP bar, conditions, badges
- [x] `src/components/characters/CharacterList.tsx`
- [x] `src/components/characters/BackstoryPanel.tsx` — AI analyze button + hooks/motivations/secrets display
- [x] `src/pages/CharactersPage.tsx`
- [x] `src/pages/CharacterDetailPage.tsx` — HP bar, conditions add/remove, ability scores, is_alive toggle, BackstoryPanel

### Step 7 — Sessions
- [x] `src/components/sessions/SessionCard.tsx`
- [x] `src/components/sessions/SessionList.tsx`
- [x] `src/components/sessions/SessionForm.tsx`
- [x] `src/components/sessions/SessionEventList.tsx`
- [x] `src/components/sessions/SessionEventForm.tsx` — all 9 event types, significance, character multi-select
- [x] `src/components/sessions/SummaryView.tsx`
- [x] `src/pages/SessionsPage.tsx`
- [x] `src/pages/SessionDetailPage.tsx` — Events tab + Summary tab + start/end session buttons

### Step 8 — Encounters + Combat Tracker
- [x] `src/components/encounters/EncounterCard.tsx`
- [x] `src/components/encounters/EncounterList.tsx`
- [x] `src/components/encounters/EncounterForm.tsx` — session select, character multi-select
- [x] `src/components/encounters/GenerateEncounterPanel.tsx` — AI generation with enemies table + terrain + rewards
- [x] `src/components/encounters/ParticipantRow.tsx` — HP bar, conditions, action budget icons, damage/heal/setHP/addCond controls
- [x] `src/components/encounters/CombatTracker.tsx` — round counter, sorted initiative table
- [x] `src/pages/EncountersPage.tsx`
- [x] `src/pages/EncounterDetailPage.tsx` — pending/active/completed state machine, local state replaced on each API response

### Step 9 — Documents
- [x] `src/components/documents/DocumentList.tsx`
- [x] `src/components/documents/UploadForm.tsx` — PDF-only file input, no Content-Type header
- [x] `src/components/documents/IngestButton.tsx` — 3s polling until completed/failed, clears on unmount
- [x] `src/pages/DocumentsPage.tsx`
- [x] `src/pages/GlobalDocumentsPage.tsx`

### Step 10 — Chat (SSE Streaming)
- [x] `src/hooks/useChat.ts` — fetch + ReadableStream SSE parser, abortController cleanup
- [x] `src/components/chat/PerspectiveSelector.tsx` — dm/player radio toggle
- [x] `src/components/chat/MessageBubble.tsx` — user right-aligned, assistant left-aligned
- [x] `src/components/chat/ChatPanel.tsx` — streaming dots, disabled input while streaming
- [x] `src/pages/ChatPage.tsx`

### Step 11 — Health Page + CSS Polish
- [x] `src/pages/HealthPage.tsx` — health status + version info + config display
- [x] `src/pages/NotFoundPage.tsx`
- [x] CSS: HP bar color breakpoints, combat row highlight, all badge variants, sidebar active link, responsive sidebar collapse

### Step 12 — End-to-End Verification
- [x] `cargo test --workspace` — **55/55 Rust tests pass** (23 api + 11 combat + 12 db + 9 pdf)
- [x] `bun run build` — **TypeScript compiles clean**, 287KB bundle, zero errors
- [ ] `cargo clippy --workspace -- -D warnings` — verify zero lint errors (not yet run)
- [ ] Manual walkthrough with live backend (requires Ollama + backend running)

---

## Remaining Work

### Immediate (before calling complete)
- [ ] Run `cargo clippy --workspace -- -D warnings` and fix any warnings
- [ ] Manual smoke test: `cargo run -p guide-api` + `bun run dev` and click through each feature

### Known Gaps / Polish Items
- [ ] **Character delete** — `CharacterDetailPage` has no delete button; `CharacterList`/`CharacterCard` has no delete action either. Add `deleteCharacter` call + ConfirmButton to `CharacterCard`.
- [ ] **CampaignDetailPage nested routing** — The tab nav links to relative paths (`characters`, `sessions`, etc.) but the `<Outlet>` must render inside the same page. Verify nested route rendering works correctly in the browser; may need index route redirect.
- [ ] **`deleteSession`** appears in `SessionsPage` but `deleteSession` is exported from `sessions.ts` — confirm the API endpoint actually exists in the backend (endpoint #17).
- [ ] **Session event delete** — no delete-event endpoint exists in the backend; UI correctly omits it.
- [ ] **Encounter `encId` param** — `EncounterDetailPage` uses `encId` but `App.tsx` defines the route param as `:encId`. Double-check consistency.
- [ ] **`useCampaign` error path** — when `campaignId` is undefined, it rejects with "No campaign ID". The hook should return gracefully on the root `/` page where no campaign is selected.
- [ ] **Document ingest polling refetch** — after ingest completes, `DocumentsPage` doesn't call `refetch()` to update the list status. Wire `onPoll` result back into local state or trigger a refetch.
- [ ] **`@types/react-router-dom` version mismatch** — installed v5.3.3 types but react-router-dom v7 is installed. The v5 types are for an older API. Uninstall `@types/react-router-dom` (v7 ships its own types): `bun remove @types/react-router-dom`.

### Future Work
- [ ] Playstyle profile UI (PlaystyleProfile model exists in backend)
- [ ] Dark mode toggle
- [ ] Keyboard shortcuts for combat tracker (space = next turn, etc.)
- [ ] Export session summaries to PDF/Markdown
- [ ] Multi-campaign sidebar with drag reorder
- [ ] Offline mode / cached data with service worker
- [ ] Tauri system tray icon with quick-access menu
- [ ] WebSocket for real-time multi-device sync
- [ ] Character inline name/stats edit (currently only HP/conditions/is_alive)
- [ ] Encounter participant inline name edit for monsters
- [ ] Session event delete (would require a new backend endpoint)

---

## Critical Implementation Notes

### SSE Streaming (Chat)
- Use `fetch` + `ReadableStream`, **NOT** `EventSource` — endpoint is POST
- Parse `event:` and `data:` lines from buffer split on `\n\n`
- Call `abortController.abort()` on effect cleanup/unmount

### Multipart Uploads (Documents)
- Use `apiMultipart` — do **NOT** set `Content-Type` header
- Browser auto-sets multipart boundary in Content-Type when using FormData

### Document Ingest Polling
- `setInterval(3000)` polling GET /documents/:id
- Clear interval when status is `completed` or `failed`
- Clear interval on component unmount

### Combat Tracker State
- State owned entirely by `EncounterDetailPage` as local `useState`
- Replaced entirely on each API response (not partial merge)
- Highlight current-turn participant row with `.current-turn` CSS class

### useApi Hook
- deps array + internal `tick` counter triggers refetch via useEffect
- `refetch()` increments tick to force re-run without changing deps

### HP Bar Colors
- `>50%` → `--hp-high: #4caf50` (green)
- `25–50%` → `--hp-mid: #ffc107` (yellow)
- `<25%` → `--hp-low: #f44336` (red)

### Conditions (15 values)
Blinded, Charmed, Deafened, Exhausted, Frightened, Grappled, Incapacitated, Invisible, Paralyzed, Petrified, Poisoned, Prone, Restrained, Stunned, Unconscious

### Route Structure
```
/ → CampaignsPage
/campaigns/:campaignId → CampaignDetailPage
  /campaigns/:campaignId/characters → CharactersPage        (nested Outlet)
  /campaigns/:campaignId/characters/:charId → CharacterDetailPage
  /campaigns/:campaignId/sessions → SessionsPage
  /campaigns/:campaignId/sessions/:sessionId → SessionDetailPage
  /campaigns/:campaignId/encounters → EncountersPage
  /campaigns/:campaignId/encounters/:encId → EncounterDetailPage
  /campaigns/:campaignId/documents → DocumentsPage
  /campaigns/:campaignId/chat → ChatPage
/documents → GlobalDocumentsPage
/health → HealthPage
* → NotFoundPage
```
