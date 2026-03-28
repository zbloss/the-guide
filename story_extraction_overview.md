Story Extraction Pipeline

Phase 0 — Document Ingestion Prerequisites

Before story extraction even starts, ingest_campaign_document has already:

1. Rendered all PDF pages via pdfium/OCR into raw text
2. Chunked the text into overlapping DocumentChunk segments
3. Embedded and stored every chunk as a lore vector in DuckDB

Story extraction is only attempted for Campaign and Supplemental document kinds (not rulebooks or DM guides).

---

Phase 1 — Document Pre-Analysis

analyze_document_structure is called on the first ~4 chunks (≤5K chars) of the document. A quick LLM call returns a DocumentContext
JSON object containing:

- campaign_setting (e.g. "Barovia")
- tone (e.g. "horror", "adventure")
- themes (e.g. ["undead", "vampires", "tragedy"])
- chapter_names, major_npcs, major_locations

This context is passed to every subsequent extraction call to keep the LLM oriented. Failures here are non-fatal — it just defaults
to empty strings.

---

Phase 2 — Chapter Grouping

group_chunks_by_chapter groups all chunks by their top-level section_path segment (the part before the first >). This gives you one
bucket per chapter. If the document has fewer than 2 chapters, it falls back to a single-call path on the entire text.

---

Phase 3 — Per-Chapter LLM Extraction (the main path)

For each chapter:

1. Window splitting — if the chapter is too large for one LLM call (max_input_chars), it's split into sequential windows
2. extract_story_for_chapter — each window gets its own LLM call with:


    - A system prompt embedding the DocumentContext (setting, themes, tone)
    - A user prompt containing the raw chunk text, document title, chapter name, and a rolling prev_chapter_summary
    - JSON mode enabled

3. The response is cleaned (strip_think_block), then extract_first_json_object strips any trailing text, and serde deserializes it
   into StoryExtractionResult
4. On success, a compact summary of major events and arcs is appended to prev_chapter_summary (capped at ~2000 chars) so the next
   chapter knows what came before — this is the cross-chapter continuity mechanism
5. On failure, the window is skipped with a warning (not a fatal error)

---

Phase 4 — Merging Chapter Results

merge_story_extractions combines all per-chapter results into a single StoryExtractionResult:

- Arcs — deduplicated by lowercase title; assigned sequential arc_order
- Events — event orders are offset by chapter_idx \* 1000 so ordering is globally stable
- NPCs, locations, factions — deduplicated by lowercase name across all chapters
- Character arcs — merged per character, with arc points renumbered in sequence

---

Phase 5 — Database Persistence

The merged result is saved to DuckDB in dependency order:

1. story_arcs → keyed by title for later lookups
2. story_events → linked to arcs via arc_title → arc_id map
3. story_subplots → linked to arcs
4. character_arcs
5. prepopulated_encounters → linked to events via story_event_title → event_id map
6. npcs, locations, factions → each has a non-fatal warn-and-continue on insert error

The StoryExtractionResult struct is what the LLM outputs — it contains the Input variants of each type (no UUIDs), which get upgraded
to full domain objects with UUIDs during the DB inserts.
