use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use futures::stream::{self, BoxStream};
use guide_api::{routes::all_routes, state::AppState};
use guide_core::AppConfig;
use guide_llm::{
    client::{
        CompletionRequest, CompletionResponse, EmbeddingRequest, LlmClient, VisionRequest,
    },
    LlmTask,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

// ── MockLlm ─────────────────────────────────────────────────────────────────

struct MockLlm;

#[async_trait]
impl LlmClient for MockLlm {
    async fn complete(&self, req: CompletionRequest) -> guide_core::Result<CompletionResponse> {
        let content = if req.task == LlmTask::EncounterGeneration {
            serde_json::json!({
                "title": "Test Encounter",
                "description": "A challenging test encounter.",
                "encounter_type": "combat",
                "challenge_rating": 2.0,
                "suggested_enemies": [{"name": "Goblin", "count": 3, "cr": 0.25}],
                "narrative_hook": "Hired by an unknown patron.",
                "alternative": null
            })
            .to_string()
        } else if req.task == LlmTask::BackstoryAnalysis {
            r#"{"motivations":[],"key_relationships":[],"secrets":[],"plot_hooks":[]}"#.into()
        } else {
            "Mock LLM response.".into()
        };
        Ok(CompletionResponse {
            content,
            model: "mock".into(),
            provider: "mock".into(),
            prompt_tokens: 0,
            completion_tokens: 0,
        })
    }

    async fn complete_stream(
        &self,
        _req: CompletionRequest,
    ) -> guide_core::Result<BoxStream<'static, guide_core::Result<String>>> {
        Ok(Box::pin(stream::once(async {
            Ok::<String, _>("mock chunk".into())
        })))
    }

    async fn embed(&self, _req: EmbeddingRequest) -> guide_core::Result<Vec<f32>> {
        Ok(vec![0.0f32; 768])
    }

    async fn complete_with_vision(
        &self,
        _req: VisionRequest,
    ) -> guide_core::Result<CompletionResponse> {
        Ok(CompletionResponse {
            content: "mock vision".into(),
            model: "mock".into(),
            provider: "mock".into(),
            prompt_tokens: 0,
            completion_tokens: 0,
        })
    }

    fn provider_name(&self) -> &str {
        "mock"
    }
}

// ── Test helpers ─────────────────────────────────────────────────────────────

async fn make_app() -> axum::Router {
    let pool = guide_db::init_sqlite(":memory:").await.unwrap();
    let config = Arc::new(AppConfig {
        host: "127.0.0.1".into(),
        port: 8000,
        database_url: ":memory:".into(),
        ollama_base_url: "http://localhost:11434/v1".into(),
        default_model: "mock".into(),
        embedding_model: "nomic-embed-text".into(),
        ocr_model: "mock".into(),
        cloud_fallback: None,
        cloud_api_key: None,
        max_upload_bytes: 10 * 1024 * 1024,
        chunk_max_chars: 1600,
        chunk_overlap_chars: 200,
        qdrant_url: String::new(),
        qdrant_collection: "guide_chunks".into(),
        embedding_dims: 768,
    });

    let state = AppState {
        config,
        llm: Arc::new(MockLlm),
        db: pool,
        qdrant: None,
    };

    all_routes(state)
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn put_json(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn delete(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

// ── Health ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health() {
    let app = make_app().await;
    let resp = app.oneshot(get("/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_version() {
    let app = make_app().await;
    let resp = app.oneshot(get("/version")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.get("version").is_some());
}

// ── Campaigns ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_and_get_campaign() {
    let app = make_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/campaigns", json!({"name": "Test Campaign"})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["name"], "Test Campaign");
    let id = body["id"].as_str().unwrap().to_string();

    let resp = app.oneshot(get(&format!("/campaigns/{id}"))).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["id"], id);
}

#[tokio::test]
async fn test_list_campaigns() {
    let app = make_app().await;
    app.clone()
        .oneshot(post_json("/campaigns", json!({"name": "Alpha"})))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_json("/campaigns", json!({"name": "Beta"})))
        .await
        .unwrap();

    let resp = app.oneshot(get("/campaigns")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn test_campaign_not_found() {
    let app = make_app().await;
    let resp = app
        .oneshot(get("/campaigns/00000000-0000-0000-0000-000000000000"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_campaign() {
    let app = make_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/campaigns", json!({"name": "Old Name"})))
        .await
        .unwrap();
    let id = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .oneshot(put_json(
            &format!("/campaigns/{id}"),
            json!({"name": "New Name"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["name"], "New Name");
}

#[tokio::test]
async fn test_delete_campaign() {
    let app = make_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/campaigns", json!({"name": "To Delete"})))
        .await
        .unwrap();
    let id = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(delete(&format!("/campaigns/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(get(&format!("/campaigns/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Characters ───────────────────────────────────────────────────────────────

async fn create_campaign(app: &axum::Router) -> String {
    let resp = app
        .clone()
        .oneshot(post_json("/campaigns", json!({"name": "Camp"})))
        .await
        .unwrap();
    json_body(resp).await["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn test_create_character() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/characters"),
            json!({"name": "Briv", "character_type": "pc", "max_hp": 50, "armor_class": 18}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["name"], "Briv");
    assert_eq!(body["current_hp"], 50);
}

#[tokio::test]
async fn test_list_characters_empty() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .oneshot(get(&format!("/campaigns/{cid}/characters")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_delete_character() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/characters"),
            json!({"name": "Aria", "character_type": "pc", "max_hp": 30, "armor_class": 14}),
        ))
        .await
        .unwrap();
    let char_id = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(delete(&format!("/campaigns/{cid}/characters/{char_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(get(&format!("/campaigns/{cid}/characters/{char_id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Sessions ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_session() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .oneshot(post_json(
            &format!("/campaigns/{cid}/sessions"),
            json!({"title": "First Session"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = json_body(resp).await;
    assert_eq!(body["session_number"], 1);
}

#[tokio::test]
async fn test_list_sessions_empty() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .oneshot(get(&format!("/campaigns/{cid}/sessions")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_start_and_end_session() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/sessions"),
            json!({"title": "Live"}),
        ))
        .await
        .unwrap();
    let sid = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/campaigns/{cid}/sessions/{sid}/start"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body["started_at"].as_str().is_some());

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/campaigns/{cid}/sessions/{sid}/end"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body["ended_at"].as_str().is_some());
}

#[tokio::test]
async fn test_create_session_event() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/sessions"),
            json!({"title": "Test"}),
        ))
        .await
        .unwrap();
    let sid = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/sessions/{sid}/events"),
            json!({"event_type": "combat", "description": "Battle erupted"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(get(&format!("/campaigns/{cid}/sessions/{sid}/events")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_session_summary_no_events() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/sessions"),
            json!({"title": "Empty"}),
        ))
        .await
        .unwrap();
    let sid = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .oneshot(get(&format!("/campaigns/{cid}/sessions/{sid}/summary")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_delete_session() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/sessions"),
            json!({"title": "Test"}),
        ))
        .await
        .unwrap();
    let sid = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(delete(&format!("/campaigns/{cid}/sessions/{sid}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .oneshot(get(&format!("/campaigns/{cid}/sessions/{sid}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Encounters ───────────────────────────────────────────────────────────────

async fn setup_encounter(app: &axum::Router) -> (String, String, String) {
    let cid = create_campaign(app).await;

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/characters"),
            json!({"name": "Warrior", "character_type": "pc", "max_hp": 40, "armor_class": 16}),
        ))
        .await
        .unwrap();
    let char_id = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/sessions"),
            json!({"title": "Session 1"}),
        ))
        .await
        .unwrap();
    let sid = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/campaigns/{cid}/encounters"),
            json!({"session_id": sid, "participant_character_ids": [char_id]}),
        ))
        .await
        .unwrap();
    let enc_id = json_body(resp).await["id"].as_str().unwrap().to_string();

    (cid, sid, enc_id)
}

#[tokio::test]
async fn test_start_encounter() {
    let app = make_app().await;
    let (cid, _sid, enc_id) = setup_encounter(&app).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/campaigns/{cid}/encounters/{enc_id}/start"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.get("encounter").is_some());
    assert_eq!(body["round"], 1);
}

#[tokio::test]
async fn test_next_turn() {
    let app = make_app().await;
    let (cid, _sid, enc_id) = setup_encounter(&app).await;

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/campaigns/{cid}/encounters/{enc_id}/start"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/campaigns/{cid}/encounters/{enc_id}/next-turn"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.get("round").is_some());
}

#[tokio::test]
async fn test_end_encounter() {
    let app = make_app().await;
    let (cid, _sid, enc_id) = setup_encounter(&app).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/campaigns/{cid}/encounters/{enc_id}/end"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["status"], "completed");
}

// ── Generate Encounter ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_generate_encounter() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .oneshot(post_json(
            &format!("/campaigns/{cid}/encounters/generate"),
            json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert!(body.get("title").is_some());
    assert!(body.get("description").is_some());
    assert!(body.get("narrative_hook").is_some());
}

// ── Chat ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_chat_empty_message() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .oneshot(post_json(
            &format!("/campaigns/{cid}/chat"),
            json!({"message": ""}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_chat_success() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .oneshot(post_json(
            &format!("/campaigns/{cid}/chat"),
            json!({"message": "What is happening?"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("text/event-stream"));
}

// ── Documents ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_documents_empty() {
    let app = make_app().await;
    let cid = create_campaign(&app).await;

    let resp = app
        .oneshot(get(&format!("/campaigns/{cid}/documents")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 0);
}
