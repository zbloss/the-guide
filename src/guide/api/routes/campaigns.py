from uuid import UUID

from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import JSONResponse

from guide.db.campaigns import CampaignRepository
from guide.errors import NotFoundError
from guide.models.campaign import CreateCampaignRequest, UpdateCampaignRequest

router = APIRouter()


def _db(request: Request):
    return request.app.state.guide.db


@router.get("/campaigns")
async def list_campaigns(request: Request):
    repo = CampaignRepository(_db(request))
    campaigns = await repo.list()
    return [c.model_dump(mode="json") for c in campaigns]


@router.post("/campaigns", status_code=201)
async def create_campaign(request: Request, body: CreateCampaignRequest):
    repo = CampaignRepository(_db(request))
    campaign = await repo.create(body)
    return campaign.model_dump(mode="json")


@router.get("/campaigns/{id}")
async def get_campaign(id: UUID, request: Request):
    repo = CampaignRepository(_db(request))
    try:
        campaign = await repo.get_by_id(id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    return campaign.model_dump(mode="json")


@router.put("/campaigns/{id}")
async def update_campaign(id: UUID, body: UpdateCampaignRequest, request: Request):
    repo = CampaignRepository(_db(request))
    try:
        campaign = await repo.update(id, body)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    return campaign.model_dump(mode="json")


@router.delete("/campaigns/{id}", status_code=204)
async def delete_campaign(id: UUID, request: Request):
    repo = CampaignRepository(_db(request))
    try:
        await repo.delete(id)
    except NotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
