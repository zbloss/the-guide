from __future__ import annotations

from dataclasses import dataclass, field

import aiosqlite

from guide.config import AppConfig
from guide.llm import LlmRouter


@dataclass
class AppState:
    config: AppConfig
    llm: LlmRouter
    db: aiosqlite.Connection
    qdrant: object | None = field(default=None)  # AsyncQdrantClient | None
