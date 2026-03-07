from __future__ import annotations

import asyncio
import base64

import httpx
import openai

from guide.errors import LlmError

from .client import (
    CompletionRequest,
    CompletionResponse,
    EmbeddingRequest,
    LlmClient,
    LlmTask,
    VisionRequest,
)


class OllamaProvider(LlmClient):
    def __init__(
        self,
        base_url: str,
        default_model: str,
        ocr_model: str,
        vision_model: str,
        embedding_model: str,
    ) -> None:
        self._client = openai.AsyncOpenAI(
            base_url=base_url,
            api_key="ollama",  # Ollama ignores the key
        )
        self._default_model = default_model
        self._ocr_model = ocr_model
        self._vision_model = vision_model
        self._embedding_model = embedding_model

    def _model_for_task(self, task: LlmTask, override: str | None) -> str:
        if override:
            return override
        match task:
            case LlmTask.ocr_extraction:
                return self._ocr_model
            case LlmTask.vision_description:
                return self._vision_model
            case LlmTask.embedding_generation:
                return self._embedding_model
            case _:
                return self._default_model

    async def complete(self, req: CompletionRequest) -> CompletionResponse:
        model = self._model_for_task(req.task, req.model_override)
        messages = [{"role": m.role, "content": m.content} for m in req.messages]

        kwargs: dict = {"model": model, "messages": messages}
        if req.temperature is not None:
            kwargs["temperature"] = req.temperature
        if req.max_tokens is not None:
            kwargs["max_tokens"] = req.max_tokens
        if req.think is not None:
            kwargs["extra_body"] = {"think": req.think}

        try:
            response = await self._client.chat.completions.create(**kwargs)
        except (httpx.ConnectError, httpx.TimeoutException, asyncio.TimeoutError) as e:
            raise LlmError(f"Ollama connection error: {e}") from e
        except openai.APIError as e:
            raise LlmError(f"Ollama API error: {e}") from e

        choice = response.choices[0] if response.choices else None
        if choice is None:
            raise LlmError("No choices returned from LLM")

        content = choice.message.content or ""
        prompt_tokens = response.usage.prompt_tokens if response.usage else 0
        completion_tokens = response.usage.completion_tokens if response.usage else 0

        return CompletionResponse(
            content=content,
            model=model,
            provider="ollama",
            prompt_tokens=prompt_tokens,
            completion_tokens=completion_tokens,
        )

    async def complete_stream(self, req: CompletionRequest):  # type: ignore[override]
        """Async generator that yields content chunks as they arrive."""
        model = self._model_for_task(req.task, req.model_override)
        messages = [{"role": m.role, "content": m.content} for m in req.messages]

        kwargs: dict = {"model": model, "messages": messages, "stream": True}
        if req.temperature is not None:
            kwargs["temperature"] = req.temperature
        if req.max_tokens is not None:
            kwargs["max_tokens"] = req.max_tokens
        if req.think is not None:
            kwargs["extra_body"] = {"think": req.think}

        try:
            stream = await self._client.chat.completions.create(**kwargs)
            async for chunk in stream:
                if chunk.choices and chunk.choices[0].delta.content:
                    yield chunk.choices[0].delta.content
        except (httpx.ConnectError, httpx.TimeoutException, asyncio.TimeoutError) as e:
            raise LlmError(f"Ollama connection error: {e}") from e
        except openai.APIError as e:
            raise LlmError(f"Ollama API error: {e}") from e

    async def embed(self, req: EmbeddingRequest) -> list[float]:
        model = req.model_override or self._embedding_model
        try:
            response = await self._client.embeddings.create(model=model, input=req.text)
        except (httpx.ConnectError, httpx.TimeoutException, asyncio.TimeoutError) as e:
            raise LlmError(f"Ollama connection error: {e}") from e
        except openai.APIError as e:
            raise LlmError(f"Ollama API error: {e}") from e
        if not response.data:
            raise LlmError("No embedding returned")
        return response.data[0].embedding

    async def complete_with_vision(self, req: VisionRequest) -> CompletionResponse:
        model = self._model_for_task(req.task, req.model_override)
        encoded = base64.b64encode(req.image_bytes).decode("ascii")
        data_url = f"data:{req.image_mime_type};base64,{encoded}"

        messages = [
            {
                "role": "user",
                "content": [
                    {"type": "image_url", "image_url": {"url": data_url}},
                    {"type": "text", "text": req.prompt},
                ],
            }
        ]

        response = await self._client.chat.completions.create(
            model=model,
            messages=messages,
        )
        choice = response.choices[0] if response.choices else None
        if choice is None:
            raise LlmError("No choices returned from vision LLM")

        content = choice.message.content or ""
        prompt_tokens = response.usage.prompt_tokens if response.usage else 0
        completion_tokens = response.usage.completion_tokens if response.usage else 0

        return CompletionResponse(
            content=content,
            model=model,
            provider="ollama",
            prompt_tokens=prompt_tokens,
            completion_tokens=completion_tokens,
        )

    @property
    def provider_name(self) -> str:
        return "ollama"
