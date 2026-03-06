from __future__ import annotations

import json
from uuid import UUID, uuid4

from fastapi import APIRouter, HTTPException, Request
from pydantic import BaseModel

from guide.db.characters import CharacterRepository
from guide.errors import NotFoundError
from guide.llm import prompts
from guide.llm.client import CompletionRequest, LlmTask, Message
from guide.models.character import Backstory, CreateCharacterRequest, HookPriority, PlotHook

router = APIRouter()


def _db(request: Request):
    return request.app.state.guide.db


def _llm(request: Request):
    return request.app.state.guide.llm


@router.get("/campaigns/{campaign_id}/characters")
async def list_characters(campaign_id: UUID, request: Request):
    repo = CharacterRepository(_db(request))
    chars = await repo.list_by_campaign(campaign_id)
    return [c.model_dump(mode="json") for c in chars]


@router.post("/campaigns/{campaign_id}/characters", status_code=201)
async def create_character(campaign_id: UUID, body: CreateCharacterRequest, request: Request):
    repo = CharacterRepository(_db(request))
    try:
        character = await repo.create(campaign_id, body)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    return character.model_dump(mode="json")


@router.get("/campaigns/{campaign_id}/characters/{char_id}")
async def get_character(campaign_id: UUID, char_id: UUID, request: Request):
    repo = CharacterRepository(_db(request))
    try:
        character = await repo.get_by_id(char_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    return character.model_dump(mode="json")


@router.delete("/campaigns/{campaign_id}/characters/{char_id}", status_code=204)
async def delete_character(campaign_id: UUID, char_id: UUID, request: Request):
    repo = CharacterRepository(_db(request))
    try:
        await repo.delete(char_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))


class AnalyzeBackstoryRequest(BaseModel):
    backstory_text: str | None = None


@router.post("/campaigns/{campaign_id}/characters/{char_id}/analyze-backstory")
async def analyze_backstory(
    campaign_id: UUID, char_id: UUID, body: AnalyzeBackstoryRequest, request: Request
):
    repo = CharacterRepository(_db(request))
    try:
        character = await repo.get_by_id(char_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))

    raw_text = body.backstory_text or (
        character.backstory.raw_text if character.backstory else None
    )
    if not raw_text or not raw_text.strip():
        raise HTTPException(status_code=422, detail="No backstory text provided")

    llm = _llm(request)
    try:
        resp = await llm.complete(
            CompletionRequest(
                task=LlmTask.backstory_analysis,
                messages=[
                    Message(role="system", content=prompts.backstory_analysis_system()),
                    Message(
                        role="user",
                        content=f"Character: {character.name}\n\nBackstory:\n{raw_text}",
                    ),
                ],
                temperature=0.3,
                max_tokens=1024,
            )
        )
    except Exception as e:
        raise HTTPException(status_code=503, detail=f"LLM unavailable: {e}")

    try:
        extracted = json.loads(resp.content.strip())
    except json.JSONDecodeError as e:
        raise HTTPException(status_code=500, detail=f"LLM response was not valid JSON: {e}")

    backstory = Backstory(
        raw_text=raw_text,
        motivations=extracted.get("motivations", []),
        key_relationships=extracted.get("key_relationships", []),
        secrets=extracted.get("secrets", []),
        extracted_hooks=[
            PlotHook(
                id=uuid4(),
                character_id=char_id,
                description=h["description"],
                priority=_parse_priority(h.get("priority", "medium")),
                llm_extracted=True,
            )
            for h in extracted.get("plot_hooks", [])
        ],
    )

    await repo.set_backstory(char_id, backstory)
    return {"character_id": str(char_id), "backstory": backstory.model_dump(mode="json")}


def _parse_priority(s: str) -> HookPriority:
    return {
        "low": HookPriority.low,
        "high": HookPriority.high,
        "critical": HookPriority.critical,
    }.get(s.lower(), HookPriority.medium)
