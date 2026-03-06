from __future__ import annotations

from datetime import datetime, timezone
from uuid import UUID, uuid4

import aiosqlite

from guide.errors import NotFoundError
from guide.models.campaign import Campaign, CreateCampaignRequest, UpdateCampaignRequest, WorldState
from guide.models.shared import GameSystem


class CampaignRepository:
    def __init__(self, db: aiosqlite.Connection) -> None:
        self._db = db

    async def create(self, req: CreateCampaignRequest) -> Campaign:
        id_ = uuid4()
        now = datetime.now(timezone.utc).isoformat()
        game_system = (req.game_system or GameSystem.dnd5e).value

        await self._db.execute(
            "INSERT INTO campaigns (id, name, description, game_system, created_at, updated_at)"
            " VALUES (?, ?, ?, ?, ?, ?)",
            (str(id_), req.name, req.description, game_system, now, now),
        )
        await self._db.commit()
        return await self.get_by_id(id_)

    async def get_by_id(self, id_: UUID) -> Campaign:
        async with self._db.execute(
            "SELECT id, name, description, game_system, world_state, created_at, updated_at"
            " FROM campaigns WHERE id = ?",
            (str(id_),),
        ) as cursor:
            row = await cursor.fetchone()

        if row is None:
            raise NotFoundError(f"Campaign {id_}")
        return _row_to_campaign(row)

    async def list(self) -> list[Campaign]:
        async with self._db.execute(
            "SELECT id, name, description, game_system, world_state, created_at, updated_at"
            " FROM campaigns ORDER BY created_at DESC"
        ) as cursor:
            rows = await cursor.fetchall()
        return [_row_to_campaign(r) for r in rows]

    async def update(self, id_: UUID, req: UpdateCampaignRequest) -> Campaign:
        now = datetime.now(timezone.utc).isoformat()
        id_str = str(id_)
        if req.name is not None:
            await self._db.execute(
                "UPDATE campaigns SET name = ?, updated_at = ? WHERE id = ?",
                (req.name, now, id_str),
            )
        if req.description is not None:
            await self._db.execute(
                "UPDATE campaigns SET description = ?, updated_at = ? WHERE id = ?",
                (req.description, now, id_str),
            )
        if req.world_state is not None:
            await self._db.execute(
                "UPDATE campaigns SET world_state = ?, updated_at = ? WHERE id = ?",
                (req.world_state.model_dump_json(), now, id_str),
            )
        await self._db.commit()
        return await self.get_by_id(id_)

    async def delete(self, id_: UUID) -> None:
        cursor = await self._db.execute("DELETE FROM campaigns WHERE id = ?", (str(id_),))
        await self._db.commit()
        if cursor.rowcount == 0:
            raise NotFoundError(f"Campaign {id_}")


def _row_to_campaign(row: aiosqlite.Row) -> Campaign:
    world_state = WorldState.model_validate_json(row["world_state"]) if row["world_state"] else None

    return Campaign(
        id=UUID(row["id"]),
        name=row["name"],
        description=row["description"],
        game_system=GameSystem(row["game_system"])
        if row["game_system"] in GameSystem._value2member_map_
        else GameSystem.dnd5e,
        world_state=world_state,
        created_at=datetime.fromisoformat(row["created_at"]),
        updated_at=datetime.fromisoformat(row["updated_at"]),
    )
