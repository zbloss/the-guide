from __future__ import annotations

import json
from typing import AsyncGenerator

import aiosqlite
import pytest_asyncio
from httpx import ASGITransport, AsyncClient

from guide.api.main import create_app
from guide.api.state import AppState
from guide.config import AppConfig
from guide.db.pool import init_db


@pytest_asyncio.fixture
async def db() -> AsyncGenerator[aiosqlite.Connection, None]:
    """In-memory SQLite database with migrations applied."""
    conn = await init_db(":memory:")
    yield conn
    await conn.close()


class _MockLlm:
    """Minimal LLM stub that returns canned responses."""

    provider_name = "mock"

    async def complete(self, req):
        from guide.llm.client import CompletionResponse

        if req.task.value == "encounter_generation":
            content = json.dumps({
                "title": "Test Encounter",
                "description": "A challenging test encounter for the party.",
                "encounter_type": "combat",
                "challenge_rating": 2.0,
                "suggested_enemies": [{"name": "Goblin", "count": 3, "cr": 0.25}],
                "narrative_hook": "The goblins were hired by an unknown patron.",
                "alternative": None,
            })
        else:
            content = '{"motivations":[],"key_relationships":[],"secrets":[],"plot_hooks":[]}'

        return CompletionResponse(content=content, model="mock", provider="mock")

    async def complete_stream(self, req):
        yield "mock response chunk"

    async def embed(self, req):
        return [0.0] * 768

    async def complete_with_vision(self, req):
        from guide.llm.client import CompletionResponse

        return CompletionResponse(content="mock vision", model="mock", provider="mock")

    def model_for_task(self, task):
        return "mock"


@pytest_asyncio.fixture
async def client(db) -> AsyncGenerator[AsyncClient, None]:
    """HTTP test client with in-memory DB and mock LLM."""
    config = AppConfig(database_url=":memory:", default_model="mock")
    application = create_app(config)

    # Override lifespan state directly
    state = AppState(config=config, llm=_MockLlm(), db=db)
    application.state.guide = state

    # Prevent lifespan from re-running
    application.router.lifespan_context = None

    async with AsyncClient(transport=ASGITransport(app=application), base_url="http://test") as ac:
        yield ac


@pytest_asyncio.fixture
async def small_upload_client(db) -> AsyncGenerator[AsyncClient, None]:
    """HTTP test client with a tiny max_upload_bytes for testing 413 responses."""
    config = AppConfig(database_url=":memory:", default_model="mock", max_upload_bytes=10)
    application = create_app(config)

    state = AppState(config=config, llm=_MockLlm(), db=db)
    application.state.guide = state
    application.router.lifespan_context = None

    async with AsyncClient(transport=ASGITransport(app=application), base_url="http://test") as ac:
        yield ac
