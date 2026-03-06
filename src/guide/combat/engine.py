from __future__ import annotations

from datetime import datetime, timezone
from uuid import UUID, uuid4

from guide.errors import InvalidInputError, NotFoundError
from guide.models.encounter import ActionBudget, CombatParticipant, Encounter
from guide.models.shared import Condition, EncounterStatus


class CombatEngine:
    def __init__(self, encounter: Encounter) -> None:
        self.encounter = encounter

    def start(self) -> None:
        if self.encounter.status != EncounterStatus.pending:
            raise InvalidInputError("Encounter has already started")

        # Sort participants by initiative_total descending; ties by modifier, then id
        self.encounter.participants.sort(
            key=lambda p: (p.initiative_total, p.initiative_modifier, str(p.id)),
            reverse=True,
        )

        self.encounter.status = EncounterStatus.active
        self.encounter.round = 1
        self.encounter.current_turn_index = 0
        self.encounter.updated_at = datetime.now(timezone.utc)

        if self.encounter.participants:
            self.encounter.participants[0].has_taken_turn = False

    def next_turn(self) -> CombatParticipant:
        if self.encounter.status != EncounterStatus.active:
            raise InvalidInputError("Encounter is not active")

        n = len(self.encounter.participants)
        if n == 0:
            raise InvalidInputError("No participants in encounter")

        current_idx = self.encounter.current_turn_index
        self.encounter.participants[current_idx].has_taken_turn = True

        next_idx = (current_idx + 1) % n
        self.encounter.current_turn_index = next_idx

        if next_idx == 0:
            self.encounter.round += 1
            for p in self.encounter.participants:
                p.has_taken_turn = False
                p.action_budget.reset(30)

        self.encounter.updated_at = datetime.now(timezone.utc)
        return self.encounter.participants[next_idx]

    def apply_hp_change(self, participant_id: UUID, delta: int) -> int:
        p = self._find_participant(participant_id)
        p.current_hp = max(0, min(p.current_hp + delta, p.max_hp))
        if p.current_hp == 0:
            p.is_defeated = True
            if Condition.unconscious not in p.conditions:
                p.conditions.append(Condition.unconscious)
        self.encounter.updated_at = datetime.now(timezone.utc)
        return p.current_hp

    def set_hp(self, participant_id: UUID, hp: int) -> int:
        p = self._find_participant(participant_id)
        delta = hp - p.current_hp
        return self.apply_hp_change(participant_id, delta)

    def add_condition(self, participant_id: UUID, condition: Condition) -> None:
        p = self._find_participant(participant_id)
        if condition not in p.conditions:
            p.conditions.append(condition)
        self.encounter.updated_at = datetime.now(timezone.utc)

    def remove_condition(self, participant_id: UUID, condition: Condition) -> None:
        p = self._find_participant(participant_id)
        p.conditions = [c for c in p.conditions if c != condition]
        self.encounter.updated_at = datetime.now(timezone.utc)

    def end(self) -> None:
        if self.encounter.status == EncounterStatus.completed:
            raise InvalidInputError("Encounter already ended")
        self.encounter.status = EncounterStatus.completed
        self.encounter.updated_at = datetime.now(timezone.utc)

    def current_participant(self) -> CombatParticipant | None:
        idx = self.encounter.current_turn_index
        if 0 <= idx < len(self.encounter.participants):
            return self.encounter.participants[idx]
        return None

    def _find_participant(self, participant_id: UUID) -> CombatParticipant:
        for p in self.encounter.participants:
            if p.id == participant_id:
                return p
        raise NotFoundError(f"Participant {participant_id}")


def build_participant(
    character_id: UUID,
    encounter_id: UUID,
    name: str,
    initiative_roll: int,
    initiative_modifier: int,
    max_hp: int,
    current_hp: int,
    armor_class: int,
    speed: int = 30,
) -> CombatParticipant:
    return CombatParticipant(
        id=uuid4(),
        encounter_id=encounter_id,
        character_id=character_id,
        name=name,
        initiative_roll=initiative_roll,
        initiative_modifier=initiative_modifier,
        initiative_total=initiative_roll + initiative_modifier,
        current_hp=current_hp,
        max_hp=max_hp,
        armor_class=armor_class,
        action_budget=ActionBudget.new(speed),
    )
