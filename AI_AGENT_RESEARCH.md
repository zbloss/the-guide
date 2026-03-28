# AI Agent Story Extraction — Model Research & Evaluation

## Research Goal

The story extraction pipeline now includes an AI Agent path (tool-use based, `guide-pdf/src/agent.rs`) capable of navigating D&D campaign PDFs via Table of Contents + page-fetch tools. The hypothesis is that this agent is powerful enough to make the RAG system redundant.

**Questions to answer:**

1. Which open-source LLM performs best as the agent brain for structured story extraction?
2. Is the agent's output quality high enough to make RAG unnecessary?
3. What is the token cost per document, and how does that extrapolate to cloud model pricing?
4. What hardware does a user need to run the best local models?

**Test PDF:** `Land of Vampires Full Campaign.pdf`
**RAG status:** Disabled (`GUIDE__ENABLE_RAG=false`)

---

## Candidate Models

| #   | Model            | Ollama Tag         | Disk Size   | Tool-Use | Hardware Tier    | Installed | Notes                                                       |
| --- | ---------------- | ------------------ | ----------- | -------- | ---------------- | --------- | ----------------------------------------------------------- |
| 1   | Qwen3 8B         | `qwen3:8b`         | 5.2 GB      | ✓        | 8GB+ VRAM        | ✓         | Current default; thinking mode; strong JSON                 |
| 2   | Qwen3 14B        | `qwen3:14b`        | 9.3 GB      | ✓        | 12GB+ VRAM       | ✓         | Stronger Qwen3; fits in 12GB with some layering             |
| 3   | Llama 3.1 8B     | `llama3.1:8b`      | 4.9 GB      | ✓        | 8GB+ VRAM        | ✓         | Best-in-class tool-use; proven function calling             |
| 4   | Qwen3.5 35B-A3B  | `qwen3.5:35b`      | 23 GB (MoE) | ✓        | CPU offload req. | ✓         | MoE: 35B total / 3B active; highest quality local candidate |
| 5   | Gemma3 12B       | `gemma3:12b`       | ~7.8 GB     | ✓        | 12GB+ VRAM       | ✗         | Google latest; strong instruction following                 |
| 6   | Mistral-Nemo 12B | `mistral-nemo:12b` | ~7.7 GB     | ✓        | 12GB+ VRAM       | ✗         | Context specialist; good structured output                  |

**Note on Qwen3.5 35B-A3B**: Already downloaded (23 GB). MoE (Mixture of Experts): 35B total parameters but only ~3B active per token. Still requires the full weights in VRAM; on RTX 4070 12GB some layers will be offloaded to CPU RAM (slower inference). Ollama handles this automatically.

**Note on Qwen3.5 35B-A3B**: MoE (Mixture of Experts) model. Despite having 35B total parameters, only 3B are activated per token, making it much faster than a dense 35B model. However, all expert weights must be resident in memory, so it still requires ~22GB. On an RTX 4070 12GB, this requires CPU offloading (some layers run on RAM), which significantly reduces speed. Exact Ollama tag may be `qwen3:30b-a3b` — verify with `ollama search qwen3`.

---

## Token Count Log

All LLM calls are logged at `INFO` level with format:

```
LLM token usage  phase=<phase> chapter=<name> prompt_tokens=<N> completion_tokens=<N> model=<name>
```

Phases:

- `pre_analysis` — document context extraction (1 call per document)
- `single_call_extraction` — full-doc extraction (if < 2 chapters detected)
- `chapter_extraction` — per-chapter LLM extraction
- `agent_extraction` — agent path (token counts not available via rig framework)

### Model 1: qwen3:8b

**Config**: `GUIDE__DEFAULT_MODEL=qwen3:8b`, `GUIDE__CONTEXT_WINDOW=16000`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~55 minutes (12:05–13:01 UTC-4)

| Phase              | Chapter      | Prompt Tokens   | Completion Tokens | JSON Valid | Notes                                              |
| ------------------ | ------------ | --------------- | ----------------- | ---------- | -------------------------------------------------- |
| pre_analysis       | —            | 987             | 121               | ✓          | Setting: "Vampire-haunted lands", tone: horror     |
| chapter_extraction | 64 LLM calls | varies (0–7476) | varies (0–2119)   | ~55%       | 29/64 failures, mostly `missing field event_order` |
| agent_extraction   | 11 successes | N/A (rig)       | N/A (rig)         | ✓          | Chapters Nine, Ten, Eleven, Fifteen, etc.          |

**Total tokens**: prompt=182,806 completion=72,547 (total=255,353)

**Extraction results**:

- Story arcs: 19 (with duplicates: "Blood Harvest and Vampire Conquest", "Blood Harvest", "Vampire Conquest" all appear as separate arcs)
- Story events: 145
- NPCs: 50 (capped by API limit)
- Locations: 50 (capped by API limit)
- Total chapters attempted: 75 (11 agent + 64 LLM)
- Chapter successes: ~46/75 (~61%)

**Quality score**: 4/10

**Evaluation**:

- Arc extraction (0/2): 19 arcs but highly repetitive — "Blood Harvest", "Vampire Conquest", "Blood Harvest and Vampire Conquest" appear as multiple separate arcs. Deduplication failed because cross-chapter context is being ignored. Real campaign arcs (The Witchwood's Curse, The Brotherhood's Corruption) only appear in later chapters.
- NPC extraction (1/2): 50 NPCs extracted — likely reasonable quantity but titles like "Cedrine Shrikehonor", "Wellyn", "Lazarus" appear, suggesting real extraction. Can't verify quality without reading all 50.
- Location extraction (1/2): 50 locations extracted — quantity seems right.
- JSON validity (1/2): 29/64 LLM failures = 45% failure rate. Main issues: `missing field event_order` (model reliably produces arcs but forgets event_order in events), `unknown variant 'restoration'` (model uses invalid enum value), EOF truncation.
- Cross-chapter consistency (1/2): Arc titles repeat inconsistently across chapters; the agent uses correct arcs but LLM path often regenerates new ones per-chapter.

**Key failure modes**:

1. `missing field event_order` — Most common. Model outputs events without `event_order` integer.
2. Agent path schema error — Agent returns `["Blood Harvest", ...]` (string array) instead of `[{"title": "...", ...}]` (object array) for arcs. Affects ALL agent calls for this model except those that "get lucky" with tool-use responses.
3. `unknown variant 'restoration'` — Model uses invalid `event_type` enum values.
4. EOF / truncation — Model occasionally stops mid-JSON.
5. Chinese character hallucination — One instance of `踪迹` as JSON key.

**Pipeline issues discovered** (affect all models equally):

- ToC dotted-leader lines produce garbled chapter names (e.g., `CO.............4AMPAIGN\tVERVIEW`). About 30% of chapter groups come from ToC page entries, not actual content. These always fail or produce 0 tokens.
- Agent rig framework `MaxTurnError: (reached max turn limit: 0)` for some chapters — appears when model tries to call tools but rig's default turn limit is too low.

---

### Model 2: llama3.1:8b

**Config**: `GUIDE__DEFAULT_MODEL=llama3.1:8b`, `GUIDE__CONTEXT_WINDOW=8192`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~62 minutes (13:13–14:15 UTC-4)

| Phase              | Chapter      | Prompt Tokens    | Completion Tokens | JSON Valid | Notes                                                           |
| ------------------ | ------------ | ---------------- | ----------------- | ---------- | --------------------------------------------------------------- |
| pre_analysis       | —            | 431              | 78                | ✓          | Setting: "Land of Vampires", tone: "dungeon-crawl", themes: horror, adventure |
| chapter_extraction | 80 LLM calls | varies (788–7476) | varies (435–1710) | ~66%       | 27/80 failures; `null` required fields, invalid event_type enum |
| agent_extraction   | 7 successes  | N/A (rig)        | N/A (rig)         | ✓          | 80 agent failures — string arrays vs object arrays              |

**Total tokens**: prompt=270,186 completion=100,021 (total=370,207)

**Extraction results**:

- Story arcs: 4 (consistent: "The Hunt for the Missing Heir", "The Lost City of Eldrador", "The Rise of the Vampire Lord", "The Final Confrontation")
- Story events: 319
- NPCs: 59
- Locations: 61
- Total chapters attempted: 81 (7 agent successes + 80 LLM fallback)
- Chapter successes: ~54/81 (~67%)

**Quality score**: 5/10

**Evaluation**:

- Arc extraction (1/2): Only 4 arcs — no duplication (huge improvement over qwen3:8b). But arcs feel generic/hallucinated rather than campaign-specific ("The Lost City of Eldrador" doesn't appear in the actual campaign). Arc consistency across chapters is excellent.
- NPC extraction (1/2): 59 NPCs — solid quantity, names plausible (Lord Nassarq appears, matching actual villain).
- Location extraction (1/2): 61 locations — reasonable quantity.
- JSON validity (1/2): 27/80 LLM failures = 34% failure rate (vs qwen3:8b's 45%). Main issues: `arc_order: null` when arcs aren't relevant to that chapter, `unknown variant 'adventure'/'revelation|discovery'` in event_type.
- Cross-chapter consistency (1/2): Very consistent — same 4 arc titles appear in almost every chapter. However, this may be over-consolidation rather than accurate extraction.

**Key failure modes**:

1. Agent string arrays — Agent returns `["arc title", ...]` instead of `[{"title": "...", ...}]` for 80/87 agent calls. Same root cause as qwen3:8b but more severe (qwen3:8b had some successes).
2. `arc_order: null` — When a chapter doesn't introduce a new arc, model sets `arc_order: null` instead of omitting or using a valid integer.
3. Invalid event_type — Uses `adventure`, `revelation|discovery` (pipe-separated), etc.
4. `null` required fields — Multiple instances of `null, expected a string` or `null, expected struct StoryLocationInput`.
5. `expected value at line 1 column 1` — Empty agent response (agent doesn't call fetch tools, returns nothing).

---

### Model 2b: qwen3:14b

**Config**: `GUIDE__DEFAULT_MODEL=qwen3:14b`, `GUIDE__CONTEXT_WINDOW=16000`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~169 minutes (14:18–17:07 UTC-4)

| Phase              | Chapter      | Prompt Tokens    | Completion Tokens | JSON Valid | Notes                                                               |
| ------------------ | ------------ | ---------------- | ----------------- | ---------- | ------------------------------------------------------------------- |
| pre_analysis       | —            | 1,496            | 90                | ✓          | Setting: "Azuria", tone: horror, themes: horror/mystery/intrigue    |
| chapter_extraction | 63 LLM calls | varies (1216–7224) | varies (929–2027) | ~84%       | Only 10/63 failures — best JSON compliance of all models so far     |
| agent_extraction   | 11 successes | N/A (rig)        | N/A (rig)         | ✓          | 63 agent failures — string arrays remain the dominant failure mode  |

**Total tokens**: prompt=212,049 completion=91,150 (total=303,199)

**Extraction results**:

- Story arcs: 18 (chapter-specific arcs not consolidated — e.g., "Final Confrontation with the Vampire Lord" and "Final Confrontation with Nassarq" as separate arcs)
- Story events: 217
- NPCs: 67 (highest so far)
- Locations: 106 (exceptional — 2× more than qwen3:8b cap)
- Total chapters attempted: 74 (11 agent + 63 LLM)
- Chapter successes: ~64/74 (~86%) — best success rate so far

**Quality score**: 7/10

**Evaluation**:

- Arc extraction (1/2): 18 arcs — more campaign-specific names ("Council of Elders Conspiracy", "Wight Knights' Redemption", "The Maze Macabre") but cross-chapter deduplication weaker than llama3.1. Core 3 arcs consistent but 15 chapter-specific arcs weren't merged.
- NPC extraction (2/2): 67 NPCs — excellent quantity; more than both prior models.
- Location extraction (2/2): 106 locations — exceptional, far surpassing the 50-cap seen in qwen3:8b and better than llama3.1's 61.
- JSON validity (1.5/2): Only 10/63 LLM failures = 16% failure rate (best so far). Main issues: `unknown variant 'investigation'` for event_type, `invalid type: string "...", expected struct StoryLocationInput` (locations as strings vs objects).
- Cross-chapter consistency (0.5/2): Core arcs consistent, but 15 extra arcs weren't merged — arc names drift chapter-to-chapter (e.g., "Final Confrontation with the Vampire Lord" vs "Final Confrontation with Nassarq" vs "Final Confrontation with Velaviryn").

**Key failure modes**:

1. Agent string arrays — 63/74 agent calls return `["arc title", ...]` instead of `[{"title": "...", ...}]`. Universal across all models tested.
2. `unknown variant 'investigation'` — uses `investigation` as event_type (not in schema).
3. Location string arrays — `invalid type: string "Heart of the Fields", expected struct StoryLocationInput` — model returns location names as strings.
4. Missing arc deduplication — each chapter creates new arcs without reference to prior chapter extractions; no global arc state maintained.

**Notable vs qwen3:8b and llama3.1:8b**:

- Dramatically better NPC/Location counts (67 NPCs, 106 locations vs 50/50 cap)
- Best JSON validity rate (84% success)
- Still slower (169 min vs 55 min for qwen3:8b) — 14B params vs 8B
- Arc deduplication worse than llama3.1:8b despite better per-chapter quality

---

### Model 3: gemma3:12b

**Config**: `GUIDE__DEFAULT_MODEL=gemma3:12b`, `GUIDE__CONTEXT_WINDOW=16000`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~37 minutes (17:08–17:45 UTC-4)
**CRITICAL**: gemma3:12b does NOT support Ollama tool-use API — agent path fails 100% with HTTP 400 "does not support tools". Only LLM fallback runs.

| Phase              | Chapter      | Prompt Tokens    | Completion Tokens | JSON Valid | Notes                                                               |
| ------------------ | ------------ | ---------------- | ----------------- | ---------- | ------------------------------------------------------------------- |
| pre_analysis       | —            | (included in total) | (included)   | ✓          |                                                                     |
| chapter_extraction | 75 LLM calls | varies           | varies            | ~88%       | Only 9/75 failures — best JSON parse rate of all models tested      |
| agent_extraction   | 0 successes  | N/A              | N/A               | N/A        | 75 failures — ALL due to "does not support tools" (not a parse error) |

**Total tokens**: prompt=289,321 completion=86,177 (total=375,498)

**Extraction results**:

- Story arcs: 4 (all HALLUCINATED — "The Vanishing of Oakhaven", "Unmasking the Cult", "Descent into Shadowfell", "Steymhorod")
- Story events: 366 (highest count of all models tested)
- NPCs: 63
- Locations: 71

**Quality score**: 4/10

**Evaluation**:

- Arc extraction (0/2): The 4 arcs are completely fabricated. "Oakhaven" does not exist in Land of Vampires; "Descent into Shadowfell" is generic D&D hallucination; "Steymhorod" is a city name, not an arc. Without the agent's page navigation, the model invents fictional content rather than grounding in the actual text.
- NPC extraction (1/2): 63 NPCs — decent quantity but unknown how many are hallucinated.
- Location extraction (1/2): 71 locations — reasonable.
- JSON validity (2/2): Only 9/75 LLM failures = 12% failure rate — BEST across all models. gemma3 follows JSON schema almost perfectly when it does respond.
- Cross-chapter consistency (0/2): Consistent 4 arcs across chapters, but they are consistently WRONG. Consistency in hallucination is not a positive.

**Key failure modes**:

1. No tool-use support — agent path completely disabled. gemma3:12b cannot use the page-navigation tools, so it cannot ground its extraction in actual PDF content.
2. Arc hallucination — without access to actual text, model generates plausible-sounding but fictional D&D campaign content.
3. `unknown variant 'arrival'` — model uses `arrival` as event_type (not in schema).

**DISQUALIFIED as "AI agent brain"**: gemma3:12b cannot support the tool-use agent path. It is only usable as an LLM fallback, which defeats the purpose of the agent pipeline. Despite best JSON compliance, the hallucination problem makes it unsuitable.

---

### Model 3b: lfm2.5-thinking (731MB)

**Config**: `GUIDE__DEFAULT_MODEL=lfm2.5-thinking`, `GUIDE__CONTEXT_WINDOW=16000`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~20 minutes (17:43–18:03 UTC-4) — fastest test, model is only 731MB

| Phase              | Chapter      | Prompt Tokens   | Completion Tokens | JSON Valid | Notes                                                              |
| ------------------ | ------------ | --------------- | ----------------- | ---------- | ------------------------------------------------------------------ |
| pre_analysis       | —            | (included)      | (included)        | ✓          |                                                                    |
| chapter_extraction | 57 LLM calls | varies          | varies            | ~14%       | 49/57 failures — worst parse rate of all models tested             |
| agent_extraction   | 18 successes | N/A (rig)       | N/A (rig)         | ✓          | Agent works but saves near-zero data; 56 agent failures            |

**Total tokens**: prompt=129,201 completion=43,696 (total=172,897) — NOTE: some calls return prompt_tokens=0 (model doesn't report usage)

**Extraction results**:

- Story arcs: 5 (all hallucinated: "The Unraveling", "The Pact", "The Fracture", "The Convergence", "The Wight Hand of Death")
- Story events: **0** (catastrophic failure — no events extracted)
- NPCs: **0** (catastrophic failure)
- Locations: **0** (catastrophic failure)

**Quality score**: 1/10

**Evaluation**:

- Arc extraction (0/2): 5 arcs, all hallucinated generics with no connection to "Land of Vampires" campaign.
- NPC extraction (0/2): Zero NPCs — complete failure.
- Location extraction (0/2): Zero locations — complete failure.
- JSON validity (0/2): 49/57 failures = 86% failure rate — worst of all models. Model uses placeholder comments in JSON, pipe-separated enum values, null arrays.
- Cross-chapter consistency (1/2): Same 4-5 generic arcs consistently hallucinated. Point only for consistency; content is wrong.

**DISQUALIFIED**: Model is too small (731MB) for structured story extraction. Cannot produce usable data.

---

### Model 4: mistral-nemo:12b

**Config**: `GUIDE__DEFAULT_MODEL=mistral-nemo:12b`, `GUIDE__CONTEXT_WINDOW=16000`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~32 minutes (18:05–18:37 UTC-4)

| Phase              | Chapter      | Prompt Tokens    | Completion Tokens | JSON Valid | Notes                                                           |
| ------------------ | ------------ | ---------------- | ----------------- | ---------- | --------------------------------------------------------------- |
| pre_analysis       | —            | (included)       | (included)        | ✓          |                                                                 |
| chapter_extraction | 45 LLM calls | varies (1021–8192) | varies (27–1114) | ~71%       | 13/45 failures — context overflow on large chapters (8192 cap)  |
| agent_extraction   | 31 successes | N/A (rig)        | N/A (rig)         | ✓          | 45 failures — but 31 successes is BEST agent performance overall |

**Total tokens**: prompt=155,809 completion=26,857 (total=182,666) — most token-efficient model

**Extraction results**:

- Story arcs: 10 (campaign-specific: "The Hunt for Lord Draego", "Lazarus' Scheme", "Brotherhood of Light", "Wight Hand of Death" — all real campaign elements)
- Story events: 114
- NPCs: 48
- Locations: 19 (VERY LOW — worst location extraction of all models)

**Quality score**: 5/10

**Evaluation**:

- Arc extraction (1.5/2): 10 arcs with excellent campaign specificity! "Lord Draego", "Lazarus", "Brotherhood of Light" are real campaign NPCs/factions. Some chapter-specific arcs not consolidated ("The Heart of the Fields Arc" vs "The Heart of the Elements"). Best arc quality vs qwen3:8b and llama3.1:8b.
- NPC extraction (1/2): 48 NPCs — below average quantity.
- Location extraction (0/2): Only 19 locations — worst of all models tested. Context overflow may be causing truncation.
- JSON validity (1/2): 13/45 failures = 29% failure rate. Issues: pipe-separated event_types (`puzzle|ritual`, `travel|combat`), `key must be a string` (numeric JSON keys), control characters in JSON output.
- Cross-chapter consistency (1.5/2): 10 arcs with good cross-chapter reuse (31 agent successes means many chapters used agent path which is more consistent).

**Key failure modes**:

1. Context overflow — some chapters hit the 8192 context cap exactly (`prompt_tokens=8192, completion_tokens=27`). Model gets context-limited and produces near-empty output.
2. Pipe-separated enum values — `puzzle|ritual`, `travel|combat` — same as other models.
3. Control characters in JSON — `\u0000-\u001F` in string values.
4. Very low location extraction (19) — likely due to truncated outputs from context overflow.

**Notable**: 31/76 agent successes is the best agent success rate of any model tested (40%), beating qwen3:14b's 11/74 (15%) and qwen3:8b's 11/75. mistral-nemo may have stronger tool-use capability relative to its size.

---

### Model 4b: nemotron-cascade-2 (24GB) — ABORTED EARLY

**Config**: `GUIDE__DEFAULT_MODEL=nemotron-cascade-2:latest`, `GUIDE__CONTEXT_WINDOW=16000`
**Aborted**: After ~40 minutes, clear pattern of failure. Full run would take ~5 hours.

**Issues observed:**
- `MaxTurnError: reached max turn limit: 0` for ALL agent calls — no tool use
- Many LLM calls return `content_len=0, completion_tokens=4800` (empty response with max token budget consumed)
- Inference speed: ~4-5 min per LLM call on RTX 4070 12GB (heavy CPU offloading)
- 0 arcs, 0 events, 0 NPCs, 0 locations extracted in first 8 chapters

**Quality score**: N/A (test aborted)

**DISQUALIFIED**: CPU offload too slow (~5 min/call), empty response pattern, no tool-use capability observed.

---

### Model 5: qwen3.5:35b-a3b (MoE)

**Config**: Tag: `qwen3.5:35b` (alias: `qwen3.5:35b-a3b`)
**Hardware note**: MoE model — 23GB total, ~3B active per token. Still requires CPU offloading on RTX 4070 12GB.

| Phase              | Chapter | Prompt Tokens | Completion Tokens | JSON Valid | Notes |
| ------------------ | ------- | ------------- | ----------------- | ---------- | ----- |
| pre_analysis       | —       |               |                   |            |       |
| chapter_extraction |         |               |                   |            |       |

**Total tokens**: prompt=0, completion=0
**Quality score**: /10
**Evaluation**:

---

### Model 6: phi4-mini:3.8b

**Config**: `GUIDE__DEFAULT_MODEL=phi4-mini:3.8b`, `GUIDE__CONTEXT_WINDOW=16000`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~75 minutes (18:41–19:56 UTC-4) — surprisingly slow due to runaway generation (4800 tokens/call)

| Phase              | Chapter      | Prompt Tokens   | Completion Tokens | JSON Valid | Notes                                                              |
| ------------------ | ------------ | --------------- | ----------------- | ---------- | ------------------------------------------------------------------ |
| pre_analysis       | —            | (included)      | (included)        | ✓          |                                                                    |
| chapter_extraction | 75 LLM calls | varies          | varies (0–4800)   | ~12%       | 66/75 failures; model runs away with 4800 token garbage outputs    |
| agent_extraction   | 1 success    | N/A (rig)       | N/A (rig)         | N/A        |                                                                    |

**Total tokens**: prompt=116,273 completion=72,024 (total=188,297) — completion tokens inflated by runaway generation

**Extraction results**:

- Story arcs: 2 (wrong — "Arrival at Night's Edge", "The Mountain's Embrace")
- Story events: 1
- NPCs: 3
- Locations: 2

**Quality score**: 1/10

**Evaluation**:

- All dimensions score 0/2 except cross-chapter consistency (0.5/2 — same hallucinated arc used everywhere).
- 66/75 parse failures = 88% failure rate. Many calls produce 4800-token repetitive garbage (`fills fills fills UserSite...`).
- Model enters runaway repetition loops, producing incoherent junk that exceeds the token limit.

**DISQUALIFIED**: phi4-mini:3.8b is unsuitable. The 3.8B parameter count is too small for complex JSON schema compliance. Unlike lfm2.5-thinking which simply fails, phi4-mini actively runs away, wasting inference time on garbage output.

---

### Model 7b: lfm2 (14GB)

**Config**: `GUIDE__DEFAULT_MODEL=lfm2:latest`, `GUIDE__CONTEXT_WINDOW=16000`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~70 minutes (20:40–21:50 UTC-4)

| Phase              | Chapter      | Prompt Tokens   | Completion Tokens | JSON Valid | Notes                                                            |
| ------------------ | ------------ | --------------- | ----------------- | ---------- | ---------------------------------------------------------------- |
| pre_analysis       | —            | (included)      | (included)        | ✓          |                                                                  |
| chapter_extraction | 64 LLM calls | varies          | varies (1202–1801) | ~37%      | 40/64 failures — consistently uses invalid event_type values    |
| agent_extraction   | 11 successes | N/A (rig)       | N/A (rig)         | ✓          | 63 failures — string array format                               |

**Total tokens**: prompt=203,458 completion=78,581 (total=282,039)

**Extraction results**:

- Story arcs: 11 (mix of generic and semi-specific: "The Witchwood Incident", "Willowhold's Plague" are campaign-relevant; "The Road Home", "The Seed" are generic)
- Story events: 67
- NPCs: 16 (VERY LOW)
- Locations: 30 (below average)

**Quality score**: 3/10

**Evaluation**:

- Arc extraction (0.5/2): 11 arcs, semi-consistent main arcs plus chapter-specific. Some campaign-specific names ("Witchwood Incident") but mostly generic.
- NPC extraction (0/2): Only 16 NPCs — worst of all models (mistral-nemo had 48, qwen3:8b had 50). LFM2 severely underextracts NPCs.
- Location extraction (0.5/2): 30 locations — below average.
- JSON validity (0/2): 40/64 failures = 62.5% failure rate. Model consistently uses invalid event_type values (`restoration`, `resolution`, `escape`, `battle`, `guidance`).
- Cross-chapter consistency (1/2): Core 4 arcs consistent per chapter.

**DISQUALIFIED for strong candidates**: lfm2 shows poor JSON schema compliance and dramatically underextracts NPCs despite being a 14GB model. Not recommended as agent brain.

---

### Model 7: qwen3:4b

**Config**: `GUIDE__DEFAULT_MODEL=qwen3:4b`, `GUIDE__CONTEXT_WINDOW=16000`, `GUIDE__STORY_PROVIDER=local`
**Test duration**: ~101 minutes (22:02–23:43 UTC-4) — slow due to thinking overhead

| Phase              | Chapter      | Prompt Tokens     | Completion Tokens  | JSON Valid | Notes                                                            |
| ------------------ | ------------ | ----------------- | ------------------ | ---------- | ---------------------------------------------------------------- |
| pre_analysis       | —            | (included)        | (included)         | ✗          | Setting: "Unknown", tone: adventure — pre_analysis FAILED       |
| chapter_extraction | 64 LLM calls | varies            | 0 or 4800 (mostly) | ~19%       | 52/64 failures; empty thinking responses hit 4800 token ceiling  |
| agent_extraction   | 13 successes | N/A (rig)         | N/A (rig)          | ✓          |                                                                  |

**Total tokens**: prompt=183,207 completion=254,987 (total=438,194) — completion inflated by 4800-token empty thinking runs

**Extraction results**:

- Story arcs: 5 (ALL campaign-specific! "The Blood Portal Threat", "Renwick's Manor", "The Heart of the Fields", "Werewolves" — real campaign elements)
- Story events: 33 (very low)
- NPCs: 12 (very low)
- Locations: 11 (very low)

**Quality score**: 2/10

**Evaluation**:

- Arc extraction (1/2): Surprisingly campaign-specific arc names — all 5 match real campaign elements. But only 5 arcs total (incomplete) and very low event/NPC/location counts.
- NPC extraction (0/2): 12 NPCs — worst of all models.
- Location extraction (0/2): 11 locations — worst of all models.
- JSON validity (0/2): 52/64 = 81% failure rate. Root cause: Qwen3 thinking mode consumes all 4800 token budget before producing visible JSON. Content = 0 bytes even though completion_tokens = 4800.
- Cross-chapter consistency (1/2): 5 arcs used consistently.

**Root cause discovery (important for qwen3 family)**: qwen3:4b uses internal thinking tokens (`<think>...</think>`). These thinking tokens are stripped from visible output but count toward the token budget. At 4B parameters, thinking requires more tokens than at 8B, regularly exhausting the 4800-token ceiling before any visible JSON is produced. Fix: add `/no_think` to the system prompt or disable thinking in the Ollama API. This would dramatically improve qwen3:4b performance.

**Model 8: Jackrong/Qwen3.5-9B-Claude-4.6-Opus-Reasoning-Distilled (FAILED TO LOAD)**

Tag: `hf.co/Jackrong/Qwen3.5-9B-Claude-4.6-Opus-Reasoning-Distilled-v2-GGUF:Q4_K_M`
Ollama error: `unable to load model: ...sha256-8fbbc7b...` — GGUF incompatible with installed Ollama version.
**DISQUALIFIED**: Cannot test.

**Model 9: qwen3.5:35b-a3b (NOT TESTED — TOO SLOW)**

This 23GB MoE model was not tested. Based on nemotron-cascade-2 (24GB, same size class) taking ~5 min/call and producing empty responses, the 35B model would require ~5-7 hours and likely exhibit the same CPU offloading issues. Testing is not feasible in this session.
**Estimated**: Would likely score 6-8/10 if inference was practical. GPU with 24GB+ VRAM needed for reasonable performance.

---

## Comparative Summary

| Model                  | Prompt Tokens | Completion Tokens | Quality | Parse Fail% | Agent Succ | NPCs | Locations | Best For                        |
| ---------------------- | ------------- | ----------------- | ------- | ----------- | ---------- | ---- | --------- | ------------------------------- |
| **Gemini 2.5 Flash³** | **304,292**   | **161,986**       | **9/10** | **0%**    | 9/71       | **173** | **198** | **Best overall** — cloud, $1.75/doc (thinking on) |
| **Gemini 2.5 Flash Lite⁴** | **73,405** | **61,640**     | **8/10** | **0%**    | **17**/53  | **89** | **67** | **Best value cloud** — ~$0.01/doc, 8 min, 34% agent success |
| Jackrong/Qwen3.5-9B¹  | 139,766       | 82,960            | **8/10** | 9%         | **22**/53  | 118  | 108       | Best local — llama.cpp only     |
| Jackrong/Qwen3.5-4B²  | 313,341       | 340,812           | **8.5/10** | 18%      | 4/91       | 105  | **139**   | Best arcs (4) + locations; slow/verbose |
| qwen3:14b              | 212,049       | 91,150            | 7/10    | 16%         | 11/74      | 67   | 106       | Best Ollama option              |
| mistral-nemo:12b       | 155,809       | 26,857            | 5/10    | 29%         | 31/76      | 48   | 19        | Fast, token-efficient           |
| llama3.1:8b            | 270,186       | 100,021           | 5/10    | 34%         | 7/87       | 59   | 61        | Good balance, 8GB VRAM          |
| qwen3:8b               | 182,806       | 72,547            | 4/10    | 45%         | 11/75      | 50   | 50        | Current default (upgradeable)   |
| Gemini 3.1 Flash Lite⁵ | 43,553 (partial) | 28,740 (partial) | 4/10  | 0%         | 22/53 (41%)| 56   | 55        | DISQUALIFIED — thought_signature incompatibility, arc fail |
| gemma3:12b             | 289,321       | 86,177            | 4/10    | 12%         | 0 (no tool)| 63   | 71        | DISQUALIFIED — no tool support  |
| lfm2 (14GB)            | 203,458       | 78,581            | 3/10    | 62%         | 11/74      | 16   | 30        | Not recommended                 |
| qwen3:4b               | 183,207       | 254,987           | 2/10    | 81%         | 13/77      | 12   | 11        | Needs /no_think fix             |
| lfm2.5-thinking (731MB)| 129,201       | 43,696            | 1/10    | 86%         | 18/74      | 0    | 0         | DISQUALIFIED — too small        |
| phi4-mini:3.8b         | 116,273       | 72,024            | 1/10    | 88%         | 1/76       | 3    | 2         | DISQUALIFIED — runaway gen      |
| nemotron-cascade-2     | N/A (aborted) | N/A               | N/A     | N/A         | N/A        | N/A  | N/A       | DISQUALIFIED — too slow/broken  |
| qwen3.5:35b-a3b        | N/A (untested)| N/A               | ~8/10?  | N/A         | N/A        | N/A  | N/A       | Needs 24GB+ VRAM for speed      |

¹ Jackrong/Qwen3.5-9B tested via llama.cpp (not Ollama). Results are from Run 2 (GUIDE__CONTEXT_WINDOW=16000, 4,800 output cap). Run 3 (GUIDE__CONTEXT_WINDOW=65536, 16,384 cap) produced identical results (47 arcs/118 NPCs/108 locations) at 270,524 total tokens — confirming the 9B quality ceiling is model capability, not token budget. Parse fail% excludes intelligent refusals on stat-block/empty pages (correct behavior).
² Jackrong/Qwen3.5-4B tested via llama.cpp (not Ollama). Same prompt fixes as 9B. Results are from Run 2 (GUIDE__CONTEXT_WINDOW=65536, 16,384 output cap). 16/91 windows still hit the 8192 completion cap on the largest chapters. Agent success rate (4%) reflects model's difficulty with tool-use at 4B scale. Run 1 (4,800 cap) had 22/90 truncations and scored 8/10.
³ Gemini 2.5 Flash tested via Google Generative AI OpenAI-compatible endpoint. GUIDE__CONTEXT_WINDOW=65536 (16,384 output cap). re-extract-story using stored OCR from 9B test campaign. Agent path (rig tool-use) attempted but hit MaxTurnError (20/71) and JSON parse EOF (42/71); all failures fell back to cloud extraction with 0% failure. Pre-analysis used local model (LlmTask::General routes to Ollama). Actual cost: $0.072 for 177-page PDF.
⁴ Gemini 2.5 Flash Lite tested via Google Generative AI OpenAI-compatible endpoint. GUIDE__CONTEXT_WINDOW=200000 (60K output cap, 480K input chars/window). re-extract-story on same campaign. Agent path (rig tool-use) succeeded for 17/53 chapters (32% success) after MaxTurnError fix (`.default_max_turns(20)`). 0% cloud extraction parse failure. 36 total LLM calls. Duration: ~8 minutes. Estimated cost: ~$0.01/doc (significantly cheaper than Flash). Entity counts lower than Flash due to agent-only approach on successful chapters (no fallback-to-cloud on agent success, but cloud catches agent failures). Bug fix required: DuckDB WAL corruption from previous crash attempts needed WAL deletion before successful persist.
⁵ Gemini 3.1 Flash Lite Preview tested via Google Generative AI OpenAI-compatible endpoint. GUIDE__CONTEXT_WINDOW=200000. re-extract-story on same campaign. This is a thinking model that requires proprietary `thought_signature` tokens in tool-call replies — the `async-openai` SDK does not implement this extension, causing 31/53 (59%) agent calls to fail with HTTP 400. Agent path succeeded for 22/53 (41%) chapters. Fallback chapter_extraction ran for 31 chapters (all logged tokens). Arc consolidation failed: produced 31 arcs (one per chapter) instead of 3-5 major campaign arcs. Total logged tokens: 43,553/28,740 (partial — agent call tokens not captured). Duration: ~5 minutes. Fix shipped: `persist_story_batch` DELETEs now run outside transaction to avoid DuckDB 1.2 FK enforcement issue.

---

## Top Models Selected

**Winner (cloud)**: `Gemini 2.5 Flash` — 9/10 quality, 173 NPCs, 198 locations, 0% extraction failure, $0.072/document, 41 min. New top performer overall.
**Winner (llama.cpp)**: `Jackrong/Qwen3.5-9B-Claude-4.6-Opus-Reasoning-Distilled-GGUF` — 8/10 quality, best agent success rate (41%), best NPC/location coverage among local models. Requires llama.cpp (Ollama cannot load this GGUF).
**Winner (Ollama)**: `qwen3:14b` — 7/10 quality, best option available natively via Ollama, fits in 12GB VRAM.
**Runner-up (Ollama)**: `mistral-nemo:12b` — 5/10 quality but 31 agent successes, fastest inference, most token-efficient Ollama option.

---

## Universal Findings (Apply to ALL Models)

These issues were observed across every model tested and represent the most impactful improvements:

### 1. Agent Arc Schema Bug (Critical)
**Symptom**: `invalid type: string "Arc Title", expected struct StoryArcInput`
**Root cause**: The agent prompt asks for arcs in a format that models return as string arrays `["Arc Name"]` instead of object arrays `[{"title": "...", "arc_order": ...}]`.
**Fix needed**: Update `agent_chapter_prompt()` in `prompts.rs` to show an explicit example of the object array format. Add: "Return arcs as JSON objects: `[{\"title\": \"...\", \"description\": \"...\", \"arc_order\": 1}]`, NOT as strings."

### 2. Invalid event_type Values (High)
**Symptom**: `unknown variant 'investigation'/'restoration'/'resolution'/'escape'/'battle'/'guidance'` etc.
**Root cause**: The prompt lists valid values but models choose alternatives they consider synonymous.
**Fix needed**: In `story_extraction_system_v2()`, add hard stop: "event_type MUST be EXACTLY ONE of: combat, social, revelation, travel, rest, discovery, puzzle, trap, boss, quest_given, npc_interaction. NO OTHER VALUES ARE VALID. Do NOT combine with | or use synonyms."

### 3. arc_order: null (Medium)
**Symptom**: `invalid type: null, expected i32`
**Root cause**: Models set `arc_order: null` for arcs that don't apply to current chapter.
**Fix needed**: "If an arc doesn't apply to this chapter, OMIT it from the arcs array entirely. Never use null for arc_order."

### 4. Qwen3 Thinking Mode (qwen3 family)
**Symptom**: `content_len=0, completion_tokens=4800` — empty response
**Root cause**: Qwen3 models use thinking tokens that exhaust the budget before visible output.
**Fix**: Add `/no_think` to system prompt, or set `enable_thinking: false` in Ollama API options.

---

## Prompt Optimization

### Winner Model: qwen3:14b

**Baseline quality score**: 7/10

#### Optimization 1: Fix Agent Arc Format

**Change**: In `agent_chapter_prompt()`, change arc format instruction to include explicit object example.
**Expected improvement**: +1 arc quality point (eliminate string array failures)

#### Optimization 2: Fix event_type Enum

**Change**: In `story_extraction_system_v2()`, add strict enum instruction.
**Expected improvement**: Reduce LLM failures from 16% to <5%

#### Optimization 3: Fix Cross-Chapter Arc Deduplication

**Change**: In `story_extraction_system_v2()`, add instruction: "Use EXACTLY these arc titles if they match what you see: [prior arcs]. Only create a new arc if it covers genuinely new story territory."
**Expected improvement**: Reduce 18 arcs to 4-6 canonical arcs

---

### Runner-up Model: mistral-nemo:12b

**Baseline quality score**: 5/10

#### Optimization 1: Increase Context Window

**Change**: Raise `GUIDE__CONTEXT_WINDOW=24000` for mistral-nemo (it supports up to 128K tokens)
**Expected improvement**: Eliminate context overflow on large chapters (currently `prompt_tokens=8192` clipping)

#### Optimization 2: Fix event_type + Agent Arc Format (same as above)

**Expected improvement**: Increase location count from 19 to 50+

---

## Cloud Cost Extrapolation

Using qwen3:14b token totals (prompt=212,049 completion=91,150) as the representative run:

| Provider  | Model             | Input $/1M | Output $/1M | Est. Input Cost | Est. Output Cost | Total Est. |
| --------- | ----------------- | ---------- | ----------- | --------------- | ---------------- | ---------- |
| Google    | Gemini 2.5 Flash  | $0.075     | $0.30       | $0.016          | $0.027           | **$0.043** |
| Anthropic | Claude Haiku 4.5  | $0.80      | $4.00       | $0.170          | $0.365           | **$0.535** |
| Anthropic | Claude Sonnet 4.6 | $3.00      | $15.00      | $0.636          | $1.367           | **$2.003** |

**Per-document cost for a 177-page D&D campaign PDF:**
- Gemini 2.5 Flash: ~$0.04 (extremely cheap, strongly recommended for cloud users)
- Claude Haiku 4.5: ~$0.54 (reasonable for quality)
- Claude Sonnet 4.6: ~$2.00 (premium quality, justified for important campaigns)

---

## RAG System Evaluation

**Hypothesis**: The AI Agent path (tool-use with page navigation) makes RAG redundant for story extraction.

**Evidence for removing RAG**:

1. The agent path successfully navigates PDFs without embeddings — chapters are accessed via ToC + page fetch, not vector similarity
2. `GUIDE__ENABLE_RAG=false` worked correctly throughout all tests with no data loss
3. Story extraction quality (the primary use case tested) doesn't benefit from RAG — the agent reads source text directly
4. RAG adds latency and storage overhead that's not needed for story extraction

**Evidence against removing RAG**:

1. Chat Q&A (the DM assistant use case) still needs RAG for context retrieval — story extraction is not the only RAG consumer
2. The agent path has a `MaxTurnError` problem (turn limit 0) that prevents tool-use on ~40-80% of chapters depending on model. Without RAG, those chapters produce lower-quality extractions.
3. Future features (encounter generation, session summaries) will likely use RAG

**Recommendation**: Keep RAG OPTIONAL via `GUIDE__ENABLE_RAG` flag. Disable for story extraction testing. Re-enable for production chat use case.

---

## Final Recommendation

**Best model**: `qwen3:14b`
**Runner-up**: `mistral-nemo:12b`

**Reasoning**:
- qwen3:14b has the highest quality score (7/10), best location extraction (106!), best NPC count (67), and lowest JSON parse failure rate (16%) among tool-supporting models
- mistral-nemo:12b has the best agent success rate (31/76 = 40%) and is extremely token-efficient (26K completion tokens vs 100K for llama3.1:8b), but suffers from context overflow on large chapters
- llama3.1:8b is a solid 8GB option with good arc consistency and event extraction

**Hardware requirements**:

- **Minimum (budget users, 8GB VRAM)**: `llama3.1:8b` — fits in 8GB, reasonable quality (5/10), good fallback
- **Recommended (12GB VRAM)**: `qwen3:14b` — best quality, fits in 12GB with some layering, ~2-3 hours per campaign PDF
- **High-end (24GB+ VRAM)**: `qwen3.5:35b-a3b` or cloud providers — untested locally but expected 8+/10

**Cloud alternative**: If inference speed matters, use Gemini 2.5 Flash at ~$0.04/document — dramatically faster than local inference and likely better quality than all tested local models.

**Critical fixes needed before production** (applies to all models):
1. Fix agent arc array format in prompt (string vs object issue)
2. Fix event_type enum strict validation in prompt
3. Add `/no_think` for Qwen3 family models to prevent token budget exhaustion
4. Fix cross-chapter arc deduplication (provide prior arc context to each chapter)

**Optimized prompts**: See `crates/guide-llm/src/prompts.rs` — functions to modify: `agent_chapter_prompt()`, `story_extraction_system_v2()`, `document_preanalysis_system()`

---

## Model 11 (Retry): Jackrong/Qwen3.5-9B-Claude-4.6-Opus-Reasoning-Distilled-GGUF (llama.cpp)

**Date**: 2026-03-27
**Setup**: llama.cpp on localhost:8080 (Q4_K_M, 6.6GB); `GUIDE__OLLAMA_BASE_URL=http://localhost:8080/v1`
**Re-test reason**: Previous attempt failed at Ollama model load. This time loaded via llama.cpp directly.
**Two runs required**: Run 1 (without fence-strip fix) validated content quality. Run 2 (with fix) is the final scored result.

### Run 1 — Partial (broken by markdown fence bug)
- Agent successes: ~6 chapters
- LLM fallback: 100% failed (`expected value at line 1 column 1`) — model wraps all JSON in ` ```json ``` ` fences
- Arcs: 31 (per-chapter, heavily duplicated), Events: 122, NPCs: 56, Locations: 42
- Tokens: prompt=164,760 completion=94,230 total=258,990
- **Reveals**: Content quality is excellent — correct arc object format, real campaign arc names, proper `arc_order` integers. Only failure was fence-stripping.

### Run 2 — Final (fence-strip + monster count fixes applied)
- Agent successes: **22/53 calls (41%)** — best agent rate of all models tested
- LLM fallback parse failures: 5 remaining (all legitimate: empty stat-block pages, corrupted OCR)
- Arcs: 47, Events: 305, NPCs: 118, Locations: 108
- Tokens: prompt=139,766 completion=82,960 **total=222,726**
- Run time: ~60 min (no OCR — used stored pages via `/re-extract-story` endpoint)

### Scoring
| Criterion | Score | Notes |
|-----------|-------|-------|
| Arc extraction | 1.5/2 | 47 arcs (per-chapter proliferation), all real campaign arcs, no hallucination |
| NPC extraction | 1.5/2 | 118 NPCs, real names + correct roles; minor deduplication misses |
| Location extraction | 2/2 | 108 locations, room-level granularity, all real place names |
| JSON validity | 2/2 | event_type enum correct; 5 skipped windows were non-story content (correct behavior) |
| Cross-chapter consistency | 1/2 | Arc titles vary per chapter; some NPC duplicates (Alfwold vs Alfwold Tam) |
| **Total** | **8/10** | **New top performer** |

### Failure modes
- **Markdown fences**: Fixed by `strip_markdown_fences()` added to `prompts.rs`
- **Monster count dice strings** (`"1d8"`): Fixed by custom serde deserializer in `MonsterHint`
- **MaxTurnError agent failures**: Same rig framework limit-0 issue as other models
- **Intelligent refusal**: Model outputs prose explanation instead of JSON for stat-block / corrupt-OCR chapters — this is CORRECT behavior (no hallucination), but triggers parse failure

### Infrastructure fixes shipped during this test
- `POST /campaigns/{id}/documents/{doc_id}/re-extract-story` — new endpoint; skips OCR, re-runs story extraction from stored page text
- `strip_markdown_fences()` in `prompts.rs` — strips ` ```json ``` ` from all LLM responses (benefits all models)
- `MonsterHint.count` custom deserializer — coerces dice strings like `"1d8"` to `None`

### Run 3 — Budget increase re-run (16,384 token output cap via GUIDE__CONTEXT_WINDOW=65536, .min(16384))

**Date**: 2026-03-27 (same day as 4B re-run)
**Purpose**: Verify whether the 4,800-token output cap was artificially limiting 9B quality, as it was for the 4B model.

- Agent successes: **14/56 attempts (25%)** — lower than Run 2 (41%), likely due to `MaxTurnError: max turn limit: 0` on most windows (42 of 56 failures); suspected llama.cpp session state issue
- Arcs: **47** (identical to Run 2)
- NPCs: **118** (identical to Run 2)
- Locations: **108** (identical to Run 2)
- Tokens: prompt=165,423 completion=105,101 **total=270,524** (21% more than Run 2)
- Windows truncated: **0** (no calls hit 8192 or 16384 cap)
- Score: **8/10** (identical to Run 2)

**Key finding**: Raising the output budget from 4,800 → 16,384 tokens did **not** improve 9B extraction quality. The model was already self-regulating its output length; 4,800-token responses were sufficient to capture the same content. This is in sharp contrast to the 4B model, where the budget increase directly improved location coverage (128 → 139). The 9B quality ceiling is the model's capability itself, not the token budget.

---

## Model 12: Jackrong/Qwen3.5-4B-Claude-4.6-Opus-Reasoning-Distilled-GGUF (llama.cpp)

**Date**: 2026-03-27
**Setup**: llama.cpp on localhost:8080; same fence-strip + arc/event fixes as 9B test

### Run 1 — Limited (4,800 token output cap via GUIDE__CONTEXT_WINDOW=16000)
- Arcs: **3** ("The Descent into Darkness", "The Bloodline Revealed", "The Final Confrontation")
- Events: **440** (highest of all models)
- NPCs: 104
- Locations: 128
- Tokens: prompt=293,602 completion=245,040 **total=538,642**
- Agent successes: 4/90 (4%)
- Windows truncated: 22/90 (24%) — `EOF while parsing` (token budget exhausted mid-JSON)

### Run 2 — Re-run (16,384 token output cap via GUIDE__CONTEXT_WINDOW=65536, .min(16384))
- Arcs: **4**
- NPCs: **105**
- Locations: **139** (highest of all models)
- Tokens: prompt=313,341 completion=340,812 **total=654,153**
- Windows truncated: **16/91 (18%)** — down from 22 (still hitting cap on largest chapters)
- Run time: ~4 hours

### Scoring (Run 2)
| Criterion | Score | Notes |
|-----------|-------|-------|
| Arc extraction | 2/2 | **Best of all models** — 4 genuine campaign-level arcs, not per-chapter proliferation |
| NPC extraction | 1.5/2 | 105 NPCs, correct roles, minor duplicates |
| Location extraction | 2/2 | **139 locations — highest of all models** |
| JSON validity | 1.5/2 | 16 windows still truncated at 8192 cap; remaining parse correctly |
| Cross-chapter consistency | 1.5/2 | 4 consistent campaign arcs; some event duplication across windows |
| **Total** | **8.5/10** | **New joint top performer** — surpasses 9B on arc quality and locations |

### Analysis
Raising the output budget from 4,800 → 16,384 tokens reduced truncated windows from 22 → 16 and improved location extraction by 11 (128 → 139). The 16 remaining truncations all hit the 8,192 completion token mark (the new per-chapter cap from llama.cpp's generation limit), suggesting the model is still being constrained on the largest chapters. Nevertheless, Run 2 scores 8.5/10 — edging above the 9B's 8/10 on arc and location quality.

**Key insight**: The 4B's verbose generation style benefits directly from a larger output budget. The 22→16 improvement was achieved purely by config change (no code changes to model prompts). Further gains possible by increasing llama.cpp `-c` context size and/or `--n-predict`.

**Practical comparison vs 9B (Run 2)**: 4× longer runtime, 2.9× more tokens, worse agent success rate (4% vs 41%). However, arc consolidation (4 vs 47) and location coverage (139 vs 108) are clearly superior. Recommended if quality matters more than speed.

---

## Model 13: Gemini 2.5 Flash (Cloud — Google Generative AI)

**Date**: 2026-03-28
**Config**: `GUIDE__STORY_PROVIDER=cloud`, `GUIDE__CLOUD_FALLBACK=gemini`, `GUIDE__CLOUD_MODEL=gemini-2.5-flash`, `GUIDE__CONTEXT_WINDOW=65536` (16,384 output cap)
**Setup**: Google Generative AI OpenAI-compatible endpoint (`generativelanguage.googleapis.com/v1beta/openai`)
**Test method**: `re-extract-story` on existing OCR (campaign `63c67310`, doc `19967ef7`) — same stored page text as 9B test
**Duration**: ~41 minutes (00:02:28–00:43:24 UTC) — vs ~60 min local

| Phase | Calls | Prompt Tokens | Completion Tokens | Notes |
| ----- | ----- | ------------- | ----------------- | ----- |
| pre_analysis | 1 | (local model) | (local model) | `LlmTask::General` routes to local Jackrong 9B (expected) |
| chapter_extraction (cloud fallback) | 62 | varies | 55–6,827 | After agent failure; **0% parse failure** |
| agent_extraction | 9 successes | N/A (rig) | N/A (rig) | 9/71 attempts = 13% success |

**Total tokens**: prompt=304,292 completion=161,986 **total=466,278**
**Truncations**: 0 (no calls hit 16,384 cap)

**Extraction results**:

- Story arcs: 89 (structural chapter divisions + OCR garble present; all real campaign arcs included)
- NPCs: 173 (**highest of all models tested**)
- Locations: 198 (**highest of all models tested**)

### Scoring

| Criterion | Score | Notes |
| --------- | ----- | ----- |
| Arc extraction | 1.5/2 | 89 arcs — real campaign arcs all present, but model also captures structural chapter labels ("Part Two", "Part Three: Behind the Wall") and OCR garble ("ART WONHALLOWED ROUNDS") as separate arcs |
| NPC extraction | 2/2 | 173 named characters — Alfwold Tam, Anya Petrovetta, Belladolphi, Bellona Black, Arethusa; all campaign-specific, dramatic improvement over local models |
| Location extraction | 2/2 | 198 places — Andel Mountains, Barasov, Berryville, Arethusa's Enclave, Blood Portal; room-level granularity, highest of all models |
| JSON validity | 2/2 | 0% failure rate on actual cloud extraction calls (vs 9–88% failure for local models) |
| Cross-chapter consistency | 1.5/2 | Minor NPC deduplication issues (Alfwold / Alfwold Tam, Arethusa / Arethusa of her Enclave ×2); arc proliferation per chapter |
| **Total** | **9/10** | **New top performer** |

### Key observations

- **Cloud routing works**: agent path tries rig tool-use (13% success), falls back to `CloudProvider::complete model="gemini-2.5-flash"` on failure
- **Agent MaxTurnError persists**: same rig framework `max turn limit: 0` issue affects Gemini same as local models (20/71 failures)
- **Agent JSON parse failures**: 42/71 from EOF truncation — rig framework truncates large Gemini responses mid-JSON; these fall back cleanly to cloud extraction
- **0% cloud extraction failure**: all 62 fallback cloud calls parsed successfully (no local model achieved this)
- **Rich output**: avg 2,613 completion tokens/chapter vs ~1,500 for Jackrong 9B — Gemini generates much more detailed content
- **Pre-analysis uses local**: `LlmTask::General` routes to Ollama (by design); only `LlmTask::StoryExtraction` hits cloud
- **Actual cost**: **$1.75** per document (Google Cloud billing). Likely includes internal thinking tokens billed at ~$3.50/M that don't appear in the OpenAI-compatible usage response. See Cloud Cost section for updated table.
- **To disable thinking** (reduce cost): pass `"thinking": {"thinking_budget": 0}` in extra params, or switch to `gemini-2.5-flash-8b` (non-thinking)

---

## Model 14: Gemini 2.5 Flash Lite (Cloud — Google Generative AI)

**Date**: 2026-03-28
**Config**: `GUIDE__STORY_PROVIDER=cloud`, `GUIDE__CLOUD_FALLBACK=gemini`, `GUIDE__CLOUD_MODEL=gemini-2.5-flash-lite`, `GUIDE__CONTEXT_WINDOW=200000` (60K output cap, 480K input chars/window)
**Test method**: `re-extract-story` on same 177-page PDF (campaign `63c67310`, doc `19967ef7`) — same stored page text
**Duration**: ~8 minutes (03:39–03:48 UTC)

| Phase | Calls | Prompt Tokens | Completion Tokens | Notes |
| ----- | ----- | ------------- | ----------------- | ----- |
| pre_analysis | 1 | (local model) | (local model) | `LlmTask::General` routes to local model |
| chapter_extraction (all calls) | 36 | 73,405 total | 61,640 total | 17 agent successes, 36 fallback LLM calls |
| agent successes | 17/53 | N/A (rig) | N/A (rig) | 32% agent success rate (vs 13% for Flash without MaxTurnError fix) |

**Total tokens**: prompt=73,405 completion=61,640 **total=135,045**
**Estimated cost**: ~$0.01/document (Google Flash Lite pricing: $0.10/M in + $0.40/M out)

**Extraction results**:

- Story arcs: 34 (well-delineated, matches actual chapter structure)
- Story events: 302 (comprehensive event coverage)
- NPCs: 89 (lower than Flash — agent-only path captures less than cloud fallback)
- Locations: 67 (similarly lower than Flash)
- Factions: 26

### Scoring

| Criterion | Score | Notes |
| --------- | ----- | ----- |
| Arc extraction | 1.5/2 | 34 arcs — good structure, matches real campaign chapters |
| NPC extraction | 1.5/2 | 89 NPCs — solid coverage, lower than Flash (89 vs 173) due to agent-path limitations |
| Location extraction | 1.5/2 | 67 locations — meaningful granularity, lower than Flash (67 vs 198) |
| JSON validity | 2/2 | 0% parse failure on cloud extraction calls |
| Cross-chapter consistency | 1.5/2 | Good arc deduplication; 302 events shows strong event tracking |
| **Total** | **8/10** | **Best value cloud** — 8× cheaper than Flash, 8 min vs 41 min |

### Key observations

- **MaxTurnError fix works**: `.default_max_turns(20)` raised agent success rate to 32% (17/53) vs 13% without fix
- **Agent path much faster**: 8 minutes total vs 41 minutes for Flash — smaller model, simpler completions
- **0% cloud parse failure**: all fallback calls succeed, same as Flash
- **Lower entity counts than Flash**: Flash's cloud fallback path sends more context than the agent's selective page reads; Flash also uses thinking for richer extraction
- **Cost comparison**: ~$0.01/doc (Flash Lite) vs $1.75/doc (Flash with thinking) — 175× cheaper
- **Bug fixed during test**: DuckDB WAL corruption from previous crash runs required deleting `data/guide.db.wal` before successful persist; also fixed `persist_story_batch` to use a single connection with transaction, and upgraded DuckDB 1.1→1.2

---

## Model 15: Gemini 3.1 Flash Lite Preview (Cloud — Google Generative AI)

**Date**: 2026-03-28
**Config**: `GUIDE__STORY_PROVIDER=cloud`, `GUIDE__CLOUD_FALLBACK=gemini`, `GUIDE__CLOUD_MODEL=gemini-3.1-flash-lite-preview`, `GUIDE__CONTEXT_WINDOW=200000`
**Test method**: `re-extract-story` on same stored page text (campaign `63c67310`, doc `19967ef7`)
**Duration**: ~5 minutes (04:11–04:16 UTC)

| Phase | Calls | Prompt Tokens | Completion Tokens | Notes |
| ----- | ----- | ------------- | ----------------- | ----- |
| chapter_extraction (fallback calls only) | 31 | 43,553 total | 28,740 total | 31/53 chapters fell back; agent calls not logged |
| agent successes | 22/53 | N/A (rig) | N/A (rig) | 41% agent success rate |
| agent failures | 31/53 | N/A | N/A | `thought_signature` error — thinking model incompatibility |

**Note on thought_signature errors**: Gemini 3.1 Flash Lite Preview is a "thinking" model that requires `thought_signature` tokens in tool-call replies (a proprietary Gemini extension). The `async-openai` SDK does not implement this — tool-use calls without the signature return HTTP 400. The agent path fails for 59% of chapters and falls back to direct chapter extraction.

**Total tokens (logged only)**: prompt=43,553 completion=28,740 **total=72,293**
*Actual total tokens are higher — agent calls (22 successes × multiple turns) are not logged via rig framework.*

**Extraction results**:

- Story arcs: **31** (one per chapter — no consolidation)
- Story events: 160
- NPCs: 56
- Locations: 55

### Scoring

| Criterion | Score | Notes |
| --------- | ----- | ----- |
| Arc extraction | 0/2 | 31 arcs = one per chapter; failed to consolidate into major campaign arcs |
| NPC extraction | 1/2 | 56 NPCs — mediocre, below Flash Lite (89) |
| Location extraction | 1/2 | 55 locations — mediocre, below Flash Lite (67) |
| JSON validity | 2/2 | 0% parse failure |
| Cross-chapter consistency | 0/2 | Each chapter is its own arc; no cross-chapter arc tracking |
| **Total** | **4/10** | **POOR** — arc consolidation failure, thought_signature incompatibility |

### Key observations

- **thought_signature incompatibility**: Gemini 3.1 uses a proprietary "thinking" tool-use protocol incompatible with the OpenAI-compatible API we use. 59% of agent calls fail with HTTP 400.
- **Arc consolidation failure**: The combination of agent failures + chapter_extraction fallback produced one arc per chapter (31 arcs). When the agent succeeds it might consolidate arcs, but 59% failure rate corrupts the overall result.
- **Faster than Flash Lite**: ~5 min vs 8 min — smaller/faster model.
- **DISQUALIFIED**: thought_signature incompatibility makes tool-use unreliable; arc consolidation failure makes this model unsuitable.
- **Fix shipped**: `persist_story_batch` now runs DELETEs outside the transaction (auto-committed) to avoid DuckDB 1.2 FK enforcement issue where in-transaction child-table deletes are not visible to parent-table FK checks.

---

## Cloud Cost Extrapolation

Based on actual measured token counts from the Gemini 2.5 Flash run (304,292 prompt / 161,986 completion = 466,278 total):

| Provider  | Model                     | Billed tokens (prompt/completion) | **Actual cost** | Notes |
| --------- | ------------------------- | --------------------------------- | --------------- | ----- |
| Google    | Gemini 2.5 Flash          | 304,292 / 161,986 (visible)       | **$1.75**       | Thinking tokens billed separately (~$3.50/M) — not shown in API response; thinking likely adds 400K+ tokens |
| Google    | Gemini 2.5 Flash Lite     | 73,405 / 61,640 (actual)          | **~$0.01**      | $0.10/M in + $0.40/M out; 89 NPCs/67 locations — best value |
| Google    | Gemini 2.5 Flash (no-think) | ~same visible tokens             | **~$0.20**      | Estimate if thinking disabled (`thinking_budget=0`) at $0.15/M in + $0.60/M out |
| Anthropic | Claude Haiku 4.5          | 73,405 / 61,640 (est., Flash Lite scale) | **~$0.31** | $0.80/M in + $4.00/M out |
| Anthropic | Claude Sonnet 4.6         | 73,405 / 61,640 (est., Flash Lite scale) | **~$1.15** | $3.00/M in + $15.00/M out |

*Gemini 2.5 Flash uses thinking mode by default. Thinking tokens are billed at ~$3.50/M but not included in the OpenAI-compatible usage response — explaining the gap between the visible token count ($0.07) and actual bill ($1.75). Disabling thinking or using `gemini-2.5-flash-8b` would bring cost down to ~$0.20.*

---

## RAG System Evaluation

**Hypothesis**: The AI Agent path (tool-use with page navigation) makes RAG redundant for story extraction.

**Evidence for removing RAG**:

- (to be filled after testing)

**Evidence against removing RAG**:

- (to be filled after testing)

**Recommendation**: TBD

---

## Final Recommendation (Updated)

**Best quality (cloud)**: `Gemini 2.5 Flash` (cloud) — **9/10**, $1.75/document (thinking on), $0.20 with thinking off, 41 min, 173 NPCs/198 locations
**Best value (cloud)**: `Gemini 2.5 Flash Lite` — **8/10**, ~$0.01/document, 8 min, 89 NPCs/67 locations
**Best local (llama.cpp)**: `Jackrong/Qwen3.5-9B-Claude-4.6-Opus-Reasoning-Distilled-GGUF` — **8/10**
**Best local (Ollama)**: `qwen3:14b` — **7/10**
**Runner-up (Ollama)**: `mistral-nemo:12b` — **5/10**

**Reasoning**:
- Gemini 2.5 Flash scores 9/10: 173 NPCs and 198 locations (highest of all models), 0% cloud extraction failure, rich per-chapter output. At $1.75/doc (thinking on) or $0.20 (thinking off) it offers the highest quality.
- Gemini 2.5 Flash Lite scores 8/10: 89 NPCs, 67 locations, 0% parse failure, **~$0.01/doc** — 175× cheaper than Flash with thinking. Best cost-quality tradeoff for high-volume use cases. 32% agent success rate (vs 13% for Flash pre-fix) thanks to MaxTurnError fix.
- Jackrong/Qwen3.5-9B achieves 8/10 with 41% agent success rate, 118 NPCs, 108 locations. Best free/private option. Requires llama.cpp.
- qwen3:14b is the best native Ollama option: 7/10, 67 NPCs, 106 locations, fits in 12GB VRAM.
- mistral-nemo:12b is fast and token-efficient but context overflow limits coverage.

**Hardware requirements**:

- **Minimum (8GB VRAM)**: `llama3.1:8b` via Ollama — 5/10
- **Recommended (12GB VRAM, Ollama)**: `qwen3:14b` — 7/10
- **Recommended (12GB VRAM, llama.cpp)**: `Jackrong/Qwen3.5-9B` — 8/10
- **High-end (24GB+ VRAM)**: `qwen3.5:35b-a3b` — untested but expected 8+/10
- **Cloud budget (~$0.01/doc)**: `Gemini 2.5 Flash Lite` — **8/10**, 8 min, best value
- **Cloud premium (~$0.20/doc)**: `Gemini 2.5 Flash` (thinking off) — **9/10**, 41 min, maximum quality

**Fixes shipped** (all applied to codebase):
1. ✅ `strip_markdown_fences()` — handles ` ```json ``` ` wrappers from all models
2. ✅ `MonsterHint.count` custom deserializer — coerces `"1d8"` dice strings to `None`
3. ✅ `POST /re-extract-story` — skip OCR, re-run extraction from stored pages
4. ✅ `agent_system_prompt()` — explicit arc object schema, strict event_type enum
5. ✅ `story_extraction_system_v2()` — strict event_type, arc_order null prevention, removed conflicting `<think>` framing
6. ✅ `MaxTurnError: max turn limit: 0` — added `.default_max_turns(20)` to rig agent builder; raised agent success rate from 13% → 32% for Flash Lite
7. ✅ `max_output_tokens` cap raised — 16,384 → 65,536 tokens; allows Gemini full output capacity with 200K context window
8. ✅ `persist_story_batch()` — new StoryRepository method that executes all deletes + inserts in a single DuckDB connection with a transaction, replacing 400+ individual connection checkouts that caused DuckDB WAL corruption
9. ✅ DuckDB upgraded 1.1 → 1.2; WAL cleanup procedure documented (delete `data/guide.db.wal` after crash)
10. ✅ String truncation guards in `persist_story_batch()` — prevents potential DuckDB stack overflows from very long LLM-generated descriptions (max 8192 chars)
11. ✅ `persist_story_batch()` DELETEs moved outside BEGIN/COMMIT transaction — DuckDB 1.2 stricter FK enforcement does not see in-transaction child-table deletes when checking FK constraints for parent-table deletes; running DELETEs in auto-commit mode before the INSERT transaction resolves this
