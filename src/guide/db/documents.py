from __future__ import annotations

from datetime import datetime, timezone
from uuid import UUID

import aiosqlite

from guide.errors import NotFoundError
from guide.models.document import CampaignDocument, DocumentKind, GlobalDocument
from guide.models.shared import IngestionStatus


class DocumentRepository:
    def __init__(self, db: aiosqlite.Connection) -> None:
        self._db = db

    async def insert(self, doc: CampaignDocument) -> CampaignDocument:
        await self._db.execute(
            "INSERT INTO campaign_documents"
            " (id, campaign_id, filename, file_size_bytes, stored_path, page_count,"
            "  document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at)"
            " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                str(doc.id),
                str(doc.campaign_id),
                doc.filename,
                doc.file_size_bytes,
                doc.stored_path,
                doc.page_count,
                doc.document_kind.value,
                doc.ingestion_status.value,
                doc.ingestion_error,
                doc.uploaded_at.isoformat(),
                doc.ingested_at.isoformat() if doc.ingested_at else None,
            ),
        )
        await self._db.commit()
        return await self.get_by_id(doc.id)

    async def get_by_id(self, id_: UUID) -> CampaignDocument:
        async with self._db.execute(
            "SELECT id, campaign_id, filename, file_size_bytes, stored_path, page_count,"
            " document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at"
            " FROM campaign_documents WHERE id = ?",
            (str(id_),),
        ) as cursor:
            row = await cursor.fetchone()

        if row is None:
            raise NotFoundError(f"Document {id_}")
        return _row_to_doc(row)

    async def list_by_campaign(self, campaign_id: UUID) -> list[CampaignDocument]:
        async with self._db.execute(
            "SELECT id, campaign_id, filename, file_size_bytes, stored_path, page_count,"
            " document_kind, ingestion_status, ingestion_error, uploaded_at, ingested_at"
            " FROM campaign_documents WHERE campaign_id = ? ORDER BY uploaded_at DESC",
            (str(campaign_id),),
        ) as cursor:
            rows = await cursor.fetchall()
        return [_row_to_doc(r) for r in rows]

    async def update_status(
        self,
        id_: UUID,
        status: IngestionStatus,
        error: str | None = None,
    ) -> None:
        await self._db.execute(
            "UPDATE campaign_documents SET ingestion_status = ?, ingestion_error = ? WHERE id = ?",
            (status.value, error, str(id_)),
        )
        await self._db.commit()

    async def update_ingested(self, id_: UUID, page_count: int) -> None:
        now = datetime.now(timezone.utc).isoformat()
        await self._db.execute(
            "UPDATE campaign_documents SET ingestion_status = 'completed',"
            " ingested_at = ?, page_count = ? WHERE id = ?",
            (now, page_count, str(id_)),
        )
        await self._db.commit()


class GlobalDocumentRepository:
    def __init__(self, db: aiosqlite.Connection) -> None:
        self._db = db

    async def insert(self, doc: GlobalDocument) -> GlobalDocument:
        await self._db.execute(
            "INSERT INTO global_documents"
            " (id, title, filename, file_size_bytes, stored_path, page_count,"
            "  ingestion_status, ingestion_error, uploaded_at, ingested_at)"
            " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                str(doc.id),
                doc.title,
                doc.filename,
                doc.file_size_bytes,
                doc.stored_path,
                doc.page_count,
                doc.ingestion_status.value,
                doc.ingestion_error,
                doc.uploaded_at.isoformat(),
                doc.ingested_at.isoformat() if doc.ingested_at else None,
            ),
        )
        await self._db.commit()
        return await self.get_by_id(doc.id)

    async def get_by_id(self, id_: UUID) -> GlobalDocument:
        async with self._db.execute(
            "SELECT id, title, filename, file_size_bytes, stored_path, page_count,"
            " ingestion_status, ingestion_error, uploaded_at, ingested_at"
            " FROM global_documents WHERE id = ?",
            (str(id_),),
        ) as cursor:
            row = await cursor.fetchone()
        if row is None:
            raise NotFoundError(f"GlobalDocument {id_}")
        return _row_to_global_doc(row)

    async def list(self) -> list[GlobalDocument]:
        async with self._db.execute(
            "SELECT id, title, filename, file_size_bytes, stored_path, page_count,"
            " ingestion_status, ingestion_error, uploaded_at, ingested_at"
            " FROM global_documents ORDER BY uploaded_at DESC"
        ) as cursor:
            rows = await cursor.fetchall()
        return [_row_to_global_doc(r) for r in rows]

    async def update_status(
        self, id_: UUID, status: IngestionStatus, error: str | None = None
    ) -> None:
        await self._db.execute(
            "UPDATE global_documents SET ingestion_status = ?, ingestion_error = ? WHERE id = ?",
            (status.value, error, str(id_)),
        )
        await self._db.commit()


def _row_to_doc(row: aiosqlite.Row) -> CampaignDocument:
    kind_val = row["document_kind"] if row["document_kind"] else "campaign"
    return CampaignDocument(
        id=UUID(row["id"]),
        campaign_id=UUID(row["campaign_id"]),
        filename=row["filename"],
        file_size_bytes=row["file_size_bytes"],
        stored_path=row["stored_path"],
        page_count=row["page_count"],
        document_kind=DocumentKind(kind_val)
        if kind_val in DocumentKind._value2member_map_
        else DocumentKind.campaign,
        ingestion_status=IngestionStatus(row["ingestion_status"]),
        ingestion_error=row["ingestion_error"],
        uploaded_at=datetime.fromisoformat(row["uploaded_at"]),
        ingested_at=datetime.fromisoformat(row["ingested_at"]) if row["ingested_at"] else None,
    )


def _row_to_global_doc(row: aiosqlite.Row) -> GlobalDocument:
    return GlobalDocument(
        id=UUID(row["id"]),
        title=row["title"],
        filename=row["filename"],
        file_size_bytes=row["file_size_bytes"],
        stored_path=row["stored_path"],
        page_count=row["page_count"],
        ingestion_status=IngestionStatus(row["ingestion_status"]),
        ingestion_error=row["ingestion_error"],
        uploaded_at=datetime.fromisoformat(row["uploaded_at"]),
        ingested_at=datetime.fromisoformat(row["ingested_at"]) if row["ingested_at"] else None,
    )
