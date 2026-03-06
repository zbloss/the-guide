"""HTTP-layer tests via httpx.AsyncClient."""

from __future__ import annotations


async def test_health(client):
    resp = await client.get("/health")
    assert resp.status_code == 200
    assert resp.json()["status"] == "ok"


async def test_version(client):
    resp = await client.get("/version")
    assert resp.status_code == 200
    data = resp.json()
    assert "version" in data


async def test_create_and_get_campaign(client):
    resp = await client.post("/campaigns", json={"name": "Test Campaign"})
    assert resp.status_code == 201
    data = resp.json()
    assert data["name"] == "Test Campaign"
    campaign_id = data["id"]

    resp = await client.get(f"/campaigns/{campaign_id}")
    assert resp.status_code == 200
    assert resp.json()["id"] == campaign_id


async def test_list_campaigns(client):
    await client.post("/campaigns", json={"name": "Alpha"})
    await client.post("/campaigns", json={"name": "Beta"})

    resp = await client.get("/campaigns")
    assert resp.status_code == 200
    assert len(resp.json()) >= 2


async def test_campaign_not_found(client):
    from uuid import uuid4

    resp = await client.get(f"/campaigns/{uuid4()}")
    assert resp.status_code == 404


async def test_update_campaign(client):
    resp = await client.post("/campaigns", json={"name": "Old Name"})
    campaign_id = resp.json()["id"]

    resp = await client.put(f"/campaigns/{campaign_id}", json={"name": "New Name"})
    assert resp.status_code == 200
    assert resp.json()["name"] == "New Name"


async def test_delete_campaign(client):
    resp = await client.post("/campaigns", json={"name": "To Delete"})
    campaign_id = resp.json()["id"]

    resp = await client.delete(f"/campaigns/{campaign_id}")
    assert resp.status_code == 204

    resp = await client.get(f"/campaigns/{campaign_id}")
    assert resp.status_code == 404


async def test_create_character(client):
    camp = await client.post("/campaigns", json={"name": "Characters Campaign"})
    campaign_id = camp.json()["id"]

    resp = await client.post(
        f"/campaigns/{campaign_id}/characters",
        json={"name": "Briv", "character_type": "pc", "max_hp": 50, "armor_class": 18},
    )
    assert resp.status_code == 201
    data = resp.json()
    assert data["name"] == "Briv"
    assert data["current_hp"] == 50


async def test_create_session(client):
    camp = await client.post("/campaigns", json={"name": "Session Campaign"})
    campaign_id = camp.json()["id"]

    resp = await client.post(f"/campaigns/{campaign_id}/sessions", json={"title": "First Session"})
    assert resp.status_code == 201
    assert resp.json()["session_number"] == 1


async def test_chat_empty_message(client):
    camp = await client.post("/campaigns", json={"name": "Chat Campaign"})
    campaign_id = camp.json()["id"]

    resp = await client.post(f"/campaigns/{campaign_id}/chat", json={"message": ""})
    assert resp.status_code == 400
