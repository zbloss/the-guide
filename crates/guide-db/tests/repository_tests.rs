use guide_core::models::{
    CharacterType, CreateCharacterRequest, CreateSessionEventRequest, CreateSessionRequest,
    EventSignificance, EventType, GameSystem,
};
use guide_db::{
    campaigns::CampaignRepository,
    characters::CharacterRepository,
    sessions::{SessionEventRepository, SessionRepository},
};
use guide_core::models::CreateCampaignRequest;
use sqlx::SqlitePool;

// ── Campaign tests ──────────────────────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_campaign_create_and_get(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = CampaignRepository::new(&pool);
    let campaign = repo
        .create(CreateCampaignRequest {
            name: "Lost Mine of Phandelver".into(),
            description: Some("A classic starter adventure".into()),
            game_system: Some(GameSystem::Dnd5e),
        })
        .await?;

    assert_eq!(campaign.name, "Lost Mine of Phandelver");
    assert_eq!(campaign.game_system, GameSystem::Dnd5e);
    assert!(campaign.description.is_some());

    let fetched = repo.get_by_id(campaign.id).await?;
    assert_eq!(fetched.id, campaign.id);
    assert_eq!(fetched.name, campaign.name);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_campaign_list(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = CampaignRepository::new(&pool);
    repo.create(CreateCampaignRequest { name: "Campaign A".into(), description: None, game_system: None }).await?;
    repo.create(CreateCampaignRequest { name: "Campaign B".into(), description: None, game_system: None }).await?;

    let campaigns = repo.list().await?;
    assert_eq!(campaigns.len(), 2);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_campaign_update(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    use guide_core::models::UpdateCampaignRequest;

    let repo = CampaignRepository::new(&pool);
    let campaign = repo
        .create(CreateCampaignRequest { name: "Original".into(), description: None, game_system: None })
        .await?;

    let updated = repo
        .update(
            campaign.id,
            UpdateCampaignRequest {
                name: Some("Updated Name".into()),
                description: Some("Added later".into()),
                world_state: None,
            },
        )
        .await?;

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.description.as_deref(), Some("Added later"));
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_campaign_delete(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = CampaignRepository::new(&pool);
    let campaign = repo
        .create(CreateCampaignRequest { name: "To Delete".into(), description: None, game_system: None })
        .await?;

    repo.delete(campaign.id).await?;

    let result = repo.get_by_id(campaign.id).await;
    assert!(result.is_err()); // NotFound
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_campaign_delete_not_found(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let repo = CampaignRepository::new(&pool);
    let result = repo.delete(uuid::Uuid::new_v4()).await;
    assert!(result.is_err());
    Ok(())
}

// ── Character tests ─────────────────────────────────────────────────────────

async fn create_test_campaign(pool: &SqlitePool) -> uuid::Uuid {
    CampaignRepository::new(pool)
        .create(CreateCampaignRequest { name: "Test Campaign".into(), description: None, game_system: None })
        .await
        .unwrap()
        .id
}

#[sqlx::test(migrations = "./migrations")]
async fn test_character_create_and_get(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_id = create_test_campaign(&pool).await;
    let repo = CharacterRepository::new(&pool);

    let character = repo
        .create(
            campaign_id,
            CreateCharacterRequest {
                name: "Briv".into(),
                character_type: CharacterType::Pc,
                class: Some("Fighter".into()),
                race: Some("Half-Orc".into()),
                level: Some(5),
                max_hp: 52,
                armor_class: 18,
                speed: Some(30),
                ability_scores: None,
                backstory_text: None,
            },
        )
        .await?;

    assert_eq!(character.name, "Briv");
    assert_eq!(character.max_hp, 52);
    assert_eq!(character.current_hp, 52);
    assert_eq!(character.level, 5);
    assert!(character.is_alive);

    let fetched = repo.get_by_id(character.id).await?;
    assert_eq!(fetched.id, character.id);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_character_list_by_campaign(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_id = create_test_campaign(&pool).await;
    let repo = CharacterRepository::new(&pool);

    for name in ["Aerith", "Barret", "Tifa"] {
        repo.create(
            campaign_id,
            CreateCharacterRequest {
                name: name.into(),
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
        .await?;
    }

    let chars = repo.list_by_campaign(campaign_id).await?;
    assert_eq!(chars.len(), 3);
    // alphabetical order
    assert_eq!(chars[0].name, "Aerith");
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_character_isolation_between_campaigns(
    pool: SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_a = create_test_campaign(&pool).await;
    let campaign_b = create_test_campaign(&pool).await;
    let repo = CharacterRepository::new(&pool);

    repo.create(
        campaign_a,
        CreateCharacterRequest {
            name: "Alice".into(),
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
    .await?;

    let chars_b = repo.list_by_campaign(campaign_b).await?;
    assert!(chars_b.is_empty());
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_character_delete(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_id = create_test_campaign(&pool).await;
    let repo = CharacterRepository::new(&pool);

    let character = repo
        .create(
            campaign_id,
            CreateCharacterRequest {
                name: "Doomed".into(),
                character_type: CharacterType::Monster,
                class: None,
                race: Some("Goblin".into()),
                level: None,
                max_hp: 7,
                armor_class: 15,
                speed: None,
                ability_scores: None,
                backstory_text: None,
            },
        )
        .await?;

    repo.delete(character.id).await?;

    let result = repo.get_by_id(character.id).await;
    assert!(result.is_err());
    Ok(())
}

// ── Session tests ───────────────────────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_session_create_and_numbering(
    pool: SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_id = create_test_campaign(&pool).await;
    let repo = SessionRepository::new(&pool);

    let s1 = repo
        .create(campaign_id, CreateSessionRequest { title: Some("The Beginning".into()), notes: None })
        .await?;
    assert_eq!(s1.session_number, 1);

    let s2 = repo
        .create(campaign_id, CreateSessionRequest { title: Some("The Middle".into()), notes: None })
        .await?;
    assert_eq!(s2.session_number, 2);

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_session_start_and_end(pool: SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_id = create_test_campaign(&pool).await;
    let repo = SessionRepository::new(&pool);

    let session = repo
        .create(campaign_id, CreateSessionRequest { title: None, notes: None })
        .await?;
    assert!(session.started_at.is_none());
    assert!(session.ended_at.is_none());

    let started = repo.start_session(session.id).await?;
    assert!(started.started_at.is_some());

    let ended = repo.end_session(session.id).await?;
    assert!(ended.ended_at.is_some());

    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn test_session_event_create_and_list(
    pool: SqlitePool,
) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_id = create_test_campaign(&pool).await;
    let session_repo = SessionRepository::new(&pool);
    let session = session_repo
        .create(campaign_id, CreateSessionRequest { title: None, notes: None })
        .await?;

    let event_repo = SessionEventRepository::new(&pool);

    let event = event_repo
        .create(
            session.id,
            campaign_id,
            CreateSessionEventRequest {
                event_type: EventType::Combat,
                description: "The party fought 3 goblins".into(),
                significance: Some(EventSignificance::Minor),
                is_player_visible: Some(true),
                involved_character_ids: None,
            },
        )
        .await?;

    assert_eq!(event.description, "The party fought 3 goblins");
    assert!(event.is_player_visible);

    // DM-only event
    event_repo
        .create(
            session.id,
            campaign_id,
            CreateSessionEventRequest {
                event_type: EventType::PlotRevealed,
                description: "BBEG secret revealed (DM only)".into(),
                significance: Some(EventSignificance::Major),
                is_player_visible: Some(false),
                involved_character_ids: None,
            },
        )
        .await?;

    let all_events = event_repo.list_by_session(session.id).await?;
    assert_eq!(all_events.len(), 2);

    let visible = event_repo.list_visible_by_session(session.id).await?;
    assert_eq!(visible.len(), 1);

    Ok(())
}
