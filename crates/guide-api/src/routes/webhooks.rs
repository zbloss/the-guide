use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use guide_core::models::{CampaignWebhook, CreateWebhookRequest};
use guide_db::{SqlitePool, WebhookRepository};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/campaigns/{campaign_id}/webhooks",
            get(list_webhooks).post(create_webhook),
        )
        .route(
            "/campaigns/{campaign_id}/webhooks/{webhook_id}",
            axum::routing::delete(delete_webhook),
        )
}

#[utoipa::path(
    get,
    path = "/campaigns/{campaign_id}/webhooks",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    responses(
        (status = 200, description = "List all webhooks for a campaign", body = [CampaignWebhook])
    )
)]
pub async fn list_webhooks(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = WebhookRepository::new(&state.db);
    Ok(Json(repo.list_by_campaign(campaign_id).await?))
}

#[utoipa::path(
    post,
    path = "/campaigns/{campaign_id}/webhooks",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID")
    ),
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook created", body = CampaignWebhook)
    )
)]
pub async fn create_webhook(
    State(state): State<AppState>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<CreateWebhookRequest>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = WebhookRepository::new(&state.db);
    Ok((StatusCode::CREATED, Json(repo.create(campaign_id, &req).await?)))
}

#[utoipa::path(
    delete,
    path = "/campaigns/{campaign_id}/webhooks/{webhook_id}",
    params(
        ("campaign_id" = Uuid, Path, description = "Campaign ID"),
        ("webhook_id" = Uuid, Path, description = "Webhook ID")
    ),
    responses(
        (status = 204, description = "Webhook deleted"),
        (status = 404, description = "Webhook not found")
    )
)]
pub async fn delete_webhook(
    State(state): State<AppState>,
    Path((_campaign_id, webhook_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, crate::error::AppError> {
    let repo = WebhookRepository::new(&state.db);
    repo.delete(webhook_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Fire webhooks for a campaign event in a background task (non-blocking).
pub fn fire_webhooks(
    pool: SqlitePool,
    campaign_id: Uuid,
    event: String,
    payload: serde_json::Value,
) {
    tokio::spawn(async move {
        let repo = WebhookRepository::new(&pool);
        let webhooks = match repo.list_by_event(campaign_id, &event).await {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("Failed to list webhooks for event {}: {}", event, e);
                return;
            }
        };

        if webhooks.is_empty() {
            return;
        }

        let client = match reqwest::Client::builder().build() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to build reqwest client: {}", e);
                return;
            }
        };

        // Build Discord-compatible content message
        let content = match event.as_str() {
            "session_start" => format!(
                "Session started: {}",
                payload.get("session_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Session")
            ),
            "session_end" => format!(
                "Session ended: {}",
                payload.get("session_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Session")
            ),
            "encounter_end" => format!(
                "Encounter ended: {}",
                payload.get("encounter_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Encounter")
            ),
            _ => format!("Event: {event}"),
        };

        let discord_payload = serde_json::json!({ "content": content });

        for webhook in webhooks {
            match client
                .post(&webhook.url)
                .json(&discord_payload)
                .send()
                .await
            {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        tracing::warn!(
                            "Webhook {} returned non-success status: {}",
                            webhook.id,
                            resp.status()
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fire webhook {}: {}", webhook.id, e);
                }
            }
        }
    });
}
