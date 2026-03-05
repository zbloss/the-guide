use std::collections::HashMap;

use guide_core::{GuideError, Result};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, UpsertPointsBuilder, VectorParamsBuilder,
};
use uuid::Uuid;

/// Create a per-campaign Qdrant collection.
/// Collection name: `campaign_{uuid}_lore`
pub async fn create_campaign_collection(
    client: &Qdrant,
    campaign_id: &str,
    vector_size: u64,
) -> Result<()> {
    let collection_name = campaign_collection_name(campaign_id);

    let exists = client
        .collection_exists(&collection_name)
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;

    if exists {
        return Ok(());
    }

    client
        .create_collection(
            CreateCollectionBuilder::new(&collection_name)
                .vectors_config(VectorParamsBuilder::new(vector_size, Distance::Cosine)),
        )
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;

    Ok(())
}

/// Drop the Qdrant collection when a campaign is deleted.
pub async fn delete_campaign_collection(client: &Qdrant, campaign_id: &str) -> Result<()> {
    let collection_name = campaign_collection_name(campaign_id);

    let exists = client
        .collection_exists(&collection_name)
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;

    if !exists {
        return Ok(());
    }

    client
        .delete_collection(collection_name)
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;

    Ok(())
}

/// A lore chunk to upsert into Qdrant.
pub struct LoreChunkInsert {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub source_document_id: Uuid,
    pub content: String,
    pub lore_type: String,
    pub significance: String,
    pub entities: Vec<String>,
    pub is_player_visible: bool,
    pub vector: Vec<f32>,
}

/// Upsert a single lore chunk into the campaign's Qdrant collection.
pub async fn upsert_lore_chunk(
    client: &Qdrant,
    campaign_id: &str,
    chunk: LoreChunkInsert,
) -> Result<()> {
    let collection_name = campaign_collection_name(campaign_id);

    let entities_json = serde_json::to_string(&chunk.entities).unwrap_or_else(|_| "[]".into());

    let mut payload: HashMap<String, qdrant_client::qdrant::Value> = HashMap::new();
    payload.insert("content".to_string(), chunk.content.into());
    payload.insert(
        "source_document_id".to_string(),
        chunk.source_document_id.to_string().into(),
    );
    payload.insert("lore_type".to_string(), chunk.lore_type.into());
    payload.insert("significance".to_string(), chunk.significance.into());
    payload.insert("entities".to_string(), entities_json.into());
    payload.insert("is_player_visible".to_string(), chunk.is_player_visible.into());

    let point = PointStruct::new(chunk.id.to_string(), chunk.vector, payload);

    client
        .upsert_points(UpsertPointsBuilder::new(&collection_name, vec![point]))
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;

    Ok(())
}

pub fn campaign_collection_name(campaign_id: &str) -> String {
    format!("campaign_{campaign_id}_lore")
}

/// Attempt to connect to Qdrant — returns None if unavailable (allows startup without Qdrant).
pub async fn try_connect(url: &str) -> Option<Qdrant> {
    match Qdrant::from_url(url).build() {
        Ok(client) => match client.health_check().await {
            Ok(_) => {
                tracing::info!("Connected to Qdrant at {url}");
                Some(client)
            }
            Err(e) => {
                tracing::warn!("Qdrant unreachable at {url}: {e}. Vector search disabled.");
                None
            }
        },
        Err(e) => {
            tracing::warn!("Failed to build Qdrant client: {e}");
            None
        }
    }
}
