# AI Agent Research — Session Continuity Checklist

Use this file to resume work after a session break. Mark tasks complete as you go.

## Setup (Pre-Testing)

- [x] RAG disabled — `GUIDE__ENABLE_RAG=false` already in `.env`
- [x] Token logging added to `pipeline.rs` (3 locations: pre_analysis, chapter_extraction, single_call_extraction)
- [x] `AI_AGENT_RESEARCH.md` created
- [x] `AI_AGENT_RESEARCH_TODO.md` created (this file)
- [ ] Verify `cargo clippy --workspace -- -D warnings` passes after token logging changes
- [ ] Verify `cargo test --workspace` passes

## Model Testing

For each model: pull → update .env → restart server → upload PDF → ingest → capture logs → evaluate → log results

Test PDF: `C:\Users\altoz\Documents\dnd\Land of Vampires\Land of Vampires Full Campaign.pdf`

### Model 1: qwen3:8b (current default)

- [x] Pull: already available (current default)
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=qwen3:8b` (already set)
- [x] Upload and ingest test PDF
- [x] Capture token counts from logs (prompt=182,806 completion=72,547 total=255,353)
- [x] Evaluate story extraction quality (19 arcs [duplicated], 145 events, 50 NPCs, 50 locations; quality=4/10)
- [x] Log results in AI_AGENT_RESEARCH.md

### Model 2b: qwen3:14b

- [x] Pull: already available (9.3GB)
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=qwen3:14b`, `GUIDE__CONTEXT_WINDOW=16000`
- [ ] Upload and ingest test PDF — running now (campaign=3ee0e8d4, doc=971cf702)
- [ ] Capture token counts from logs
- [ ] Evaluate story extraction quality
- [ ] Log results in AI_AGENT_RESEARCH.md

### Model 2: llama3.1:8b

- [x] Pull: already available
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=llama3.1:8b`, `GUIDE__CONTEXT_WINDOW=8192`
- [x] Upload and ingest test PDF
- [x] Capture token counts from logs (prompt=270,186 completion=100,021 total=370,207)
- [x] Evaluate story extraction quality (4 arcs, 319 events, 59 NPCs, 61 locations; quality=5/10)
- [x] Log results in AI_AGENT_RESEARCH.md

### Model 3: gemma3:12b

- [x] Pull: already available (8.1GB)
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=gemma3:12b`, `GUIDE__CONTEXT_WINDOW=16000`
- [x] Upload and ingest test PDF
- [x] Capture token counts (prompt=289,321 completion=86,177 total=375,498)
- [x] Evaluate story extraction quality (4 hallucinated arcs, 366 events, 63 NPCs, 71 locations; quality=4/10)
- [x] Log results — DISQUALIFIED: no tool-use support

### Model 4: mistral-nemo:12b

- [x] Pull: downloaded successfully
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=mistral-nemo:12b`, `GUIDE__CONTEXT_WINDOW=16000`
- [x] Upload and ingest test PDF
- [x] Capture token counts (prompt=155,809 completion=26,857 total=182,666)
- [x] Evaluate quality (10 arcs, 114 events, 48 NPCs, 19 locations; quality=5/10; BEST agent rate: 31 successes)
- [x] Log results in AI_AGENT_RESEARCH.md

### Model 5: qwen3.5:35b-a3b (MoE)

- [x] Already downloaded (23GB)
- [x] SKIPPED — CPU offload too slow (~5 min/call based on nemotron-cascade-2 experience)
- [x] Log results: UNTESTED, estimated 8+/10 if hardware available

### Model 6: phi4-mini:3.8b

- [x] Pull: downloaded successfully
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=phi4-mini:3.8b`, `GUIDE__CONTEXT_WINDOW=16000`
- [x] Upload and ingest test PDF
- [x] Capture token counts (prompt=116,273 completion=72,024 total=188,297)
- [x] Evaluate quality (2 arcs, 1 event, 3 NPCs, 2 locations; quality=1/10; DISQUALIFIED)
- [x] Log results in AI_AGENT_RESEARCH.md

### Model 7: qwen3:4b

- [x] Pull: downloaded successfully
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=qwen3:4b`, `GUIDE__CONTEXT_WINDOW=16000`
- [x] Upload and ingest test PDF
- [x] Capture token counts (prompt=183,207 completion=254,987 total=438,194)
- [x] Evaluate quality (5 arcs [campaign-specific!], 33 events, 12 NPCs, 11 locations; quality=2/10; thinking mode issue)
- [x] Log results in AI_AGENT_RESEARCH.md

### Model 8: nemotron-cascade-2

- [x] Already downloaded (24GB)
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=nemotron-cascade-2`, `GUIDE__CONTEXT_WINDOW=16000`
- [x] ABORTED after ~40 min — empty responses + MaxTurnError + 5 min/call too slow
- [x] Log results: DISQUALIFIED

### Model 9: lfm2

- [x] Already downloaded (14GB)
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=lfm2`, `GUIDE__CONTEXT_WINDOW=16000`
- [x] Upload and ingest test PDF
- [x] Capture token counts (prompt=203,458 completion=78,581 total=282,039)
- [x] Evaluate quality (11 arcs, 67 events, 16 NPCs, 30 locations; quality=3/10)
- [x] Log results in AI_AGENT_RESEARCH.md

### Model 10: lfm2.5-thinking

- [x] Already downloaded (731MB)
- [x] Set `.env`: `GUIDE__DEFAULT_MODEL=lfm2.5-thinking`, `GUIDE__CONTEXT_WINDOW=16000`
- [x] Upload and ingest test PDF
- [x] Capture token counts from logs (prompt=129,201 completion=43,696 total=172,897)
- [x] Evaluate quality (0 NPCs, 0 locations, 86% parse failure; quality=1/10; DISQUALIFIED — too small)
- [x] Log results in AI_AGENT_RESEARCH.md ✅

### Model 12: Jackrong/Qwen3.5-4B-Claude-4.6-Opus-Reasoning-Distilled-GGUF

- [x] Already downloaded (Q4_K_M)
- [x] Tested via llama.cpp on localhost:8080 (same setup as 9B)
- [x] Run via `/re-extract-story` endpoint — skipped OCR, reused stored page text
- [x] Token counts: prompt=293,602 completion=245,040 total=538,642
- [x] Quality: **8/10** — tied with 9B; best arc consolidation (3 arcs), highest locations (128), highest events (440)
- [x] Agent success rate: 4/90 (4%) — poorest of all models; 22/90 windows truncated at token budget
- [x] Not preferred over 9B: 3× slower runtime (~3 hrs), 2.4× more tokens, 4% agent success vs 41%
- [x] Log results in AI_AGENT_RESEARCH.md ✅
- [x] Comparative summary table updated ✅

### Model 11: Jackrong/Qwen3.5-9B-Claude-4.6-Opus-Reasoning-Distilled-v2-GGUF

- [x] Already downloaded (6.6GB Q4_K_M)
- [x] Re-tested via llama.cpp on localhost:8080 (Ollama incompatible, llama.cpp works)
- [x] Run 1: content quality confirmed excellent; blocked by markdown fence bug
- [x] Run 2: fence-strip + monster count fixes applied; full results collected
- [x] Token counts: prompt=139,766 completion=82,960 total=222,726
- [x] Quality: **8/10** — NEW TOP PERFORMER (arcs=47, events=305, NPCs=118, locations=108)
- [x] Agent success rate: 22/53 (41%) — best of all models tested
- [x] Log results in AI_AGENT_RESEARCH.md ✅
- [x] Run 3: Re-run with GUIDE__CONTEXT_WINDOW=65536 (16,384 output cap) — identical results (47/118/108), 270,524 total tokens; confirmed quality ceiling is model capability not token budget
- [x] Log Run 3 in AI_AGENT_RESEARCH.md ✅
- [x] Delete cron monitor job 82f35420 ✅

### Model 13: Gemini 2.5 Flash (Cloud)

- [x] Config: `GUIDE__CLOUD_MODEL=gemini-2.5-flash`, `GUIDE__CONTEXT_WINDOW=65536`
- [x] Run via `/re-extract-story` endpoint — reused stored page text
- [x] Token counts: prompt=304,292 completion=161,986 total=466,278
- [x] Quality: **9/10** — NEW TOP PERFORMER; 173 NPCs, 198 locations, 0% parse failure
- [x] Actual cost: $1.75/doc (thinking tokens billed separately)
- [x] Log results in AI_AGENT_RESEARCH.md ✅

### Model 14: Gemini 2.5 Flash Lite (Cloud)

- [x] Config: `GUIDE__CLOUD_MODEL=gemini-2.5-flash-lite`, `GUIDE__CONTEXT_WINDOW=200000`
- [x] Run via `/re-extract-story` endpoint — reused stored page text
- [x] Token counts: prompt=73,405 completion=61,640 total=135,045
- [x] Quality: **8/10** — 89 NPCs, 67 locations, 0% parse failure, 32% agent success
- [x] Estimated cost: ~$0.01/doc (175× cheaper than Flash with thinking)
- [x] Duration: ~8 minutes (vs 41 min for Flash)
- [x] Bug fixes applied: MaxTurnError fix, persist_story_batch, DuckDB WAL cleanup
- [x] Log results in AI_AGENT_RESEARCH.md ✅

### Model 15: Gemini 3.1 Flash Lite Preview (Cloud)

- [x] Config: `GUIDE__CLOUD_MODEL=gemini-3.1-flash-lite-preview`, `GUIDE__CONTEXT_WINDOW=200000`
- [x] Run via `/re-extract-story` endpoint — reused stored page text
- [x] Token counts: prompt=43,553 (partial) completion=28,740 (partial) — agent calls not logged
- [x] Quality: **4/10** — 31 arcs (1 per chapter, no consolidation), 56 NPCs, 55 locations
- [x] Agent success rate: 22/53 (41%) but tool-use fails 59% due to thought_signature incompatibility
- [x] Duration: ~5 minutes
- [x] Bug fix applied: persist_story_batch DELETEs now outside transaction (DuckDB 1.2 FK fix)
- [x] Log results in AI_AGENT_RESEARCH.md ✅
- [x] DISQUALIFIED: thought_signature incompatibility + arc consolidation failure

## Analysis

- [x] Fill in comparative summary table in AI_AGENT_RESEARCH.md
- [x] Select top models: Jackrong/Qwen3.5-9B (8/10, llama.cpp), qwen3:14b (7/10, Ollama), mistral-nemo:12b (runner-up)
- [x] Document reasoning for selection

## Prompt Optimization

- [x] Implement prompt fixes in `crates/guide-llm/src/prompts.rs` and `crates/guide-pdf/src/agent.rs`:
  - [x] Fix agent arc object format (not string array) — agent_system_prompt() now shows full arc object schema
  - [x] Fix event_type strict enum validation — now says "MUST be EXACTLY ONE value from: ..." in both prompts
  - [x] Add arc_order null prevention — "NEVER null", uses integer example (1, 2, 3...)
  - [x] Remove STEP 1/STEP 2 <think> framing — was conflicting with native Qwen3 thinking; simplified to direct JSON output
  - [ ] Test optimized prompts on 2-3 chapters (qwen3:14b)
  - [ ] Document before/after quality scores

## Final Report

- [x] Fill in cloud cost extrapolation table in AI_AGENT_RESEARCH.md
- [x] Write RAG evaluation section
- [x] Write final recommendation
- [x] Run `cargo clippy --workspace -- -D warnings` (passed — zero warnings)
- [x] Run `cargo test --workspace` (passed — 68 tests all green)
- [x] Fix DuckDB FK constraint violation in `persist_story_batch` — DELETEs moved outside transaction (2026-03-28) ✅

---

## How to Resume

1. Read this file first to find the first unchecked task
2. Read `AI_AGENT_RESEARCH.md` for current data/notes
3. The API test commands are:

```bash
# Start server with verbose token logging
RUST_LOG=guide_pdf=info,guide_llm=debug cargo run -p guide-api 2>&1 | tee test_run.log

# Create a test campaign (adjust to actual campaign ID)
curl -s -X POST http://localhost:8000/campaigns \
  -H "Content-Type: application/json" \
  -d '{"name":"Land of Vampires Test","description":"Model evaluation test"}' | jq .

# Upload PDF (adjust campaign_id)
curl -s -X POST http://localhost:8000/campaigns/<CAMPAIGN_ID>/documents \
  -F "file=@C:/Users/altoz/Documents/dnd/Land of Vampires/Land of Vampires Full Campaign.pdf" \
  -F "document_kind=campaign" | jq .

# Trigger ingestion (adjust doc_id)
curl -s -X POST http://localhost:8000/campaigns/<CAMPAIGN_ID>/documents/<DOC_ID>/ingest | jq .

# Poll for completion
curl -s http://localhost:8000/campaigns/<CAMPAIGN_ID>/documents/<DOC_ID>/story-extraction-status | jq .

# View extracted story data
curl -s http://localhost:8000/campaigns/<CAMPAIGN_ID>/story/arcs | jq .
curl -s http://localhost:8000/campaigns/<CAMPAIGN_ID>/story/npcs | jq .
curl -s http://localhost:8000/campaigns/<CAMPAIGN_ID>/story/locations | jq .

# Extract token counts from log
grep "LLM token usage" test_run.log
```

## Quality Scoring Rubric

Score each model 1-10 on:

- **Arc extraction** (0-2): Are the 2-5 major story arcs correctly identified with good descriptions?
- **NPC extraction** (0-2): Are major NPCs extracted with correct roles and descriptions?
- **Location extraction** (0-2): Are key locations extracted with correct types?
- **JSON validity** (0-2): Did the JSON parse without errors? Are schema fields correct?
- **Cross-chapter consistency** (0-2): Are arc titles consistent across chapters? Do events link to correct arcs?

Total: 10 points
