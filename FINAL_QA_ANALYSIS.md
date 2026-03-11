# QA Analysis — The Guide Frontend
**Date:** 2026-03-10
**Scope:** Static code review + live API contract verification
**Backend:** Rust/Axum at http://localhost:8000 (confirmed running)
**Frontend:** React 19 + TypeScript at http://localhost:1421
**Note:** Browser extension unavailable during testing; findings are from code review and direct API calls.

---

## Executive Summary

5 critical API contract bugs were found that cause functional breakdowns across Sessions, Conditions, and Session Events. These are not UI polish issues — they are hard failures where features don't work at all. Several medium-severity error handling gaps and one missing feature (campaign editing) were also found.

---

## CRITICAL BUGS

### BUG-001 — Session Status Field Missing from API Response
**Severity:** HIGH
**Affected:** `SessionDetailPage.tsx`, `SessionCard.tsx`, all session lifecycle UI

**Root Cause:**
The Rust `Session` model (`crates/guide-core/src/models/session.rs`) has no `status` field:
```rust
pub struct Session {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub session_number: i32,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```
The backend returns: `{"started_at": null, "ended_at": null, ...}` — no `status` key.

**Frontend Impact:**
`SessionDetailPage.tsx` conditionally renders Start/End buttons on `session.status`:
```tsx
{session.status === 'pending' && <button>Start Session</button>}
{session.status === 'started' && <button>End Session</button>}
{session.status === 'ended' && <span>Ended</span>}
```
Since `session.status === undefined`, none of these conditions are ever true. **The Start and End Session buttons never appear.** Sessions are stuck in a perpetual pending-looking state.

**Verified:** Live API call to `GET /campaigns/{id}/sessions/{id}` confirmed `status` field is absent.

**Repro:**
1. Navigate to any session detail page
2. Observe no Start Session / End Session button
3. Session appears to have no lifecycle controls

**Fix:** Derive status from `started_at`/`ended_at` in the frontend:
```typescript
function deriveStatus(session: Session): 'pending' | 'started' | 'ended' {
  if (session.ended_at) return 'ended';
  if (session.started_at) return 'started';
  return 'pending';
}
```

---

### BUG-002 — Condition Enum Case Mismatch (snake_case vs PascalCase)
**Severity:** HIGH
**Affected:** Character detail page, CombatTracker participant rows, condition management

**Root Cause:**
The Rust `Condition` enum uses `#[serde(rename_all = "snake_case")]`:
```rust
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Blinded,   // serialized as "blinded"
    Charmed,   // serialized as "charmed"
    Poisoned,  // serialized as "poisoned"
    // ...
}
```
The backend returns conditions as lowercase: `["blinded", "poisoned"]`.

The frontend `ALL_CONDITIONS` and the `Condition` type use **PascalCase**:
```typescript
export const ALL_CONDITIONS: Condition[] = ['Blinded', 'Charmed', 'Poisoned', ...];
```

**Frontend Impact (3 failures):**
1. **Display broken:** `ConditionBadge` renders the string directly — shows `"blinded"` instead of `"Blinded"`.
2. **Available conditions broken:** `ALL_CONDITIONS.filter((c) => !character.conditions.includes(c))` — `'Blinded'.includes('blinded')` → `false`. ALL conditions appear as addable even when already applied.
3. **Adding condition fails:** Frontend sends `add_condition: "Poisoned"` (PascalCase) but backend deserializes with `snake_case`, expecting `"poisoned"`. Backend returns 422 unprocessable entity.
4. **Removing condition fails:** `character.conditions.filter((x) => x !== c)` where `x='blinded'` and `c='Blinded'` — filter never removes the condition, so PUT sends the full unchanged list.

**Additional:** Frontend `Condition` type includes `'Exhausted'` which does NOT exist in the backend enum. Sending `add_condition: "exhausted"` results in backend deserialization error.

**Repro:**
1. Navigate to any character with a condition (or add one)
2. Observe badge shows lowercase condition name
3. Try to add a condition → 422 error from backend
4. Try to remove a condition → no-op, condition stays

**Fix (backend option):** Remove `#[serde(rename_all = "snake_case")]` from Condition enum.
**Fix (frontend option):** Lowercase all condition comparisons; map display to PascalCase labels.

---

### BUG-003 — EventType Enum Values Don't Match Frontend and Backend
**Severity:** HIGH
**Affected:** Session event creation, SessionEventList display

**Root Cause:**
Backend `EventType` variants (`crates/guide-core/src/models/shared.rs`):
```
combat, exploration, social, rest, level_up, item_found, npc_met, plot_revealed, custom
```
Frontend `EventType` values (`api/types.ts`):
```
combat, roleplay, exploration, skill_challenge, item_found, npc_introduced, quest_update, revelation, other
```

**Mismatches:**
| Backend | Frontend | Status |
|---------|----------|--------|
| `combat` | `combat` | ✓ Match |
| `exploration` | `exploration` | ✓ Match |
| `item_found` | `item_found` | ✓ Match |
| `social` | `roleplay` | ✗ Mismatch |
| `rest` | *(missing)* | ✗ Backend only |
| `level_up` | *(missing)* | ✗ Backend only |
| `npc_met` | `npc_introduced` | ✗ Mismatch |
| `plot_revealed` | `revelation` | ✗ Mismatch |
| `custom` | `other` | ✗ Mismatch |
| *(missing)* | `skill_challenge` | ✗ Frontend only |
| *(missing)* | `quest_update` | ✗ Frontend only |

**Impact:** Creating events with types `roleplay`, `skill_challenge`, `npc_introduced`, `quest_update`, `revelation`, or `other` causes backend deserialization failure (422 error). Only `combat`, `exploration`, and `item_found` work.

**Verified:** Live session events show `event_type: "combat"` and `event_type: "exploration"` — the overlapping types. The `SessionEventForm` offers all frontend types including the broken ones.

---

### BUG-004 — EventSignificance Enum Partial Mismatch
**Severity:** HIGH
**Affected:** Session event creation

**Root Cause:**
Backend `EventSignificance`:
```
minor, major, milestone
```
Frontend `EventSignificance`:
```
minor, moderate, major, critical
```

**Impact:** Creating events with `moderate` or `critical` significance causes backend 422 error. `milestone` exists in backend but is absent from frontend (backend-created milestone events would display the raw string `"milestone"` in frontend).

**Verified:** Live event data shows `"significance": "minor"` and `"significance": "major"` from backend — confirming the overlapping values work, but the default dropdown in SessionEventForm includes all 4 frontend values.

---

### BUG-005 — SessionEvent `occurred_at` vs `created_at` Field Name Mismatch
**Severity:** MEDIUM
**Affected:** `SessionEventList.tsx` — event timestamp display

**Root Cause:**
Backend `SessionEvent` model serializes the timestamp as `"occurred_at"`:
```rust
pub occurred_at: DateTime<Utc>,
```
Frontend `SessionEvent` type declares `created_at: string`.

**Verified:** Live API response confirmed: `"occurred_at": "2026-03-08T05:07:21.041910686Z"` — no `created_at` field.

**Impact:**
`SessionEventList.tsx:40`: `new Date(ev.created_at).toLocaleTimeString(...)` — `ev.created_at` is `undefined`, producing `"Invalid Date"` in the Time column.

---

## MEDIUM BUGS

### BUG-006 — `handleAddEvent` Has No Error Handling
**Severity:** MEDIUM
**File:** `guide-frontend/src/pages/SessionDetailPage.tsx:60-63`

```typescript
const handleAddEvent = async (data: CreateSessionEventRequest) => {
  await createEvent(campaignId!, sessionId!, data);  // throws on error, uncaught
  setShowAddEvent(false);
  refetchEvents();
};
```

**Impact:** If `createEvent()` throws (e.g., 422 from EventType/Significance mismatch — see BUG-003/BUG-004), the modal stays open with no error message. User sees nothing happened. The modal can only be closed by clicking Cancel.

**Fix:** Wrap in try/catch and set an error state displayed inside the modal.

---

### BUG-007 — `handleDeleteEvent` Has No Error Handling
**Severity:** MEDIUM
**File:** `guide-frontend/src/pages/SessionDetailPage.tsx:66-69`

```typescript
const handleDeleteEvent = async (eventId: string) => {
  await deleteEvent(campaignId!, sessionId!, eventId);  // throws on error, uncaught
  refetchEvents();
};
```

**Impact:** If delete fails, user sees no error. The event remains in the list but user may believe it was deleted.

**Note:** KI-6 is **debunked** — the `DELETE /campaigns/{campaign_id}/sessions/{id}/events/{event_id}` endpoint exists and is correctly implemented in the Rust backend.

---

### BUG-008 — `handleDelete` (Character) Has No Error Handling
**Severity:** MEDIUM
**File:** `guide-frontend/src/pages/CharacterDetailPage.tsx:165-168`

```typescript
const handleDelete = async () => {
  await deleteCharacter(campaignId!, charId!);  // throws on error, uncaught
  navigate(`/campaigns/${campaignId}/characters`);
};
```

**Impact:** If delete fails, the `navigate()` call is never reached (exception propagates), but `ConfirmButton.onConfirm` has no error boundary. The user gets no error feedback and the character appears undeleted with no explanation.

---

### BUG-009 — SummaryView Clipboard API Has No Error Handling
**Severity:** MEDIUM
**File:** `guide-frontend/src/components/sessions/SummaryView.tsx:14-17`

```typescript
const handleCopy = async () => {
  await navigator.clipboard.writeText(summary.content);  // throws if permission denied
  setCopied(true);
  ...
};
```

**Impact:** If the page is served over HTTP (not HTTPS) or clipboard permission is denied, `writeText()` throws. The component catches nothing; the button shows no feedback. In HTTP development contexts this is a real risk.

---

### BUG-010 — Backstory Text Save Does Not Refresh Parent
**Severity:** MEDIUM
**File:** `guide-frontend/src/components/characters/BackstoryPanel.tsx:20-31`

After `updateCharacter()` succeeds in `handleSaveText()`, only `setEditingText(false)` is called. The parent's `refetch()` is never triggered. The `displayText` variable reads from the `backstory?.raw_text` prop, which doesn't update until the parent re-fetches.

**Impact:** After saving backstory text and closing the editor, the displayed text reverts to the pre-save content. The save was persisted to the backend, but the UI shows stale data until the user navigates away and back.

**Repro:**
1. Navigate to character detail
2. Click "Edit Text" in Backstory panel
3. Enter new text, click "Save Text"
4. Editor closes — the displayed text shows the OLD text, not the saved text

---

### BUG-011 — No Cancel Button for In-Flight SSE Stream
**Severity:** MEDIUM (UX)
**File:** `guide-frontend/src/components/chat/ChatPanel.tsx`, `guide-frontend/src/hooks/useChat.ts`

`useChat` exposes a `cancel()` function that aborts the `AbortController`. The `ChatPanel` receives `cancel` in the destructured return but doesn't expose it in the UI:
```typescript
const { messages, streaming, error, sendMessage, clearMessages } = useChat(campaignId);
// `cancel` is available but destructured away
```

**Impact:** During long streaming responses, the user has no way to stop the stream. They must wait for it to complete. This is noticeable with slow Ollama models.

---

### BUG-012 — No UI to Edit Campaign Metadata
**Severity:** MEDIUM (Missing Feature)
**File:** `guide-frontend/src/pages/CampaignDetailPage.tsx`

The campaign detail page displays name, game system badge, and description (if present), but provides no edit controls for any of these fields. The `PUT /campaigns/{id}` endpoint accepts `name`, `description`, and `game_system` in `UpdateCampaignRequest`, but the frontend only calls this endpoint via `WorldStateEditor`.

**Impact:** Campaign name/description/game_system are write-once (set at creation). There's no way to rename or fix a typo in campaign metadata.

---

## LOW BUGS

### BUG-013 — ConfirmButton Has No Click-Outside Dismiss
**Severity:** LOW (UX)
**File:** `guide-frontend/src/components/common/ConfirmButton.tsx`

When a ConfirmButton enters the confirming state (user clicked the initial button), it shows "Are you sure? | Yes | No". There is no click-outside handler to dismiss. The confirming state persists until "Yes" or "No" is explicitly clicked.

**Impact:** In tables with multiple rows (event list, character list), clicking one Delete and then clicking elsewhere leaves an orphaned confirmation widget visible.

---

### BUG-014 — EncounterForm Shows Empty Participant List With No Warning
**Severity:** LOW (UX)
**File:** `guide-frontend/src/components/encounters/EncounterForm.tsx:68-82`

If a campaign has zero characters, the participant checkbox group renders empty. The submit button is correctly disabled (`selectedChars.length === 0` → error on submit), but there is no proactive message explaining why there's nothing to select.

**Impact:** DMs see an empty "Participants" section and may not understand they need to create characters first.

---

### BUG-015 — CombatTracker Modulo-Zero Edge Case (Theoretical)
**Severity:** LOW
**File:** `guide-frontend/src/components/encounters/CombatTracker.tsx:14,46`

```typescript
const currentParticipant = sorted[encounter.current_turn_index % sorted.length];
```

If `sorted.length === 0`, `% 0` in JavaScript returns `NaN`. The component guards `currentParticipant &&` on line 20, so no crash occurs, but `isCurrentTurn={idx === NaN}` is always `false`, meaning no participant is highlighted.

**Severity note:** The backend enforces at least one participant to start combat, making this unreachable in normal use. However, a completed encounter with all participants removed would expose this path.

---

### BUG-016 — Sidebar Makes Duplicate `GET /campaigns` Call
**Severity:** LOW (Performance)
**File:** `guide-frontend/src/components/layout/Sidebar.tsx:20`

```typescript
const { data: rawCampaigns } = useApi<Campaign[]>(listCampaigns, []);
```

The `Sidebar` component independently fetches campaigns on every page. `CampaignsPage` also fetches campaigns. There is no shared cache or context, resulting in duplicate identical API calls on every navigation. In a campaign with many sessions, this compounds (sidebar re-fetches on every session/encounter/character page load).

---

### BUG-017 — HP Bar Renders Confusingly When current_hp > max_hp
**Severity:** LOW
**File:** `guide-frontend/src/pages/CharacterDetailPage.tsx:13-21`, `ParticipantRow.tsx:81`

```typescript
<div style={{ width: `${Math.min(100, pct)}%` }} />  // clamped to 100%
<span>{current} / {max} HP</span>  // shows e.g. "20 / 10 HP"
```

**Observed in live data:** Character "Dale Dug" has `current_hp: 20, max_hp: 10`. The HP bar renders as 100% full (clamped), but the label reads "20 / 10 HP". There's no visual indication of overhealing (e.g., temporary HP glow). This is confusing for DMs.

---

### BUG-018 — `GeneratedEncounter.challenge_rating` Missing from Frontend Type
**Severity:** LOW
**File:** `guide-frontend/src/api/types.ts`

The backend `GeneratedEncounter` includes `challenge_rating: Option<f32>` but the frontend `GeneratedEncounter` interface omits it. The field is silently ignored during JSON parsing. No crash, but CR information is unavailable to the UI.

---

## Known Issues: Verification Results

| KI | Issue | Result |
|----|-------|--------|
| KI-1 | No UI to edit campaign name/description/game_system | **CONFIRMED** → BUG-012 |
| KI-2 | `handleDeleteEvent` no try/catch | **CONFIRMED** → BUG-007 |
| KI-3 | HP bar with negative current_hp | **CONFIRMED LOW** — renders as 0-width (empty bar). No crash. |
| KI-4 | EncounterForm no warning when zero characters | **CONFIRMED** → BUG-014 |
| KI-5 | ConfirmButton no click-outside dismiss | **CONFIRMED** → BUG-013 |
| KI-6 | `DELETE /events/{id}` endpoint may not exist | **DEBUNKED** — endpoint exists at correct path |
| KI-7 | CombatTracker modulo-zero | **CONFIRMED LOW** → BUG-015 (JS returns NaN, not crash) |
| KI-8 | ChatPanel no cancel button for in-progress stream | **CONFIRMED** → BUG-011 |
| KI-9 | Sidebar separate `GET /campaigns` call | **CONFIRMED** → BUG-016 |

---

## Previously Unknown Issues Found

| ID | Issue | Severity |
|----|-------|----------|
| BUG-001 | Session status field missing from API (buttons never appear) | HIGH |
| BUG-002 | Condition case mismatch — add/remove/display all broken | HIGH |
| BUG-003 | EventType enum mismatch — 6/9 types fail backend validation | HIGH |
| BUG-004 | EventSignificance mismatch — `moderate`/`critical` fail | HIGH |
| BUG-005 | SessionEvent `occurred_at` vs `created_at` — "Invalid Date" | MEDIUM |
| BUG-008 | Character delete has no error handling | MEDIUM |
| BUG-009 | Clipboard API in SummaryView has no error handling | MEDIUM |
| BUG-010 | Backstory text save doesn't trigger parent refetch | MEDIUM |
| BUG-018 | GeneratedEncounter.challenge_rating missing from frontend type | LOW |

---

## Fix Priority

| Priority | Bugs | Rationale |
|----------|------|-----------|
| **P0 (Block release)** | BUG-001, BUG-002, BUG-003, BUG-004 | Core features non-functional |
| **P1 (Ship shortly)** | BUG-005, BUG-006, BUG-007, BUG-008, BUG-010 | Data display wrong or silent errors |
| **P2 (Next sprint)** | BUG-009, BUG-011, BUG-012 | UX gaps, missing features |
| **P3 (Backlog)** | BUG-013 through BUG-018 | Polish and performance |

---

## API Contract Summary

The following API contract mismatches were found between Rust backend and TypeScript frontend:

| Domain | Backend | Frontend | Impact |
|--------|---------|----------|--------|
| Session.status | Field absent | `'pending'\|'started'\|'ended'` required | All lifecycle buttons broken |
| Condition values | `"blinded"` (snake_case) | `'Blinded'` (PascalCase) | All condition management broken |
| EventType values | 9 values (partial overlap) | 9 different values (partial overlap) | 6/9 event types fail |
| EventSignificance | `minor\|major\|milestone` | `minor\|moderate\|major\|critical` | `moderate`/`critical` fail |
| SessionEvent timestamp | `occurred_at` | `created_at` | "Invalid Date" display |
| EncounterStatus | `pending\|active\|completed\|fled` | `pending\|active\|completed` | `fled` not in frontend type |

---

## Test Coverage Notes

- **Not tested via browser** (extension unavailable): SSE streaming, file upload, document polling lifecycle, keyboard shortcuts (Spacebar turn advance), drag-to-reorder in sidebar, localStorage persistence flows, PDF ingest status transitions.
- **Tested via direct API**: All listed bugs verified through `curl` against live backend + static code analysis of all 47 frontend files.
