from __future__ import annotations

import logging
from enum import Enum

from guide.config import AppConfig
from guide.errors import LlmError

from .client import (
    CompletionRequest,
    CompletionResponse,
    EmbeddingRequest,
    LlmClient,
    LlmTask,
    VisionRequest,
)
from .cloud import CloudProvider
from .ollama import OllamaProvider

logger = logging.getLogger(__name__)


class RoutingStrategy(str, Enum):
    always_local = "always_local"
    local_with_fallback = "local_with_fallback"
    always_cloud = "always_cloud"


_LOCAL_ONLY_TASKS = {
    LlmTask.ocr_extraction,
    LlmTask.vision_description,
    LlmTask.embedding_generation,
}


class LlmRouter(LlmClient):
    def __init__(
        self,
        strategy: RoutingStrategy,
        local: LlmClient,
        cloud: LlmClient | None = None,
    ) -> None:
        self._strategy = strategy
        self._local = local
        self._cloud = cloud

    @classmethod
    def from_config(cls, config: AppConfig) -> "LlmRouter":
        local = OllamaProvider(
            base_url=config.ollama_base_url,
            default_model=config.default_model,
            ocr_model=config.ocr_model,
            vision_model=config.vision_model,
            embedding_model=config.embedding_model,
        )

        if config.cloud_api_key and config.cloud_fallback:
            base_url: str | None = None
            model: str
            label: str

            if config.cloud_fallback == "openai":
                model, label = "gpt-4o", "openai"
            elif config.cloud_fallback == "gemini":
                base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
                model, label = "gemini-1.5-flash", "gemini"
            else:
                logger.warning("Unknown cloud_fallback '%s', using always_local", config.cloud_fallback)
                return cls(RoutingStrategy.always_local, local)

            cloud = CloudProvider(
                api_key=config.cloud_api_key,
                model=model,
                base_url=base_url,
                label=label,
            )
            return cls(RoutingStrategy.local_with_fallback, local, cloud)

        return cls(RoutingStrategy.always_local, local)

    def _select_provider(self, task: LlmTask) -> LlmClient:
        if task in _LOCAL_ONLY_TASKS:
            return self._local

        if self._strategy == RoutingStrategy.always_cloud and self._cloud:
            return self._cloud
        return self._local

    async def complete(self, req: CompletionRequest) -> CompletionResponse:
        provider = self._select_provider(req.task)
        try:
            return await provider.complete(req)
        except Exception as local_err:
            if self._strategy == RoutingStrategy.local_with_fallback and self._cloud:
                logger.warning("Local LLM failed (%s), falling back to cloud", local_err)
                return await self._cloud.complete(req)
            raise

    async def embed(self, req: EmbeddingRequest) -> list[float]:
        return await self._local.embed(req)

    async def complete_with_vision(self, req: VisionRequest) -> CompletionResponse:
        return await self._local.complete_with_vision(req)

    @property
    def provider_name(self) -> str:
        return "router"
