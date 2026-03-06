/// Integration tests for guide-db repositories.
/// Uses an in-memory SQLite database so no external services are required.
use guide_core::models::{
    CharacterType, CreateCampaignRequest, CreateCharacterRequest, CreateSessionEventRequest,
    CreateSessionRequest, EventSignificance, EventType, GameSystem, UpdateCampaignRequest,
};
use guide_db::{
    campaigns::CampaignRepository,
    characters::CharacterRepository,
    sessions::{SessionEventRepository, SessionRepository},
};
use uuid::Uuid;

async fn test_pool() -> guide_db::SqlitePool {
    guide_db::init_sqlite("sqlite::memory:")
        .await
        .expect("Failed to create in-memory pool")
}

// ── Campaign tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_campaign_create_and_get() {
    let pool = test_pool().await;
    let repo = CampaignRepository::new(&pool);

    let campaign = repo
        .create(CreateCampaignRequest {
            name: "Lost Mine of Phandelver".to_string(),
            description: Some("A classic starter adventure".to_string()),
            game_system: Some(GameSystem::Dnd5e),
        })
        .await
        .expect("Failed to create campaign");

    assert_eq!(campaign.name, "Lost Mine of Phandelver");
    assert_eq!(campaign.game_system, GameSystem::Dnd5e);
    assert!(campaign.description.is_some());

    let fetched = repo.get_by_id(campaign.id).await.expect("Failed to fetch");
    assert_eq!(fetched.id, campaign.id);
    assert_eq!(fetched.name, campaign.name);
}

#[tokio::test]
async fn test_campaign_list() {
    let pool = test_pool().await;
    let repo = CampaignRepository::new(&pool);

    repo.create(CreateCampaignRequest {
        name: "Campaign A".to_string(),
        description: None,
        game_system: None,
    })
    .await
    .unwrap();

    repo.create(CreateCampaignRequest {
        name: "Campaign B".to_string(),
        description: None,
        game_system: None,
    })
    .await
    .unwrap();

    let list = repo.list().await.expect("Failed to list");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn test_campaign_update() {
    let pool = test_pool().await;
    let repo = CampaignRepository::new(&pool);

    let campaign = repo
        .create(CreateCampaignRequest {
            name: "Original".to_string(),
            description: None,
            game_system: None,
        })
        .await
        .unwrap();

    let updated = repo
        .update(
            campaign.id,
            UpdateCampaignRequest {
                name: Some("Updated Name".to_string()),
                description: Some("Added later".to_string()),
                world_state: None,
            },
        )
        .await
        .expect("Failed to update");

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.description.as_deref(), Some("Added later"));
}

#[tokio::test]
async fn test_campaign_delete() {
    let pool = test_pool().await;
    let repo = CampaignRepository::new(&pool);

    let campaign = repo
        .create(CreateCampaignRequest {
            name: "To Delete".to_string(),
            description: None,
            game_system: None,
        })
        .await
        .unwrap();

    repo.delete(campaign.id).await.expect("Failed to delete");

    let result = repo.get_by_id(campaign.id).await;
    assert!(matches!(result, Err(guide_core::GuideError::NotFound(_))));
}

#[tokio::test]
async fn test_campaign_delete_not_found() {
    let pool = test_pool().await;
    let repo = CampaignRepository::new(&pool);

    let result = repo.delete(Uuid::new_v4()).await;
    assert!(matches!(result, Err(guide_core::GuideError::NotFound(_))));
}

// ── Character tests ───────────────────────────────────────────────────────────

async fn create_test_campaign(pool: &guide_db::SqlitePool) -> Uuid {
    let repo = CampaignRepository::new(pool);
    repo.create(CreateCampaignRequest {
        name: "Test Campaign".to_string(),
        description: None,
        game_system: None,
    })
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn test_character_create_and_get() {
    let pool = test_pool().await;
    let campaign_id = create_test_campaign(&pool).await;
    let repo = CharacterRepository::new(&pool);

    let character = repo
        .create(
            campaign_id,
            CreateCharacterRequest {
                name: "Briv".to_string(),
                character_type: CharacterType::Pc,
                class: Some("Fighter".to_string()),
                race: Some("Half-Orc".to_string()),
                level: Some(5),
                max_hp: 52,
                armor_class: 18,
                speed: Some(30),
                ability_scores: None,
                backstory_text: None,
            },
        )
        .await
        .expect("Failed to create character");

    assert_eq!(character.name, "Briv");
    assert_eq!(character.max_hp, 52);
    assert_eq!(character.current_hp, 52); // starts at max
    assert_eq!(character.level, 5);
    assert!(character.is_alive);

    let fetched = repo.get_by_id(character.id).await.unwrap();
    assert_eq!(fetched.id, character.id);
}

#[tokio::test]
async fn test_character_list_by_campaign() {
    let pool = test_pool().await;
    let campaign_id = create_test_campaign(&pool).await;
    let repo = CharacterRepository::new(&pool);

    let names = ["Aerith", "Barret", "Tifa"];
    for name in names {
        repo.create(
            campaign_id,
            CreateCharacterRequest {
                name: name.to_string(),
                character_type: CharacterType::Pc,
                class: None,
                race: None,
                level: None,
                max_hp: 30,
                armor_class: 12,
                speed: None,
                ability_scores: None,
                backstory_text: None,
            },
        )
        .await
        .unwrap();
    }

    let list = repo.list_by_campaign(campaign_id).await.unwrap();
    assert_eq!(list.len(), 3);
    // Should be sorted alphabetically
    assert_eq!(list[0].name, "Aerith");
}

#[tokio::test]
async fn test_character_isolation_between_campaigns() {
    let pool = test_pool().await;
    let campaign_a = create_test_campaign(&pool).await;
    let campaign_b = create_test_campaign(&pool).await;
    let repo = CharacterRepository::new(&pool);

    repo.create(
        campaign_a,
        CreateCharacterRequest {
            name: "Alice".to_string(),
            character_type: CharacterType::Pc,
            class: None,
            race: None,
            level: None,
            max_hp: 20,
            armor_class: 10,
            speed: None,
            ability_scores: None,
            backstory_text: None,
        },
    )
    .await
    .unwrap();

    let list_b = repo.list_by_campaign(campaign_b).await.unwrap();
    assert!(list_b.is_empty(), "Campaign B should not see Campaign A's characters");
}

#[tokio::test]
async fn test_character_delete() {
    let pool = test_pool().await;
    let campaign_id = create_test_campaign(&pool).await;
    let repo = CharacterRepository::new(&pool);

    let character = repo
        .create(
            campaign_id,
            CreateCharacterRequest {
                name: "Doomed".to_string(),
                character_type: CharacterType::Monster,
                class: None,
                race: Some("Goblin".to_string()),
                level: None,
                max_hp: 7,
                armor_class: 15,
                speed: None,
                ability_scores: None,
                backstory_text: None,
            },
        )
        .await
        .unwrap();

    repo.delete(character.id).await.unwrap();
    let result = repo.get_by_id(character.id).await;
    assert!(matches!(result, Err(guide_core::GuideError::NotFound(_))));
}

// ── Session tests ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_session_create_and_numbering() {
    let pool = test_pool().await;
    let campaign_id = create_test_campaign(&pool).await;
    let repo = SessionRepository::new(&pool);

    let s1 = repo
        .create(
            campaign_id,
            CreateSessionRequest {
                title: Some("The Beginning".to_string()),
                notes: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(s1.session_number, 1);

    let s2 = repo
        .create(
            campaign_id,
            CreateSessionRequest {
                title: Some("The Middle".to_string()),
                notes: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(s2.session_number, 2);
}

#[tokio::test]
async fn test_session_start_and_end() {
    let pool = test_pool().await;
    let campaign_id = create_test_campaign(&pool).await;
    let repo = SessionRepository::new(&pool);

    let session = repo
        .create(
            campaign_id,
            CreateSessionRequest { title: None, notes: None },
        )
        .await
        .unwrap();

    assert!(session.started_at.is_none());
    assert!(session.ended_at.is_none());

    let started = repo.start_session(session.id).await.unwrap();
    assert!(started.started_at.is_some());

    let ended = repo.end_session(session.id).await.unwrap();
    assert!(ended.ended_at.is_some());
}

#[tokio::test]
async fn test_session_event_create_and_list() {
    let pool = test_pool().await;
    let campaign_id = create_test_campaign(&pool).await;

    let session = SessionRepository::new(&pool)
        .create(
            campaign_id,
            CreateSessionRequest { title: None, notes: None },
        )
        .await
        .unwrap();

    let event_repo = SessionEventRepository::new(&pool);

    let event = event_repo
        .create(
            session.id,
            campaign_id,
            CreateSessionEventRequest {
                event_type: EventType::Combat,
                description: "The party fought 3 goblins".to_string(),
                significance: Some(EventSignificance::Minor),
                is_player_visible: Some(true),
                involved_character_ids: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(event.description, "The party fought 3 goblins");
    assert!(event.is_player_visible);

    // DM-only event
    event_repo
        .create(
            session.id,
            campaign_id,
            CreateSessionEventRequest {
                event_type: EventType::PlotRevealed,
                description: "BBEG secret revealed (DM only)".to_string(),
                significance: Some(EventSignificance::Major),
                is_player_visible: Some(false),
                involved_character_ids: None,
            },
        )
        .await
        .unwrap();

    let all_events = event_repo.list_by_session(session.id).await.unwrap();
    assert_eq!(all_events.len(), 2);

    let visible_events = event_repo.list_visible_by_session(session.id).await.unwrap();
    assert_eq!(visible_events.len(), 1, "Only player-visible event should be returned");
}
