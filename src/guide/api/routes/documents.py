from __future__ import annotations

import logging
from datetime import datetime, timezone
from pathlib import Path
from uuid import UUID, uuid4

from fastapi import APIRouter, BackgroundTasks, File, HTTPException, Query, Request, UploadFile

from guide.db.campaigns import CampaignRepository
from guide.db.documents import DocumentRepository
from guide.errors import NotFoundError
from guide.llm.client import CompletionRequest, LlmTask, Message
from guide.models.document import CampaignDocument
from guide.models.shared import IngestionStatus
from guide.pdf.extractor import extract_document
from guide.pdf.pipeline import ingest_campaign_document, is_already_indexed

logger = logging.getLogger(__name__)

router = APIRouter()


def _db(r: Request):
    return r.app.state.guide.db


@router.get("/campaigns/{campaign_id}/documents")
async def list_documents(campaign_id: UUID, request: Request):
    cam_repo = CampaignRepository(_db(request))
    try:
        await cam_repo.get_by_id(campaign_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))

    doc_repo = DocumentRepository(_db(request))
    docs = await doc_repo.list_by_campaign(campaign_id)
    return [d.model_dump(mode="json") for d in docs]


@router.post("/campaigns/{campaign_id}/documents", status_code=201)
async def upload_document(
    campaign_id: UUID,
    request: Request,
    file: UploadFile = File(...),
):
    cam_repo = CampaignRepository(_db(request))
    try:
        await cam_repo.get_by_id(campaign_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))

    max_bytes = request.app.state.guide.config.max_upload_bytes
    file_bytes = await file.read()

    if len(file_bytes) > max_bytes:
        raise HTTPException(
            status_code=413,
            detail=f"File exceeds maximum size of {max_bytes} bytes",
        )

    if not file.filename:
        raise HTTPException(status_code=400, detail="Missing filename")

    if not file_bytes.startswith(b"%PDF"):
        raise HTTPException(status_code=400, detail="Uploaded file is not a valid PDF")

    doc_id = uuid4()
    dir_path = Path(f"data/documents/{campaign_id}")
    dir_path.mkdir(parents=True, exist_ok=True)
    stored_path = str(dir_path / f"{doc_id}.pdf")

    Path(stored_path).write_bytes(file_bytes)

    doc = CampaignDocument(
        id=doc_id,
        campaign_id=campaign_id,
        filename=file.filename,
        file_size_bytes=len(file_bytes),
        stored_path=stored_path,
        uploaded_at=datetime.now(timezone.utc),
    )

    doc_repo = DocumentRepository(_db(request))
    saved = await doc_repo.insert(doc)
    return saved.model_dump(mode="json")


@router.get("/campaigns/{campaign_id}/documents/{doc_id}")
async def get_document(campaign_id: UUID, doc_id: UUID, request: Request):
    doc_repo = DocumentRepository(_db(request))
    try:
        doc = await doc_repo.get_by_id(doc_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))

    if doc.campaign_id != campaign_id:
        raise HTTPException(status_code=404, detail="Document not found")
    return doc.model_dump(mode="json")


@router.post("/campaigns/{campaign_id}/documents/{doc_id}/ingest", status_code=202)
async def ingest_document(
    campaign_id: UUID,
    doc_id: UUID,
    request: Request,
    background_tasks: BackgroundTasks,
    force: bool = Query(False, description="Re-ingest even if already indexed"),
):
    doc_repo = DocumentRepository(_db(request))
    try:
        doc = await doc_repo.get_by_id(doc_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))

    if doc.campaign_id != campaign_id:
        raise HTTPException(status_code=404, detail="Document not found")

    await doc_repo.update_status(doc_id, IngestionStatus.processing)

    cfg = request.app.state.guide.config
    db = _db(request)
    llm = request.app.state.guide.llm
    qdrant = request.app.state.guide.qdrant

    async def _call_llm(prompt: str) -> str:
        resp = await llm.complete(
            CompletionRequest(
                task=LlmTask.campaign_assistant,
                messages=[Message(role="user", content=prompt)],
                temperature=0,
            )
        )
        return resp.content

    from guide.llm.client import EmbeddingRequest

    async def _embed(text: str) -> list[float]:
        return await llm.embed(EmbeddingRequest(text=text))

    background_tasks.add_task(
        _run_ingestion,
        campaign_id,
        doc_id,
        doc.stored_path,
        db,
        cfg.device,
        cfg.num_threads,
        doc.filename,
        _call_llm,
        force,
        _embed,
        qdrant,
        cfg.qdrant_collection,
        cfg.chunk_max_chars,
    )

    return {"status": "processing", "doc_id": str(doc_id)}


async def _run_ingestion(
    campaign_id: UUID,
    doc_id: UUID,
    stored_path: str,
    db,
    device: str,
    num_threads: int,
    doc_name: str | None = None,
    call_llm=None,
    force: bool = False,
    embed=None,
    qdrant=None,
    collection: str = "guide_chunks",
    chunk_max_chars: int = 2048,
) -> None:
    scope = str(campaign_id)
    if not force and is_already_indexed(scope, doc_id):
        logger.info("Doc %s already indexed, skipping extraction", doc_id)
        return

    doc_repo = DocumentRepository(db)
    try:
        pdf_bytes = Path(stored_path).read_bytes()
        extraction = await extract_document(pdf_bytes, device=device, num_threads=num_threads)
        await ingest_campaign_document(
            campaign_id, doc_id, extraction, db, doc_name, call_llm,
            embed=embed, qdrant=qdrant, collection=collection,
            chunk_max_chars=chunk_max_chars, force=force,
        )
        logger.info("Ingested %d pages for doc %s", len(extraction.pages), doc_id)
    except Exception as exc:
        logger.exception("Ingestion failed for doc %s", doc_id)
        try:
            await doc_repo.update_status(doc_id, IngestionStatus.failed, str(exc))
        except Exception:
            logger.exception("Failed to update status to failed for doc %s", doc_id)
