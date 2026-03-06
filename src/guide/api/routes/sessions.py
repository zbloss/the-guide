from __future__ import annotations

from datetime import datetime, timezone
from uuid import UUID

from fastapi import APIRouter, HTTPException, Query, Request

from guide.db.sessions import SessionEventRepository, SessionRepository
from guide.errors import NotFoundError
from guide.llm.client import CompletionRequest, LlmTask, Message
from guide.llm import prompts
from guide.models.session import (
    CreateSessionEventRequest,
    CreateSessionRequest,
    SessionSummary,
)
from guide.models.shared import Perspective

router = APIRouter()


def _db(r: Request):
    return r.app.state.guide.db


def _llm(r: Request):
    return r.app.state.guide.llm


@router.get("/campaigns/{campaign_id}/sessions")
async def list_sessions(campaign_id: UUID, request: Request):
    repo = SessionRepository(_db(request))
    sessions = await repo.list_by_campaign(campaign_id)
    return [s.model_dump(mode="json") for s in sessions]


@router.post("/campaigns/{campaign_id}/sessions", status_code=201)
async def create_session(campaign_id: UUID, body: CreateSessionRequest, request: Request):
    repo = SessionRepository(_db(request))
    session = await repo.create(campaign_id, body)
    return session.model_dump(mode="json")


@router.get("/campaigns/{campaign_id}/sessions/{session_id}")
async def get_session(campaign_id: UUID, session_id: UUID, request: Request):
    repo = SessionRepository(_db(request))
    try:
        session = await repo.get_by_id(session_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    return session.model_dump(mode="json")


@router.delete("/campaigns/{campaign_id}/sessions/{session_id}", status_code=204)
async def delete_session(campaign_id: UUID, session_id: UUID, request: Request):
    repo = SessionRepository(_db(request))
    try:
        await repo.delete(session_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))


@router.post("/campaigns/{campaign_id}/sessions/{session_id}/start")
async def start_session(campaign_id: UUID, session_id: UUID, request: Request):
    repo = SessionRepository(_db(request))
    try:
        session = await repo.start_session(session_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    return session.model_dump(mode="json")


@router.post("/campaigns/{campaign_id}/sessions/{session_id}/end")
async def end_session(campaign_id: UUID, session_id: UUID, request: Request):
    repo = SessionRepository(_db(request))
    try:
        session = await repo.end_session(session_id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    return session.model_dump(mode="json")


@router.get("/campaigns/{campaign_id}/sessions/{session_id}/events")
async def list_events(campaign_id: UUID, session_id: UUID, request: Request):
    repo = SessionEventRepository(_db(request))
    events = await repo.list_by_session(session_id)
    return [e.model_dump(mode="json") for e in events]


@router.post("/campaigns/{campaign_id}/sessions/{session_id}/events", status_code=201)
async def create_event(
    campaign_id: UUID, session_id: UUID, body: CreateSessionEventRequest, request: Request
):
    repo = SessionEventRepository(_db(request))
    event = await repo.create(session_id, campaign_id, body)
    return event.model_dump(mode="json")


@router.get("/campaigns/{campaign_id}/sessions/{session_id}/summary")
async def session_summary(
    campaign_id: UUID,
    session_id: UUID,
    request: Request,
    perspective: str = Query("dm"),
):
    persp = Perspective.player if perspective in ("player", "players") else Perspective.dm

    event_repo = SessionEventRepository(_db(request))
    if persp == Perspective.player:
        events = await event_repo.list_visible_by_session(session_id)
    else:
        events = await event_repo.list_by_session(session_id)

    if not events:
        raise HTTPException(status_code=422, detail="No events recorded for this session yet")

    event_list = "\n".join(
        f"{i+1}. [{e.event_type.value}] {e.description} (significance: {e.significance.value})"
        for i, e in enumerate(events)
    )

    system_prompt = (
        prompts.session_summary_dm_system() if persp == Perspective.dm
        else prompts.session_summary_player_system()
    )

    llm = _llm(request)
    try:
        resp = await llm.complete(CompletionRequest(
            task=LlmTask.session_summary,
            messages=[
                Message(role="system", content=system_prompt),
                Message(role="user", content=f"Session events:\n\n{event_list}"),
            ],
            temperature=0.7,
            max_tokens=1500,
        ))
    except Exception as e:
        raise HTTPException(status_code=503, detail=f"LLM unavailable: {e}")

    return SessionSummary(
        session_id=session_id,
        perspective=persp,
        content=resp.content,
        generated_at=datetime.now(timezone.utc),
    ).model_dump(mode="json")
