"""End-to-end pipeline test: PDF → Docling → PageIndex tree → Ollama query.

Usage:
    uv run python scripts/test_pipeline.py <pdf_path> "<query>"
"""

from __future__ import annotations

import asyncio
import json
import sys
import time
from pathlib import Path
from uuid import uuid4

# ── make sure src/ is on the path ────────────────────────────────────────────
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

import logging

from guide.config import AppConfig
from guide.hardware import detect_device, log_hardware_summary, resolve_num_threads
from guide.llm.client import CompletionRequest, LlmTask, Message
from guide.llm.router import LlmRouter
from guide.pdf.extractor import extract_document
from guide.pdf.pipeline import _build_index, _build_node_map, _extract_json_text, _strip_text

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s  %(levelname)-7s  %(name)s — %(message)s",
)
logger = logging.getLogger("pipeline_test")


async def main(pdf_path: str, query: str) -> None:
    cfg = AppConfig()
    device = detect_device(cfg.device)
    num_threads = resolve_num_threads(cfg.num_threads)
    log_hardware_summary(device, num_threads)

    llm = LlmRouter.from_config(cfg)

    # ── 1. Extract ────────────────────────────────────────────────────────────
    logger.info("=== STEP 1: Docling extraction ===")
    t0 = time.perf_counter()
    pdf_bytes = Path(pdf_path).read_bytes()
    logger.info("PDF size: %.1f MB", len(pdf_bytes) / 1_048_576)

    extraction = await extract_document(pdf_bytes, device=device, num_threads=num_threads)

    elapsed = time.perf_counter() - t0
    logger.info(
        "Extraction done in %.1fs — %d pages, markdown length: %s chars",
        elapsed,
        len(extraction.pages),
        f"{len(extraction.full_markdown):,}",
    )

    # ── 2. Build PageIndex tree ───────────────────────────────────────────────
    logger.info("=== STEP 2: Building PageIndex tree ===")
    t0 = time.perf_counter()
    doc_id = uuid4()
    index_data = _build_index(extraction.full_markdown, doc_name=Path(pdf_path).stem)
    elapsed = time.perf_counter() - t0

    structure = index_data.get("structure", [])
    node_map = _build_node_map(structure)
    logger.info(
        "Tree built in %.2fs — %d top-level sections, %d total nodes",
        elapsed,
        len(structure),
        len(node_map),
    )

    # Show the top-level table of contents
    logger.info("--- Table of contents (top level) ---")
    for node in structure:
        children = len(node.get("nodes", []))
        suffix = f"  [{children} subsections]" if children else ""
        logger.info("  [%s] %s%s", node["node_id"], node["title"], suffix)

    # ── 3. Persist to disk ────────────────────────────────────────────────────
    logger.info("=== STEP 3: Persisting index to disk ===")
    index_dir = Path("data/indexes/test")
    index_dir.mkdir(parents=True, exist_ok=True)
    index_file = index_dir / f"{doc_id}.json"
    index_file.write_text(json.dumps(index_data, ensure_ascii=False), encoding="utf-8")
    size_kb = index_file.stat().st_size / 1024
    logger.info("Index saved → %s  (%.1f KB)", index_file, size_kb)

    # ── 4. LLM-reasoning retrieval ────────────────────────────────────────────
    logger.info("=== STEP 4: PageIndex retrieval — query: %r ===", query)
    tree_for_prompt = _strip_text(structure)
    search_prompt = (
        "You are searching a D&D campaign document for relevant content.\n\n"
        f"Question: {query}\n\n"
        "Document structure (node_id and title only):\n"
        f"{json.dumps(tree_for_prompt, indent=2)}\n\n"
        "Return the node_ids most likely to contain the answer. JSON only:\n"
        '{"node_ids": ["0001", "0002"]}'
    )

    logger.info("Sending tree to Ollama for node selection...")
    t0 = time.perf_counter()
    selection_resp = await llm.complete(
        CompletionRequest(
            task=LlmTask.campaign_assistant,
            messages=[Message(role="user", content=search_prompt)],
            temperature=0,
        )
    )
    elapsed = time.perf_counter() - t0
    logger.info("Node selection response (%.1fs):\n%s", elapsed, selection_resp.content)

    try:
        selected_ids: list[str] = json.loads(_extract_json_text(selection_resp.content)).get(
            "node_ids", []
        )
    except Exception as e:
        logger.warning("Failed to parse node IDs: %s — falling back to empty", e)
        selected_ids = []

    logger.info("Selected node IDs: %s", selected_ids)

    # ── 5. Fetch node text and build context ──────────────────────────────────
    logger.info("=== STEP 5: Retrieving node text ===")
    context_chunks: list[str] = []
    for node_id in selected_ids:
        node = node_map.get(node_id)
        if node:
            text = node.get("text", "")
            logger.info(
                "  Node %s — %r — %d chars",
                node_id,
                node.get("title", ""),
                len(text),
            )
            context_chunks.append(f"[{node.get('title', '')}]\n{text}")

    if not context_chunks:
        logger.warning("No matching nodes found — answering without context")

    context = "\n\n---\n\n".join(context_chunks) if context_chunks else "(no context found)"

    # ── 6. Final answer ───────────────────────────────────────────────────────
    logger.info("=== STEP 6: Generating final answer ===")
    answer_prompt = (
        "You are The Guide, an AI assistant for a Dungeon Master running a D&D campaign. "
        "Answer the question using only the provided context. Be detailed and specific.\n\n"
        f"## Campaign Context\n{context}\n\n"
        f"## Question\n{query}"
    )
    t0 = time.perf_counter()
    answer_resp = await llm.complete(
        CompletionRequest(
            task=LlmTask.campaign_assistant,
            messages=[Message(role="user", content=answer_prompt)],
            temperature=0.3,
            max_tokens=1024,
        )
    )
    elapsed = time.perf_counter() - t0

    print("\n" + "=" * 70)
    print(f"QUERY: {query}")
    print("=" * 70)
    print(answer_resp.content)
    print("=" * 70)
    print(
        f"(model: {answer_resp.model}  tokens: {answer_resp.prompt_tokens}+{answer_resp.completion_tokens}  time: {elapsed:.1f}s)"
    )

    # Cleanup test index
    index_file.unlink(missing_ok=True)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(f'Usage: uv run python {sys.argv[0]} <pdf_path> "<query>"')
        sys.exit(1)
    asyncio.run(main(sys.argv[1], sys.argv[2]))
