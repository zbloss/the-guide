"""PageIndex-based document ingestion pipeline.

Replaces Qdrant vector search with PageIndex's LLM-reasoning-based retrieval.
Indexes are stored on disk at data/indexes/{campaign_id or 'global'}/{doc_id}.json.
"""
from __future__ import annotations

import json
from pathlib import Path
from uuid import UUID

import aiosqlite

from guide.db.documents import DocumentRepository, GlobalDocumentRepository
from guide.models.shared import IngestionStatus
from guide.pdf.extractor import PageExtraction

_INDEX_BASE = Path("data/indexes")


def _index_path(scope: str, doc_id: UUID) -> Path:
    return _INDEX_BASE / scope / f"{doc_id}.json"


async def ingest_campaign_document(
    campaign_id: UUID,
    doc_id: UUID,
    pages: list[PageExtraction],
    db: aiosqlite.Connection,
) -> int:
    """Build and persist a PageIndex for a campaign document. Returns page count."""
    scope = str(campaign_id)
    index_file = _index_path(scope, doc_id)
    index_file.parent.mkdir(parents=True, exist_ok=True)

    index_data = _build_index(pages)
    index_file.write_text(json.dumps(index_data, ensure_ascii=False), encoding="utf-8")

    repo = DocumentRepository(db)
    await repo.update_ingested(doc_id, len(pages))
    return len(pages)


async def ingest_global_document(
    doc_id: UUID,
    pages: list[PageExtraction],
    db: aiosqlite.Connection,
) -> int:
    """Build and persist a PageIndex for a global rulebook. Returns page count."""
    scope = "global"
    index_file = _index_path(scope, doc_id)
    index_file.parent.mkdir(parents=True, exist_ok=True)

    index_data = _build_index(pages)
    index_file.write_text(json.dumps(index_data, ensure_ascii=False), encoding="utf-8")

    repo = GlobalDocumentRepository(db)
    await repo.update_status(doc_id, IngestionStatus.completed)
    return len(pages)


def _build_index(pages: list[PageExtraction]) -> dict:
    """Serialize pages into a JSON structure for disk storage."""
    return {
        "pages": [
            {
                "page_number": p.page_number,
                "raw_text": p.raw_text,
                "headings": p.headings,
                "is_dm_only": p.is_dm_only,
            }
            for p in pages
        ]
    }


def load_index(scope: str, doc_id: UUID) -> dict | None:
    """Load a persisted index from disk, returning None if absent."""
    path = _index_path(scope, doc_id)
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def query_indexes(
    scopes: list[str],
    doc_ids: list[UUID],
    query: str,
    player_visible_only: bool = False,
    limit: int = 5,
) -> list[dict]:
    """Simple keyword retrieval across loaded indexes.

    In production, this would use PageIndex's LLM-reasoning retrieval.
    For now, performs substring matching as a functional placeholder.
    """
    results: list[dict] = []
    query_lower = query.lower()

    for scope, doc_id in zip(scopes, doc_ids):
        index = load_index(scope, doc_id)
        if not index:
            continue
        for page in index.get("pages", []):
            if player_visible_only and page.get("is_dm_only"):
                continue
            text = page.get("raw_text", "")
            if query_lower in text.lower():
                results.append({
                    "content": text[:1000],
                    "section_path": " > ".join(page.get("headings", [])),
                    "doc_id": str(doc_id),
                    "page_number": page.get("page_number", 0),
                    "score": _score(text, query_lower),
                })

    results.sort(key=lambda r: r["score"], reverse=True)
    return results[:limit]


def _score(text: str, query: str) -> float:
    """Naive relevance score based on query term frequency."""
    return text.lower().count(query) / max(len(text), 1) * 1000
