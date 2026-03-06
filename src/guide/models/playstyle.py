from __future__ import annotations

from datetime import datetime, timezone
from enum import Enum
from uuid import UUID

from pydantic import BaseModel, Field


class GeneratedEncounterType(str, Enum):
    combat = "combat"
    social = "social"
    exploration = "exploration"
    puzzle = "puzzle"
    mixed = "mixed"


class EnemySuggestion(BaseModel):
    name: str
    count: int = 1
    cr: float | None = None


class GeneratedEncounter(BaseModel):
    title: str
    description: str
    encounter_type: GeneratedEncounterType
    challenge_rating: float | None = None
    suggested_enemies: list[EnemySuggestion] = Field(default_factory=list)
    narrative_hook: str
    alternative: str | None = None


class PlaystyleProfile(BaseModel):
    campaign_id: UUID
    combat_affinity: float = 0.33
    social_affinity: float = 0.33
    exploration_affinity: float = 0.34
    preferred_difficulty: float = 0.5
    sessions_sampled: int = 0
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))

    @classmethod
    def default_for(cls, campaign_id: UUID) -> "PlaystyleProfile":
        return cls(campaign_id=campaign_id)
