from __future__ import annotations

from uuid import UUID

from fastapi import APIRouter, HTTPException, Request, Response

from guide.combat.engine import CombatEngine, build_participant
from guide.combat.initiative import roll_d20
from guide.db.characters import CharacterRepository
from guide.db.encounters import EncounterRepository
from guide.errors import InvalidInputError, NotFoundError
from guide.models.character import AbilityScores
from guide.models.encounter import (
    CreateEncounterRequest,
    EncounterSummary,
    UpdateParticipantRequest,
)

router = APIRouter()


def _db(r: Request):
    return r.app.state.guide.db


@router.get("/campaigns/{campaign_id}/sessions/{session_id}/encounters")
async def list_encounters(campaign_id: UUID, session_id: UUID, request: Request):
    repo = EncounterRepository(_db(request))
    encounters = await repo.list_by_session(session_id)
    return [e.model_dump(mode="json") for e in encounters]


@router.post("/campaigns/{campaign_id}/sessions/{session_id}/encounters", status_code=201)
async def create_encounter(
    campaign_id: UUID, session_id: UUID, body: CreateEncounterRequest, request: Request,
    response: Response,
):
    enc_repo = EncounterRepository(_db(request))
    char_repo = CharacterRepository(_db(request))

    characters = []
    missing = []
    wrong_campaign = []
    for cid in body.participant_character_ids:
        try:
            char = await char_repo.get_by_id(cid)
            if char.campaign_id != campaign_id:
                wrong_campaign.append(str(cid))
            else:
                characters.append(char)
        except NotFoundError:
            missing.append(str(cid))

    if missing:
        raise HTTPException(status_code=400, detail=f"Unknown character IDs: {', '.join(missing)}")
    if wrong_campaign:
        raise HTTPException(status_code=400, detail=f"Characters do not belong to this campaign: {', '.join(wrong_campaign)}")

    encounter = await enc_repo.create(campaign_id, body)

    for character in characters:
        dex_mod = AbilityScores.modifier(character.ability_scores.dexterity)
        roll = roll_d20()
        participant = build_participant(
            character_id=character.id,
            encounter_id=encounter.id,
            name=character.name,
            initiative_roll=roll,
            initiative_modifier=dex_mod,
            max_hp=character.max_hp,
            current_hp=character.current_hp,
            armor_class=character.armor_class,
            speed=character.speed,
        )
        await enc_repo.add_participant(participant)

    encounter = await enc_repo.get_by_id(encounter.id)
    response.headers["Location"] = (
        f"/campaigns/{campaign_id}/sessions/{session_id}/encounters/{encounter.id}"
    )
    return encounter.model_dump(mode="json")


@router.get("/campaigns/{campaign_id}/encounters/{enc_id}")
async def get_encounter(campaign_id: UUID, enc_id: UUID, request: Request):
    repo = EncounterRepository(_db(request))
    try:
        enc = await repo.get_by_id(enc_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    if enc.campaign_id != campaign_id:
        raise HTTPException(status_code=404, detail="Encounter not found")
    return enc.model_dump(mode="json")


@router.post("/campaigns/{campaign_id}/encounters/{enc_id}/start")
async def start_encounter(campaign_id: UUID, enc_id: UUID, request: Request):
    repo = EncounterRepository(_db(request))
    try:
        encounter = await repo.get_by_id(enc_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    if encounter.campaign_id != campaign_id:
        raise HTTPException(status_code=404, detail="Encounter not found")

    engine = CombatEngine(encounter)
    try:
        engine.start()
    except InvalidInputError as e:
        raise HTTPException(status_code=409, detail=str(e))

    await repo.save_state(engine.encounter)
    return EncounterSummary(
        encounter=engine.encounter,
        current_participant=engine.current_participant(),
        round=engine.encounter.round,
    ).model_dump(mode="json")


@router.post("/campaigns/{campaign_id}/encounters/{enc_id}/next-turn")
async def next_turn(campaign_id: UUID, enc_id: UUID, request: Request):
    repo = EncounterRepository(_db(request))
    try:
        encounter = await repo.get_by_id(enc_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    if encounter.campaign_id != campaign_id:
        raise HTTPException(status_code=404, detail="Encounter not found")

    engine = CombatEngine(encounter)
    try:
        next_participant = engine.next_turn()
    except InvalidInputError as e:
        raise HTTPException(status_code=409, detail=str(e))

    await repo.save_state(engine.encounter)
    return EncounterSummary(
        encounter=engine.encounter,
        current_participant=next_participant,
        round=engine.encounter.round,
    ).model_dump(mode="json")


@router.put("/campaigns/{campaign_id}/encounters/{enc_id}/participants/{char_id}")
async def update_participant(
    campaign_id: UUID, enc_id: UUID, char_id: UUID, body: UpdateParticipantRequest, request: Request
):
    repo = EncounterRepository(_db(request))
    try:
        encounter = await repo.get_by_id(enc_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    if encounter.campaign_id != campaign_id:
        raise HTTPException(status_code=404, detail="Encounter not found")

    engine = CombatEngine(encounter)

    # Locate participant by character_id
    participant = next(
        (p for p in engine.encounter.participants if p.character_id == char_id), None
    )
    if participant is None:
        raise HTTPException(status_code=404, detail="Participant not found in encounter")
    pid = participant.id

    if body.hp_delta is not None:
        engine.apply_hp_change(pid, body.hp_delta)
    if body.set_hp is not None:
        engine.set_hp(pid, body.set_hp)
    if body.add_condition is not None:
        engine.add_condition(pid, body.add_condition)
    if body.remove_condition is not None:
        engine.remove_condition(pid, body.remove_condition)

    # Action budget
    p = next(p for p in engine.encounter.participants if p.id == pid)
    if body.spend_action:
        p.action_budget.has_action = False
    if body.spend_bonus_action:
        p.action_budget.has_bonus_action = False
    if body.spend_reaction:
        p.action_budget.has_reaction = False
    if body.spend_movement is not None:
        p.action_budget.movement_remaining = max(
            0, p.action_budget.movement_remaining - body.spend_movement
        )

    await repo.save_state(engine.encounter)
    updated = next(p for p in engine.encounter.participants if p.id == pid)
    return updated.model_dump(mode="json")


@router.post("/campaigns/{campaign_id}/encounters/{enc_id}/end")
async def end_encounter(campaign_id: UUID, enc_id: UUID, request: Request):
    repo = EncounterRepository(_db(request))
    try:
        encounter = await repo.get_by_id(enc_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    if encounter.campaign_id != campaign_id:
        raise HTTPException(status_code=404, detail="Encounter not found")

    engine = CombatEngine(encounter)
    try:
        engine.end()
    except InvalidInputError as e:
        raise HTTPException(status_code=409, detail=str(e))

    await repo.save_state(engine.encounter)
    return engine.encounter.model_dump(mode="json")
