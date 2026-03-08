use std::path::Path;
use std::sync::Arc;

use guide_core::{
    models::{CampaignDocument, DocSummary, DocumentKind, GlobalDocument, MetaIndex},
    AppConfig, Result,
};
use guide_db::{
    documents::{DocumentRepository, GlobalDocumentRepository},
    qdrant::{
        self, campaign_collection_name, ensure_collection, global_collection_name, LoreChunkInsert,
    },
    SqlitePool,
};
use guide_llm::{EmbeddingRequest, LlmClient};
use qdrant_client::Qdrant;
use uuid::Uuid;

use crate::{chunker, extractor};

const META_INDEX_DIR: &str = "data/indexes";

pub async fn ingest_campaign_document(
    pdf_path: &Path,
    doc: &CampaignDocument,
    llm: Arc<dyn LlmClient>,
    config: &AppConfig,
    qdrant: Option<&Qdrant>,
    db: &SqlitePool,
) -> Result<usize> {
    let doc_repo = DocumentRepository::new(db);
    doc_repo
        .update_status(doc.id, &guide_core::models::IngestionStatus::Processing, None)
        .await?;

    let pages = extractor::extract_pages(
        pdf_path,
        &doc.document_kind,
        llm.as_ref(),
        &config.ocr_model,
        0,
    )
    .await?;

    let page_count = pages.len() as i32;

    let chunks = chunker::chunk_document(pages, config.chunk_max_chars, config.chunk_overlap_chars)
        .await?;

    let chunk_count = chunks.len();

    if let Some(q) = qdrant {
        let collection = campaign_collection_name(&doc.campaign_id.to_string());
        ensure_collection(q, &collection, config.embedding_dims).await?;

        let mut lore_chunks = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            let embedding = llm
                .embed(EmbeddingRequest {
                    text: chunk.content.clone(),
                    model_override: None,
                })
                .await?;

            // Deterministic UUID v5: namespace = doc_id, name = section_path + content hash
            let chunk_id = Uuid::new_v5(
                &doc.id,
                format!("{}:{}", chunk.section_path, &chunk.content[..chunk.content.len().min(64)]).as_bytes(),
            );

            lore_chunks.push(LoreChunkInsert {
                id: chunk_id,
                campaign_id: Some(doc.campaign_id),
                source_document_id: doc.id,
                document_kind: doc.document_kind.clone(),
                content: chunk.content.clone(),
                lore_type: "plot".to_string(),
                significance: "minor".to_string(),
                entities: Vec::new(),
                is_player_visible: chunk.is_player_visible,
                page_range: chunk.page_range,
                section_path: chunk.section_path.clone(),
                doc_title: doc.filename.clone(),
                vector: embedding,
            });
        }

        qdrant::upsert_chunks(q, &collection, lore_chunks).await?;
    }

    doc_repo.update_ingested(doc.id, Some(page_count)).await?;

    // Update MetaIndex
    let scope = doc.campaign_id.to_string();
    let excerpt = chunks
        .first()
        .map(|c| c.content.chars().take(2000).collect::<String>())
        .unwrap_or_default();
    let summary = generate_doc_summary(llm.as_ref(), &doc.filename, &excerpt).await;
    let doc_summary = DocSummary {
        doc_id: doc.id,
        doc_name: doc.filename.clone(),
        filename: doc.filename.clone(),
        summary,
        scope: scope.clone(),
        ingested_at: chrono::Utc::now(),
    };
    add_to_meta_index(&scope, doc_summary).await?;

    Ok(chunk_count)
}

pub async fn ingest_global_document(
    pdf_path: &Path,
    doc: &GlobalDocument,
    llm: Arc<dyn LlmClient>,
    config: &AppConfig,
    qdrant: Option<&Qdrant>,
    db: &SqlitePool,
) -> Result<usize> {
    let doc_repo = GlobalDocumentRepository::new(db);
    doc_repo
        .update_status(doc.id, &guide_core::models::IngestionStatus::Processing, None)
        .await?;

    let pages = extractor::extract_pages(
        pdf_path,
        &DocumentKind::Rulebook,
        llm.as_ref(),
        &config.ocr_model,
        0,
    )
    .await?;

    let page_count = pages.len() as i32;

    let chunks = chunker::chunk_document(pages, config.chunk_max_chars, config.chunk_overlap_chars)
        .await?;

    let chunk_count = chunks.len();

    if let Some(q) = qdrant {
        let collection = global_collection_name();
        ensure_collection(q, collection, config.embedding_dims).await?;

        let mut lore_chunks = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            let embedding = llm
                .embed(EmbeddingRequest {
                    text: chunk.content.clone(),
                    model_override: None,
                })
                .await?;

            let chunk_id = Uuid::new_v5(
                &doc.id,
                format!("{}:{}", chunk.section_path, &chunk.content[..chunk.content.len().min(64)]).as_bytes(),
            );

            lore_chunks.push(LoreChunkInsert {
                id: chunk_id,
                campaign_id: None,
                source_document_id: doc.id,
                document_kind: DocumentKind::Rulebook,
                content: chunk.content.clone(),
                lore_type: "mechanic".to_string(),
                significance: "minor".to_string(),
                entities: Vec::new(),
                is_player_visible: true,
                page_range: chunk.page_range,
                section_path: chunk.section_path.clone(),
                doc_title: doc.title.clone(),
                vector: embedding,
            });
        }

        qdrant::upsert_chunks(q, collection, lore_chunks).await?;
    }

    doc_repo.update_ingested(doc.id, Some(page_count)).await?;

    let excerpt = chunks
        .first()
        .map(|c| c.content.chars().take(2000).collect::<String>())
        .unwrap_or_default();
    let summary = generate_doc_summary(llm.as_ref(), &doc.title, &excerpt).await;
    let doc_summary = DocSummary {
        doc_id: doc.id,
        doc_name: doc.title.clone(),
        filename: doc.filename.clone(),
        summary,
        scope: "global".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    add_to_meta_index("global", doc_summary).await?;

    Ok(chunk_count)
}

pub async fn query_indexes(
    query: &str,
    campaign_id: Option<Uuid>,
    player_visible_only: bool,
    llm: &dyn LlmClient,
    _config: &AppConfig,
    qdrant: Option<&Qdrant>,
) -> Result<Vec<guide_core::models::RankedChunk>> {
    let embedding = llm
        .embed(EmbeddingRequest {
            text: query.to_string(),
            model_override: None,
        })
        .await?;

    let Some(q) = qdrant else {
        return Ok(Vec::new());
    };

    let mut results = Vec::new();

    if let Some(cid) = campaign_id {
        let collection = campaign_collection_name(&cid.to_string());
        let mut campaign_chunks = qdrant::query_chunks(
            q,
            &collection,
            embedding.clone(),
            5,
            player_visible_only,
        )
        .await?;
        results.append(&mut campaign_chunks);
    }

    let mut global_chunks =
        qdrant::query_chunks(q, global_collection_name(), embedding, 3, false).await?;
    results.append(&mut global_chunks);

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(results)
}

pub async fn load_meta_index(scope: &str) -> Result<MetaIndex> {
    let path = meta_index_path(scope);
    match tokio::fs::read_to_string(&path).await {
        Ok(s) => Ok(serde_json::from_str(&s)?),
        Err(_) => Ok(MetaIndex {
            scope: scope.to_string(),
            entries: Vec::new(),
        }),
    }
}

pub async fn save_meta_index(index: &MetaIndex) -> Result<()> {
    let path = meta_index_path(&index.scope);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(index)?;
    tokio::fs::write(&path, json).await?;
    Ok(())
}

pub async fn add_to_meta_index(scope: &str, doc_summary: DocSummary) -> Result<()> {
    let mut index = load_meta_index(scope).await?;
    index.entries.retain(|e| e.doc_id != doc_summary.doc_id);
    index.entries.push(doc_summary);
    save_meta_index(&index).await
}

async fn generate_doc_summary(llm: &dyn LlmClient, doc_name: &str, excerpt: &str) -> String {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let prompt = guide_llm::prompts::doc_summary_prompt(doc_name, excerpt);
    let req = CompletionRequest {
        task: LlmTask::General,
        messages: vec![Message {
            role: MessageRole::User,
            content: prompt,
        }],
        model_override: None,
        temperature: Some(0.3),
        max_tokens: Some(200),
    };

    match llm.complete(req).await {
        Ok(resp) => resp.content,
        Err(_) => String::new(),
    }
}

fn meta_index_path(scope: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(META_INDEX_DIR)
        .join(scope)
        .join("meta.json")
}
