from __future__ import annotations

from pathlib import Path
from uuid import UUID

from fastapi import APIRouter, HTTPException, Request
from pydantic import BaseModel

from guide.llm.client import CompletionRequest, LlmTask, Message
from guide.models.shared import Perspective
from guide.pdf.pipeline import query_indexes

router = APIRouter()

MAX_MESSAGE_CHARS = 4_000


class ChatRequest(BaseModel):
    message: str
    perspective: Perspective | None = None
    context_limit: int | None = None


class ChatResponse(BaseModel):
    answer: str
    context_chunks_used: int
    model: str
    provider: str


@router.post("/campaigns/{campaign_id}/chat")
async def chat(campaign_id: UUID, body: ChatRequest, request: Request):
    if not body.message:
        raise HTTPException(status_code=400, detail="message must not be empty")
    if len(body.message) > MAX_MESSAGE_CHARS:
        raise HTTPException(
            status_code=413,
            detail=f"message exceeds maximum length of {MAX_MESSAGE_CHARS} characters",
        )

    perspective = body.perspective or Perspective.dm
    context_limit = body.context_limit or 5
    llm = request.app.state.guide.llm

    # PageIndex RAG: query indexes for this campaign
    index_scope = str(campaign_id)
    doc_ids = _list_doc_ids(index_scope)
    global_doc_ids = _list_doc_ids("global")

    chunks = query_indexes(
        scopes=[index_scope] * len(doc_ids) + ["global"] * len(global_doc_ids),
        doc_ids=doc_ids + global_doc_ids,
        query=body.message,
        player_visible_only=(perspective == Perspective.player),
        limit=context_limit,
    )

    context = _build_context(perspective, chunks)

    try:
        resp = await llm.complete(CompletionRequest(
            task=LlmTask.campaign_assistant,
            messages=[
                Message(role="system", content=context),
                Message(role="user", content=body.message),
            ],
            temperature=0.7,
            max_tokens=1024,
        ))
    except Exception as e:
        raise HTTPException(status_code=503, detail=f"LLM unavailable: {e}")

    return ChatResponse(
        answer=resp.content,
        context_chunks_used=len(chunks),
        model=resp.model,
        provider=resp.provider,
    ).model_dump()


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


def _build_context(perspective: Perspective, chunks: list[dict]) -> str:
    role_instruction = (
        "You are The Guide, an AI assistant for a Dungeon Master running a D&D campaign. "
        "You have access to full campaign lore including DM-only information. "
        "Be concise, accurate, and helpful."
        if perspective == Perspective.dm
        else
        "You are The Guide, an AI assistant for players in a D&D campaign. "
        "You MUST NOT reveal DM-only information, secret plot points, or unrevealed lore. "
        "Only share what the players have discovered in-game."
    )

    if not chunks:
        return f"{role_instruction}\n\nNo campaign-specific lore is available yet."

    context_block = "\n\n".join(
        f"[{i+1}] {c.get('section_path', '')}\n{c['content']}"
        for i, c in enumerate(chunks)
    )
    return (
        f"{role_instruction}\n\n"
        "## Campaign Context\n"
        f"{context_block}"
    )
