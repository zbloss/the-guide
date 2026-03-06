from __future__ import annotations

from dataclasses import dataclass

import aiosqlite

from guide.config import AppConfig
from guide.llm import LlmRouter


@dataclass
class AppState:
    config: AppConfig
    llm: LlmRouter
    db: aiosqlite.Connection
