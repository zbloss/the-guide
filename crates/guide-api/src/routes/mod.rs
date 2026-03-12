pub mod calendar;
pub mod campaigns;
pub mod characters;
pub mod chat;
pub mod documents;
pub mod encounters;
pub mod factions;
pub mod generate;
pub mod health;
pub mod homebrew;
pub mod loot;
pub mod openapi;
pub mod sessions;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::state::AppState;
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;

pub fn all_routes(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(campaigns::router())
        .merge(characters::router())
        .merge(sessions::router())
        .merge(encounters::router())
        .merge(documents::router())
        .merge(generate::router())
        .merge(chat::router())
        .merge(calendar::router())
        .merge(loot::router())
        .merge(homebrew::router())
        .merge(factions::router())
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
