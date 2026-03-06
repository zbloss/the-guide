"""PageIndex-based document ingestion pipeline.

After Docling extracts a PDF, its full markdown output is fed into a
PageIndex-compatible tree builder (vendored from VectifyAI/PageIndex,
MIT licence) that is entirely local — no hosted API key required.

Tree structures are stored on disk at:
  data/indexes/{campaign_id}/{doc_id}.json   (campaign documents)
  data/indexes/global/{doc_id}.json          (rulebooks)

At query time the tree (minus node text) is sent to the local Ollama LLM,
which selects the relevant node IDs. The caller supplies a `call_llm`
coroutine so this module stays decoupled from the LLM layer.
"""

from __future__ import annotations

import json
import re
from collections.abc import Awaitable, Callable
from pathlib import Path
from uuid import UUID

import aiosqlite

from guide.db.documents import DocumentRepository, GlobalDocumentRepository
from guide.models.shared import IngestionStatus
from guide.pdf.extractor import DocumentExtraction

_INDEX_BASE = Path("data/indexes")


def _index_path(scope: str, doc_id: UUID) -> Path:
    return _INDEX_BASE / scope / f"{doc_id}.json"


# ---------------------------------------------------------------------------
# Tree building — vendored from VectifyAI/PageIndex (page_index_md.py, MIT)
# Pure Python; no LLM calls during ingestion.
# ---------------------------------------------------------------------------


def _extract_nodes_from_markdown(markdown: str) -> tuple[list[dict], list[str]]:
    """Scan markdown for ATX headers (#–######) and return their positions."""
    header_re = re.compile(r"^(#{1,6})\s+(.+)$")
    code_fence_re = re.compile(r"^```")
    node_list: list[dict] = []
    lines = markdown.split("\n")
    in_code_block = False

    for line_num, line in enumerate(lines, 1):
        stripped = line.strip()
        if code_fence_re.match(stripped):
            in_code_block = not in_code_block
            continue
        if not stripped or in_code_block:
            continue
        m = header_re.match(stripped)
        if m:
            node_list.append({"node_title": m.group(2).strip(), "line_num": line_num})

    return node_list, lines


def _extract_node_text_content(node_list: list[dict], lines: list[str]) -> list[dict]:
    """Attach the markdown text slice that belongs to each header node."""
    header_re = re.compile(r"^(#{1,6})")
    all_nodes: list[dict] = []

    for node in node_list:
        line_content = lines[node["line_num"] - 1]
        m = header_re.match(line_content)
        if m is None:
            continue
        all_nodes.append(
            {
                "title": node["node_title"],
                "line_num": node["line_num"],
                "level": len(m.group(1)),
            }
        )

    for i, node in enumerate(all_nodes):
        start = node["line_num"] - 1
        end = all_nodes[i + 1]["line_num"] - 1 if i + 1 < len(all_nodes) else len(lines)
        node["text"] = "\n".join(lines[start:end]).strip()

    return all_nodes


def _build_tree_from_nodes(node_list: list[dict]) -> list[dict]:
    """Convert the flat, level-annotated node list into a nested tree."""
    if not node_list:
        return []

    stack: list[tuple[dict, int]] = []
    root_nodes: list[dict] = []
    counter = 1

    for node in node_list:
        level = node["level"]
        tree_node: dict = {
            "title": node["title"],
            "node_id": str(counter).zfill(4),
            "text": node["text"],
            "nodes": [],
        }
        counter += 1

        while stack and stack[-1][1] >= level:
            stack.pop()

        if not stack:
            root_nodes.append(tree_node)
        else:
            stack[-1][0]["nodes"].append(tree_node)

        stack.append((tree_node, level))

    return root_nodes


def _clean_tree(nodes: list[dict]) -> list[dict]:
    """Drop the empty 'nodes' list from leaf nodes for a tidy JSON output."""
    cleaned = []
    for node in nodes:
        out: dict = {
            "title": node["title"],
            "node_id": node["node_id"],
            "text": node["text"],
        }
        if node["nodes"]:
            out["nodes"] = _clean_tree(node["nodes"])
        cleaned.append(out)
    return cleaned


def _build_index(full_markdown: str, doc_name: str) -> dict:
    """Build a PageIndex tree from Docling-extracted markdown."""
    node_list, lines = _extract_nodes_from_markdown(full_markdown)

    if not node_list:
        # No headers found — treat whole document as a single node
        return {
            "doc_name": doc_name,
            "structure": [{"title": doc_name, "node_id": "0001", "text": full_markdown}],
        }

    nodes_with_text = _extract_node_text_content(node_list, lines)
    tree = _build_tree_from_nodes(nodes_with_text)
    tree = _clean_tree(tree)
    return {"doc_name": doc_name, "structure": tree}


# ---------------------------------------------------------------------------
# Ingestion
# ---------------------------------------------------------------------------


async def ingest_campaign_document(
    campaign_id: UUID,
    doc_id: UUID,
    extraction: DocumentExtraction,
    db: aiosqlite.Connection,
) -> int:
    """Build and persist a PageIndex tree for a campaign document. Returns page count."""
    scope = str(campaign_id)
    index_file = _index_path(scope, doc_id)
    index_file.parent.mkdir(parents=True, exist_ok=True)

    index_data = _build_index(extraction.full_markdown, doc_name=str(doc_id))
    index_file.write_text(json.dumps(index_data, ensure_ascii=False), encoding="utf-8")

    repo = DocumentRepository(db)
    await repo.update_ingested(doc_id, len(extraction.pages))
    return len(extraction.pages)


async def ingest_global_document(
    doc_id: UUID,
    extraction: DocumentExtraction,
    db: aiosqlite.Connection,
) -> int:
    """Build and persist a PageIndex tree for a global rulebook. Returns page count."""
    scope = "global"
    index_file = _index_path(scope, doc_id)
    index_file.parent.mkdir(parents=True, exist_ok=True)

    index_data = _build_index(extraction.full_markdown, doc_name=str(doc_id))
    index_file.write_text(json.dumps(index_data, ensure_ascii=False), encoding="utf-8")

    repo = GlobalDocumentRepository(db)
    await repo.update_status(doc_id, IngestionStatus.completed)
    return len(extraction.pages)


# ---------------------------------------------------------------------------
# Retrieval helpers
# ---------------------------------------------------------------------------


def load_index(scope: str, doc_id: UUID) -> dict | None:
    """Load a persisted index from disk, returning None if absent."""
    path = _index_path(scope, doc_id)
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def _strip_text(tree: list[dict]) -> list[dict]:
    """Return a copy of the tree with 'text' removed (for the selection prompt)."""
    result = []
    for node in tree:
        clean = {k: v for k, v in node.items() if k != "text"}
        if "nodes" in clean:
            clean["nodes"] = _strip_text(clean["nodes"])
        result.append(clean)
    return result


def _build_node_map(tree: list[dict], out: dict | None = None) -> dict[str, dict]:
    """Flatten the tree into a {node_id: node} lookup map."""
    if out is None:
        out = {}
    for node in tree:
        if "node_id" in node:
            out[node["node_id"]] = node
        if "nodes" in node:
            _build_node_map(node["nodes"], out)
    return out


def _extract_json_text(text: str) -> str:
    """Strip ```json ... ``` fences that local LLMs often add around JSON."""
    start = text.find("```json")
    if start != -1:
        text = text[start + 7 :]
    end = text.rfind("```")
    if end != -1:
        text = text[:end]
    return text.strip()


# ---------------------------------------------------------------------------
# Retrieval
# ---------------------------------------------------------------------------


async def query_indexes(
    scopes: list[str],
    doc_ids: list[UUID],
    query: str,
    call_llm: Callable[[str], Awaitable[str]],
    player_visible_only: bool = False,
    limit: int = 5,
) -> list[dict]:
    """LLM-reasoning retrieval across PageIndex trees.

    Steps:
      1. For each index, strip node text and send tree + query to the LLM.
      2. LLM returns the node IDs most likely to contain the answer.
      3. Fetch the full text from those nodes and return as context chunks.

    Args:
        call_llm: Async callable ``async (prompt: str) -> str`` backed by Ollama.
    """
    results: list[dict] = []

    for scope, doc_id in zip(scopes, doc_ids):
        if len(results) >= limit:
            break

        index = load_index(scope, doc_id)
        if not index:
            continue

        tree = index.get("structure", [])
        if not tree:
            continue

        tree_for_prompt = _strip_text(tree)
        search_prompt = (
            "You are searching a D&D campaign document for relevant content.\n\n"
            f"Question: {query}\n\n"
            "Document structure (node_id and title only):\n"
            f"{json.dumps(tree_for_prompt, indent=2)}\n\n"
            "Return the node_ids most likely to contain the answer. JSON only:\n"
            '{"node_ids": ["0001", "0002"]}'
        )

        try:
            response_text = await call_llm(search_prompt)
            selected_ids: list[str] = json.loads(_extract_json_text(response_text)).get(
                "node_ids", []
            )
        except Exception:
            selected_ids = []

        node_map = _build_node_map(tree)
        for node_id in selected_ids:
            node = node_map.get(node_id)
            if not node:
                continue
            results.append(
                {
                    "content": node.get("text", ""),
                    "section_path": node.get("title", ""),
                    "doc_id": str(doc_id),
                    "node_id": node_id,
                }
            )
            if len(results) >= limit:
                break

    return results
