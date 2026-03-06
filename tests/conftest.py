from __future__ import annotations

import asyncio
from typing import AsyncGenerator

import aiosqlite
import pytest
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
        return CompletionResponse(
            content='{"motivations":[],"key_relationships":[],"secrets":[],"plot_hooks":[]}',
            model="mock",
            provider="mock",
        )

    async def embed(self, req):
        return [0.0] * 768

    async def complete_with_vision(self, req):
        from guide.llm.client import CompletionResponse
        return CompletionResponse(content="mock vision", model="mock", provider="mock")


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

    async with AsyncClient(
        transport=ASGITransport(app=application), base_url="http://test"
    ) as ac:
        yield ac
