"""Tests for the Qdrant RAG path and PageIndex utilities."""

from __future__ import annotations

import json
from uuid import UUID, uuid4

import pytest
import pytest_asyncio

from guide.pdf.pipeline import (
    _flatten_nodes_to_chunks,
    _query_indexes_qdrant,
    _upsert_chunks_to_qdrant,
    ensure_collection,
    query_indexes,
)

COLLECTION = "test_chunks"
EMBEDDING_DIMS = 768


async def _mock_embed(text: str) -> list[float]:
    """Returns a fixed unit vector — deterministic for tests."""
    return [0.1] * EMBEDDING_DIMS


@pytest_asyncio.fixture
async def qdrant():
    from qdrant_client import AsyncQdrantClient

    client = AsyncQdrantClient(location=":memory:")
    yield client
    await client.close()


@pytest_asyncio.fixture
async def qdrant_col(qdrant):
    """Qdrant client with the test collection already created."""
    await ensure_collection(qdrant, COLLECTION, EMBEDDING_DIMS)
    yield qdrant


# ---------------------------------------------------------------------------
# ensure_collection
# ---------------------------------------------------------------------------


async def test_ensure_collection_creates_collection(qdrant):
    assert not await qdrant.collection_exists(COLLECTION)
    await ensure_collection(qdrant, COLLECTION, EMBEDDING_DIMS)
    assert await qdrant.collection_exists(COLLECTION)
    # Idempotent second call
    await ensure_collection(qdrant, COLLECTION, EMBEDDING_DIMS)
    assert await qdrant.collection_exists(COLLECTION)


# ---------------------------------------------------------------------------
# _flatten_nodes_to_chunks — splitting + overlap
# ---------------------------------------------------------------------------


def test_flatten_nodes_to_chunks_splitting():
    big_text = "x" * 5000
    tree = [{"title": "Big Section", "node_id": "0001", "text": big_text}]
    chunks = _flatten_nodes_to_chunks(
        tree,
        "doc-id",
        "Test Doc",
        "global",
        chunk_max_chars=2048,
        chunk_overlap_chars=64,
    )
    assert len(chunks) > 1
    for chunk in chunks:
        assert len(chunk["content"]) <= 2048

    # Verify overlapping windows: start of chunk N+1 == end of chunk N (64 chars)
    step = 2048 - 64  # 1984
    for i in range(len(chunks) - 1):
        assert chunks[i]["content"][step:] == chunks[i + 1]["content"][: 2048 - step]


def test_flatten_nodes_to_chunks_default_visible_propagation():
    tree = [{"title": "DM Section", "node_id": "0001", "text": "Secret info"}]
    chunks = _flatten_nodes_to_chunks(
        tree,
        "doc-id",
        "DM Notes",
        "global",
        chunk_max_chars=2048,
        chunk_overlap_chars=64,
        default_visible=False,
    )
    assert len(chunks) == 1
    assert chunks[0]["is_player_visible"] is False


def test_flatten_nodes_to_chunks_node_override_visible():
    """A node can explicitly override the document-level default."""
    tree = [
        {
            "title": "Public Section",
            "node_id": "0001",
            "text": "Safe for players",
            "is_player_visible": True,
        }
    ]
    chunks = _flatten_nodes_to_chunks(
        tree,
        "doc-id",
        "Mixed Doc",
        "global",
        chunk_max_chars=2048,
        chunk_overlap_chars=64,
        default_visible=False,
    )
    assert chunks[0]["is_player_visible"] is True


# ---------------------------------------------------------------------------
# ingest + query via Qdrant
# ---------------------------------------------------------------------------


async def test_ingest_and_query_qdrant(qdrant_col):
    doc_id = str(uuid4())
    chunks = [
        {
            "doc_id": doc_id,
            "scope": "global",
            "doc_name": "Test Doc",
            "section_path": "Chapter 1",
            "content": "Dragons breathe fire in the dungeon.",
            "node_id": "0001",
            "is_player_visible": True,
        }
    ]
    await _upsert_chunks_to_qdrant(chunks, _mock_embed, qdrant_col, COLLECTION)

    results = await _query_indexes_qdrant(
        "What do dragons do?",
        _mock_embed,
        qdrant_col,
        COLLECTION,
        limit=5,
        scopes=["global"],
        doc_ids=[UUID(doc_id)],
    )
    assert len(results) == 1
    assert results[0]["content"] == "Dragons breathe fire in the dungeon."
    assert results[0]["section_path"] == "Chapter 1"
    assert "score" in results[0]


async def test_query_indexes_qdrant_path(qdrant_col):
    doc_id = uuid4()
    chunks = [
        {
            "doc_id": str(doc_id),
            "scope": "global",
            "doc_name": "Rulebook",
            "section_path": "Combat Rules",
            "content": "Initiative determines turn order in combat.",
            "node_id": "0002",
            "is_player_visible": True,
        }
    ]
    await _upsert_chunks_to_qdrant(chunks, _mock_embed, qdrant_col, COLLECTION)

    results = await query_indexes(
        scopes=["global"],
        doc_ids=[doc_id],
        query="How does initiative work?",
        embed=_mock_embed,
        qdrant=qdrant_col,
        collection=COLLECTION,
    )
    assert isinstance(results, list)
    assert len(results) >= 1
    assert results[0]["content"] == "Initiative determines turn order in combat."


async def test_query_indexes_llm_fallback():
    """When embed/qdrant are None, falls back to LLM path (returns empty for empty doc list)."""
    results = await query_indexes(
        scopes=[],
        doc_ids=[],
        query="some query",
        embed=None,
        qdrant=None,
    )
    assert isinstance(results, list)
    assert results == []


# ---------------------------------------------------------------------------
# player_visible_only filter
# ---------------------------------------------------------------------------


async def test_player_visible_only_filter(qdrant_col):
    doc_id = str(uuid4())
    dm_chunk = {
        "doc_id": doc_id,
        "scope": "global",
        "doc_name": "DM Notes",
        "section_path": "Secret Section",
        "content": "The villain's secret identity is revealed here.",
        "node_id": "0003",
        "is_player_visible": False,
    }
    await _upsert_chunks_to_qdrant([dm_chunk], _mock_embed, qdrant_col, COLLECTION)

    results = await _query_indexes_qdrant(
        "Who is the villain?",
        _mock_embed,
        qdrant_col,
        COLLECTION,
        limit=5,
        scopes=["global"],
        doc_ids=[UUID(doc_id)],
        player_visible_only=True,
    )
    assert len(results) == 0


async def test_player_visible_only_allows_visible_chunks(qdrant_col):
    doc_id = str(uuid4())
    public_chunk = {
        "doc_id": doc_id,
        "scope": "global",
        "doc_name": "Player Guide",
        "section_path": "World Lore",
        "content": "The kingdom of Valdris has stood for a thousand years.",
        "node_id": "0004",
        "is_player_visible": True,
    }
    await _upsert_chunks_to_qdrant([public_chunk], _mock_embed, qdrant_col, COLLECTION)

    results = await _query_indexes_qdrant(
        "Tell me about the kingdom",
        _mock_embed,
        qdrant_col,
        COLLECTION,
        limit=5,
        scopes=["global"],
        doc_ids=[UUID(doc_id)],
        player_visible_only=True,
    )
    assert len(results) == 1
    assert results[0]["content"] == "The kingdom of Valdris has stood for a thousand years."


# ---------------------------------------------------------------------------
# chat endpoint with Qdrant wired in
# ---------------------------------------------------------------------------


async def test_chat_with_qdrant_enabled(db):
    from httpx import ASGITransport, AsyncClient

    from guide.api.main import create_app
    from guide.api.state import AppState
    from guide.config import AppConfig
    from guide.llm.client import CompletionResponse

    class _MockLlm:
        provider_name = "mock"

        async def complete(self, req):
            return CompletionResponse(content="{}", model="mock", provider="mock")

        async def complete_stream(self, req):
            yield "mock chunk"

        async def embed(self, req):
            return [0.0] * 768

        async def complete_with_vision(self, req):
            return CompletionResponse(content="mock", model="mock", provider="mock")

        def model_for_task(self, task):
            return "mock"

    from qdrant_client import AsyncQdrantClient

    config = AppConfig(database_url=":memory:", default_model="mock")
    application = create_app(config)

    qdrant_client = AsyncQdrantClient(location=":memory:")
    await ensure_collection(qdrant_client, config.qdrant_collection, config.embedding_dims)

    state = AppState(config=config, llm=_MockLlm(), db=db, qdrant=qdrant_client)
    application.state.guide = state
    application.router.lifespan_context = None

    async with AsyncClient(transport=ASGITransport(app=application), base_url="http://test") as ac:
        resp = await ac.post("/campaigns", json={"name": "RAG Test Campaign"})
        assert resp.status_code == 201
        campaign_id = resp.json()["id"]

        resp = await ac.post(
            f"/campaigns/{campaign_id}/chat",
            json={"message": "What is happening in the campaign?"},
        )
        assert resp.status_code == 200
        assert resp.headers["content-type"].startswith("text/event-stream")

    await qdrant_client.close()
