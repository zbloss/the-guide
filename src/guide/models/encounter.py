from __future__ import annotations

from datetime import datetime
from uuid import UUID

from pydantic import BaseModel, Field

from .shared import Condition, EncounterStatus


class ActionBudget(BaseModel):
    has_action: bool = True
    has_bonus_action: bool = True
    has_reaction: bool = True
    movement_remaining: int = 30

    @classmethod
    def new(cls, speed: int = 30) -> "ActionBudget":
        return cls(movement_remaining=speed)

    def reset(self, speed: int = 30) -> None:
        self.has_action = True
        self.has_bonus_action = True
        self.has_reaction = True
        self.movement_remaining = speed


class CombatParticipant(BaseModel):
    id: UUID
    encounter_id: UUID
    character_id: UUID
    name: str
    initiative_roll: int = 0
    initiative_modifier: int = 0
    initiative_total: int = 0
    current_hp: int = 10
    max_hp: int = 10
    armor_class: int = 10
    conditions: list[Condition] = Field(default_factory=list)
    action_budget: ActionBudget = Field(default_factory=ActionBudget)
    has_taken_turn: bool = False
    is_defeated: bool = False


class Encounter(BaseModel):
    id: UUID
    session_id: UUID
    campaign_id: UUID
    name: str | None = None
    description: str | None = None
    status: EncounterStatus = EncounterStatus.pending
    round: int = 0
    current_turn_index: int = 0
    participants: list[CombatParticipant] = Field(default_factory=list)
    created_at: datetime
    updated_at: datetime


class EncounterSummary(BaseModel):
    encounter: Encounter
    current_participant: CombatParticipant | None = None
    round: int


class CreateEncounterRequest(BaseModel):
    session_id: UUID
    name: str | None = None
    description: str | None = None
    participant_character_ids: list[UUID] = Field(default_factory=list)


class AddParticipantRequest(BaseModel):
    character_id: UUID
    initiative_roll: int | None = None


class UpdateParticipantRequest(BaseModel):
    hp_delta: int | None = None
    set_hp: int | None = None
    add_condition: Condition | None = None
    remove_condition: Condition | None = None
    spend_action: bool | None = None
    spend_bonus_action: bool | None = None
    spend_reaction: bool | None = None
    spend_movement: int | None = None
