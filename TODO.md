# TODO.md — The Guide: Remaining Improvements

---

## High Priority

- [ ] **Missing FK on `playstyle_profiles.campaign_id`** — `003_playstyle.sql` has no `FOREIGN KEY ... REFERENCES campaigns(id) ON DELETE CASCADE`; orphaned profiles survive campaign deletion.
- [ ] **Race condition in session number generation** (`db/sessions.py`) — The UNIQUE index is in place, but `create()` still reads max session_number then inserts outside a transaction. Concurrent writes hit the constraint and return a 500 instead of retrying cleanly.
- [ ] **LLM providers swallow connection errors** (`llm/ollama.py`, `llm/cloud.py`) — Neither `OllamaProvider` nor `CloudProvider` catch network errors (`httpx.ConnectError`, `asyncio.TimeoutError`, `openai.RateLimitError`). These propagate as untyped 500s instead of `LlmError`.
- [ ] **PDF extraction crash on malformed pages** (`pdf/extractor.py:111–137`) — Items missing `prov` attributes cause `item.prov[0].page_no` to crash. Corrupt PDFs can produce `full_markdown` but empty page lists, silently breaking RAG indexing.

---

## Medium Priority

- [ ] **No rollback on DB errors** — All repository methods commit without try/except rollback. If `commit()` itself fails the connection is left in an undefined state.
- [ ] **`SessionEventRepository.create()` null check** (`db/sessions.py:125–134`) — After inserting and re-querying the row, if the row is `None` (concurrent delete race), `_row_to_event(row)` crashes with `AttributeError`.
- [ ] **`playstyle_profiles` schema gaps** (`003_playstyle.sql`) — `updated_at` lacks `DEFAULT (datetime('now'))` and `campaign_id` should be explicitly `NOT NULL`.
- [ ] **Generic exception handler leaks internals** (`api/main.py:94–97`) — The global `Exception` handler returns raw exception messages (DB paths, errors) to clients. Should return a generic message in production and log the detail.
- [ ] **Background ingestion task silently swallows errors** (`routes/documents.py:158–190`) — If `update_status(..., failed)` itself throws, the error is lost. Clients have no way to poll ingestion status/error after the `202`.
- [ ] **Missing `Location` header on all `201` responses** — REST convention; all POST-create endpoints are missing it.
- [ ] **`CloudProvider` lacks `_model_for_task()`** (`llm/cloud.py`) — `OllamaProvider` maps tasks to specific models; `CloudProvider` uses one model for everything. `LlmRouter.model_for_task()` works around this with an `isinstance` check, coupling the router to implementation details.
- [ ] **`is_player_visible` always `True` in `_flatten_nodes_to_chunks`** (`pdf/pipeline.py:279,296`) — DM-only content is never marked non-player-visible, so `player_visible_only=True` retrieval doesn't filter secrets or plot twists.

---

## Low Priority

- [ ] **Missing index on `plot_hooks.campaign_id`** (`001_initial.sql`) — Only `character_id` is indexed; campaign-scoped queries scan the full table.
- [ ] **No HP check constraints** — Schema allows `current_hp > max_hp` and negative `max_hp`. Should add `CHECK (max_hp > 0)` and `CHECK (current_hp >= 0)`.
- [ ] **`CharacterRepository.update()` doesn't verify row existence** — Silent no-op if the character was deleted concurrently; inconsistent with `delete()` which checks rowcount.
- [ ] **Rate limiter ignores `X-Forwarded-For`** (`api/main.py:74–91`) — Behind a reverse proxy, all requests appear as one IP.
- [ ] **Rate limiter `_rate_buckets` is unbounded** (`api/main.py:74–91`) — No eviction; could OOM under many unique IPs.
- [ ] **`chunk_max_tokens` config is dead code** (`config.py:18`) — Ingestion pipeline uses `chunk_max_chars` with a hardcoded default; the configured `chunk_max_tokens` is never read.
- [ ] **No logging in `_fallback_extract()`** (`pdf/extractor.py:146–152`) — Fallback extraction (when Docling is unavailable) is silent; operators won't know PDFs are being skipped.
- [ ] **`CombatEngine.start()` allows zero-participant encounters** (`combat/engine.py`) — `start()` transitions to `active` with an empty participant list; `next_turn()` will immediately raise. Should validate `len(participants) > 0` at start.
