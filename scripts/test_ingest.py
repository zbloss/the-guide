"""End-to-end ingestion test for MetaIndex / DocSelector.

Directly calls pipeline functions (no HTTP server needed) so there is no
50 MB upload limit.  Requires Ollama to be running at localhost:11434.

Already-indexed documents are skipped by default.  Set FORCE_REINGEST=True
to re-extract and re-index every document regardless.

Usage:
    uv run python scripts/test_ingest.py
"""

from __future__ import annotations

import asyncio
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from uuid import uuid4

# Ensure src/ is on path
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from guide.config import AppConfig
from guide.db.campaigns import CampaignRepository
from guide.db.documents import DocumentRepository, GlobalDocumentRepository
from guide.db.pool import init_db
from guide.llm.client import CompletionRequest, LlmTask, Message
from guide.llm.router import LlmRouter
from guide.models.campaign import CreateCampaignRequest
from guide.models.document import CampaignDocument, GlobalDocument
from guide.pdf.extractor import extract_document
from guide.pdf.pipeline import (
    _load_meta_index,
    ingest_campaign_document,
    ingest_global_document,
    select_relevant_docs,
)

# Set to True to force re-extraction and re-indexing of all documents
FORCE_REINGEST = True

DND_DIR = Path("/mnt/c/Users/altoz/Documents/dnd")
LOV_DIR = DND_DIR / "Land of Vampires"

GLOBAL_PDFS = [
    ("2024 Player's Handbook", DND_DIR / "2024_DnD_Players_Handbook.pdf"),
    ("2024 Dungeon Master's Guide", DND_DIR / "2024_DnD_DM_Guide.pdf"),
    ("2024 Monster Manual", DND_DIR / "2024_DnD_Monster_Manual.pdf"),
]

CAMPAIGN_PDFS = [
    ("Adventurer's Guide to Azuria", LOV_DIR / "Adventurer's Guide to Azuria_LoV.pdf"),
    ("Land of Vampires Full Campaign", LOV_DIR / "Land of Vampires Full Campaign.pdf"),
]

TEST_QUERIES = [
    "What spells can a wizard cast at level 1?",
    "What are the hit points and abilities of a vampire?",
    "Who is the vampire lord ruling Azuria?",
    "How does a Dungeon Master prepare and run encounters?",
    "What playable character races are available?",
]


def ts() -> str:
    return datetime.now().strftime("%H:%M:%S")


def _find_meta_entry(scope: str, doc_name: str):
    """Return the existing MetaIndex entry matching doc_name, or None."""
    return next(
        (e for e in _load_meta_index(scope).entries if e.doc_name == doc_name),
        None,
    )


async def make_llm_caller(llm: LlmRouter, think: bool | None = None):
    async def call_llm(prompt: str) -> str:
        resp = await llm.complete(
            CompletionRequest(
                task=LlmTask.campaign_assistant,
                messages=[Message(role="user", content=prompt)],
                temperature=0,
                think=think,
            )
        )
        return resp.content

    return call_llm


async def ingest_global(pdf_path: Path, title: str, db, call_llm) -> None:
    existing = _find_meta_entry("global", title)
    if existing and not FORCE_REINGEST:
        print(f"  [{ts()}] Already indexed (doc_id={existing.doc_id}), skipping")
        return

    doc_id = existing.doc_id if existing else uuid4()
    print(f"  [{ts()}] Extracting: {pdf_path.name} ({pdf_path.stat().st_size // 1_048_576} MB)...")
    t0 = time.time()
    pdf_bytes = pdf_path.read_bytes()
    extraction = await extract_document(pdf_bytes)
    elapsed = time.time() - t0
    print(f"  [{ts()}] Extracted {len(extraction.pages)} pages in {elapsed:.1f}s")

    if not existing:
        repo = GlobalDocumentRepository(db)
        doc = GlobalDocument(
            id=doc_id,
            title=title,
            filename=pdf_path.name,
            file_size_bytes=len(pdf_bytes),
            stored_path=str(pdf_path),
            uploaded_at=datetime.now(timezone.utc),
        )
        await repo.insert(doc)

    print(f"  [{ts()}] Building index + generating summary...")
    await ingest_global_document(doc_id, extraction, db, doc_name=title, call_llm=call_llm)
    print(f"  [{ts()}] Done: {title}")


async def ingest_campaign_doc(
    pdf_path: Path, name: str, campaign_id, db, call_llm
) -> None:
    scope = str(campaign_id)
    existing = _find_meta_entry(scope, name)
    if existing and not FORCE_REINGEST:
        print(f"  [{ts()}] Already indexed (doc_id={existing.doc_id}), skipping")
        return

    doc_id = existing.doc_id if existing else uuid4()
    print(f"  [{ts()}] Extracting: {pdf_path.name} ({pdf_path.stat().st_size // 1_048_576} MB)...")
    t0 = time.time()
    pdf_bytes = pdf_path.read_bytes()
    extraction = await extract_document(pdf_bytes)
    elapsed = time.time() - t0
    print(f"  [{ts()}] Extracted {len(extraction.pages)} pages in {elapsed:.1f}s")

    if not existing:
        repo = DocumentRepository(db)
        doc = CampaignDocument(
            id=doc_id,
            campaign_id=campaign_id,
            filename=pdf_path.name,
            file_size_bytes=len(pdf_bytes),
            stored_path=str(pdf_path),
            uploaded_at=datetime.now(timezone.utc),
        )
        await repo.insert(doc)

    print(f"  [{ts()}] Building index + generating summary...")
    await ingest_campaign_document(
        campaign_id, doc_id, extraction, db, doc_name=name, call_llm=call_llm
    )
    print(f"  [{ts()}] Done: {name}")


async def main() -> None:
    print(f"[{ts()}] Initializing The Guide test harness  (FORCE_REINGEST={FORCE_REINGEST})")

    config = AppConfig()
    llm = LlmRouter.from_config(config)
    # think=None during ingest (allow reasoning for better summaries)
    # think=False for DocSelector queries (routing only — speed matters)
    call_llm_ingest = await make_llm_caller(llm, think=None)
    call_llm_fast = await make_llm_caller(llm, think=False)

    db = await init_db(config.database_url)
    print(f"[{ts()}] DB: {config.database_url}")

    # -----------------------------------------------------------------------
    # Resolve or create campaign — reuse existing "Land of Vampires" if present
    # -----------------------------------------------------------------------
    camp_repo = CampaignRepository(db)
    campaigns = await camp_repo.list()
    campaign = next((c for c in campaigns if c.name == "Land of Vampires"), None)
    if campaign is None:
        campaign = await camp_repo.create(
            CreateCampaignRequest(
                name="Land of Vampires",
                description="A gothic horror campaign set in Azuria, land ruled by vampires.",
            )
        )
        print(f"[{ts()}] Campaign created: {campaign.name} ({campaign.id})")
    else:
        print(f"[{ts()}] Campaign found:   {campaign.name} ({campaign.id})")

    campaign_id = campaign.id

    # -----------------------------------------------------------------------
    # Ingest global rulebooks
    # -----------------------------------------------------------------------
    print(f"\n[{ts()}] === Ingesting global rulebooks ({len(GLOBAL_PDFS)} books) ===")
    for title, path in GLOBAL_PDFS:
        print(f"\n[{ts()}] >> {title}")
        try:
            await ingest_global(path, title, db, call_llm_ingest)
        except Exception as e:
            print(f"  [{ts()}] ERROR ingesting {title}: {e}")

    # -----------------------------------------------------------------------
    # Ingest campaign documents
    # -----------------------------------------------------------------------
    print(f"\n[{ts()}] === Ingesting campaign documents ({len(CAMPAIGN_PDFS)} docs) ===")
    for name, path in CAMPAIGN_PDFS:
        print(f"\n[{ts()}] >> {name}")
        try:
            await ingest_campaign_doc(path, name, campaign_id, db, call_llm_ingest)
        except Exception as e:
            print(f"  [{ts()}] ERROR ingesting {name}: {e}")

    # -----------------------------------------------------------------------
    # Print MetaIndex contents
    # -----------------------------------------------------------------------
    print(f"\n[{ts()}] === MetaIndex contents ===")

    global_meta = _load_meta_index("global")
    print(f"\nGlobal scope ({len(global_meta.entries)} entries):")
    for e in global_meta.entries:
        print(f"  • {e.doc_name}")
        print(f"    summary: {e.summary[:120]}...")

    campaign_meta = _load_meta_index(str(campaign_id))
    print(f"\nCampaign scope ({len(campaign_meta.entries)} entries):")
    for e in campaign_meta.entries:
        print(f"  • {e.doc_name}")
        print(f"    summary: {e.summary[:120]}...")

    # -----------------------------------------------------------------------
    # Test DocSelector
    # -----------------------------------------------------------------------
    print(f"\n[{ts()}] === DocSelector test queries ===")
    all_entries = global_meta.entries + campaign_meta.entries
    id_to_name = {str(e.doc_id): e.doc_name for e in all_entries}
    for query in TEST_QUERIES:
        print(f"\n  [{ts()}] Query: \"{query}\"")
        t0 = time.time()
        try:
            selected = await select_relevant_docs(campaign_id, query, call_llm_fast)
            elapsed = time.time() - t0
            names = [id_to_name.get(str(d), str(d)) for _, d in selected]
            print(f"  [{ts()}] Done in {elapsed:.1f}s — Selected ({len(selected)}): {names}")
        except Exception as e:
            elapsed = time.time() - t0
            print(f"  [{ts()}] ERROR after {elapsed:.1f}s: {e}")

    # -----------------------------------------------------------------------
    # Index file inventory
    # -----------------------------------------------------------------------
    print(f"\n[{ts()}] === Index files written ===")
    index_base = Path("data/indexes")
    for p in sorted(index_base.rglob("meta.json")):
        print(f"  {p}  ({p.stat().st_size:,} bytes)")

    print(f"\n[{ts()}] Done.")
    await db.close()


if __name__ == "__main__":
    asyncio.run(main())
