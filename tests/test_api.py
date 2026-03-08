"""HTTP-layer tests via httpx.AsyncClient."""

from __future__ import annotations

from uuid import uuid4


# ---------------------------------------------------------------------------
# Health / version
# ---------------------------------------------------------------------------


async def test_health(client):
    resp = await client.get("/health")
    assert resp.status_code == 200
    assert resp.json()["status"] == "ok"


async def test_version(client):
    resp = await client.get("/version")
    assert resp.status_code == 200
    data = resp.json()
    assert "version" in data


# ---------------------------------------------------------------------------
# Campaigns
# ---------------------------------------------------------------------------


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


# ---------------------------------------------------------------------------
# Characters
# ---------------------------------------------------------------------------


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


async def test_list_characters(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await client.get(f"/campaigns/{cid}/characters")
    assert resp.status_code == 200
    assert resp.json() == []


async def test_get_character(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    ch = await client.post(
        f"/campaigns/{cid}/characters",
        json={"name": "Aria", "character_type": "pc", "max_hp": 30, "armor_class": 14},
    )
    char_id = ch.json()["id"]

    resp = await client.get(f"/campaigns/{cid}/characters/{char_id}")
    assert resp.status_code == 200
    assert resp.json()["id"] == char_id


async def test_delete_character(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    ch = await client.post(
        f"/campaigns/{cid}/characters",
        json={"name": "Aria", "character_type": "pc", "max_hp": 30, "armor_class": 14},
    )
    char_id = ch.json()["id"]

    resp = await client.delete(f"/campaigns/{cid}/characters/{char_id}")
    assert resp.status_code == 204

    resp = await client.get(f"/campaigns/{cid}/characters/{char_id}")
    assert resp.status_code == 404


async def test_character_not_found(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await client.get(f"/campaigns/{cid}/characters/{uuid4()}")
    assert resp.status_code == 404


async def test_analyze_backstory_no_text(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    ch = await client.post(
        f"/campaigns/{cid}/characters",
        json={"name": "Aria", "character_type": "pc", "max_hp": 30, "armor_class": 14},
    )
    char_id = ch.json()["id"]

    resp = await client.post(
        f"/campaigns/{cid}/characters/{char_id}/analyze-backstory", json={}
    )
    assert resp.status_code == 422


# ---------------------------------------------------------------------------
# Sessions
# ---------------------------------------------------------------------------


async def test_create_session(client):
    camp = await client.post("/campaigns", json={"name": "Session Campaign"})
    campaign_id = camp.json()["id"]

    resp = await client.post(f"/campaigns/{campaign_id}/sessions", json={"title": "First Session"})
    assert resp.status_code == 201
    assert resp.json()["session_number"] == 1


async def test_list_sessions(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await client.get(f"/campaigns/{cid}/sessions")
    assert resp.status_code == 200
    assert resp.json() == []


async def test_get_session(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    sess = await client.post(f"/campaigns/{cid}/sessions", json={"title": "Test"})
    sid = sess.json()["id"]

    resp = await client.get(f"/campaigns/{cid}/sessions/{sid}")
    assert resp.status_code == 200
    assert resp.json()["id"] == sid


async def test_delete_session(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    sess = await client.post(f"/campaigns/{cid}/sessions", json={"title": "Test"})
    sid = sess.json()["id"]

    resp = await client.delete(f"/campaigns/{cid}/sessions/{sid}")
    assert resp.status_code == 204

    resp = await client.get(f"/campaigns/{cid}/sessions/{sid}")
    assert resp.status_code == 404


async def test_start_and_end_session(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    sess = await client.post(f"/campaigns/{cid}/sessions", json={"title": "Live"})
    sid = sess.json()["id"]

    resp = await client.post(f"/campaigns/{cid}/sessions/{sid}/start")
    assert resp.status_code == 200
    assert resp.json()["started_at"] is not None

    resp = await client.post(f"/campaigns/{cid}/sessions/{sid}/end")
    assert resp.status_code == 200
    assert resp.json()["ended_at"] is not None


async def test_create_session_event(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    sess = await client.post(f"/campaigns/{cid}/sessions", json={"title": "Test"})
    sid = sess.json()["id"]

    resp = await client.post(
        f"/campaigns/{cid}/sessions/{sid}/events",
        json={"event_type": "combat", "description": "Battle erupted in the tavern"},
    )
    assert resp.status_code == 201

    resp = await client.get(f"/campaigns/{cid}/sessions/{sid}/events")
    assert resp.status_code == 200
    assert len(resp.json()) == 1


async def test_session_summary_no_events(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    sess = await client.post(f"/campaigns/{cid}/sessions", json={"title": "Empty"})
    sid = sess.json()["id"]

    resp = await client.get(f"/campaigns/{cid}/sessions/{sid}/summary")
    assert resp.status_code == 422


# ---------------------------------------------------------------------------
# Encounters
# ---------------------------------------------------------------------------


async def _setup_encounter(client):
    """Create campaign → PC → session → encounter. Returns (cid, char_id, sid, enc_id)."""
    camp = await client.post("/campaigns", json={"name": "Enc Campaign"})
    cid = camp.json()["id"]

    ch = await client.post(
        f"/campaigns/{cid}/characters",
        json={"name": "Warrior", "character_type": "pc", "max_hp": 40, "armor_class": 16},
    )
    char_id = ch.json()["id"]

    sess = await client.post(f"/campaigns/{cid}/sessions", json={"title": "Session 1"})
    sid = sess.json()["id"]

    enc = await client.post(
        f"/campaigns/{cid}/sessions/{sid}/encounters",
        json={"session_id": sid, "participant_character_ids": [char_id]},
    )
    enc_id = enc.json()["id"]

    return cid, char_id, sid, enc_id


async def test_create_encounter_no_pcs(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    sess = await client.post(f"/campaigns/{cid}/sessions", json={"title": "S"})
    sid = sess.json()["id"]

    resp = await client.post(
        f"/campaigns/{cid}/sessions/{sid}/encounters",
        json={"session_id": sid, "participant_character_ids": [str(uuid4())]},
    )
    assert resp.status_code == 400


async def test_start_encounter(client):
    cid, _, sid, enc_id = await _setup_encounter(client)

    resp = await client.post(f"/campaigns/{cid}/encounters/{enc_id}/start")
    assert resp.status_code == 200
    data = resp.json()
    assert "encounter" in data
    assert data["round"] == 1


async def test_next_turn(client):
    cid, _, sid, enc_id = await _setup_encounter(client)
    await client.post(f"/campaigns/{cid}/encounters/{enc_id}/start")

    resp = await client.post(f"/campaigns/{cid}/encounters/{enc_id}/next-turn")
    assert resp.status_code == 200
    assert "round" in resp.json()


async def test_end_encounter(client):
    cid, _, sid, enc_id = await _setup_encounter(client)

    resp = await client.post(f"/campaigns/{cid}/encounters/{enc_id}/end")
    assert resp.status_code == 200
    assert resp.json()["status"] == "completed"


# ---------------------------------------------------------------------------
# Documents
# ---------------------------------------------------------------------------


async def test_list_documents_empty(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await client.get(f"/campaigns/{cid}/documents")
    assert resp.status_code == 200
    assert resp.json() == []


async def test_upload_document(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await client.post(
        f"/campaigns/{cid}/documents",
        files={"file": ("rules.pdf", b"%PDF-1.4 test content", "application/pdf")},
    )
    assert resp.status_code == 201
    data = resp.json()
    assert data["filename"] == "rules.pdf"


async def test_get_document(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    upload = await client.post(
        f"/campaigns/{cid}/documents",
        files={"file": ("lore.pdf", b"%PDF-1.4 lore content", "application/pdf")},
    )
    doc_id = upload.json()["id"]

    resp = await client.get(f"/campaigns/{cid}/documents/{doc_id}")
    assert resp.status_code == 200
    assert resp.json()["id"] == doc_id


async def test_upload_document_too_large(small_upload_client):
    camp = await small_upload_client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await small_upload_client.post(
        f"/campaigns/{cid}/documents",
        files={"file": ("big.pdf", b"x" * 100, "application/pdf")},
    )
    assert resp.status_code == 413


# ---------------------------------------------------------------------------
# Chat
# ---------------------------------------------------------------------------


async def test_chat_empty_message(client):
    camp = await client.post("/campaigns", json={"name": "Chat Campaign"})
    campaign_id = camp.json()["id"]

    resp = await client.post(f"/campaigns/{campaign_id}/chat", json={"message": ""})
    assert resp.status_code == 400


async def test_chat_success(client):
    camp = await client.post("/campaigns", json={"name": "Chat"})
    cid = camp.json()["id"]

    resp = await client.post(f"/campaigns/{cid}/chat", json={"message": "What is happening?"})
    assert resp.status_code == 200
    assert "text/event-stream" in resp.headers.get("content-type", "")


async def test_chat_message_too_long(client):
    camp = await client.post("/campaigns", json={"name": "Chat"})
    cid = camp.json()["id"]

    resp = await client.post(f"/campaigns/{cid}/chat", json={"message": "x" * 4001})
    assert resp.status_code == 413


# ---------------------------------------------------------------------------
# Generate Encounter
# ---------------------------------------------------------------------------


async def test_generate_encounter_no_pcs(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await client.post(f"/campaigns/{cid}/encounters/generate", json={})
    assert resp.status_code == 422


async def test_generate_encounter(client):
    camp = await client.post("/campaigns", json={"name": "Gen"})
    cid = camp.json()["id"]
    await client.post(
        f"/campaigns/{cid}/characters",
        json={"name": "Hero", "character_type": "pc", "max_hp": 40, "armor_class": 15},
    )

    resp = await client.post(f"/campaigns/{cid}/encounters/generate", json={})
    assert resp.status_code == 200
    data = resp.json()
    assert "title" in data
    assert "description" in data
    assert "narrative_hook" in data


# ---------------------------------------------------------------------------
# Security: cross-campaign data leakage
# ---------------------------------------------------------------------------


async def test_character_wrong_campaign_returns_404(client):
    camp1 = await client.post("/campaigns", json={"name": "C1"})
    camp1_id = camp1.json()["id"]
    camp2 = await client.post("/campaigns", json={"name": "C2"})
    camp2_id = camp2.json()["id"]

    ch = await client.post(
        f"/campaigns/{camp1_id}/characters",
        json={"name": "Aria", "character_type": "pc", "max_hp": 30, "armor_class": 14},
    )
    char_id = ch.json()["id"]

    resp = await client.get(f"/campaigns/{camp2_id}/characters/{char_id}")
    assert resp.status_code == 404


async def test_delete_character_wrong_campaign_returns_404(client):
    camp1 = await client.post("/campaigns", json={"name": "C1"})
    camp1_id = camp1.json()["id"]
    camp2 = await client.post("/campaigns", json={"name": "C2"})
    camp2_id = camp2.json()["id"]

    ch = await client.post(
        f"/campaigns/{camp1_id}/characters",
        json={"name": "Aria", "character_type": "pc", "max_hp": 30, "armor_class": 14},
    )
    char_id = ch.json()["id"]

    resp = await client.delete(f"/campaigns/{camp2_id}/characters/{char_id}")
    assert resp.status_code == 404

    # Character should still exist in the correct campaign
    resp = await client.get(f"/campaigns/{camp1_id}/characters/{char_id}")
    assert resp.status_code == 200


async def test_session_wrong_campaign_returns_404(client):
    camp1 = await client.post("/campaigns", json={"name": "C1"})
    camp1_id = camp1.json()["id"]
    camp2 = await client.post("/campaigns", json={"name": "C2"})
    camp2_id = camp2.json()["id"]

    sess = await client.post(f"/campaigns/{camp1_id}/sessions", json={"title": "Test"})
    sid = sess.json()["id"]

    resp = await client.get(f"/campaigns/{camp2_id}/sessions/{sid}")
    assert resp.status_code == 404


async def test_create_character_invalid_campaign(client):
    resp = await client.post(
        f"/campaigns/{uuid4()}/characters",
        json={"name": "Ghost", "character_type": "pc", "max_hp": 20, "armor_class": 12},
    )
    assert resp.status_code == 404


async def test_create_session_invalid_campaign(client):
    resp = await client.post(f"/campaigns/{uuid4()}/sessions", json={"title": "Ghost Session"})
    assert resp.status_code == 404


async def test_encounter_participant_wrong_campaign(client):
    camp1 = await client.post("/campaigns", json={"name": "C1"})
    camp1_id = camp1.json()["id"]
    camp2 = await client.post("/campaigns", json={"name": "C2"})
    camp2_id = camp2.json()["id"]

    # Character belongs to camp1
    ch = await client.post(
        f"/campaigns/{camp1_id}/characters",
        json={"name": "Warrior", "character_type": "pc", "max_hp": 40, "armor_class": 16},
    )
    char_id = ch.json()["id"]

    # Session and encounter in camp2
    sess = await client.post(f"/campaigns/{camp2_id}/sessions", json={"title": "S"})
    sid = sess.json()["id"]

    resp = await client.post(
        f"/campaigns/{camp2_id}/sessions/{sid}/encounters",
        json={"session_id": sid, "participant_character_ids": [char_id]},
    )
    assert resp.status_code == 400


# ---------------------------------------------------------------------------
# Bug fixes: PUT /characters/{id}, PDF validation, generate validation
# ---------------------------------------------------------------------------


async def test_update_character(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    ch = await client.post(
        f"/campaigns/{cid}/characters",
        json={"name": "Aria", "character_type": "pc", "max_hp": 30, "armor_class": 14},
    )
    char_id = ch.json()["id"]

    resp = await client.put(
        f"/campaigns/{cid}/characters/{char_id}",
        json={"current_hp": 15, "is_alive": True},
    )
    assert resp.status_code == 200
    data = resp.json()
    assert data["current_hp"] == 15


async def test_upload_non_pdf_returns_400(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await client.post(
        f"/campaigns/{cid}/documents",
        files={"file": ("notes.txt", b"just some text content", "text/plain")},
    )
    assert resp.status_code == 400


async def test_generate_encounter_invalid_level(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]

    resp = await client.post(
        f"/campaigns/{cid}/encounters/generate", json={"party_level_override": 0}
    )
    assert resp.status_code == 400


async def test_session_summary_player_perspective_case_insensitive(client):
    camp = await client.post("/campaigns", json={"name": "C"})
    cid = camp.json()["id"]
    sess = await client.post(f"/campaigns/{cid}/sessions", json={"title": "Empty"})
    sid = sess.json()["id"]

    # "Player" (capitalized) should work the same as "player" — no events means 422
    resp = await client.get(f"/campaigns/{cid}/sessions/{sid}/summary?perspective=Player")
    assert resp.status_code == 422
