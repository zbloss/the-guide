"""Qdrant-backed document ingestion pipeline.

After Docling extracts a PDF, its full markdown output is fed into a
PageIndex-compatible tree builder (vendored from VectifyAI/PageIndex,
MIT licence) that is entirely local — no hosted API key required.

Tree structures are stored on disk at:
  data/indexes/{campaign_id}/{doc_id}.json   (campaign documents)
  data/indexes/global/{doc_id}.json          (rulebooks)

At query time, chunks are retrieved via Qdrant vector similarity search.
When Qdrant is unavailable (qdrant=None), falls back to the legacy
LLM-reasoning retrieval path so existing tests continue to pass.

The MetaIndex (LLM-generated doc summaries + select_relevant_docs) is
preserved unchanged — it still picks which documents to search before
retrieval begins.
"""

from __future__ import annotations

import json
import re
from collections.abc import Awaitable, Callable
from datetime import datetime, timezone
from pathlib import Path
from uuid import NAMESPACE_URL, UUID, uuid5

import aiosqlite

from guide.db.documents import DocumentRepository, GlobalDocumentRepository
from guide.llm.prompts import doc_selector_prompt, doc_summary_prompt
from guide.models.document import DocSummary, MetaIndex
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
# MetaIndex helpers
# ---------------------------------------------------------------------------


def is_already_indexed(scope: str, doc_id: UUID) -> bool:
    """Return True if a content index file already exists on disk for this doc."""
    return _index_path(scope, doc_id).exists()


def _meta_path(scope: str) -> Path:
    return _INDEX_BASE / scope / "meta.json"


def _load_meta_index(scope: str) -> MetaIndex:
    path = _meta_path(scope)
    if not path.exists():
        return MetaIndex(scope=scope)
    try:
        return MetaIndex.model_validate_json(path.read_text(encoding="utf-8"))
    except Exception:
        return MetaIndex(scope=scope)


def _save_meta_index(meta: MetaIndex) -> None:
    path = _meta_path(meta.scope)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(meta.model_dump_json(indent=2), encoding="utf-8")


def _add_to_meta_index(scope: str, entry: DocSummary) -> None:
    meta = _load_meta_index(scope)
    meta.entries = [e for e in meta.entries if e.doc_id != entry.doc_id]
    meta.entries.append(entry)
    _save_meta_index(meta)


_TOC_KEYWORDS = {"contents", "table of contents", "chapters", "chapter list"}


def _build_summary_excerpt(full_markdown: str, max_chars: int = 3000) -> str:
    """Return a content-rich excerpt, skipping front-matter (credits, legal).

    Strategy (in order):
    1. If a Table of Contents heading is found, take text from that line onward.
    2. Otherwise skip the first 10 % of the document (covers most credits pages)
       and sample from there.
    """
    lines = full_markdown.split("\n")

    for i, line in enumerate(lines):
        if any(kw in line.lower() for kw in _TOC_KEYWORDS):
            excerpt = "\n".join(lines[i : i + 150])
            if len(excerpt) >= 300:
                return excerpt[:max_chars]

    skip = max(0, len(full_markdown) // 10)
    return full_markdown[skip : skip + max_chars]


async def _generate_doc_summary(
    full_markdown: str,
    doc_name: str,
    call_llm: Callable[[str], Awaitable[str]] | None,
) -> str:
    if call_llm is None:
        return ""
    excerpt = _build_summary_excerpt(full_markdown)
    try:
        return (await call_llm(doc_summary_prompt(doc_name, excerpt))).strip()
    except Exception:
        return ""


# ---------------------------------------------------------------------------
# Qdrant helpers
# ---------------------------------------------------------------------------


async def ensure_collection(qdrant, collection: str, embedding_dims: int) -> None:
    """Create the Qdrant collection if it does not already exist (idempotent)."""
    from qdrant_client.models import Distance, VectorParams

    exists = await qdrant.collection_exists(collection)
    if not exists:
        await qdrant.create_collection(
            collection_name=collection,
            vectors_config=VectorParams(size=embedding_dims, distance=Distance.COSINE),
        )


def _flatten_nodes_to_chunks(
    tree: list[dict],
    doc_id: str,
    doc_name: str,
    scope: str,
    chunk_max_chars: int,
    chunk_overlap_chars: int,
    parent_path: str = "",
    default_visible: bool = True,
) -> list[dict]:
    """DFS over the tree, splitting large nodes into overlapping chunks."""
    chunks: list[dict] = []
    for node in tree:
        title = node.get("title", "")
        section_path = f"{parent_path} > {title}" if parent_path else title
        text = node.get("text", "")
        node_id = node.get("node_id", "")

        is_player_visible = node.get("is_player_visible", default_visible)

        if len(text) <= chunk_max_chars:
            chunks.append(
                {
                    "doc_id": doc_id,
                    "scope": scope,
                    "doc_name": doc_name,
                    "section_path": section_path,
                    "content": text,
                    "node_id": node_id,
                    "is_player_visible": is_player_visible,
                }
            )
        else:
            step = max(1, chunk_max_chars - chunk_overlap_chars)
            parts = range(0, len(text), step)
            total = len(list(parts))
            for part_idx, start in enumerate(range(0, len(text), step), 1):
                chunk_text = text[start : start + chunk_max_chars]
                chunks.append(
                    {
                        "doc_id": doc_id,
                        "scope": scope,
                        "doc_name": doc_name,
                        "section_path": f"{section_path} [part {part_idx}/{total}]",
                        "content": chunk_text,
                        "node_id": node_id,
                        "is_player_visible": is_player_visible,
                    }
                )

        child_nodes = node.get("nodes", [])
        if child_nodes:
            chunks.extend(
                _flatten_nodes_to_chunks(
                    child_nodes, doc_id, doc_name, scope,
                    chunk_max_chars, chunk_overlap_chars, section_path,
                    default_visible=default_visible,
                )
            )

    return chunks


async def _upsert_chunks_to_qdrant(
    chunks: list[dict],
    embed: Callable[[str], Awaitable[list[float]]],
    qdrant,
    collection: str,
) -> None:
    """Embed each chunk and upsert to Qdrant in batches of 100."""
    from qdrant_client.models import PointStruct

    BATCH_SIZE = 100
    points: list[PointStruct] = []

    for chunk in chunks:
        vector = await embed(chunk["content"])
        point_id = str(uuid5(NAMESPACE_URL, f"{chunk['doc_id']}|{chunk['section_path']}"))
        points.append(
            PointStruct(
                id=point_id,
                vector=vector,
                payload=chunk,
            )
        )

    for i in range(0, len(points), BATCH_SIZE):
        batch = points[i : i + BATCH_SIZE]
        await qdrant.upsert(collection_name=collection, points=batch)


# ---------------------------------------------------------------------------
# Ingestion
# ---------------------------------------------------------------------------


async def ingest_campaign_document(
    campaign_id: UUID,
    doc_id: UUID,
    extraction: DocumentExtraction,
    db: aiosqlite.Connection,
    doc_name: str | None = None,
    call_llm: Callable[[str], Awaitable[str]] | None = None,
    *,
    embed: Callable[[str], Awaitable[list[float]]] | None = None,
    qdrant=None,
    collection: str = "guide_chunks",
    chunk_max_chars: int = 2048,
    chunk_overlap_chars: int = 64,
    force: bool = False,
    is_player_visible: bool = True,
) -> int:
    """Build and persist a PageIndex tree for a campaign document. Returns page count."""
    scope = str(campaign_id)
    name = doc_name or str(doc_id)
    index_file = _index_path(scope, doc_id)
    index_file.parent.mkdir(parents=True, exist_ok=True)

    index_data = _build_index(extraction.full_markdown, doc_name=name)
    index_file.write_text(json.dumps(index_data, ensure_ascii=False), encoding="utf-8")

    if force and qdrant is not None:
        from qdrant_client.models import FieldCondition, Filter, MatchValue
        await qdrant.delete(
            collection_name=collection,
            points_selector=Filter(
                must=[FieldCondition(key="doc_id", match=MatchValue(value=str(doc_id)))]
            ),
        )

    if embed is not None and qdrant is not None:
        chunks = _flatten_nodes_to_chunks(
            index_data["structure"], str(doc_id), name, scope, chunk_max_chars, chunk_overlap_chars,
            default_visible=is_player_visible,
        )
        await _upsert_chunks_to_qdrant(chunks, embed, qdrant, collection)

    summary = await _generate_doc_summary(extraction.full_markdown, name, call_llm)
    _add_to_meta_index(
        scope,
        DocSummary(
            doc_id=doc_id,
            doc_name=name,
            filename=name,
            summary=summary,
            scope=scope,
            ingested_at=datetime.now(timezone.utc),
        ),
    )

    repo = DocumentRepository(db)
    await repo.update_ingested(doc_id, len(extraction.pages))
    return len(extraction.pages)


async def ingest_global_document(
    doc_id: UUID,
    extraction: DocumentExtraction,
    db: aiosqlite.Connection,
    doc_name: str | None = None,
    call_llm: Callable[[str], Awaitable[str]] | None = None,
    *,
    embed: Callable[[str], Awaitable[list[float]]] | None = None,
    qdrant=None,
    collection: str = "guide_chunks",
    chunk_max_chars: int = 2048,
    chunk_overlap_chars: int = 64,
    force: bool = False,
    is_player_visible: bool = True,
) -> int:
    """Build and persist a PageIndex tree for a global rulebook. Returns page count."""
    scope = "global"
    name = doc_name or str(doc_id)
    index_file = _index_path(scope, doc_id)
    index_file.parent.mkdir(parents=True, exist_ok=True)

    index_data = _build_index(extraction.full_markdown, doc_name=name)
    index_file.write_text(json.dumps(index_data, ensure_ascii=False), encoding="utf-8")

    if force and qdrant is not None:
        from qdrant_client.models import FieldCondition, Filter, MatchValue
        await qdrant.delete(
            collection_name=collection,
            points_selector=Filter(
                must=[FieldCondition(key="doc_id", match=MatchValue(value=str(doc_id)))]
            ),
        )

    if embed is not None and qdrant is not None:
        chunks = _flatten_nodes_to_chunks(
            index_data["structure"], str(doc_id), name, scope, chunk_max_chars, chunk_overlap_chars,
            default_visible=is_player_visible,
        )
        await _upsert_chunks_to_qdrant(chunks, embed, qdrant, collection)

    summary = await _generate_doc_summary(extraction.full_markdown, name, call_llm)
    _add_to_meta_index(
        scope,
        DocSummary(
            doc_id=doc_id,
            doc_name=name,
            filename=name,
            summary=summary,
            scope=scope,
            ingested_at=datetime.now(timezone.utc),
        ),
    )

    repo = GlobalDocumentRepository(db)
    await repo.update_status(doc_id, IngestionStatus.completed)
    return len(extraction.pages)


# ---------------------------------------------------------------------------
# Retrieval helpers (legacy LLM path)
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
    """Extract JSON from LLM output, stripping fences, think tags, and prose."""
    import re

    # Strip <think>...</think> blocks (qwen3 chain-of-thought)
    text = re.sub(r"<think>.*?</think>", "", text, flags=re.DOTALL).strip()

    # Handle ```json ... ``` or ``` ... ``` fences
    fence = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", text, re.DOTALL)
    if fence:
        return fence.group(1).strip()

    # Fall back: grab from first { to last }
    start = text.find("{")
    end = text.rfind("}")
    if start != -1 and end != -1 and end > start:
        return text[start : end + 1].strip()

    return text.strip()


# ---------------------------------------------------------------------------
# Retrieval
# ---------------------------------------------------------------------------


async def query_indexes(
    scopes: list[str],
    doc_ids: list[UUID],
    query: str,
    call_llm: Callable[[str], Awaitable[str]] | None = None,
    player_visible_only: bool = False,
    limit: int = 5,
    embed: Callable[[str], Awaitable[list[float]]] | None = None,
    qdrant=None,
    collection: str = "guide_chunks",
    campaign_id: str | None = None,
) -> list[dict]:
    """Retrieve relevant chunks across indexed documents.

    Qdrant path (when embed and qdrant are provided):
      If campaign_id is given, searches the entire collection filtered to that
      campaign's scope + "global" in a single vector query — no doc pre-selection needed.
      Otherwise falls back to per-doc search using scopes/doc_ids.

    Fallback path (when embed or qdrant is None):
      LLM-reasoning retrieval over PageIndex trees — preserves all existing tests.
    """
    if embed is not None and qdrant is not None:
        return await _query_indexes_qdrant(
            query, embed, qdrant, collection, limit,
            campaign_id=campaign_id, scopes=scopes, doc_ids=doc_ids,
            player_visible_only=player_visible_only,
        )
    return await _query_indexes_llm(scopes, doc_ids, query, call_llm, limit)


async def _query_indexes_qdrant(
    query: str,
    embed: Callable[[str], Awaitable[list[float]]],
    qdrant,
    collection: str,
    limit: int,
    campaign_id: str | None = None,
    scopes: list[str] | None = None,
    doc_ids: list[UUID] | None = None,
    player_visible_only: bool = False,
) -> list[dict]:
    from qdrant_client.models import FieldCondition, Filter, MatchValue

    query_vector = await embed(query)

    if campaign_id is not None:
        # Single search across all chunks in this campaign + global rulebooks
        scope_filter = Filter(
            should=[
                FieldCondition(key="scope", match=MatchValue(value=campaign_id)),
                FieldCondition(key="scope", match=MatchValue(value="global")),
            ]
        )
        must_conditions: list = [scope_filter]
        if player_visible_only:
            must_conditions.append(
                FieldCondition(key="is_player_visible", match=MatchValue(value=True))
            )
        search_filter = Filter(must=must_conditions)
        result = await qdrant.query_points(
            collection_name=collection,
            query=query_vector,
            query_filter=search_filter,
            limit=limit,
            with_payload=True,
        )
        return [
            {
                "content": hit.payload.get("content", ""),
                "section_path": hit.payload.get("section_path", ""),
                "doc_id": hit.payload.get("doc_id", ""),
                "doc_name": hit.payload.get("doc_name", ""),
                "node_id": hit.payload.get("node_id", ""),
                "score": hit.score,
            }
            for hit in result.points
        ]

    # Fallback: per-doc search (used when campaign_id is not provided)
    all_hits: list[dict] = []
    for scope, doc_id in zip(scopes or [], doc_ids or []):
        must_conditions = [
            FieldCondition(key="scope", match=MatchValue(value=scope)),
            FieldCondition(key="doc_id", match=MatchValue(value=str(doc_id))),
        ]
        if player_visible_only:
            must_conditions.append(
                FieldCondition(key="is_player_visible", match=MatchValue(value=True))
            )
        search_filter = Filter(must=must_conditions)
        result = await qdrant.query_points(
            collection_name=collection,
            query=query_vector,
            query_filter=search_filter,
            limit=limit,
            with_payload=True,
        )
        for hit in result.points:
            payload = hit.payload or {}
            all_hits.append(
                {
                    "content": payload.get("content", ""),
                    "section_path": payload.get("section_path", ""),
                    "doc_id": payload.get("doc_id", str(doc_id)),
                    "doc_name": payload.get("doc_name", ""),
                    "node_id": payload.get("node_id", ""),
                    "score": hit.score,
                }
            )
    all_hits.sort(key=lambda h: h["score"], reverse=True)
    return all_hits[:limit]


async def _query_indexes_llm(
    scopes: list[str],
    doc_ids: list[UUID],
    query: str,
    call_llm: Callable[[str], Awaitable[str]] | None,
    limit: int,
) -> list[dict]:
    """Legacy LLM-reasoning retrieval across PageIndex trees."""
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


async def select_relevant_docs(
    campaign_id: UUID,
    query: str,
    call_llm: Callable[[str], Awaitable[str]],
) -> list[tuple[str, UUID]]:
    """Choose which indexed documents are likely relevant to *query*.

    Reads MetaIndex files for both the campaign scope and "global" scope,
    then asks the LLM to select relevant doc_ids.  Falls back to returning
    all docs if the LLM fails or MetaIndex is absent.
    """
    campaign_scope = str(campaign_id)
    all_entries = (
        _load_meta_index(campaign_scope).entries + _load_meta_index("global").entries
    )

    # Fallback: if no MetaIndex entries exist, scan filesystem like before
    if not all_entries:
        return _fallback_list_all_docs(campaign_scope)

    # Fast path: single doc — skip LLM call
    if len(all_entries) == 1:
        return [(e.scope, e.doc_id) for e in all_entries]

    try:
        response = await call_llm(doc_selector_prompt(query, all_entries))
        selected_ids: list[str] = json.loads(_extract_json_text(response)).get("doc_ids", [])
        entry_map = {str(e.doc_id): e for e in all_entries}
        result = [
            (entry_map[id_].scope, entry_map[id_].doc_id)
            for id_ in selected_ids
            if id_ in entry_map
        ]
        if result:
            return result
    except Exception:
        pass

    return [(e.scope, e.doc_id) for e in all_entries]  # fallback: all docs


def _fallback_list_all_docs(campaign_scope: str) -> list[tuple[str, UUID]]:
    """Filesystem fallback for campaigns with no MetaIndex (pre-migration content)."""
    result = []
    for scope in [campaign_scope, "global"]:
        d = _INDEX_BASE / scope
        if not d.exists():
            continue
        for f in d.glob("*.json"):
            if f.stem == "meta":
                continue
            try:
                result.append((scope, UUID(f.stem)))
            except ValueError:
                pass
    return result
