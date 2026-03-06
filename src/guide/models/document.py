from __future__ import annotations

from datetime import datetime
from enum import Enum
from uuid import UUID

from pydantic import BaseModel

from .shared import IngestionStatus


class DocumentKind(str, Enum):
    campaign = "campaign"
    rulebook = "rulebook"


class CampaignDocument(BaseModel):
    id: UUID
    campaign_id: UUID
    filename: str
    file_size_bytes: int = 0
    stored_path: str
    page_count: int | None = None
    document_kind: DocumentKind = DocumentKind.campaign
    ingestion_status: IngestionStatus = IngestionStatus.pending
    ingestion_error: str | None = None
    uploaded_at: datetime
    ingested_at: datetime | None = None


class GlobalDocument(BaseModel):
    id: UUID
    title: str
    filename: str
    file_size_bytes: int = 0
    stored_path: str
    page_count: int | None = None
    ingestion_status: IngestionStatus = IngestionStatus.pending
    ingestion_error: str | None = None
    uploaded_at: datetime
    ingested_at: datetime | None = None


class RankedChunk(BaseModel):
    content: str
    section_path: str = ""
    doc_title: str = ""
    score: float = 0.0


class DocSummary(BaseModel):
    doc_id: UUID
    doc_name: str
    filename: str
    summary: str  # empty string if LLM unavailable
    scope: str    # campaign_id string OR "global"
    ingested_at: datetime


class MetaIndex(BaseModel):
    scope: str
    entries: list[DocSummary] = []
