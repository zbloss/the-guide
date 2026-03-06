from __future__ import annotations

import json
from datetime import datetime, timezone
from uuid import UUID, uuid4

import aiosqlite

from guide.errors import NotFoundError
from guide.models.session import (
    CreateSessionEventRequest,
    CreateSessionRequest,
    Session,
    SessionEvent,
)
from guide.models.shared import EventSignificance, EventType


class SessionRepository:
    def __init__(self, db: aiosqlite.Connection) -> None:
        self._db = db

    async def create(self, campaign_id: UUID, req: CreateSessionRequest) -> Session:
        id_ = uuid4()
        now = datetime.now(timezone.utc).isoformat()

        # Auto-number sessions within campaign
        async with self._db.execute(
            "SELECT COALESCE(MAX(session_number), 0) + 1 FROM sessions WHERE campaign_id = ?",
            (str(campaign_id),),
        ) as cursor:
            row = await cursor.fetchone()
            session_number = row[0] if row else 1

        await self._db.execute(
            "INSERT INTO sessions"
            " (id, campaign_id, session_number, title, notes, created_at, updated_at)"
            " VALUES (?, ?, ?, ?, ?, ?, ?)",
            (str(id_), str(campaign_id), session_number, req.title, req.notes, now, now),
        )
        await self._db.commit()
        return await self.get_by_id(id_)

    async def get_by_id(self, id_: UUID) -> Session:
        async with self._db.execute(
            "SELECT id, campaign_id, session_number, title, notes,"
            " started_at, ended_at, created_at, updated_at"
            " FROM sessions WHERE id = ?",
            (str(id_),),
        ) as cursor:
            row = await cursor.fetchone()

        if row is None:
            raise NotFoundError(f"Session {id_}")
        return _row_to_session(row)

    async def list_by_campaign(self, campaign_id: UUID) -> list[Session]:
        async with self._db.execute(
            "SELECT id, campaign_id, session_number, title, notes,"
            " started_at, ended_at, created_at, updated_at"
            " FROM sessions WHERE campaign_id = ? ORDER BY session_number ASC",
            (str(campaign_id),),
        ) as cursor:
            rows = await cursor.fetchall()
        return [_row_to_session(r) for r in rows]

    async def start_session(self, id_: UUID) -> Session:
        now = datetime.now(timezone.utc).isoformat()
        await self._db.execute(
            "UPDATE sessions SET started_at = ?, updated_at = ? WHERE id = ?",
            (now, now, str(id_)),
        )
        await self._db.commit()
        return await self.get_by_id(id_)

    async def end_session(self, id_: UUID) -> Session:
        now = datetime.now(timezone.utc).isoformat()
        await self._db.execute(
            "UPDATE sessions SET ended_at = ?, updated_at = ? WHERE id = ?",
            (now, now, str(id_)),
        )
        await self._db.commit()
        return await self.get_by_id(id_)

    async def delete(self, id_: UUID) -> None:
        cursor = await self._db.execute("DELETE FROM sessions WHERE id = ?", (str(id_),))
        await self._db.commit()
        if cursor.rowcount == 0:
            raise NotFoundError(f"Session {id_}")


class SessionEventRepository:
    def __init__(self, db: aiosqlite.Connection) -> None:
        self._db = db

    async def create(
        self,
        session_id: UUID,
        campaign_id: UUID,
        req: CreateSessionEventRequest,
    ) -> SessionEvent:
        id_ = uuid4()
        now = datetime.now(timezone.utc).isoformat()
        significance = (req.significance or EventSignificance.minor).value
        is_visible = int(req.is_player_visible if req.is_player_visible is not None else True)
        char_ids = json.dumps([str(c) for c in (req.involved_character_ids or [])])

        await self._db.execute(
            "INSERT INTO session_events"
            " (id, session_id, campaign_id, event_type, description, significance,"
            "  is_player_visible, involved_character_ids, occurred_at)"
            " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                str(id_),
                str(session_id),
                str(campaign_id),
                req.event_type.value,
                req.description,
                significance,
                is_visible,
                char_ids,
                now,
            ),
        )
        await self._db.commit()

        async with self._db.execute(
            "SELECT id, session_id, campaign_id, event_type, description, significance,"
            " is_player_visible, involved_character_ids, occurred_at"
            " FROM session_events WHERE id = ?",
            (str(id_),),
        ) as cursor:
            row = await cursor.fetchone()
        return _row_to_event(row)

    async def list_by_session(self, session_id: UUID) -> list[SessionEvent]:
        async with self._db.execute(
            "SELECT id, session_id, campaign_id, event_type, description, significance,"
            " is_player_visible, involved_character_ids, occurred_at"
            " FROM session_events WHERE session_id = ? ORDER BY occurred_at ASC",
            (str(session_id),),
        ) as cursor:
            rows = await cursor.fetchall()
        return [_row_to_event(r) for r in rows]

    async def list_visible_by_session(self, session_id: UUID) -> list[SessionEvent]:
        async with self._db.execute(
            "SELECT id, session_id, campaign_id, event_type, description, significance,"
            " is_player_visible, involved_character_ids, occurred_at"
            " FROM session_events WHERE session_id = ? AND is_player_visible = 1"
            " ORDER BY occurred_at ASC",
            (str(session_id),),
        ) as cursor:
            rows = await cursor.fetchall()
        return [_row_to_event(r) for r in rows]


def _row_to_session(row: aiosqlite.Row) -> Session:
    return Session(
        id=UUID(row["id"]),
        campaign_id=UUID(row["campaign_id"]),
        session_number=row["session_number"],
        title=row["title"],
        notes=row["notes"],
        started_at=datetime.fromisoformat(row["started_at"]) if row["started_at"] else None,
        ended_at=datetime.fromisoformat(row["ended_at"]) if row["ended_at"] else None,
        created_at=datetime.fromisoformat(row["created_at"]),
        updated_at=datetime.fromisoformat(row["updated_at"]),
    )


def _row_to_event(row: aiosqlite.Row) -> SessionEvent:
    char_ids = [UUID(c) for c in json.loads(row["involved_character_ids"] or "[]")]

    return SessionEvent(
        id=UUID(row["id"]),
        session_id=UUID(row["session_id"]),
        campaign_id=UUID(row["campaign_id"]),
        event_type=EventType(row["event_type"])
        if row["event_type"] in EventType._value2member_map_
        else EventType.custom,
        description=row["description"],
        significance=EventSignificance(row["significance"]),
        is_player_visible=bool(row["is_player_visible"]),
        involved_character_ids=char_ids,
        occurred_at=datetime.fromisoformat(row["occurred_at"]),
    )
