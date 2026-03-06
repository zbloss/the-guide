from __future__ import annotations

from datetime import datetime
from uuid import UUID

from pydantic import BaseModel, Field

from .shared import GameSystem


class WorldState(BaseModel):
    current_location: str | None = None
    current_date_in_world: str | None = None
    active_quests: list[str] = Field(default_factory=list)
    completed_quests: list[str] = Field(default_factory=list)
    custom_notes: str | None = None


class Campaign(BaseModel):
    id: UUID
    name: str
    description: str | None = None
    game_system: GameSystem = GameSystem.dnd5e
    world_state: WorldState | None = None
    created_at: datetime
    updated_at: datetime


class CreateCampaignRequest(BaseModel):
    name: str
    description: str | None = None
    game_system: GameSystem | None = None


class UpdateCampaignRequest(BaseModel):
    name: str | None = None
    description: str | None = None
    world_state: WorldState | None = None
