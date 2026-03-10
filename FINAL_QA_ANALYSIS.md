# Final QA Analysis — The Guide Frontend
**Date:** 2026-03-09
**Method:** Full static code review (backend Rust + frontend TypeScript/React)
**Backend:** Rust/Axum at http://localhost:8000
**Frontend:** React/Vite (bun) at http://localhost:1421
**Scope:** All 14 pages, all API client files, all shared components, backend route handlers

---

## Executive Summary

The frontend has **5 critical type mismatches** between Rust backend structs and TypeScript interface declarations that will silently break backstory display, combat participant rendering, and hook priority filtering at runtime. Additionally, several medium-severity UI bugs exist around action budget tracking, document ingestion callbacks, and encounter generation types. The playstyle profile is fully disconnected from the backend (localStorage only). UX issues are generally minor. There are no XSS or security concerns.

---

## Bug Reports

### BUG-001 — `PlotHook` frontend interface completely mismatches backend struct
**Severity:** Critical
**File:** `guide-frontend/src/api/types.ts:91–95` vs `crates/guide-core/src/models/character.rs:58–66`

**Backend sends:**
```json
{ "id": "uuid", "character_id": "uuid", "description": "...", "priority": "high", "is_active": true, "llm_extracted": true }
```

**Frontend expects (`types.ts:91`):**
```typescript
interface PlotHook { summary: string; priority: HookPriority; related_npcs: string[]; }
```

**Impact:** `BackstoryPanel` renders `hook.summary` which is always `undefined` (backend sends `hook.description`). All plot hooks from `analyze-backstory` appear blank. `related_npcs` is always `undefined` (field doesn't exist in backend).

**Steps to reproduce:** Create a character with backstory text, trigger "Analyze with AI" — the extracted plot hooks section will render with empty summaries.

**Fix:** Update `PlotHook` in `types.ts` to match the backend struct.

---

### BUG-002 — `Backstory.hooks` field name doesn't match backend `extracted_hooks`
**Severity:** Critical
**File:** `guide-frontend/src/api/types.ts:97–103` vs `crates/guide-core/src/models/character.rs:49–56`

**Backend sends:**
```json
{ "raw_text": "...", "extracted_hooks": [...], "motivations": [...], "key_relationships": [...], "secrets": [...] }
```

**Frontend expects:**
```typescript
interface Backstory { raw_text: string | null; hooks: PlotHook[]; ... }
```

**Impact:** `backstory.hooks` is always `undefined` because the backend field is named `extracted_hooks`. The backstory analysis panel renders no hooks even when analysis succeeds. Additionally, backend `raw_text` is `String` (not nullable), so the `| null` check is incorrect but harmless.

**Fix:** Rename `hooks` → `extracted_hooks` in the `Backstory` interface in `types.ts`.

---

### BUG-003 — `CombatParticipant.initiative_bonus` vs backend `initiative_modifier`
**Severity:** Critical
**File:** `guide-frontend/src/api/types.ts:138` vs `crates/guide-core/src/models/encounter.rs:29`

**Backend sends field:** `initiative_modifier`
**Frontend declares:** `initiative_bonus: number`

**Impact:** `participant.initiative_bonus` is always `undefined` at runtime. The CombatTracker renders `p.initiative_total` correctly (field name matches), but `initiative_bonus` is displayed as `undefined` in any view that uses it, and the type system provides false confidence.

**Fix:** Rename `initiative_bonus` → `initiative_modifier` in `CombatParticipant` in `types.ts`.

---

### BUG-004 — `CombatParticipant.is_active` vs backend `is_defeated` (inverted logic)
**Severity:** Critical
**File:** `guide-frontend/src/api/types.ts:145` vs `crates/guide-core/src/models/encounter.rs:37`

**Backend sends:** `is_defeated: bool` (true when HP ≤ 0)
**Frontend declares:** `is_active: boolean`

**Impact:** `participant.is_active` is always `undefined` because the backend sends `is_defeated`. Any defeated/alive styling or filtering in the combat tracker will silently fail. The two fields also have **inverted semantics** — a component checking `is_active === false` to dim a row would need to check `is_defeated === true` instead.

**Fix:** Replace `is_active: boolean` with `is_defeated: boolean` in `CombatParticipant` in `types.ts`.

---

### BUG-005 — `HookPriority` type missing `'critical'` value
**Severity:** High
**File:** `guide-frontend/src/api/types.ts:60` vs `crates/guide-core/src/models/character.rs:68–75`

**Backend enum values:** `low | medium | high | critical`
**Frontend type:** `type HookPriority = 'low' | 'medium' | 'high'`

**Impact:** If the backend returns a hook with `priority: 'critical'`, TypeScript will treat it as an invalid enum member at type-check time, and any switch/conditional that handles hook priority will not have a case for `'critical'`, silently falling through or showing no styling.

**Fix:** Add `'critical'` to the `HookPriority` union type.

---

### BUG-006 — `GeneratedEncounterType` missing `'mixed'` variant
**Severity:** High
**File:** `guide-frontend/src/api/types.ts:58` vs `crates/guide-core/src/models/encounter.rs:111–119`

**Backend enum values:** `combat | social | exploration | puzzle | mixed`
**Frontend type:** `'combat' | 'social' | 'exploration' | 'puzzle'`

**Impact:** If the LLM generates a `mixed` encounter type (a valid backend variant), the frontend type is invalid. Any icon/badge rendering that switches on `encounter_type` will have no matching case.

**Fix:** Add `'mixed'` to `GeneratedEncounterType`.

---

### BUG-007 — Action budget checkboxes silently do nothing (backend handler ignores `spend_*` fields)
**Severity:** High
**File:** `guide-frontend/src/components/encounters/ParticipantRow.tsx:124–137` vs `crates/guide-api/src/routes/encounters.rs:246–268`

**What the UI does:** Lines 126, 130, 134 render checkboxes for Action/Bonus/Reaction and call `doUpdate({ spend_action: bool })` etc.

**What the backend does:** The `update_participant` handler at `encounters.rs:246` processes `hp_delta`, `set_hp`, `add_condition`, `remove_condition`, and `name` only. There is **no code** to handle `spend_action`, `spend_bonus_action`, or `spend_reaction`.

**Impact:** Clicking the Action/Bonus/Reaction checkboxes sends a valid API request which the backend silently ignores. The UI checkbox state appears to toggle but resets when the page re-renders from the server response. Spend movement (`spend_movement`) is also accepted in the request type but never processed.

**Steps to reproduce:** Start an encounter, click the "Action" checkbox for a participant — it reverts immediately on next render.

**Fix:** Implement `spend_action`, `spend_bonus_action`, `spend_reaction`, and `spend_movement` in the Rust `update_participant` handler to call the CombatEngine equivalent methods.

---

### BUG-008 — `GlobalDocumentsPage` missing `onComplete` callback causes stale ingestion status
**Severity:** Medium
**File:** `guide-frontend/src/pages/GlobalDocumentsPage.tsx:36–41` vs `guide-frontend/src/pages/DocumentsPage.tsx:40–46`

**DocumentsPage (correct):**
```tsx
<IngestButton ... onComplete={refetch} />
```

**GlobalDocumentsPage (missing):**
```tsx
<IngestButton ... onPoll={() => getGlobalDoc(d.id)} />
// onComplete is absent
```

**Impact:** When a global document's ingestion completes, `IngestButton` calls `onComplete?.()` which is `undefined`. The page's document list is never refetched. The "Ingested ✓" badge will appear in the button, but other document metadata (like `ingested_at`) won't update in the list table until the user manually refreshes.

**Fix:** Add `onComplete={refetch}` to the `IngestButton` in `GlobalDocumentsPage.tsx`.

---

### BUG-009 — `CreateEncounterRequest.name` required in frontend form but optional in backend
**Severity:** Medium
**File:** `guide-frontend/src/api/types.ts:281–286` vs `crates/guide-core/src/models/encounter.rs:67–72`

**Backend:** `name: Option<String>` (optional)
**Frontend request type:** `name: string` (required, no `?`)

**Impact:** The frontend `CreateEncounterRequest` forces callers to supply a `name`. This is stricter than the backend requires — if any form omits the name, TypeScript compilation would fail (not a runtime bug). But it also means the form cannot be submitted without a name even though the backend would happily accept a nameless encounter. The `EncounterDetailPage.tsx:61` renders `displayed.name` directly — if the backend allows null names and a nameless encounter is created via API, the page renders `null` literally in the `<h1>`.

**Fix:** Change `name: string` to `name?: string` in `CreateEncounterRequest`. Add a null guard in `EncounterDetailPage` for `displayed.name`.

---

### BUG-010 — `doUpdate` in `CharacterDetailPage` swallows errors silently
**Severity:** Medium
**File:** `guide-frontend/src/pages/CharacterDetailPage.tsx:130–138`

```typescript
const doUpdate = async (changes) => {
  setUpdating(true);
  try {
    await updateCharacter(campaignId!, charId!, changes);
    refetch();
  } finally {
    setUpdating(false);
  }
};
```

**Impact:** The `try/finally` block has no `catch`. If `updateCharacter` throws (network error, 422, 500), the error is silently swallowed. The spinner stops, `refetch()` is skipped, and the UI shows the old stale data with no error banner. The user has no idea the HP change or condition update failed.

**Fix:** Add a `catch` block that sets an error state and displays an `ErrorBanner`.

---

### BUG-011 — `ParticipantRow.tsx` error in `doUpdate` is logged but not surfaced to user
**Severity:** Medium
**File:** `guide-frontend/src/components/encounters/ParticipantRow.tsx:32–33`

```typescript
} catch (e) {
  console.error(e);
}
```

**Impact:** If a participant update fails (e.g., invalid HP value, network error), the participant row shows no feedback. The `loading` spinner stops and the row returns to its previous state without any error message. DMs have no way to know if a damage/heal action was recorded.

**Fix:** Add an error state to `ParticipantRow` and display an inline error message when `doUpdate` fails.

---

### BUG-012 — Session summary endpoint returns 400 if session has no events
**Severity:** Medium
**File:** `crates/guide-api/src/routes/sessions.rs:257–260`

```rust
if events.is_empty() {
    return Err(GuideError::InvalidInput("Session has no events to summarize".into()).into());
}
```

**Impact:** Clicking "Generate Summary" on a session with no events shows a raw error banner `"Session has no events to summarize"`. The `summaryError` state shows this correctly, but the UX is poor — the Generate button should be disabled or show a tooltip when no events exist, rather than letting the user attempt an impossible action.

**Fix:** In `SessionDetailPage`, disable the "Generate Summary" button when `events` array is empty, and show a hint: "Add events before generating a summary."

---

### BUG-013 — `CombatTracker` turn-index modulo is stale after participant re-sort
**Severity:** Medium
**File:** `guide-frontend/src/components/encounters/CombatTracker.tsx:12,44`

```typescript
const sorted = [...encounter.participants].sort((a, b) => b.initiative_total - a.initiative_total);
const currentParticipant = sorted[encounter.current_turn_index % sorted.length];
// ...
isCurrentTurn={idx === encounter.current_turn_index % sorted.length}
```

**Impact:** The backend's `current_turn_index` is an offset into the **sorted** initiative order. The frontend re-sorts on every render. This works correctly when participants maintain their relative order, but if two participants have equal `initiative_total`, JavaScript's sort is not guaranteed stable across all environments, and the turn indicator could highlight the wrong participant.

**Fix:** Sort should use a stable tiebreaker (e.g., `participant.id` alphabetically) to guarantee consistent ordering.

---

### BUG-014 — `SessionEventList` timezone ambiguity
**Severity:** Low
**File:** `guide-frontend/src/components/sessions/SessionEventList.tsx:40`

```typescript
new Date(ev.created_at).toLocaleTimeString()
```

**Impact:** Backend returns UTC timestamps (RFC 3339). `toLocaleTimeString()` converts to browser local time with no visual indicator of timezone. For DMs in different timezones or running retrospective reviews, the displayed time is confusing.

**Fix:** Add `{ timeZoneName: 'short' }` to `toLocaleTimeString()` options, or use UTC display consistently.

---

### BUG-015 — `SummaryView` copy-to-clipboard `setTimeout` leaks if component unmounts
**Severity:** Low
**File:** `guide-frontend/src/components/sessions/SummaryView.tsx:14`

```typescript
setTimeout(() => setCopied(false), 2000)
```

**Impact:** If user navigates away within 2 seconds of copying, `setCopied(false)` fires on an unmounted component. React 18 suppresses this warning (no-op), but it's a minor resource leak.

**Fix:** Capture the timeout ID in a `useRef` and clear it in a `useEffect` cleanup.

---

### BUG-016 — `Header.tsx` health check has no backoff when backend is offline
**Severity:** Low
**File:** `guide-frontend/src/components/layout/Header.tsx:33`

```typescript
setInterval(check, 30_000)
```

**Impact:** When the backend goes offline, the health check fires every 30 seconds indefinitely with no exponential backoff. In a long offline session, this produces a stream of failed requests logged to the console, making debugging harder.

**Fix:** Implement exponential backoff or a circuit breaker pattern.

---

## Product Enhancements

### ENH-001 — Playstyle Profile: backend integration missing
**Priority:** High
**File:** `guide-frontend/src/pages/PlaystylePage.tsx`

The playstyle profile is stored in `localStorage` only and is never sent to the backend. The profile is intended to personalize encounter generation and AI responses, but since the backend never receives it, it has no effect. Multi-device usage (DM at table vs. prep at home) loses the profile.

**Recommendation:** Add `POST /playstyle` and `GET /playstyle` endpoints. On page load, merge the backend profile with localStorage. On save, persist to both.

---

### ENH-002 — Encounter EncountersPage: `session_id` query param behavior unclear
**Priority:** High
**File:** `crates/guide-api/src/routes/encounters.rs:60–66`

The `GET /campaigns/{id}/encounters` handler accepts `session_id` as a query parameter in the OpenAPI docstring but the actual handler ignores it and calls `repo.list_by_campaign(campaign_id)` without filtering by session. This means encounters cannot be filtered per session, which was the intended API shape. The EncountersPage would show all encounters across all sessions.

**Recommendation:** Either implement session filtering in `list_encounters`, or remove the `session_id` param from the OpenAPI spec to eliminate false expectations.

---

### ENH-003 — No visual confirmation of unsaved stats in `CharacterDetailPage`
**Priority:** Medium
**File:** `guide-frontend/src/pages/CharacterDetailPage.tsx`

The `EditStatsForm` has no "dirty state" indicator — it doesn't show which fields have been changed. Users can accidentally close the form without saving. There's no autosave and no "unsaved changes" warning.

**Recommendation:** Track dirty state in the form, show a visual indicator (e.g., yellow background on changed fields), and add a browser `beforeunload` guard when in edit mode.

---

### ENH-004 — Spacebar shortcut fires even in modal dialogs
**Priority:** Medium
**File:** `guide-frontend/src/pages/EncounterDetailPage.tsx:39–50`

The keyboard handler at line 43 checks only for `INPUT`, `TEXTAREA`, and `SELECT` tags to suppress the Space→Next Turn shortcut. If a modal or dialog is open (e.g., a confirm dialog), pressing Space still fires `nextTurn`.

**Recommendation:** Also check for an active modal: `if (document.querySelector('[role="dialog"]')) return;`

---

### ENH-005 — No global error boundary
**Priority:** Medium
**All pages**

There is no React `ErrorBoundary` component wrapping the app or individual pages. If a component throws during render (e.g., due to an unexpected null from the API), the entire page tree unmounts with a blank screen and no error message.

**Recommendation:** Add a top-level `ErrorBoundary` in `App.tsx` that displays a friendly "Something went wrong" screen with a retry button.

---

### ENH-006 — Session summary: stale summary visible after perspective change
**Priority:** Medium
**File:** `guide-frontend/src/pages/SessionDetailPage.tsx:30–35`

When a DM generates a DM summary, then switches the perspective selector to "Player" and clicks generate again, the old DM summary flickers out correctly. However, if the user switches perspective without clicking generate, the stale summary remains visible with the incorrect perspective label.

**Recommendation:** Clear the `summary` state when `perspective` changes: `useEffect(() => { setSummary(null); }, [perspective]);`

---

### ENH-007 — `PlaystylePage` "Saved" feedback resets too quickly
**Priority:** Low
**File:** `guide-frontend/src/pages/PlaystylePage.tsx:64,87`

`useEffect(() => { setSaved(false); }, [profile])` resets the "✓ Saved" text immediately when profile state is next updated. Since `setSaved(true)` is synchronous and `setSaved(false)` is effect-triggered, in practice the feedback may show for only one render frame.

**Recommendation:** Use `setTimeout(() => setSaved(false), 2000)` instead of the profile-dependency effect, consistent with the pattern used in `SummaryView`.

---

### ENH-008 — Download filename collision for session summaries
**Priority:** Low
**File:** `guide-frontend/src/components/sessions/SummaryView.tsx:24`

Filename: `session-summary-${summary.session_id}-${summary.perspective}.md`. Downloading a summary twice overwrites the previous file. A DM might want to preserve multiple iterations.

**Recommendation:** Append a timestamp: `session-summary-${summary.session_id}-${summary.perspective}-${Date.now()}.md`

---

### ENH-009 — No retry button for failed API loads
**Priority:** Low
**All pages using `useApi`**

When an API call fails on initial page load, the `ErrorBanner` shows the error message but there is no retry button. Users must refresh the entire page.

**Recommendation:** Add an optional `onRetry` callback to `ErrorBanner` that triggers `refetch()` from `useApi`. Pass `refetch` as `onRetry` on pages where it makes sense.

---

### ENH-010 — No empty-state for encounters list without sessions
**Priority:** Low
**File:** `guide-frontend/src/pages/EncounterDetailPage.tsx`

When no participants exist in an encounter (`pending` status, `participants.length === 0`), the page shows only "Start Combat" with no explanation that participants need to be added first.

**Recommendation:** Show a warning: "This encounter has no participants. Add characters to the campaign before starting combat."

---

## Backend–Frontend Type Mismatch Summary

| # | Backend Field | Backend Type | Frontend Field | Frontend Type | Impact |
|---|--------------|-------------|----------------|---------------|--------|
| 1 | `PlotHook.description` | `String` | `PlotHook.summary` | `string` | Hook text always blank |
| 2 | `Backstory.extracted_hooks` | `Vec<PlotHook>` | `Backstory.hooks` | `PlotHook[]` | Hooks never rendered |
| 3 | `CombatParticipant.initiative_modifier` | `i32` | `CombatParticipant.initiative_bonus` | `number` | Field always `undefined` |
| 4 | `CombatParticipant.is_defeated` | `bool` | `CombatParticipant.is_active` | `boolean` | Inverted + missing field |
| 5 | `HookPriority::Critical` | `"critical"` | (missing) | — | Critical hooks untyped |
| 6 | `GeneratedEncounterType::Mixed` | `"mixed"` | (missing) | — | Mixed encounters untyped |
| 7 | `Encounter.name` | `Option<String>` | `EncounterSummary.name` | `string \| null` | ✓ Correct |
| 8 | `Character.ability_scores` | always set | `ability_scores` | `AbilityScores` | ✓ Backend always sets it |
| 9 | `PlotHook.related_npcs` | (doesn't exist) | `PlotHook.related_npcs` | `string[]` | Field always `undefined` |

---

## Verified Working (No Issues Found)

- Campaign CRUD API calls and types are correctly aligned
- Session lifecycle (create/start/end) types are correct
- `SessionSummary` response shape matches backend JSON
- `useChat.ts` SSE parsing logic is correct — splits on `\n\n`, handles `event:token`, `event:done`, `event:error`
- `IngestButton` polling logic is clean (clears interval on unmount, handles failure state)
- `ParticipantRow` HP bar percentage calculation correctly guards for `max_hp === 0`
- `BackstoryPanel` disable logic for the Analyze button is correct (disabled when no text)
- `EncounterDetailPage` Space-bar shortcut correctly ignores input/textarea/select elements
- Document upload (multipart form) is correctly implemented in `UploadForm`
- `ConfirmButton` delete guard pattern is consistent across pages

---

## Priority Fix Order

| Priority | Bug | Effort |
|----------|-----|--------|
| 1 | BUG-001: PlotHook field mismatch (`summary` vs `description`, missing fields) | Small — types.ts change |
| 2 | BUG-002: Backstory field mismatch (`hooks` vs `extracted_hooks`) | Small — types.ts change |
| 3 | BUG-003: `initiative_bonus` vs `initiative_modifier` | Small — types.ts change |
| 4 | BUG-004: `is_active` vs `is_defeated` (inverted logic) | Small — types.ts + component |
| 5 | BUG-005: Missing `HookPriority.critical` | Small — types.ts change |
| 6 | BUG-006: Missing `GeneratedEncounterType.mixed` | Small — types.ts change |
| 7 | BUG-007: Action budget spend fields ignored by backend | Medium — Rust handler |
| 8 | BUG-008: GlobalDocumentsPage missing `onComplete` | Trivial — one prop |
| 9 | BUG-010: CharacterDetailPage `doUpdate` swallows errors | Small — add catch |
| 10 | BUG-011: ParticipantRow no user-facing error on update failure | Small — error state |
| 11 | ENH-001: Playstyle profile not sent to backend | Large — new API + endpoints |
| 12 | ENH-002: `session_id` filter not implemented in list encounters | Medium — Rust handler |
| 13 | ENH-005: No global React error boundary | Small — wrap App.tsx |
| 14 | ENH-006: Stale summary on perspective change | Trivial — one useEffect |
| 15 | BUG-012: Better UX for "no events" summary error | Small — disable button |
