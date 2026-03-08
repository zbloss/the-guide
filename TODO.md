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
| Frontend build | `cd guide-frontend && bun run build` |

## All 38 Backend API Endpoints

### Health
1. `GET /health` — health status
2. `GET /version` — version info

### Campaigns (6)
3. `GET /campaigns` — list all
4. `POST /campaigns` — create
5. `GET /campaigns/:id` — get one
6. `PUT /campaigns/:id` — update (incl. world state)
7. `DELETE /campaigns/:id` — delete

### Characters (7)
8. `GET /campaigns/:id/characters` — list
9. `POST /campaigns/:id/characters` — create
10. `GET /campaigns/:id/characters/:cid` — get one
11. `PUT /campaigns/:id/characters/:cid` — update
12. `DELETE /campaigns/:id/characters/:cid` — delete
13. `POST /campaigns/:id/characters/:cid/analyze-backstory` — AI backstory analysis

### Sessions (8)
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
- [ ] `cd guide-frontend && bun add react-router-dom`
- [ ] `bun add -D @types/react-router-dom`
- [ ] Create `src/api/` directory with: `client.ts`, `types.ts`, `campaigns.ts`, `characters.ts`, `sessions.ts`, `encounters.ts`, `documents.ts`, `chat.ts`, `health.ts`
- [ ] Create `src/hooks/` directory with: `useApi.ts`, `useCampaign.ts`, `useChat.ts`
- [ ] Create `src/components/layout/`: `Sidebar.tsx`, `Header.tsx`, `Layout.tsx`
- [ ] Create `src/components/common/`: `LoadingSpinner.tsx`, `ErrorBanner.tsx`, `ConfirmButton.tsx`, `Badge.tsx`, `Modal.tsx`, `FormField.tsx`
- [ ] Create `src/components/campaigns/`: `CampaignCard.tsx`, `CampaignForm.tsx`, `WorldStateEditor.tsx`
- [ ] Create `src/components/characters/`: `CharacterList.tsx`, `CharacterCard.tsx`, `CharacterForm.tsx`, `BackstoryPanel.tsx`, `ConditionBadge.tsx`
- [ ] Create `src/components/sessions/`: `SessionList.tsx`, `SessionCard.tsx`, `SessionForm.tsx`, `SessionEventList.tsx`, `SessionEventForm.tsx`, `SummaryView.tsx`
- [ ] Create `src/components/encounters/`: `EncounterList.tsx`, `EncounterCard.tsx`, `EncounterForm.tsx`, `CombatTracker.tsx`, `ParticipantRow.tsx`, `GenerateEncounterPanel.tsx`
- [ ] Create `src/components/documents/`: `DocumentList.tsx`, `UploadForm.tsx`, `IngestButton.tsx`
- [ ] Create `src/components/chat/`: `ChatPanel.tsx`, `MessageBubble.tsx`, `PerspectiveSelector.tsx`
- [ ] Create `src/pages/`: all 13 page files
- [ ] Update `tauri.conf.json`: width 1280, height 800, minWidth 1024, minHeight 700

### Step 2 — API Foundation
- [ ] `src/api/types.ts` — all TypeScript interfaces + enums
- [ ] `src/api/client.ts` — BASE_URL, ApiError, apiFetch, apiGet, apiPost, apiPut, apiDelete, apiMultipart
- [ ] `src/api/campaigns.ts` — listCampaigns, createCampaign, getCampaign, updateCampaign, deleteCampaign
- [ ] `src/api/characters.ts` — CRUD + analyzeBackstory
- [ ] `src/api/sessions.ts` — CRUD + start/end + events + summary
- [ ] `src/api/encounters.ts` — CRUD + start/nextTurn/end + updateParticipant + generateEncounter
- [ ] `src/api/documents.ts` — campaign docs + global docs
- [ ] `src/api/chat.ts` — raw fetch for SSE streaming
- [ ] `src/api/health.ts` — getHealth, getVersion

### Step 3 — Routing + Layout
- [ ] `src/main.tsx` — add BrowserRouter wrapper
- [ ] `src/App.tsx` — full route tree with Layout parent
- [ ] `src/components/layout/Layout.tsx` — Sidebar + main content
- [ ] `src/components/layout/Sidebar.tsx` — title, campaign list, global links
- [ ] `src/components/layout/Header.tsx` — campaign name + backend status pill
- [ ] `src/App.css` — reset, CSS vars, layout tokens, HP bar classes

### Step 4 — useApi Hook + CampaignsPage
- [ ] `src/hooks/useApi.ts` — generic data fetcher hook
- [ ] `src/components/common/LoadingSpinner.tsx`
- [ ] `src/components/common/ErrorBanner.tsx`
- [ ] `src/components/common/Modal.tsx` — ReactDOM.createPortal
- [ ] `src/components/common/ConfirmButton.tsx`
- [ ] `src/components/common/FormField.tsx`
- [ ] `src/components/common/Badge.tsx`
- [ ] `src/components/campaigns/CampaignCard.tsx`
- [ ] `src/components/campaigns/CampaignForm.tsx`
- [ ] `src/pages/CampaignsPage.tsx`

### Step 5 — CampaignDetailPage + WorldStateEditor
- [ ] `src/components/campaigns/WorldStateEditor.tsx` — editable world state with tag lists
- [ ] `src/pages/CampaignDetailPage.tsx` — detail + tab nav + WorldStateEditor

### Step 6 — Characters
- [ ] `src/components/characters/ConditionBadge.tsx`
- [ ] `src/components/characters/CharacterForm.tsx` — create form with all fields
- [ ] `src/components/characters/CharacterCard.tsx`
- [ ] `src/components/characters/CharacterList.tsx`
- [ ] `src/components/characters/BackstoryPanel.tsx` — AI analyze button + results
- [ ] `src/pages/CharactersPage.tsx`
- [ ] `src/pages/CharacterDetailPage.tsx` — HP bar, conditions, backstory, inline edit

### Step 7 — Sessions
- [ ] `src/components/sessions/SessionCard.tsx`
- [ ] `src/components/sessions/SessionList.tsx`
- [ ] `src/components/sessions/SessionForm.tsx`
- [ ] `src/components/sessions/SessionEventList.tsx`
- [ ] `src/components/sessions/SessionEventForm.tsx`
- [ ] `src/components/sessions/SummaryView.tsx`
- [ ] `src/pages/SessionsPage.tsx`
- [ ] `src/pages/SessionDetailPage.tsx` — events tab + summary tab + start/end buttons

### Step 8 — Encounters + Combat Tracker
- [ ] `src/components/encounters/EncounterCard.tsx`
- [ ] `src/components/encounters/EncounterList.tsx`
- [ ] `src/components/encounters/EncounterForm.tsx`
- [ ] `src/components/encounters/GenerateEncounterPanel.tsx`
- [ ] `src/components/encounters/ParticipantRow.tsx` — HP bar, conditions, action budget, controls
- [ ] `src/components/encounters/CombatTracker.tsx` — round counter, participant table
- [ ] `src/pages/EncountersPage.tsx`
- [ ] `src/pages/EncounterDetailPage.tsx` — state machine: pending/active/ended

### Step 9 — Documents
- [ ] `src/components/documents/DocumentList.tsx`
- [ ] `src/components/documents/UploadForm.tsx` — multipart, no Content-Type header
- [ ] `src/components/documents/IngestButton.tsx` — polling every 3s until completed/failed
- [ ] `src/pages/DocumentsPage.tsx`
- [ ] `src/pages/GlobalDocumentsPage.tsx`

### Step 10 — Chat (SSE Streaming)
- [ ] `src/hooks/useChat.ts` — fetch + ReadableStream SSE parser, NOT EventSource
- [ ] `src/components/chat/PerspectiveSelector.tsx`
- [ ] `src/components/chat/MessageBubble.tsx`
- [ ] `src/components/chat/ChatPanel.tsx`
- [ ] `src/pages/ChatPage.tsx`

### Step 11 — Health Page + CSS Polish
- [ ] `src/pages/HealthPage.tsx` — health + version display
- [ ] `src/pages/NotFoundPage.tsx`
- [ ] CSS polish: HP bar colors, combat row highlight, badge variants, sidebar active link, responsive

### Step 12 — End-to-End Verification
- [ ] `cargo test --workspace` — 55 Rust tests pass
- [ ] `cargo clippy --workspace -- -D warnings` — zero lint errors
- [ ] `cd guide-frontend && bun run build` — TypeScript compiles clean
- [ ] Manual walkthrough: create campaign → world state → characters → session → encounter → docs → chat → health

---

## Critical Implementation Notes

### SSE Streaming (Chat)
- Use `fetch` + `ReadableStream`, **NOT** `EventSource` — endpoint is POST
- Parse `event:` and `data:` lines from buffer split on `\n\n`
- Call `abortController.abort()` on component unmount

### Multipart Uploads (Documents)
- Use `apiMultipart` — do **NOT** set `Content-Type` header
- Browser auto-sets multipart boundary in Content-Type when using FormData

### Document Ingest Polling
- `setInterval(3000)` polling GET /documents/:id
- Clear interval when status is `completed` or `failed`
- Clear interval on component unmount

### Combat Tracker State
- State owned entirely by `EncounterDetailPage`
- Replace state entirely on each API response (not partial merge)
- Highlight current-turn participant row

### useApi Hook
- deps array triggers refetch via useEffect
- `refetch()` function increments an internal counter to force re-run

### HP Bar Colors
- `>50%` → `--hp-high: #4caf50` (green)
- `25–50%` → `--hp-mid: #ffc107` (yellow)
- `<25%` → `--hp-low: #f44336` (red)

### Conditions (14 values)
Blinded, Charmed, Deafened, Exhausted, Frightened, Grappled, Incapacitated, Invisible, Paralyzed, Petrified, Poisoned, Prone, Restrained, Stunned, Unconscious

### Route Structure
```
/ → CampaignsPage
/campaigns/:campaignId → CampaignDetailPage
/campaigns/:campaignId/characters → CharactersPage
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

---

## Next Steps (Future Work)

- Playstyle profile UI (PlaystyleProfile model exists in backend)
- Dark mode toggle
- Keyboard shortcuts for combat tracker
- Export session summaries to PDF/Markdown
- Multi-campaign sidebar with drag reorder
- Offline mode / cached data
- Tauri system tray icon with quick-access menu
- WebSocket for real-time multi-device sync
