//! Contextual encounter generation (Phase 6).
//!
//! Pipeline:
//!   1. Load party composition from campaign characters
//!   2. Embed a context query → Qdrant search for relevant lore
//!   3. Build prompt with party state + lore context + playstyle profile
//!   4. LLM returns a structured encounter suggestion (JSON)
//!   5. Parse and return GeneratedEncounter

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use guide_core::models::{
    CharacterType, GeneratedEncounter, GeneratedEncounterType, EnemySuggestion, PlaystyleProfile,
};
use guide_db::characters::CharacterRepository;
use guide_llm::client::{CompletionRequest, EmbeddingRequest, LlmTask, Message, MessageRole};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/campaigns/{campaign_id}/encounters/generate",
        post(generate_encounter),
    )
}

#[derive(Debug, Deserialize)]
pub struct GenerateEncounterRequest {
    /// Optional narrative context to guide the encounter
    pub context: Option<String>,
    /// Override the encounter type preference
    pub preferred_type: Option<GeneratedEncounterType>,
    /// Party average level (if not computed automatically)
    pub party_level_override: Option<i32>,
}

async fn generate_encounter(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<GenerateEncounterRequest>,
) -> impl IntoResponse {
    // ── 1. Load party composition ─────────────────────────────────────────────
    let char_repo = CharacterRepository::new(&state.db);
    let characters = match char_repo.list_by_campaign(campaign_id).await {
        Ok(c) => c,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let pcs: Vec<_> = characters
        .iter()
        .filter(|c| matches!(c.character_type, CharacterType::Pc) && c.is_alive)
        .collect();

    if pcs.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "No living PCs in campaign — add characters before generating an encounter",
        );
    }

    let party_level = req.party_level_override.unwrap_or_else(|| {
        let sum: i32 = pcs.iter().map(|c| c.level).sum();
        sum / pcs.len() as i32
    });

    let party_summary = pcs
        .iter()
        .map(|c| {
            format!(
                "- {} ({}{}Lv{})",
                c.name,
                c.class.as_deref().unwrap_or("Unknown class"),
                c.race.as_deref().map(|r| format!(", {r}, ")).unwrap_or(" ".into()),
                c.level
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // ── 2. Retrieve lore context from Qdrant ──────────────────────────────────
    let context_query = req.context.clone().unwrap_or_else(|| {
        format!("encounter for party level {party_level}")
    });

    let lore_context = retrieve_lore_context(&state, &campaign_id.to_string(), &context_query)
        .await
        .unwrap_or_default();

    // ── 3. Build prompt ───────────────────────────────────────────────────────
    let profile = PlaystyleProfile::default_for(campaign_id); // Phase 6+: load from DB

    let type_preference = req
        .preferred_type
        .as_ref()
        .map(|t| format!("{t:?}").to_lowercase())
        .unwrap_or_else(|| infer_preferred_type(&profile));

    let system_prompt = format!(
        "You are a D&D encounter designer. Generate a contextually relevant encounter for the party.\n\
         Return ONLY valid JSON (no markdown, no explanation) matching this schema:\n\
         {{\n\
           \"title\": \"<encounter title>\",\n\
           \"description\": \"<2-3 sentence atmospheric description>\",\n\
           \"encounter_type\": \"combat|social|exploration|puzzle|mixed\",\n\
           \"challenge_rating\": <number|null>,\n\
           \"suggested_enemies\": [{{\n\
             \"name\": \"<creature name>\",\n\
             \"count\": <number>,\n\
             \"cr\": <number|null>\n\
           }}],\n\
           \"narrative_hook\": \"<1 sentence connecting this encounter to the campaign narrative>\",\n\
           \"alternative\": \"<optional alternative approach, e.g. social solution>\"\n\
         }}\n\
         Guidelines:\n\
         - The encounter should feel organic to the campaign setting, not random\n\
         - Scale appropriately for a level {party_level} party\n\
         - Lean toward encounter type: {type_preference}\n\
         - Use campaign lore if provided to ground the encounter in the world"
    );

    let lore_section = if lore_context.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n## Relevant Campaign Lore\n{}",
            lore_context.join("\n\n")
        )
    };

    let user_message = format!(
        "Party composition (average level {party_level}):\n{party_summary}{lore_section}{}",
        req.context
            .as_deref()
            .map(|c| format!("\n\nAdditional context: {c}"))
            .unwrap_or_default()
    );

    // ── 4. LLM call ───────────────────────────────────────────────────────────
    let completion = state
        .llm
        .complete(CompletionRequest {
            task: LlmTask::EncounterGeneration,
            messages: vec![
                Message { role: MessageRole::System, content: system_prompt },
                Message { role: MessageRole::User, content: user_message },
            ],
            model_override: None,
            temperature: Some(0.8),
            max_tokens: Some(800),
        })
        .await;

    let llm_text = match completion {
        Ok(r) => r.content,
        Err(e) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("LLM unavailable: {e}"),
            )
        }
    };

    // ── 5. Parse and return ───────────────────────────────────────────────────
    #[derive(serde::Deserialize)]
    struct RawEncounter {
        title: String,
        description: String,
        encounter_type: String,
        challenge_rating: Option<f32>,
        #[serde(default)]
        suggested_enemies: Vec<RawEnemy>,
        narrative_hook: String,
        alternative: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct RawEnemy {
        name: String,
        count: u32,
        cr: Option<f32>,
    }

    let raw: RawEncounter = match serde_json::from_str(llm_text.trim()) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("LLM returned non-JSON encounter: {e}\n{llm_text}");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("LLM response was not valid JSON: {e}"),
            );
        }
    };

    let generated = GeneratedEncounter {
        title: raw.title,
        description: raw.description,
        encounter_type: match raw.encounter_type.as_str() {
            "social" => GeneratedEncounterType::Social,
            "exploration" => GeneratedEncounterType::Exploration,
            "puzzle" => GeneratedEncounterType::Puzzle,
            "mixed" => GeneratedEncounterType::Mixed,
            _ => GeneratedEncounterType::Combat,
        },
        challenge_rating: raw.challenge_rating,
        suggested_enemies: raw
            .suggested_enemies
            .into_iter()
            .map(|e| EnemySuggestion { name: e.name, count: e.count, cr: e.cr })
            .collect(),
        narrative_hook: raw.narrative_hook,
        alternative: raw.alternative,
    };

    (StatusCode::OK, Json(generated)).into_response()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn infer_preferred_type(profile: &PlaystyleProfile) -> String {
    if profile.combat_affinity >= profile.social_affinity
        && profile.combat_affinity >= profile.exploration_affinity
    {
        "combat".into()
    } else if profile.social_affinity >= profile.exploration_affinity {
        "social".into()
    } else {
        "exploration".into()
    }
}

async fn retrieve_lore_context(
    state: &AppState,
    campaign_id: &str,
    query: &str,
) -> Option<Vec<String>> {
    use guide_db::qdrant::{search_campaign_lore, search_global_rules};

    let qdrant = state.qdrant.as_ref()?;

    let embedding = state
        .llm
        .embed(EmbeddingRequest { text: query.to_string(), model_override: None })
        .await
        .ok()?;

    let campaign_chunks =
        search_campaign_lore(qdrant, campaign_id, embedding.clone(), 3, false).await.ok()?;
    let global_chunks = search_global_rules(qdrant, embedding, 2).await.ok()?;

    let mut all = [campaign_chunks, global_chunks].concat();
    all.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Some(all.into_iter().map(|c| c.content).collect())
}

fn error_response(status: StatusCode, msg: &str) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": msg }))).into_response()
}
