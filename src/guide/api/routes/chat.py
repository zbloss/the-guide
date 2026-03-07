from __future__ import annotations

import json
import time
from uuid import UUID

from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import StreamingResponse
from pydantic import BaseModel

from guide.llm.client import CompletionRequest, EmbeddingRequest, LlmTask, Message
from guide.models.shared import Perspective
from guide.pdf.pipeline import query_indexes, select_relevant_docs

router = APIRouter()

MAX_MESSAGE_CHARS = 4_000


class ChatRequest(BaseModel):
    message: str
    perspective: Perspective | None = None
    context_limit: int | None = None



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

    async def _embed(text: str) -> list[float]:
        return await llm.embed(EmbeddingRequest(text=text))

    qdrant = request.app.state.guide.qdrant
    collection = request.app.state.guide.config.qdrant_collection

    if qdrant is not None:
        # Qdrant path: single vector search across campaign + global chunks — no doc pre-selection
        chunks = await query_indexes(
            scopes=[],
            doc_ids=[],
            query=body.message,
            call_llm=_call_llm,
            player_visible_only=(perspective == Perspective.player),
            limit=context_limit,
            embed=_embed,
            qdrant=qdrant,
            collection=collection,
            campaign_id=str(campaign_id),
        )
    else:
        # LLM fallback: pre-select docs then query their PageIndex trees
        selected = await select_relevant_docs(campaign_id, body.message, _call_llm)
        chunks = await query_indexes(
            scopes=[s for s, _ in selected],
            doc_ids=[d for _, d in selected],
            query=body.message,
            call_llm=_call_llm,
            player_visible_only=(perspective == Perspective.player),
            limit=context_limit,
        )

    context = _build_context(perspective, chunks)
    model = llm.model_for_task(LlmTask.campaign_assistant)
    provider = llm.provider_name

    async def _token_stream():
        total_chars = 0
        t_start = time.perf_counter()

        try:
            stream_req = CompletionRequest(
                task=LlmTask.campaign_assistant,
                messages=[
                    Message(role="system", content=context),
                    Message(role="user", content=body.message),
                ],
                temperature=0.7,
            )
            async for chunk in llm.complete_stream(stream_req):
                total_chars += len(chunk)
                yield f"data: {json.dumps({'type': 'token', 'content': chunk})}\n\n"
        except Exception as e:
            yield f"data: {json.dumps({'type': 'error', 'detail': str(e)})}\n\n"
            return

        elapsed = time.perf_counter() - t_start
        approx_tokens = max(total_chars / 4, 1)
        tokens_per_second = round(approx_tokens / elapsed, 1) if elapsed > 0 else 0.0

        yield f"data: {json.dumps({'type': 'done', 'model': model, 'provider': provider, 'context_chunks_used': len(chunks), 'tokens_per_second': tokens_per_second})}\n\n"

    return StreamingResponse(
        _token_stream(),
        media_type="text/event-stream",
        headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"},
    )


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
