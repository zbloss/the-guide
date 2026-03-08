"""Tests for MetaIndex and DocSelector (pipeline.py)."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from uuid import uuid4

import pytest
import pytest_asyncio

from guide.models.document import DocSummary, MetaIndex
from guide.pdf.extractor import DocumentExtraction, PageExtraction
from guide.pdf.pipeline import (
    _add_to_meta_index,
    _generate_doc_summary,
    _load_meta_index,
    ingest_campaign_document,
    select_relevant_docs,
)

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def index_base(tmp_path, monkeypatch):
    monkeypatch.setattr("guide.pdf.pipeline._INDEX_BASE", tmp_path)
    return tmp_path


def _make_entry(scope: str, doc_name: str = "Test Doc", summary: str = "A test doc.") -> DocSummary:
    return DocSummary(
        doc_id=uuid4(),
        doc_name=doc_name,
        filename=f"{doc_name}.pdf",
        summary=summary,
        scope=scope,
        ingested_at=datetime.now(timezone.utc),
    )


# ---------------------------------------------------------------------------
# _load_meta_index
# ---------------------------------------------------------------------------


def test_load_meta_index_missing_returns_empty(index_base):
    meta = _load_meta_index("nonexistent-scope")
    assert isinstance(meta, MetaIndex)
    assert meta.entries == []


def test_load_meta_index_corrupt_file_returns_empty(index_base):
    scope = "corrupt-scope"
    path = index_base / scope / "meta.json"
    path.parent.mkdir(parents=True)
    path.write_text("not valid json {{{{", encoding="utf-8")

    meta = _load_meta_index(scope)
    assert meta.entries == []


# ---------------------------------------------------------------------------
# _add_to_meta_index
# ---------------------------------------------------------------------------


def test_add_to_meta_index_creates_file(index_base):
    scope = "camp-abc"
    entry = _make_entry(scope)
    _add_to_meta_index(scope, entry)

    meta_path = index_base / scope / "meta.json"
    assert meta_path.exists()

    loaded = _load_meta_index(scope)
    assert len(loaded.entries) == 1
    assert loaded.entries[0].doc_id == entry.doc_id


def test_add_to_meta_index_deduplicates(index_base):
    scope = "camp-dedup"
    entry = _make_entry(scope)

    _add_to_meta_index(scope, entry)
    # Update summary and write again with same doc_id
    updated = entry.model_copy(update={"summary": "Updated summary."})
    _add_to_meta_index(scope, updated)

    loaded = _load_meta_index(scope)
    assert len(loaded.entries) == 1
    assert loaded.entries[0].summary == "Updated summary."


# ---------------------------------------------------------------------------
# _generate_doc_summary
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_generate_doc_summary_no_llm():
    result = await _generate_doc_summary("some markdown", "MyDoc", call_llm=None)
    assert result == ""


@pytest.mark.asyncio
async def test_generate_doc_summary_llm_failure():
    async def bad_llm(prompt: str) -> str:
        raise RuntimeError("LLM unavailable")

    result = await _generate_doc_summary("some markdown", "MyDoc", call_llm=bad_llm)
    assert result == ""


@pytest.mark.asyncio
async def test_generate_doc_summary_strips_whitespace():
    async def llm(prompt: str) -> str:
        return "  summary with spaces  "

    result = await _generate_doc_summary("some markdown", "MyDoc", call_llm=llm)
    assert result == "summary with spaces"


# ---------------------------------------------------------------------------
# select_relevant_docs
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_select_relevant_docs_empty_meta():
    campaign_id = uuid4()

    async def llm(prompt: str) -> str:
        raise AssertionError("LLM should not be called with empty meta")

    result = await select_relevant_docs(campaign_id, "What are the rules?", llm)
    assert result == []


@pytest.mark.asyncio
async def test_select_relevant_docs_single_doc_no_llm_call():
    campaign_id = uuid4()
    scope = str(campaign_id)
    entry = _make_entry(scope)
    _add_to_meta_index(scope, entry)

    llm_called = False

    async def llm(prompt: str) -> str:
        nonlocal llm_called
        llm_called = True
        return '{"doc_ids": []}'

    result = await select_relevant_docs(campaign_id, "What happened?", llm)
    assert not llm_called
    assert result == [(scope, entry.doc_id)]


@pytest.mark.asyncio
async def test_select_relevant_docs_llm_selects_subset():
    campaign_id = uuid4()
    scope = str(campaign_id)

    entries = [_make_entry(scope, f"Doc {i}") for i in range(3)]
    for e in entries:
        _add_to_meta_index(scope, e)

    selected_ids = [str(entries[0].doc_id), str(entries[2].doc_id)]

    async def llm(prompt: str) -> str:
        return json.dumps({"doc_ids": selected_ids})

    result = await select_relevant_docs(campaign_id, "Some query", llm)
    assert len(result) == 2
    returned_doc_ids = {str(d) for _, d in result}
    assert returned_doc_ids == set(selected_ids)


@pytest.mark.asyncio
async def test_select_relevant_docs_llm_failure_fallback_all():
    campaign_id = uuid4()
    scope = str(campaign_id)

    entries = [_make_entry(scope, f"Doc {i}") for i in range(3)]
    for e in entries:
        _add_to_meta_index(scope, e)

    async def llm(prompt: str) -> str:
        raise RuntimeError("LLM down")

    result = await select_relevant_docs(campaign_id, "Some query", llm)
    assert len(result) == 3


@pytest.mark.asyncio
async def test_select_relevant_docs_invalid_json_fallback_all():
    campaign_id = uuid4()
    scope = str(campaign_id)

    entries = [_make_entry(scope, f"Doc {i}") for i in range(2)]
    for e in entries:
        _add_to_meta_index(scope, e)

    async def llm(prompt: str) -> str:
        return "not valid json at all"

    result = await select_relevant_docs(campaign_id, "Some query", llm)
    assert len(result) == 2


@pytest.mark.asyncio
async def test_select_relevant_docs_unknown_doc_id_fallback():
    campaign_id = uuid4()
    scope = str(campaign_id)

    entries = [_make_entry(scope, f"Doc {i}") for i in range(2)]
    for e in entries:
        _add_to_meta_index(scope, e)

    async def llm(prompt: str) -> str:
        # Returns a UUID that is not in the meta index
        return json.dumps({"doc_ids": [str(uuid4())]})

    result = await select_relevant_docs(campaign_id, "Some query", llm)
    # Empty result from LLM selection → fallback to all
    assert len(result) == 2


# ---------------------------------------------------------------------------
# ingest_campaign_document writes MetaIndex
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def db():
    from guide.db.pool import init_db

    conn = await init_db(":memory:")
    yield conn
    await conn.close()


@pytest.mark.asyncio
async def test_ingest_campaign_document_writes_meta(index_base, db):
    from guide.db.campaigns import CampaignRepository
    from guide.db.documents import DocumentRepository
    from guide.models.campaign import CreateCampaignRequest
    from guide.models.document import CampaignDocument

    # Insert a campaign so the FK constraint is satisfied
    camp_repo = CampaignRepository(db)
    campaign = await camp_repo.create(CreateCampaignRequest(name="Test Campaign"))
    campaign_id = campaign.id
    doc_id = uuid4()

    # Insert document row
    doc_repo = DocumentRepository(db)
    stored_path = str(index_base / "dummy.pdf")
    await doc_repo.insert(
        CampaignDocument(
            id=doc_id,
            campaign_id=campaign_id,
            filename="adventure.pdf",
            stored_path=stored_path,
            uploaded_at=datetime.now(timezone.utc),
        )
    )

    extraction = DocumentExtraction(
        pages=[PageExtraction(page_number=1, raw_text="# Chapter One\nHello world")],
        full_markdown="# Chapter One\nHello world",
    )

    summary_called = False

    async def mock_llm(prompt: str) -> str:
        nonlocal summary_called
        summary_called = True
        return "An adventure document for testing."

    await ingest_campaign_document(
        campaign_id,
        doc_id,
        extraction,
        db,
        doc_name="adventure.pdf",
        call_llm=mock_llm,
    )

    meta_path = index_base / str(campaign_id) / "meta.json"
    assert meta_path.exists(), "meta.json should have been created"

    loaded = _load_meta_index(str(campaign_id))
    assert len(loaded.entries) == 1
    entry = loaded.entries[0]
    assert entry.doc_id == doc_id
    assert entry.doc_name == "adventure.pdf"
    assert entry.summary == "An adventure document for testing."
    assert summary_called
