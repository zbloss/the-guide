from __future__ import annotations

import json
from datetime import datetime, timezone
from uuid import UUID, uuid4

import aiosqlite

from guide.errors import NotFoundError
from guide.models.character import (
    AbilityScores,
    Backstory,
    Character,
    CreateCharacterRequest,
    UpdateCharacterRequest,
)
from guide.models.shared import CharacterType, Condition


class CharacterRepository:
    def __init__(self, db: aiosqlite.Connection) -> None:
        self._db = db

    async def create(self, campaign_id: UUID, req: CreateCharacterRequest) -> Character:
        id_ = uuid4()
        now = datetime.now(timezone.utc).isoformat()
        char_type = req.character_type.value
        ability_scores = req.ability_scores or AbilityScores()
        level = req.level or 1
        speed = req.speed or 30

        try:
            await self._db.execute(
                "INSERT INTO characters"
                " (id, campaign_id, name, character_type, class, race, level, max_hp, current_hp,"
                "  armor_class, speed, ability_scores, conditions, backstory, is_alive,"
                "  created_at, updated_at)"
                " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    str(id_),
                    str(campaign_id),
                    req.name,
                    char_type,
                    req.class_,
                    req.race,
                    level,
                    req.max_hp,
                    req.max_hp,
                    req.armor_class,
                    speed,
                    ability_scores.model_dump_json(),
                    "[]",
                    None,
                    1,
                    now,
                    now,
                ),
            )
            await self._db.commit()
        except Exception:
            await self._db.rollback()
            raise
        return await self.get_by_id(id_)

    async def get_by_id(self, id_: UUID) -> Character:
        async with self._db.execute(
            "SELECT id, campaign_id, name, character_type, class, race, level, max_hp,"
            " current_hp, armor_class, speed, ability_scores, conditions, backstory,"
            " is_alive, created_at, updated_at"
            " FROM characters WHERE id = ?",
            (str(id_),),
        ) as cursor:
            row = await cursor.fetchone()

        if row is None:
            raise NotFoundError(f"Character {id_}")
        return _row_to_character(row)

    async def list_by_campaign(self, campaign_id: UUID) -> list[Character]:
        async with self._db.execute(
            "SELECT id, campaign_id, name, character_type, class, race, level, max_hp,"
            " current_hp, armor_class, speed, ability_scores, conditions, backstory,"
            " is_alive, created_at, updated_at"
            " FROM characters WHERE campaign_id = ? ORDER BY name ASC",
            (str(campaign_id),),
        ) as cursor:
            rows = await cursor.fetchall()
        return [_row_to_character(r) for r in rows]

    async def update(self, id_: UUID, req: UpdateCharacterRequest) -> Character:
        now = datetime.now(timezone.utc).isoformat()
        id_str = str(id_)
        try:
            last_cursor = None
            if req.current_hp is not None:
                last_cursor = await self._db.execute(
                    "UPDATE characters SET current_hp = ?, updated_at = ? WHERE id = ?",
                    (req.current_hp, now, id_str),
                )
            if req.conditions is not None:
                last_cursor = await self._db.execute(
                    "UPDATE characters SET conditions = ?, updated_at = ? WHERE id = ?",
                    (json.dumps([c.value for c in req.conditions]), now, id_str),
                )
            if req.is_alive is not None:
                last_cursor = await self._db.execute(
                    "UPDATE characters SET is_alive = ?, updated_at = ? WHERE id = ?",
                    (int(req.is_alive), now, id_str),
                )
            if last_cursor is not None and last_cursor.rowcount == 0:
                raise NotFoundError(f"Character {id_}")
            await self._db.commit()
        except Exception:
            await self._db.rollback()
            raise
        return await self.get_by_id(id_)

    async def delete(self, id_: UUID) -> None:
        try:
            cursor = await self._db.execute("DELETE FROM characters WHERE id = ?", (str(id_),))
            await self._db.commit()
        except Exception:
            await self._db.rollback()
            raise
        if cursor.rowcount == 0:
            raise NotFoundError(f"Character {id_}")

    async def set_backstory(self, id_: UUID, backstory: Backstory) -> None:
        try:
            await self._db.execute(
                "UPDATE characters SET backstory = ?, updated_at = ? WHERE id = ?",
                (backstory.model_dump_json(), datetime.now(timezone.utc).isoformat(), str(id_)),
            )
            await self._db.commit()
        except Exception:
            await self._db.rollback()
            raise


def _row_to_character(row: aiosqlite.Row) -> Character:
    ability_scores = AbilityScores.model_validate_json(row["ability_scores"])
    conditions_raw = json.loads(row["conditions"] or "[]")
    conditions = [Condition(c) for c in conditions_raw if c in Condition._value2member_map_]
    backstory = Backstory.model_validate_json(row["backstory"]) if row["backstory"] else None

    return Character(
        id=UUID(row["id"]),
        campaign_id=UUID(row["campaign_id"]),
        name=row["name"],
        character_type=CharacterType(row["character_type"]),
        class_=row["class"],
        race=row["race"],
        level=row["level"],
        max_hp=row["max_hp"],
        current_hp=row["current_hp"],
        armor_class=row["armor_class"],
        speed=row["speed"],
        ability_scores=ability_scores,
        conditions=conditions,
        backstory=backstory,
        is_alive=bool(row["is_alive"]),
        created_at=datetime.fromisoformat(row["created_at"]),
        updated_at=datetime.fromisoformat(row["updated_at"]),
    )
