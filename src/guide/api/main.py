from __future__ import annotations

import logging
import time
from collections import OrderedDict
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse

from guide.api.routes import (
    campaigns,
    characters,
    chat,
    documents,
    encounters,
    generate,
    health,
    sessions,
)
from guide.api.state import AppState
from guide.config import AppConfig
from guide.db.pool import close_db, init_db
from guide.hardware import detect_device, log_hardware_summary, resolve_num_threads
from guide.llm.router import LlmRouter

logger = logging.getLogger(__name__)


@asynccontextmanager
async def lifespan(app: FastAPI):
    config: AppConfig = app.state.guide_config  # type: ignore[attr-defined]

    device = detect_device(config.device)
    num_threads = resolve_num_threads(config.num_threads)
    log_hardware_summary(device, num_threads)

    # Store resolved values back into config so routes can read them without
    # re-running detection on every request.
    config.device = device
    config.num_threads = num_threads

    db = await init_db(config.database_url)
    llm = LlmRouter.from_config(config)

    from qdrant_client import AsyncQdrantClient
    from guide.pdf.pipeline import ensure_collection

    qdrant_client = AsyncQdrantClient(url=config.qdrant_url)
    try:
        await ensure_collection(qdrant_client, config.qdrant_collection, config.embedding_dims)
        logger.info("Qdrant ready — url=%s collection=%s", config.qdrant_url, config.qdrant_collection)
    except Exception as exc:
        logger.warning("Qdrant unavailable (%s) — vector retrieval disabled", exc)
        qdrant_client = None

    app.state.guide = AppState(config=config, llm=llm, db=db, qdrant=qdrant_client)
    logger.info("The Guide started — db=%s model=%s", config.database_url, config.default_model)

    yield

    if qdrant_client is not None:
        await qdrant_client.close()
    await close_db()
    logger.info("The Guide stopped")


def create_app(config: AppConfig | None = None) -> FastAPI:
    cfg = config or AppConfig()

    app = FastAPI(title="The Guide", version="0.1.0", lifespan=lifespan)
    app.state.guide_config = cfg  # passed into lifespan

    # Rate limiting — per-IP token bucket; disabled when limit == 0
    _MAX_RATE_BUCKETS = 10_000
    _rate_buckets: OrderedDict[str, list[float]] = OrderedDict()

    @app.middleware("http")
    async def _rate_limit(request: Request, call_next):
        limit = cfg.max_requests_per_minute
        if limit > 0:
            forwarded_for = request.headers.get("x-forwarded-for")
            if forwarded_for:
                ip = forwarded_for.split(",")[0].strip()
            else:
                ip = request.client.host if request.client else "unknown"
            now = time.time()
            bucket = _rate_buckets.get(ip, [])
            _rate_buckets[ip] = [t for t in bucket if now - t < 60.0]
            _rate_buckets.move_to_end(ip)
            if len(_rate_buckets[ip]) >= limit:
                return JSONResponse(
                    status_code=429,
                    content={"error": "Rate limit exceeded. Please slow down."},
                )
            _rate_buckets[ip].append(now)
            while len(_rate_buckets) > _MAX_RATE_BUCKETS:
                _rate_buckets.popitem(last=False)
        return await call_next(request)

    # Exception handlers
    @app.exception_handler(Exception)
    async def generic_handler(request: Request, exc: Exception) -> JSONResponse:
        logger.exception("Unhandled exception")
        return JSONResponse(status_code=500, content={"detail": "Internal server error"})

    # Register routers
    app.include_router(health.router)
    app.include_router(campaigns.router)
    app.include_router(characters.router)
    app.include_router(sessions.router)
    app.include_router(encounters.router)
    app.include_router(generate.router)
    app.include_router(chat.router)
    app.include_router(documents.router)

    return app


# Entry point for `uvicorn guide.api.main:app`
app = create_app()
