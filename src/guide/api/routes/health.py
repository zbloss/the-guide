from fastapi import APIRouter
from fastapi.responses import JSONResponse

router = APIRouter()


@router.get("/health")
async def health() -> JSONResponse:
    return JSONResponse({"status": "ok"})


@router.get("/version")
async def version() -> JSONResponse:
    return JSONResponse({"version": "0.1.0", "name": "the-guide"})
