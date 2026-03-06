from __future__ import annotations

import json
from pathlib import Path
from uuid import UUID

from fastapi import APIRouter, HTTPException, Request
from pydantic import BaseModel

from guide.db.characters import CharacterRepository
from guide.llm.client import CompletionRequest, LlmTask, Message
from guide.models.playstyle import (
    EnemySuggestion,
    GeneratedEncounter,
    GeneratedEncounterType,
    PlaystyleProfile,
)
from guide.models.shared import CharacterType
from guide.pdf.pipeline import query_indexes

router = APIRouter()


class GenerateEncounterRequest(BaseModel):
    context: str | None = None
    preferred_type: GeneratedEncounterType | None = None
    party_level_override: int | None = None


@router.post("/campaigns/{campaign_id}/encounters/generate")
async def generate_encounter(campaign_id: UUID, body: GenerateEncounterRequest, request: Request):
    char_repo = CharacterRepository(request.app.state.guide.db)
    characters = await char_repo.list_by_campaign(campaign_id)

    pcs = [c for c in characters if c.character_type == CharacterType.pc and c.is_alive]
    if not pcs:
        raise HTTPException(
            status_code=422,
            detail="No living PCs in campaign — add characters before generating an encounter",
        )

    party_level = body.party_level_override or (sum(c.level for c in pcs) // len(pcs))

    party_summary = "\n".join(
        f"- {c.name} ({c.class_ or 'Unknown class'}"
        + (f", {c.race}" if c.race else "")
        + f", Lv{c.level})"
        for c in pcs
    )

    # Retrieve lore context via PageIndex
    context_query = body.context or f"encounter for party level {party_level}"
    scope = str(campaign_id)
    doc_ids = _list_doc_ids(scope)
    global_doc_ids = _list_doc_ids("global")
    chunks = query_indexes(
        scopes=[scope] * len(doc_ids) + ["global"] * len(global_doc_ids),
        doc_ids=doc_ids + global_doc_ids,
        query=context_query,
        limit=5,
    )

    profile = PlaystyleProfile.default_for(campaign_id)
    type_preference = body.preferred_type.value if body.preferred_type else _infer_type(profile)

    lore_section = (
        "\n\n## Relevant Campaign Lore\n" + "\n\n".join(c["content"] for c in chunks)
        if chunks
        else ""
    )

    system_prompt = (
        f"You are a D&D encounter designer. Generate a contextually relevant encounter for the party.\n"
        "Return ONLY valid JSON (no markdown, no explanation) matching this schema:\n"
        "{\n"
        '  "title": "<encounter title>",\n'
        '  "description": "<2-3 sentence atmospheric description>",\n'
        '  "encounter_type": "combat|social|exploration|puzzle|mixed",\n'
        '  "challenge_rating": <number|null>,\n'
        '  "suggested_enemies": [{"name": "<creature>", "count": <n>, "cr": <number|null>}],\n'
        '  "narrative_hook": "<1 sentence connecting to campaign narrative>",\n'
        '  "alternative": "<optional social/non-combat alternative>"\n'
        "}\n"
        f"Guidelines:\n"
        f"- Scale appropriately for a level {party_level} party\n"
        f"- Lean toward encounter type: {type_preference}\n"
        "- Use campaign lore if provided to ground the encounter in the world"
    )

    user_message = (
        f"Party composition (average level {party_level}):\n{party_summary}"
        f"{lore_section}" + (f"\n\nAdditional context: {body.context}" if body.context else "")
    )

    llm = request.app.state.guide.llm
    try:
        resp = await llm.complete(
            CompletionRequest(
                task=LlmTask.encounter_generation,
                messages=[
                    Message(role="system", content=system_prompt),
                    Message(role="user", content=user_message),
                ],
                temperature=0.8,
                max_tokens=800,
            )
        )
    except Exception as e:
        raise HTTPException(status_code=503, detail=f"LLM unavailable: {e}")

    try:
        raw = json.loads(resp.content.strip())
    except json.JSONDecodeError as e:
        raise HTTPException(status_code=500, detail=f"LLM response was not valid JSON: {e}")

    enc_type_map = {
        "social": GeneratedEncounterType.social,
        "exploration": GeneratedEncounterType.exploration,
        "puzzle": GeneratedEncounterType.puzzle,
        "mixed": GeneratedEncounterType.mixed,
    }
    generated = GeneratedEncounter(
        title=raw["title"],
        description=raw["description"],
        encounter_type=enc_type_map.get(
            raw.get("encounter_type", ""), GeneratedEncounterType.combat
        ),
        challenge_rating=raw.get("challenge_rating"),
        suggested_enemies=[
            EnemySuggestion(name=e["name"], count=e.get("count", 1), cr=e.get("cr"))
            for e in raw.get("suggested_enemies", [])
        ],
        narrative_hook=raw["narrative_hook"],
        alternative=raw.get("alternative"),
    )
    return generated.model_dump(mode="json")


def _list_doc_ids(scope: str) -> list[UUID]:
    index_dir = Path("data/indexes") / scope
    if not index_dir.exists():
        return []
    ids = []
    for f in index_dir.glob("*.json"):
        try:
            ids.append(UUID(f.stem))
        except ValueError:
            pass
    return ids


def _infer_type(profile: PlaystyleProfile) -> str:
    if (
        profile.combat_affinity >= profile.social_affinity
        and profile.combat_affinity >= profile.exploration_affinity
    ):
        return "combat"
    if profile.social_affinity >= profile.exploration_affinity:
        return "social"
    return "exploration"
