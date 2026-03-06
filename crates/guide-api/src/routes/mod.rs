pub mod campaigns;
pub mod characters;
pub mod chat;
pub mod documents;
pub mod encounters;
pub mod generate;
pub mod global_documents;
pub mod health;
pub mod sessions;

use axum::Router;

use crate::state::AppState;

pub fn all_routes(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(campaigns::router())
        .merge(characters::router())
        .merge(sessions::router())
        .merge(encounters::router())
        .merge(documents::router())
        .merge(global_documents::router())
        .merge(generate::router())
        .merge(chat::router())
        .with_state(state)
}
