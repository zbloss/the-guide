from __future__ import annotations

from datetime import datetime
from uuid import UUID

from pydantic import BaseModel, Field

from .shared import EventSignificance, EventType, Perspective


class Session(BaseModel):
    id: UUID
    campaign_id: UUID
    session_number: int = 1
    title: str | None = None
    notes: str | None = None
    started_at: datetime | None = None
    ended_at: datetime | None = None
    created_at: datetime
    updated_at: datetime


class SessionEvent(BaseModel):
    id: UUID
    session_id: UUID
    campaign_id: UUID
    event_type: EventType
    description: str
    significance: EventSignificance = EventSignificance.minor
    is_player_visible: bool = True
    involved_character_ids: list[UUID] = Field(default_factory=list)
    occurred_at: datetime


class SessionSummary(BaseModel):
    session_id: UUID
    perspective: Perspective
    content: str
    generated_at: datetime


class CreateSessionRequest(BaseModel):
    title: str | None = None
    notes: str | None = None


class CreateSessionEventRequest(BaseModel):
    event_type: EventType
    description: str
    significance: EventSignificance | None = None
    is_player_visible: bool | None = None
    involved_character_ids: list[UUID] | None = None
