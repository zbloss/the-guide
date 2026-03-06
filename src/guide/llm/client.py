from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum


class LlmTask(str, Enum):
    ocr_extraction = "ocr_extraction"
    vision_description = "vision_description"
    embedding_generation = "embedding_generation"
    campaign_assistant = "campaign_assistant"
    backstory_analysis = "backstory_analysis"
    encounter_generation = "encounter_generation"
    session_summary = "session_summary"
    general = "general"


@dataclass
class Message:
    role: str  # "system" | "user" | "assistant"
    content: str


@dataclass
class CompletionRequest:
    task: LlmTask
    messages: list[Message]
    model_override: str | None = None
    temperature: float | None = None
    max_tokens: int | None = None
    think: bool | None = None  # False disables chain-of-thought on models that support it


@dataclass
class CompletionResponse:
    content: str
    model: str
    provider: str
    prompt_tokens: int = 0
    completion_tokens: int = 0


@dataclass
class EmbeddingRequest:
    text: str
    model_override: str | None = None


@dataclass
class VisionRequest:
    task: LlmTask
    prompt: str
    image_bytes: bytes
    image_mime_type: str
    model_override: str | None = None


class LlmClient(ABC):
    @abstractmethod
    async def complete(self, req: CompletionRequest) -> CompletionResponse: ...

    @abstractmethod
    async def embed(self, req: EmbeddingRequest) -> list[float]: ...

    @abstractmethod
    async def complete_with_vision(self, req: VisionRequest) -> CompletionResponse: ...

    @property
    @abstractmethod
    def provider_name(self) -> str: ...
