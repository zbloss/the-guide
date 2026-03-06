from __future__ import annotations

import openai

from guide.errors import LlmError

from .client import (
    CompletionRequest,
    CompletionResponse,
    EmbeddingRequest,
    LlmClient,
    VisionRequest,
)


class CloudProvider(LlmClient):
    """OpenAI or Gemini cloud provider via OpenAI-compatible API."""

    def __init__(
        self,
        api_key: str,
        model: str,
        base_url: str | None,
        label: str,
    ) -> None:
        self._client = openai.AsyncOpenAI(
            api_key=api_key,
            base_url=base_url,
        )
        self._model = model
        self._label = label

    async def complete(self, req: CompletionRequest) -> CompletionResponse:
        model = req.model_override or self._model
        messages = [{"role": m.role, "content": m.content} for m in req.messages]

        kwargs: dict = {"model": model, "messages": messages}
        if req.temperature is not None:
            kwargs["temperature"] = req.temperature
        if req.max_tokens is not None:
            kwargs["max_tokens"] = req.max_tokens

        response = await self._client.chat.completions.create(**kwargs)
        choice = response.choices[0] if response.choices else None
        if choice is None:
            raise LlmError("No choices returned from cloud LLM")

        content = choice.message.content or ""
        prompt_tokens = response.usage.prompt_tokens if response.usage else 0
        completion_tokens = response.usage.completion_tokens if response.usage else 0

        return CompletionResponse(
            content=content,
            model=model,
            provider=self._label,
            prompt_tokens=prompt_tokens,
            completion_tokens=completion_tokens,
        )

    async def embed(self, req: EmbeddingRequest) -> list[float]:
        raise LlmError("Cloud provider does not support embeddings in this config")

    async def complete_with_vision(self, req: VisionRequest) -> CompletionResponse:
        raise LlmError("Vision not supported by cloud fallback in this config")

    @property
    def provider_name(self) -> str:
        return self._label
