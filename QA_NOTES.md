# QA Notes — The Guide (React + Rust Backend)

**Date:** 2026-03-09
**Backend:** Rust/Axum, port 8000
**Frontend:** React/Vite (bun), port (dev)
**API Spec:** `GET http://localhost:8000/api-docs/openapi.json`

---

## 1. API Endpoint Inventory

### Health
| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Returns `{ status: "ok" }` |
| GET | `/version` | Returns `{ name, version }` |

### Campaigns
| Method | Path | Description |
|--------|------|-------------|
| GET | `/campaigns` | List all campaigns |
| POST | `/campaigns` | Create campaign (`name` required, optional `description`, `game_system`) |
| GET | `/campaigns/{id}` | Get campaign by ID |
| PUT | `/campaigns/{id}` | Update campaign |
| DELETE | `/campaigns/{id}` | Delete campaign → 204 |

### Characters
| Method | Path | Description |
|--------|------|-------------|
| GET | `/campaigns/{cid}/characters` | List characters in campaign |
| POST | `/campaigns/{cid}/characters` | Create character (`name`, `character_type`, `max_hp`, `armor_class` required) |
| GET | `/campaigns/{cid}/characters/{id}` | Get character |
| PUT | `/campaigns/{cid}/characters/{id}` | Update character (all fields optional) |
| DELETE | `/campaigns/{cid}/characters/{id}` | Delete character → 204 |
| POST | `/campaigns/{cid}/characters/{id}/analyze-backstory` | LLM backstory analysis → updated Character |

### Sessions
| Method | Path | Description |
|--------|------|-------------|
| GET | `/campaigns/{cid}/sessions` | List sessions |
| POST | `/campaigns/{cid}/sessions` | Create session (all fields optional) |
| GET | `/campaigns/{cid}/sessions/{id}` | Get session |
| DELETE | `/campaigns/{cid}/sessions/{id}` | Delete session → 204 |
| POST | `/campaigns/{cid}/sessions/{id}/start` | Start session |
| POST | `/campaigns/{cid}/sessions/{id}/end` | End session |
| GET | `/campaigns/{cid}/sessions/{id}/events` | List session events |
| POST | `/campaigns/{cid}/sessions/{id}/events` | Create event (`event_type`, `description` required) |
| GET | `/campaigns/{cid}/sessions/{id}/summary` | LLM-generated summary (`?perspective=dm\|player`) |

### Encounters
| Method | Path | Description |
|--------|------|-------------|
| GET | `/campaigns/{cid}/encounters` | List encounters (`session_id` query param — marked required in spec!) |
| POST | `/campaigns/{cid}/encounters` | Create encounter (`participant_character_ids` required) |
| POST | `/campaigns/{cid}/encounters/generate` | AI-generate encounter suggestion |
| GET | `/campaigns/{cid}/encounters/{id}` | Get encounter |
| DELETE | `/campaigns/{cid}/encounters/{id}` | Delete encounter → 204 |
| POST | `/campaigns/{cid}/encounters/{id}/start` | Start encounter |
| POST | `/campaigns/{cid}/encounters/{id}/next-turn` | Advance to next turn |
| POST | `/campaigns/{cid}/encounters/{id}/end` | End encounter |
| PUT | `/campaigns/{cid}/encounters/{id}/participants/{pid}` | Update participant (HP, conditions, action budget) |

### Documents (Campaign-scoped)
| Method | Path | Description |
|--------|------|-------------|
| GET | `/campaigns/{cid}/documents` | List campaign documents |
| POST | `/campaigns/{cid}/documents` | Upload document (multipart/form-data) |
| GET | `/campaigns/{cid}/documents/{did}` | Get document metadata |
| POST | `/campaigns/{cid}/documents/{did}/ingest` | Start ingestion → 202 Accepted |

### Documents (Global / Rulebooks)
| Method | Path | Description |
|--------|------|-------------|
| GET | `/documents` | List global documents |
| POST | `/documents` | Upload global document (multipart/form-data) |
| GET | `/documents/{did}` | Get global document |
| POST | `/documents/{did}/ingest` | Start ingestion → 202 Accepted |

### Chat
| Method | Path | Description |
|--------|------|-------------|
| POST | `/campaigns/{cid}/chat` | Campaign-aware chat (`message` required; returns SSE stream `text/plain`) |

---

## 2. Frontend Route Inventory

| UI Route | Page Component | Backend Coverage |
|----------|---------------|-----------------|
| `/` | CampaignsPage | `GET/POST /campaigns` |
| `/campaigns/:campaignId` | CampaignDetailPage | `GET /campaigns/{id}` |
| `/campaigns/:campaignId/characters` | CharactersPage | `GET/POST characters` |
| `/campaigns/:campaignId/characters/:charId` | CharacterDetailPage | `GET/PUT/DELETE character`, `analyze-backstory` |
| `/campaigns/:campaignId/sessions` | SessionsPage | `GET/POST sessions` |
| `/campaigns/:campaignId/sessions/:sessionId` | SessionDetailPage | `GET session`, events, summary, start/end |
| `/campaigns/:campaignId/encounters` | EncountersPage | `GET/POST/generate encounters` |
| `/campaigns/:campaignId/encounters/:encId` | EncounterDetailPage | `GET encounter`, start/next-turn/end, update-participant |
| `/campaigns/:campaignId/documents` | DocumentsPage | `GET/POST/ingest campaign documents` |
| `/campaigns/:campaignId/chat` | ChatPage | `POST /chat` (SSE) |
| `/documents` | GlobalDocumentsPage | `GET/POST/ingest global documents` |
| `/playstyle` | PlaystylePage | **NO backend endpoint exists** |
| `/health` | HealthPage | `GET /health`, `GET /version` |
| `/*` | NotFoundPage | — |

---

## 3. Key Findings & Risk Areas

### 3.1 API Spec Bugs / Inconsistencies

| # | Issue | Severity |
|---|-------|----------|
| 1 | `GET /campaigns/{cid}/encounters` — `session_id` query param is marked `required: true` in the OpenAPI spec but the description says "optional filtering". If the frontend omits it, the API may 400. | High |
| 2 | `POST /campaigns/{cid}/chat` returns `text/plain` SSE but spec lists it under `200` response, not a streaming response type. SSE connection handling needs verification. | Medium |
| 3 | `PlaystylePage` (`/playstyle`) route exists in the frontend with no corresponding backend endpoint in the OpenAPI spec. The page will either fail silently or show empty state. | Medium |

### 3.2 Feature Areas to Test

1. **Campaign CRUD** — Create, list, detail view, update, delete.
2. **Character Management** — Create (all 3 types: `pc`, `npc`, `monster`), view, update HP/conditions, delete. Backstory LLM analysis.
3. **Session Lifecycle** — Create → Start → Add Events (all EventTypes & Significance levels) → End → Summary (DM and Player perspectives).
4. **Combat / Encounter Lifecycle** — Create with participants → Start → Next Turn (initiative order) → Update Participant (HP delta, conditions, action budget) → End.
5. **AI Encounter Generation** — POST with optional `context` + `party_level`.
6. **Document Upload & Ingestion** — Campaign-scoped and global; upload multipart, trigger ingest, poll status (`pending → processing → completed/failed`).
7. **Campaign Chat (SSE)** — Send message, verify streaming text response renders incrementally.
8. **Health / Version** — Simple smoke test.
9. **Playstyle Page** — Verify graceful degradation (no backend endpoint).
10. **Error Handling** — 404 for invalid IDs, navigate to `NotFoundPage` (`/*`).

### 3.3 Data Model Notes

- `CharacterType` enum: `pc | npc | monster`
- `Condition` enum (14 D&D 5e conditions): blinded, charmed, deafened, frightened, grappled, incapacitated, invisible, paralyzed, petrified, poisoned, prone, restrained, stunned, unconscious
- `EncounterStatus` enum: `pending | active | completed | fled`
- `EventType` enum: combat, exploration, social, rest, level_up, item_found, npc_met, plot_revealed, custom
- `EventSignificance` enum: `minor | major | milestone`
- `GameSystem` enum: `dnd5e | pathfinder2e`
- `IngestionStatus` enum: `pending | processing | completed | failed`
- `Perspective` enum: `dm | player`
- `HookPriority` enum: `low | medium | high | critical`

---

## 4. Test Execution Plan

### Phase 1 — Smoke Tests (Backend + Frontend baseline)
- [ ] `GET /health` → 200 `{ status: "ok" }`
- [ ] `GET /version` → 200 `{ name, version }`
- [ ] Frontend loads at dev URL, renders CampaignsPage with no JS errors in console
- [ ] Navigate to `/health` → HealthPage renders

### Phase 2 — Campaign Flows
- [ ] Create campaign (valid payload)
- [ ] Create campaign with missing `name` → expect 400/422
- [ ] List campaigns → new campaign appears
- [ ] View campaign detail page
- [ ] Update campaign name + description
- [ ] Delete campaign → removed from list

### Phase 3 — Character Flows
- [ ] Create PC with full payload (name, class, race, ability scores, backstory)
- [ ] Create NPC and Monster with minimal required fields
- [ ] List characters — all 3 appear
- [ ] View CharacterDetailPage — stats render
- [ ] Update HP (PUT with `current_hp`)
- [ ] Apply and remove conditions
- [ ] Trigger `analyze-backstory` (LLM) — verify extracted_hooks appear
- [ ] Delete character → removed from list

### Phase 4 — Session Flows
- [ ] Create session (no title/notes — all optional)
- [ ] Create session with title
- [ ] Start session → `started_at` populated
- [ ] Add events of each `event_type` and `significance`
- [ ] End session → `ended_at` populated
- [ ] Generate DM summary — verify SSE/JSON response
- [ ] Generate player summary (perspective=player) — verify spoiler filtering

### Phase 5 — Encounter / Combat Flows
- [ ] Create encounter with 2+ participant character IDs
- [ ] Verify `GET /encounters` behavior with and without `session_id` param (API bug check)
- [ ] Start encounter → status = `active`
- [ ] Advance turns (next-turn) — verify `current_turn_index` increments
- [ ] Update participant HP via `hp_delta` and `set_hp`
- [ ] Add condition to participant, remove condition
- [ ] Spend action, bonus action, reaction, movement
- [ ] End encounter → status = `completed`
- [ ] Generate encounter via AI (`POST /generate`) with optional context

### Phase 6 — Documents & Ingestion
- [ ] Upload campaign document (PDF) via multipart form
- [ ] Verify document appears in list with `ingestion_status: pending`
- [ ] Trigger ingest → 202 returned
- [ ] Poll document until status = `completed` or `failed`
- [ ] Repeat for global document

### Phase 7 — Chat (SSE)
- [ ] Send chat message within a campaign
- [ ] Verify streaming response renders token-by-token in ChatPage
- [ ] Send with `perspective` and `context_limit` fields

### Phase 8 — Error & Edge Cases
- [ ] Navigate to `/campaigns/invalid-uuid/characters` → expect error state or redirect
- [ ] Navigate to unknown path → NotFoundPage renders
- [ ] `/playstyle` → document behavior (no backend)
- [ ] Delete campaign that has children (characters/sessions/encounters) → verify cascade behavior
- [ ] Create character with negative HP or 0 armor_class — check validation

---

## 5. Environment Assumptions

- Backend running at `http://localhost:8000`
- Frontend dev server running (bun)
- Ollama running locally with default model configured for LLM endpoints
- No Qdrant required (PageIndex fallback active)
- SQLite DB at default path (WAL mode)
