use std::collections::HashMap;

use guide_core::{
    models::{DocumentKind, RankedChunk},
    GuideError, Result,
};
use qdrant_client::{
    qdrant::{
        Condition, CreateCollectionBuilder, Distance, Filter, PointStruct, SearchParamsBuilder,
        SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
    },
    Qdrant,
};
use uuid::Uuid;

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

pub async fn ensure_collection(client: &Qdrant, name: &str, vector_size: u64) -> Result<()> {
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

pub async fn upsert_chunks(
    client: &Qdrant,
    collection: &str,
    chunks: Vec<LoreChunkInsert>,
) -> Result<()> {
    for batch in chunks.chunks(100) {
        let points: Vec<PointStruct> = batch
            .iter()
            .map(|chunk| {
                let entities_json =
                    serde_json::to_string(&chunk.entities).unwrap_or_else(|_| "[]".into());
                let doc_kind_str = match chunk.document_kind {
                    DocumentKind::Campaign => "campaign",
                    DocumentKind::Rulebook => "rulebook",
                };

                let mut payload: HashMap<String, qdrant_client::qdrant::Value> = HashMap::new();
                payload.insert("content".into(), chunk.content.clone().into());
                payload.insert(
                    "source_document_id".into(),
                    chunk.source_document_id.to_string().into(),
                );
                payload.insert("document_kind".into(), doc_kind_str.into());
                payload.insert("lore_type".into(), chunk.lore_type.clone().into());
                payload.insert("significance".into(), chunk.significance.clone().into());
                payload.insert("entities".into(), entities_json.into());
                payload.insert("is_player_visible".into(), chunk.is_player_visible.into());
                payload
                    .insert("page_start".into(), (chunk.page_range.0 as i64).into());
                payload.insert("page_end".into(), (chunk.page_range.1 as i64).into());
                payload
                    .insert("section_path".into(), chunk.section_path.clone().into());
                payload.insert("doc_title".into(), chunk.doc_title.clone().into());

                PointStruct::new(chunk.id.to_string(), chunk.vector.clone(), payload)
            })
            .collect();

        client
            .upsert_points(UpsertPointsBuilder::new(collection, points))
            .await
            .map_err(|e| GuideError::Qdrant(e.to_string()))?;
    }
    Ok(())
}

pub async fn query_chunks(
    client: &Qdrant,
    collection: &str,
    embedding: Vec<f32>,
    limit: usize,
    player_visible_only: bool,
) -> Result<Vec<RankedChunk>> {
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
        builder =
            builder.filter(Filter::must([Condition::matches("is_player_visible", true)]));
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
            Some(RankedChunk {
                content,
                section_path,
                doc_title,
                score: scored.score,
            })
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
