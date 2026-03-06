"""Port of the 12 Rust DB integration tests."""
from __future__ import annotations

from uuid import uuid4

import pytest

from guide.db.campaigns import CampaignRepository
from guide.db.characters import CharacterRepository
from guide.db.sessions import SessionEventRepository, SessionRepository
from guide.errors import NotFoundError
from guide.models.campaign import CreateCampaignRequest, UpdateCampaignRequest
from guide.models.character import CreateCharacterRequest
from guide.models.session import CreateSessionEventRequest, CreateSessionRequest
from guide.models.shared import (
    CharacterType,
    EventSignificance,
    EventType,
    GameSystem,
)


# ── Campaign tests ─────────────────────────────────────────────────────────────

async def test_campaign_create_and_get(db):
    repo = CampaignRepository(db)
    campaign = await repo.create(CreateCampaignRequest(
        name="Lost Mine of Phandelver",
        description="A classic starter adventure",
        game_system=GameSystem.dnd5e,
    ))

    assert campaign.name == "Lost Mine of Phandelver"
    assert campaign.game_system == GameSystem.dnd5e
    assert campaign.description is not None

    fetched = await repo.get_by_id(campaign.id)
    assert fetched.id == campaign.id
    assert fetched.name == campaign.name


async def test_campaign_list(db):
    repo = CampaignRepository(db)
    await repo.create(CreateCampaignRequest(name="Campaign A"))
    await repo.create(CreateCampaignRequest(name="Campaign B"))

    campaigns = await repo.list()
    assert len(campaigns) == 2


async def test_campaign_update(db):
    repo = CampaignRepository(db)
    campaign = await repo.create(CreateCampaignRequest(name="Original"))

    updated = await repo.update(campaign.id, UpdateCampaignRequest(
        name="Updated Name",
        description="Added later",
    ))

    assert updated.name == "Updated Name"
    assert updated.description == "Added later"


async def test_campaign_delete(db):
    repo = CampaignRepository(db)
    campaign = await repo.create(CreateCampaignRequest(name="To Delete"))

    await repo.delete(campaign.id)

    with pytest.raises(NotFoundError):
        await repo.get_by_id(campaign.id)


async def test_campaign_delete_not_found(db):
    repo = CampaignRepository(db)
    with pytest.raises(NotFoundError):
        await repo.delete(uuid4())


# ── Character tests ────────────────────────────────────────────────────────────

async def _create_test_campaign(db):
    repo = CampaignRepository(db)
    campaign = await repo.create(CreateCampaignRequest(name="Test Campaign"))
    return campaign.id


async def test_character_create_and_get(db):
    campaign_id = await _create_test_campaign(db)
    repo = CharacterRepository(db)

    character = await repo.create(campaign_id, CreateCharacterRequest(
        name="Briv",
        character_type=CharacterType.pc,
        class_="Fighter",
        race="Half-Orc",
        level=5,
        max_hp=52,
        armor_class=18,
        speed=30,
    ))

    assert character.name == "Briv"
    assert character.max_hp == 52
    assert character.current_hp == 52
    assert character.level == 5
    assert character.is_alive

    fetched = await repo.get_by_id(character.id)
    assert fetched.id == character.id


async def test_character_list_by_campaign(db):
    campaign_id = await _create_test_campaign(db)
    repo = CharacterRepository(db)

    for name in ["Aerith", "Barret", "Tifa"]:
        await repo.create(campaign_id, CreateCharacterRequest(
            name=name, character_type=CharacterType.pc, max_hp=30, armor_class=12,
        ))

    chars = await repo.list_by_campaign(campaign_id)
    assert len(chars) == 3
    assert chars[0].name == "Aerith"  # alphabetical


async def test_character_isolation_between_campaigns(db):
    campaign_a = await _create_test_campaign(db)
    campaign_b = await _create_test_campaign(db)
    repo = CharacterRepository(db)

    await repo.create(campaign_a, CreateCharacterRequest(
        name="Alice", character_type=CharacterType.pc, max_hp=20, armor_class=10,
    ))

    chars_b = await repo.list_by_campaign(campaign_b)
    assert len(chars_b) == 0


async def test_character_delete(db):
    campaign_id = await _create_test_campaign(db)
    repo = CharacterRepository(db)

    character = await repo.create(campaign_id, CreateCharacterRequest(
        name="Doomed", character_type=CharacterType.monster, max_hp=7, armor_class=15, race="Goblin",
    ))

    await repo.delete(character.id)

    with pytest.raises(NotFoundError):
        await repo.get_by_id(character.id)


# ── Session tests ──────────────────────────────────────────────────────────────

async def test_session_create_and_numbering(db):
    campaign_id = await _create_test_campaign(db)
    repo = SessionRepository(db)

    s1 = await repo.create(campaign_id, CreateSessionRequest(title="The Beginning"))
    assert s1.session_number == 1

    s2 = await repo.create(campaign_id, CreateSessionRequest(title="The Middle"))
    assert s2.session_number == 2


async def test_session_start_and_end(db):
    campaign_id = await _create_test_campaign(db)
    repo = SessionRepository(db)

    session = await repo.create(campaign_id, CreateSessionRequest())
    assert session.started_at is None
    assert session.ended_at is None

    started = await repo.start_session(session.id)
    assert started.started_at is not None

    ended = await repo.end_session(session.id)
    assert ended.ended_at is not None


async def test_session_event_create_and_list(db):
    campaign_id = await _create_test_campaign(db)

    session = await SessionRepository(db).create(campaign_id, CreateSessionRequest())
    event_repo = SessionEventRepository(db)

    event = await event_repo.create(
        session.id, campaign_id,
        CreateSessionEventRequest(
            event_type=EventType.combat,
            description="The party fought 3 goblins",
            significance=EventSignificance.minor,
            is_player_visible=True,
        ),
    )
    assert event.description == "The party fought 3 goblins"
    assert event.is_player_visible

    # DM-only event
    await event_repo.create(
        session.id, campaign_id,
        CreateSessionEventRequest(
            event_type=EventType.plot_revealed,
            description="BBEG secret revealed (DM only)",
            significance=EventSignificance.major,
            is_player_visible=False,
        ),
    )

    all_events = await event_repo.list_by_session(session.id)
    assert len(all_events) == 2

    visible = await event_repo.list_visible_by_session(session.id)
    assert len(visible) == 1
