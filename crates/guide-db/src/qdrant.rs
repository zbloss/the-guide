use std::collections::HashMap;

use guide_core::{GuideError, Result, models::{DocumentKind, RankedChunk}};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, Distance, Filter, PointStruct,
    SearchParamsBuilder, SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
};
use uuid::Uuid;

/// Create a per-campaign Qdrant collection.
/// Collection name: `campaign_{uuid}_lore`
pub async fn create_campaign_collection(
    client: &Qdrant,
    campaign_id: &str,
    vector_size: u64,
) -> Result<()> {
    ensure_collection(client, &campaign_collection_name(campaign_id), vector_size).await
}

/// Create the global rulebook collection if it doesn't exist.
pub async fn create_global_collection(client: &Qdrant, vector_size: u64) -> Result<()> {
    ensure_collection(client, global_collection_name(), vector_size).await
}

async fn ensure_collection(client: &Qdrant, name: &str, vector_size: u64) -> Result<()> {
    let exists = client
        .collection_exists(name)
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;

    if exists {
        return Ok(());
    }

    client
        .create_collection(
            CreateCollectionBuilder::new(name)
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
    pub campaign_id: Option<Uuid>,
    pub source_document_id: Uuid,
    pub document_kind: DocumentKind,
    pub content: String,
    pub lore_type: String,
    pub significance: String,
    pub entities: Vec<String>,
    pub is_player_visible: bool,
    pub page_range: (u32, u32),
    pub section_path: String,
    pub doc_title: String,
    pub vector: Vec<f32>,
}

/// Upsert a single lore chunk into the specified Qdrant collection.
pub async fn upsert_lore_chunk(
    client: &Qdrant,
    collection: &str,
    chunk: LoreChunkInsert,
) -> Result<()> {
    let entities_json = serde_json::to_string(&chunk.entities).unwrap_or_else(|_| "[]".into());
    let doc_kind_str = match chunk.document_kind {
        DocumentKind::Campaign => "campaign",
        DocumentKind::Rulebook => "rulebook",
    };

    let mut payload: HashMap<String, qdrant_client::qdrant::Value> = HashMap::new();
    payload.insert("content".to_string(), chunk.content.into());
    payload.insert(
        "source_document_id".to_string(),
        chunk.source_document_id.to_string().into(),
    );
    payload.insert("document_kind".to_string(), doc_kind_str.into());
    payload.insert("lore_type".to_string(), chunk.lore_type.into());
    payload.insert("significance".to_string(), chunk.significance.into());
    payload.insert("entities".to_string(), entities_json.into());
    payload.insert("is_player_visible".to_string(), chunk.is_player_visible.into());
    payload.insert("page_start".to_string(), (chunk.page_range.0 as i64).into());
    payload.insert("page_end".to_string(), (chunk.page_range.1 as i64).into());
    payload.insert("section_path".to_string(), chunk.section_path.into());
    payload.insert("doc_title".to_string(), chunk.doc_title.into());

    let point = PointStruct::new(chunk.id.to_string(), chunk.vector, payload);

    client
        .upsert_points(UpsertPointsBuilder::new(collection, vec![point]))
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;

    Ok(())
}

/// Search campaign-specific lore collection, with optional player-visibility filter.
pub async fn search_campaign_lore(
    client: &Qdrant,
    campaign_id: &str,
    embedding: Vec<f32>,
    limit: usize,
    player_visible_only: bool,
) -> Result<Vec<RankedChunk>> {
    let collection = campaign_collection_name(campaign_id);
    search_collection(client, &collection, embedding, limit, player_visible_only).await
}

/// Search the global rulebook collection (no spoiler filter applied).
pub async fn search_global_rules(
    client: &Qdrant,
    embedding: Vec<f32>,
    limit: usize,
) -> Result<Vec<RankedChunk>> {
    search_collection(client, global_collection_name(), embedding, limit, false).await
}

async fn search_collection(
    client: &Qdrant,
    collection: &str,
    embedding: Vec<f32>,
    limit: usize,
    player_visible_only: bool,
) -> Result<Vec<RankedChunk>> {
    // Collection may not exist yet — treat as empty rather than error
    let exists = client
        .collection_exists(collection)
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;
    if !exists {
        return Ok(Vec::new());
    }

    let mut builder = SearchPointsBuilder::new(collection, embedding, limit as u64)
        .with_payload(true)
        .params(SearchParamsBuilder::default().hnsw_ef(128).exact(false));

    if player_visible_only {
        builder = builder.filter(Filter::must([Condition::matches("is_player_visible", true)]));
    }

    let results = client
        .search_points(builder)
        .await
        .map_err(|e| GuideError::Qdrant(e.to_string()))?;

    let chunks = results
        .result
        .into_iter()
        .filter_map(|scored| {
            let content = scored.payload.get("content")?.as_str()?.to_string();
            let section_path = scored
                .payload
                .get("section_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let doc_title = scored
                .payload
                .get("doc_title")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            Some(RankedChunk { content, section_path, doc_title, score: scored.score })
        })
        .collect();

    Ok(chunks)
}

pub fn campaign_collection_name(campaign_id: &str) -> String {
    format!("campaign_{campaign_id}_lore")
}

pub fn global_collection_name() -> &'static str {
    "global_rules"
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
