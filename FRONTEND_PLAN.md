Here is Claude's plan:
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
The Guide — React/TypeScript/Tauri Frontend Implementation Plan

Context

The Guide is an AI-powered D&D DM assistant with a fully functional Rust backend (Axum on port 8000). The backend exposes a rich REST
API for campaign management, combat tracking, RAG-grounded chat, PDF ingestion, session logging, and LLM-powered generation. A Tauri
v2 desktop app scaffold exists at guide-frontend/ with React 19 + TypeScript + Bun + Vite — but contains only the default "greet"
demo. This plan builds a functional UI exposing all backend capabilities. Aesthetics are secondary; functionality is primary.

---

Package Additions

Run once before starting:

cd guide-frontend
bun add react-router-dom
bun add -D @types/react-router-dom

No other external dependencies. Use native fetch, useState/useEffect, no Redux/Zustand/React Query.

---

Complete Directory Structure

guide-frontend/src/
main.tsx # wrap in BrowserRouter
App.tsx # all route definitions
App.css # global reset + layout tokens (~150-200 lines)

api/
client.ts # BASE_URL='http://localhost:8000', apiFetch, ApiError, get/post/put/del/fetchMultipart
types.ts # ALL TypeScript interfaces mirroring Rust models
campaigns.ts # campaign CRUD functions
characters.ts # character CRUD + analyze-backstory
sessions.ts # session CRUD + start/end + events + summary
encounters.ts # encounter CRUD + start/next-turn/end + participants + generate
documents.ts # campaign docs + global docs
chat.ts # SSE streaming chat
health.ts # health + version

hooks/
useApi.ts # generic: useApi<T>(fetcher, deps) → {data, loading, error, refetch}
useCampaign.ts # thin wrapper for single campaign fetch by ID
useChat.ts # SSE streaming hook (fetch+ReadableStream, NOT EventSource)

components/
layout/
Sidebar.tsx # left nav: campaign list + global links
Header.tsx # top bar: campaign name + backend status badge
Layout.tsx # Sidebar + Header + <Outlet />

     common/
       LoadingSpinner.tsx
       ErrorBanner.tsx
       ConfirmButton.tsx            # button with inline "are you sure?" confirmation
       Badge.tsx                    # status pill by variant (ingestion, encounter, etc.)
       Modal.tsx                    # generic portal modal
       FormField.tsx                # label + input/select wrapper

     campaigns/
       CampaignCard.tsx
       CampaignForm.tsx             # create/edit (name, description, game_system)
       WorldStateEditor.tsx         # editable location, quests, notes

     characters/
       CharacterList.tsx            # table: name, type, race/class, level, HP bar, AC
       CharacterCard.tsx
       CharacterForm.tsx            # create form (all fields)
       BackstoryPanel.tsx           # raw text + extracted hooks + "Analyze" button
       ConditionBadge.tsx           # pill per D&D condition

     sessions/
       SessionList.tsx
       SessionCard.tsx
       SessionForm.tsx
       SessionEventList.tsx
       SessionEventForm.tsx         # event_type, description, significance, character multi-select
       SummaryView.tsx              # dm/player perspective selector + LLM output

     encounters/
       EncounterList.tsx
       EncounterCard.tsx
       EncounterForm.tsx            # session picker, name, character multi-select
       CombatTracker.tsx            # live combat: round, Next Turn, End Encounter
       ParticipantRow.tsx           # HP bar, conditions, damage/heal buttons, action budget
       GenerateEncounterPanel.tsx   # context + party_level form + result display

     documents/
       DocumentList.tsx
       UploadForm.tsx               # <input type="file"> + POST multipart
       IngestButton.tsx             # triggers ingest, polls every 3s until done/failed

     chat/
       ChatPanel.tsx                # full SSE chat UI
       MessageBubble.tsx            # user / assistant bubbles
       PerspectiveSelector.tsx      # dm / player radio toggle

pages/
CampaignsPage.tsx
CampaignDetailPage.tsx # world state + tab nav to sub-sections
CharactersPage.tsx
CharacterDetailPage.tsx
SessionsPage.tsx
SessionDetailPage.tsx # Events tab + Summary tab
EncountersPage.tsx
EncounterDetailPage.tsx # combat tracker
DocumentsPage.tsx
GlobalDocumentsPage.tsx
ChatPage.tsx
HealthPage.tsx
NotFoundPage.tsx

---

Routing Structure

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

-                                          → NotFoundPage

Use a parent <Route element={<Layout />}> wrapping all campaign routes. Sidebar fetches campaign list independently for the nav.

Also update guide-frontend/src-tauri/tauri.conf.json: set window width: 1280, height: 800, minWidth: 1024, minHeight: 700.

---

Key Implementation Details

API Client (api/client.ts)

export const BASE_URL = 'http://localhost:8000';

export class ApiError extends Error {
constructor(public status: number, message: string) { super(message); }
}

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
const res = await fetch(`${BASE_URL}${path}`, {
headers: { 'Content-Type': 'application/json', ...init?.headers },
...init,
});
if (!res.ok) {
const text = await res.text().catch(() => res.statusText);
throw new ApiError(res.status, text);
}
if (res.status === 204) return undefined as T;
return res.json();
}

For multipart uploads, do NOT set Content-Type — let the browser set the boundary automatically.

SSE Chat Streaming (hooks/useChat.ts)

EventSource CANNOT be used because the chat endpoint requires POST. Use fetch + ReadableStream:

const response = await fetch(`${BASE_URL}/campaigns/${campaignId}/chat`, {
method: 'POST',
headers: { 'Content-Type': 'application/json' },
body: JSON.stringify({ message: text, perspective }),
signal: abortController.signal,
});

const reader = response.body!.getReader();
const decoder = new TextDecoder();
let buffer = '';

while (true) {
const { done, value } = await reader.read();
if (done) break;
buffer += decoder.decode(value, { stream: true });
const parts = buffer.split('\n\n');
buffer = parts.pop() ?? '';
for (const part of parts) {
const eventLine = part.split('\n').find(l => l.startsWith('event:'));
const dataLine = part.split('\n').find(l => l.startsWith('data:'));
const event = eventLine?.slice(7).trim();
const data = dataLine?.slice(5).trim();
if (event === 'token' && data) { /_ append to assistant message _/ }
else if (event === 'done') { /_ finalize _/ }
else if (event === 'error') { /_ show error _/ }
}
}

- Use useRef<string> to accumulate tokens, call setState on each token for live display.
- Abort via abortController.abort() in effect cleanup on unmount.

Combat Tracker State

EncounterDetailPage owns EncounterSummary | null state locally. Every action (start, next-turn, update-participant) replaces state
with the API response. No polling needed — all mutations return updated state.

Document Ingest Polling

IngestButton uses setInterval(3000) to call getDocument() until status is completed or failed. Clean up interval on unmount.

State Management

No global store. Each page uses useApi<T> for fetching. Mutations call the API directly in event handlers, then either refetch() or
update local state with the returned entity.

---

All Backend API Endpoints

┌────────────────┬──────────────────────────────────────────────────────────────┬─────────────────────────────┐
│ Method │ Path │ Purpose │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET │ /health │ Health check │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET │ /version │ Version info │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/POST │ /campaigns │ List/create campaigns │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/PUT/DELETE │ /campaigns/{id} │ Get/update/delete campaign │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/POST │ /campaigns/{id}/characters │ List/create characters │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/PUT/DELETE │ /campaigns/{id}/characters/{cid} │ Get/update/delete character │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/characters/{cid}/analyze-backstory │ LLM backstory analysis │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/POST │ /campaigns/{id}/sessions │ List/create sessions │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/DELETE │ /campaigns/{id}/sessions/{sid} │ Get/delete session │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/sessions/{sid}/start │ Start session │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/sessions/{sid}/end │ End session │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/POST │ /campaigns/{id}/sessions/{sid}/events │ List/create session events │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET │ /campaigns/{id}/sessions/{sid}/summary?perspective=dm|player │ LLM session summary │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/POST │ /campaigns/{id}/encounters │ List/create encounters │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/DELETE │ /campaigns/{id}/encounters/{eid} │ Get/delete encounter │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/encounters/{eid}/start │ Start combat │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/encounters/{eid}/next-turn │ Advance turn │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/encounters/{eid}/end │ End combat │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ PUT │ /campaigns/{id}/encounters/{eid}/participants/{pid} │ Update HP/conditions │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/encounters/generate │ LLM encounter generation │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/POST │ /campaigns/{id}/documents │ List/upload campaign PDFs │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET │ /campaigns/{id}/documents/{did} │ Get document status │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/documents/{did}/ingest │ Start ingestion │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET/POST │ /documents │ List/upload global docs │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ GET │ /documents/{did} │ Get global doc status │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /documents/{did}/ingest │ Start global doc ingestion │
├────────────────┼──────────────────────────────────────────────────────────────┼─────────────────────────────┤
│ POST │ /campaigns/{id}/chat │ SSE streaming chat with RAG │
└────────────────┴──────────────────────────────────────────────────────────────┴─────────────────────────────┘

---

Implementation Task Sequence

Task 1 — Install dependencies + scaffold empty files

- bun add react-router-dom && bun add -D @types/react-router-dom
- Create all directory structure with empty placeholder exports
- Verify bun run dev still starts

Task 2 — API foundation (api/types.ts + api/client.ts)

- All TypeScript types mirroring every Rust model
- apiFetch, ApiError, get/post/put/del/fetchMultipart

Task 3 — Routing skeleton + Layout

- main.tsx: wrap in <BrowserRouter>
- App.tsx: all routes wired to stub pages
- Layout.tsx, Sidebar.tsx, Header.tsx: minimal skeletons
- Update tauri.conf.json window size

Task 4 — useApi hook + CampaignsPage

- hooks/useApi.ts
- Common components: Modal, ErrorBanner, LoadingSpinner
- CampaignCard, CampaignForm, CampaignsPage
- Verify: create campaign, see it listed

Task 5 — CampaignDetailPage + WorldStateEditor

- World state inline editing with Save button

Task 6 — Characters

- api/characters.ts
- CharactersPage, CharacterDetailPage, CharacterForm, CharacterList, BackstoryPanel, ConditionBadge
- Verify: create character, view detail, analyze backstory (LLM)

Task 7 — Sessions

- api/sessions.ts
- SessionsPage, SessionDetailPage, SessionForm, SessionEventList, SessionEventForm, SummaryView, PerspectiveSelector
- Verify: create session, start it, add events, generate summary

Task 8 — Encounters + Combat Tracker

- api/encounters.ts (includes generate endpoint)
- EncountersPage, EncounterDetailPage, EncounterForm, CombatTracker, ParticipantRow, GenerateEncounterPanel
- Verify: create encounter, start combat, advance turns, apply damage/healing

Task 9 — Documents

- api/documents.ts
- DocumentsPage, GlobalDocumentsPage, DocumentList, UploadForm, IngestButton, Badge
- Verify: upload PDF, trigger ingest, poll until completed/failed

Task 10 — Chat (SSE Streaming)

- api/chat.ts
- hooks/useChat.ts (fetch+ReadableStream SSE)
- ChatPage, ChatPanel, MessageBubble
- Verify: send message, watch tokens stream in, switch perspective

Task 11 — Health page + CSS polish

- api/health.ts, HealthPage
- Polish App.css: HP bar colors (green/yellow/red), combat row highlighting, badge variants, sidebar nav styles

Task 12 — End-to-end smoke test

- Start backend: cargo run -p guide-api
- Start frontend: bun run tauri dev
- Walk: create campaign → add characters → start session → add events → create encounter → run combat → upload doc → ingest → chat →
  view summary

---

Critical Files

┌────────────────────────────────────────────────────────────┬───────────────────────────────────────────────┐
│ File │ Importance │
├────────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
│ guide-frontend/src/api/types.ts │ Foundation — every other file depends on this │
├────────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
│ guide-frontend/src/api/client.ts │ Core HTTP layer │
├────────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
│ guide-frontend/src/hooks/useChat.ts │ Most complex — POST-based SSE streaming │
├────────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
│ guide-frontend/src/App.tsx │ All route definitions │
├────────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
│ guide-frontend/src/components/encounters/CombatTracker.tsx │ Highest-complexity component │
├────────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
│ guide-frontend/src-tauri/tauri.conf.json │ Window size update │
└────────────────────────────────────────────────────────────┴───────────────────────────────────────────────┘

---

Verification

End-to-end test requires:

1.  Backend running: cargo run -p guide-api (port 8000)
2.  Frontend dev: bun run tauri dev (Tauri desktop window)
3.  All 12 task verification steps completed in sequence
