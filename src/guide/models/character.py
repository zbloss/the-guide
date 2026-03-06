from __future__ import annotations

from datetime import datetime
from enum import Enum
from uuid import UUID, uuid4

from pydantic import BaseModel, Field

from .shared import CharacterType, Condition


class HookPriority(str, Enum):
    low = "low"
    medium = "medium"
    high = "high"
    critical = "critical"


class AbilityScores(BaseModel):
    strength: int = 10
    dexterity: int = 10
    constitution: int = 10
    intelligence: int = 10
    wisdom: int = 10
    charisma: int = 10

    @staticmethod
    def modifier(score: int) -> int:
        return (score - 10) // 2

    def initiative_modifier(self) -> int:
        return self.modifier(self.dexterity)


class PlotHook(BaseModel):
    id: UUID
    character_id: UUID
    description: str
    priority: HookPriority = HookPriority.medium
    is_active: bool = True
    llm_extracted: bool = False


class Backstory(BaseModel):
    raw_text: str
    extracted_hooks: list[PlotHook] = Field(default_factory=list)
    motivations: list[str] = Field(default_factory=list)
    key_relationships: list[str] = Field(default_factory=list)
    secrets: list[str] = Field(default_factory=list)


class Character(BaseModel):
    id: UUID
    campaign_id: UUID
    name: str
    character_type: CharacterType
    class_: str | None = Field(None, alias="class")
    race: str | None = None
    level: int = 1
    max_hp: int = 10
    current_hp: int = 10
    armor_class: int = 10
    speed: int = 30
    ability_scores: AbilityScores = Field(default_factory=AbilityScores)
    conditions: list[Condition] = Field(default_factory=list)
    backstory: Backstory | None = None
    is_alive: bool = True
    created_at: datetime
    updated_at: datetime

    model_config = {"populate_by_name": True}


class CreateCharacterRequest(BaseModel):
    name: str
    character_type: CharacterType
    class_: str | None = Field(None, alias="class")
    race: str | None = None
    level: int | None = None
    max_hp: int
    armor_class: int
    speed: int | None = None
    ability_scores: AbilityScores | None = None
    backstory_text: str | None = None

    model_config = {"populate_by_name": True}


class UpdateCharacterRequest(BaseModel):
    current_hp: int | None = None
    conditions: list[Condition] | None = None
    is_alive: bool | None = None
