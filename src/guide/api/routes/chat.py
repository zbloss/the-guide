from __future__ import annotations

from uuid import UUID

from fastapi import APIRouter, HTTPException, Request
from pydantic import BaseModel

from guide.llm.client import CompletionRequest, LlmTask, Message
from guide.models.shared import Perspective
from guide.pdf.pipeline import query_indexes, select_relevant_docs

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

    async def _call_llm(prompt: str) -> str:
        resp = await llm.complete(
            CompletionRequest(
                task=LlmTask.campaign_assistant,
                messages=[Message(role="user", content=prompt)],
                temperature=0,
                think=False,
            )
        )
        return resp.content

    # PageIndex RAG: select relevant docs then query their indexes
    selected = await select_relevant_docs(campaign_id, body.message, _call_llm)
    scopes = [s for s, _ in selected]
    doc_ids = [d for _, d in selected]

    chunks = await query_indexes(
        scopes=scopes,
        doc_ids=doc_ids,
        query=body.message,
        call_llm=_call_llm,
        player_visible_only=(perspective == Perspective.player),
        limit=context_limit,
    )

    context = _build_context(perspective, chunks)

    try:
        resp = await llm.complete(
            CompletionRequest(
                task=LlmTask.campaign_assistant,
                messages=[
                    Message(role="system", content=context),
                    Message(role="user", content=body.message),
                ],
                temperature=0.7,
                max_tokens=1024,
            )
        )
    except Exception as e:
        raise HTTPException(status_code=503, detail=f"LLM unavailable: {e}")

    return ChatResponse(
        answer=resp.content,
        context_chunks_used=len(chunks),
        model=resp.model,
        provider=resp.provider,
    ).model_dump()


def _build_context(perspective: Perspective, chunks: list[dict]) -> str:
    role_instruction = (
        "You are The Guide, an AI assistant for a Dungeon Master running a D&D campaign. "
        "You have access to full campaign lore including DM-only information. "
        "Be concise, accurate, and helpful."
        if perspective == Perspective.dm
        else "You are The Guide, an AI assistant for players in a D&D campaign. "
        "You MUST NOT reveal DM-only information, secret plot points, or unrevealed lore. "
        "Only share what the players have discovered in-game."
    )

    if not chunks:
        return f"{role_instruction}\n\nNo campaign-specific lore is available yet."

    context_block = "\n\n".join(
        f"[{i + 1}] {c.get('section_path', '')}\n{c['content']}" for i, c in enumerate(chunks)
    )
    return f"{role_instruction}\n\n## Campaign Context\n{context_block}"
