from __future__ import annotations

import json
from datetime import datetime, timezone
from uuid import UUID, uuid4

import aiosqlite

from guide.errors import NotFoundError
from guide.models.encounter import (
    ActionBudget,
    CombatParticipant,
    CreateEncounterRequest,
    Encounter,
)
from guide.models.shared import Condition, EncounterStatus


class EncounterRepository:
    def __init__(self, db: aiosqlite.Connection) -> None:
        self._db = db

    async def create(self, campaign_id: UUID, req: CreateEncounterRequest) -> Encounter:
        id_ = uuid4()
        now = datetime.now(timezone.utc).isoformat()

        await self._db.execute(
            "INSERT INTO encounters"
            " (id, session_id, campaign_id, name, description, status, round,"
            "  current_turn_index, created_at, updated_at)"
            " VALUES (?, ?, ?, ?, ?, 'pending', 0, 0, ?, ?)",
            (
                str(id_), str(req.session_id), str(campaign_id),
                req.name, req.description, now, now,
            ),
        )
        await self._db.commit()
        return await self.get_by_id(id_)

    async def get_by_id(self, id_: UUID) -> Encounter:
        async with self._db.execute(
            "SELECT id, session_id, campaign_id, name, description, status, round,"
            " current_turn_index, created_at, updated_at"
            " FROM encounters WHERE id = ?",
            (str(id_),),
        ) as cursor:
            row = await cursor.fetchone()

        if row is None:
            raise NotFoundError(f"Encounter {id_}")

        encounter = _row_to_encounter(row)
        encounter.participants = await self._list_participants(id_)
        return encounter

    async def list_by_session(self, session_id: UUID) -> list[Encounter]:
        async with self._db.execute(
            "SELECT id, session_id, campaign_id, name, description, status, round,"
            " current_turn_index, created_at, updated_at"
            " FROM encounters WHERE session_id = ? ORDER BY created_at ASC",
            (str(session_id),),
        ) as cursor:
            rows = await cursor.fetchall()

        encounters = []
        for row in rows:
            enc = _row_to_encounter(row)
            enc.participants = await self._list_participants(enc.id)
            encounters.append(enc)
        return encounters

    async def save_state(self, encounter: Encounter) -> None:
        now = datetime.now(timezone.utc).isoformat()
        await self._db.execute(
            "UPDATE encounters SET status = ?, round = ?, current_turn_index = ?,"
            " updated_at = ? WHERE id = ?",
            (
                encounter.status.value, encounter.round,
                encounter.current_turn_index, now, str(encounter.id),
            ),
        )
        for p in encounter.participants:
            await self._save_participant(p)
        await self._db.commit()

    async def add_participant(self, participant: CombatParticipant) -> None:
        conditions_json = json.dumps([c.value for c in participant.conditions])
        budget_json = participant.action_budget.model_dump_json()

        await self._db.execute(
            "INSERT INTO combat_participants"
            " (id, encounter_id, character_id, name, initiative_roll, initiative_modifier,"
            "  initiative_total, current_hp, max_hp, armor_class, conditions, action_budget,"
            "  has_taken_turn, is_defeated)"
            " VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                str(participant.id), str(participant.encounter_id),
                str(participant.character_id), participant.name,
                participant.initiative_roll, participant.initiative_modifier,
                participant.initiative_total, participant.current_hp, participant.max_hp,
                participant.armor_class, conditions_json, budget_json,
                int(participant.has_taken_turn), int(participant.is_defeated),
            ),
        )
        await self._db.commit()

    async def _save_participant(self, p: CombatParticipant) -> None:
        conditions_json = json.dumps([c.value for c in p.conditions])
        budget_json = p.action_budget.model_dump_json()

        await self._db.execute(
            "UPDATE combat_participants SET initiative_roll = ?, initiative_modifier = ?,"
            " initiative_total = ?, current_hp = ?, conditions = ?, action_budget = ?,"
            " has_taken_turn = ?, is_defeated = ? WHERE id = ?",
            (
                p.initiative_roll, p.initiative_modifier, p.initiative_total,
                p.current_hp, conditions_json, budget_json,
                int(p.has_taken_turn), int(p.is_defeated), str(p.id),
            ),
        )

    async def _list_participants(self, encounter_id: UUID) -> list[CombatParticipant]:
        async with self._db.execute(
            "SELECT id, encounter_id, character_id, name, initiative_roll, initiative_modifier,"
            " initiative_total, current_hp, max_hp, armor_class, conditions, action_budget,"
            " has_taken_turn, is_defeated"
            " FROM combat_participants WHERE encounter_id = ?"
            " ORDER BY initiative_total DESC, initiative_modifier DESC",
            (str(encounter_id),),
        ) as cursor:
            rows = await cursor.fetchall()
        return [_row_to_participant(r) for r in rows]


def _row_to_encounter(row: aiosqlite.Row) -> Encounter:
    return Encounter(
        id=UUID(row["id"]),
        session_id=UUID(row["session_id"]),
        campaign_id=UUID(row["campaign_id"]),
        name=row["name"],
        description=row["description"],
        status=EncounterStatus(row["status"]),
        round=row["round"],
        current_turn_index=row["current_turn_index"],
        created_at=datetime.fromisoformat(row["created_at"]),
        updated_at=datetime.fromisoformat(row["updated_at"]),
    )


def _row_to_participant(row: aiosqlite.Row) -> CombatParticipant:
    conditions_raw = json.loads(row["conditions"] or "[]")
    conditions = [Condition(c) for c in conditions_raw if c in Condition._value2member_map_]
    action_budget = ActionBudget.model_validate_json(row["action_budget"])

    return CombatParticipant(
        id=UUID(row["id"]),
        encounter_id=UUID(row["encounter_id"]),
        character_id=UUID(row["character_id"]),
        name=row["name"],
        initiative_roll=row["initiative_roll"],
        initiative_modifier=row["initiative_modifier"],
        initiative_total=row["initiative_total"],
        current_hp=row["current_hp"],
        max_hp=row["max_hp"],
        armor_class=row["armor_class"],
        conditions=conditions,
        action_budget=action_budget,
        has_taken_turn=bool(row["has_taken_turn"]),
        is_defeated=bool(row["is_defeated"]),
    )
