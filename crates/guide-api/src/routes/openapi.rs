use utoipa::OpenApi;
use guide_core::models::*;
use crate::routes::*;

#[derive(OpenApi)]
#[openapi(
    paths(
        health::health,
        health::version,
        campaigns::list_campaigns,
        campaigns::create_campaign,
        campaigns::get_campaign,
        campaigns::update_campaign,
        campaigns::delete_campaign,
        characters::list_characters,
        characters::create_character,
        characters::get_character,
        characters::update_character,
        characters::delete_character,
        characters::analyze_backstory,
        sessions::list_sessions,
        sessions::create_session,
        sessions::get_session,
        sessions::delete_session,
        sessions::start_session,
        sessions::end_session,
        sessions::list_events,
        sessions::create_event,
        sessions::get_summary,
        encounters::list_encounters,
        encounters::create_encounter,
        encounters::get_encounter,
        encounters::delete_encounter,
        encounters::start_encounter,
        encounters::next_turn,
        encounters::end_encounter,
        encounters::update_participant,
        documents::list_documents,
        documents::upload_document,
        documents::get_document,
        documents::ingest_document,
        documents::list_global,
        documents::upload_global,
        documents::get_global,
        documents::ingest_global,
        generate::generate_encounter,
        chat::chat,
    ),
    components(
        schemas(
            Campaign, WorldState, CreateCampaignRequest, UpdateCampaignRequest,
            Character, AbilityScores, Backstory, PlotHook, HookPriority, CreateCharacterRequest, UpdateCharacterRequest, CharacterType, Condition,
            Session, SessionEvent, SessionSummary, CreateSessionRequest, CreateSessionEventRequest, Perspective,
            Encounter, CombatParticipant, ActionBudget, CreateEncounterRequest, AddParticipantRequest, UpdateParticipantRequest, EncounterStatus, EncounterSummary,
            CampaignDocument, GlobalDocument, DocumentKind, IngestionStatus, RankedChunk, DocSummary, MetaIndex,
            generate::GenerateRequest, chat::ChatRequest,
            guide_core::models::GeneratedEncounter,
            guide_core::models::GameSystem,
            guide_core::models::EventType,
            guide_core::models::EventSignificance,
            guide_core::models::LoreType,
        )
    ),
    tags(
        (name = "The Guide", description = "DPM Campaign Management API")
    )
)]
pub struct ApiDoc;
