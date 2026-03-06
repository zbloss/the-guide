from __future__ import annotations

import logging
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

    app.state.guide = AppState(config=config, llm=llm, db=db)
    logger.info("The Guide started — db=%s model=%s", config.database_url, config.default_model)

    yield

    await close_db()
    logger.info("The Guide stopped")


def create_app(config: AppConfig | None = None) -> FastAPI:
    cfg = config or AppConfig()

    app = FastAPI(title="The Guide", version="0.1.0", lifespan=lifespan)
    app.state.guide_config = cfg  # passed into lifespan

    # Exception handlers
    @app.exception_handler(Exception)
    async def generic_handler(request: Request, exc: Exception) -> JSONResponse:
        logger.exception("Unhandled error: %s", exc)
        return JSONResponse(status_code=500, content={"error": str(exc)})

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
