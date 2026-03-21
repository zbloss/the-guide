use std::path::Path;
use std::sync::Arc;

use guide_core::{
    models::{CampaignDocument, DocSummary, DocumentKind, GlobalDocument, MetaIndex, StoryExtractionResult},
    AppConfig, GuideError, Result,
};
use guide_db::{
    documents::{DocumentRepository, GlobalDocumentRepository},
    qdrant::{
        self, campaign_collection_name, ensure_collection, global_collection_name, LoreChunkInsert,
    },
    story::StoryRepository,
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

    // Persist raw OCR text before chunking so the preview modal has data even
    // if chunking or embedding fails later.
    let page_tuples: Vec<(u32, String, bool)> = pages
        .iter()
        .map(|p| (p.page_num, p.raw_text.clone(), p.is_dm_only))
        .collect();
    doc_repo.save_page_ocr(doc.id, &page_tuples).await?;

    let chunks = chunker::chunk_document(pages, config.chunk_max_chars, config.chunk_overlap_chars)
        .await?;

    let chunk_count = chunks.len();

    if let Some(q) = qdrant {
        let collection = campaign_collection_name(&doc.campaign_id.to_string());
        ensure_collection(q, &collection, config.embedding_dims).await?;

        let mut lore_chunks = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            let embed_text = truncate_for_embed(&chunk.content);
            let embedding = match llm.embed(EmbeddingRequest { text: embed_text, model_override: None }).await {
                Ok(v) => v,
                Err(e) => { tracing::warn!("Embed skipped for chunk in {}: {e}", chunk.section_path); continue; }
            };

            // Deterministic UUID v5: namespace = doc_id, name = section_path + content hash
            let chunk_id = Uuid::new_v5(
                &doc.id,
                format!("{}:{}", chunk.section_path, &chunk.content[..chunk.content.floor_char_boundary(64)]).as_bytes(),
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

    // Story extraction for Campaign / Supplemental docs
    if matches!(
        doc.document_kind,
        DocumentKind::Campaign | DocumentKind::Supplemental
    ) {
        if let Err(e) = extract_story(doc, &chunks, llm.clone(), db).await {
            tracing::error!("Story extraction failed for doc {}: {e}", doc.id);
            let _ = DocumentRepository::new(db)
                .update_story_extraction_status(doc.id, "failed", Some(&e.to_string()))
                .await;
        }
    }

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
        &DocumentKind::DmGuide,
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
            let embed_text = truncate_for_embed(&chunk.content);
            let embedding = match llm.embed(EmbeddingRequest { text: embed_text, model_override: None }).await {
                Ok(v) => v,
                Err(e) => { tracing::warn!("Embed skipped for chunk in {}: {e}", chunk.section_path); continue; }
            };

            let chunk_id = Uuid::new_v5(
                &doc.id,
                format!("{}:{}", chunk.section_path, &chunk.content[..chunk.content.floor_char_boundary(64)]).as_bytes(),
            );

            lore_chunks.push(LoreChunkInsert {
                id: chunk_id,
                campaign_id: None,
                source_document_id: doc.id,
                document_kind: DocumentKind::DmGuide,
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
        json_mode: false,
    };

    match llm.complete(req).await {
        Ok(resp) => resp.content,
        Err(_) => String::new(),
    }
}

/// Truncate text to a char boundary safe for embedding models (≤8 000 chars ≈ 2 000 tokens).
fn truncate_for_embed(text: &str) -> String {
    const MAX_CHARS: usize = 8_000;
    if text.len() <= MAX_CHARS {
        text.to_string()
    } else {
        text[..text.floor_char_boundary(MAX_CHARS)].to_string()
    }
}

fn meta_index_path(scope: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(META_INDEX_DIR)
        .join(scope)
        .join("meta.json")
}

fn chapter_name_of(chunk: &crate::chunker::DocumentChunk) -> String {
    chunk
        .section_path
        .split(" > ")
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "(Introduction)".to_string())
}

fn group_chunks_by_chapter(chunks: &[crate::chunker::DocumentChunk]) -> Vec<(String, Vec<&crate::chunker::DocumentChunk>)> {
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::HashMap<String, Vec<&crate::chunker::DocumentChunk>> = std::collections::HashMap::new();

    for chunk in chunks {
        let name = chapter_name_of(chunk);
        if !map.contains_key(&name) {
            order.push(name.clone());
        }
        map.entry(name).or_default().push(chunk);
    }

    order.into_iter().map(|name| {
        let group = map.remove(&name).unwrap_or_default();
        (name, group)
    }).collect()
}

fn join_chunks(chunks: &[&crate::chunker::DocumentChunk]) -> String {
    chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join("\n\n")
}

fn join_and_truncate(chunks: &[&crate::chunker::DocumentChunk], max_chars: usize) -> String {
    let text = join_chunks(chunks);
    if text.len() <= max_chars {
        text
    } else {
        text[..text.floor_char_boundary(max_chars)].to_string()
    }
}

/// Split a chapter's chunks into windows each fitting within `max_chars`.
/// Always emits at least one window (even if a single chunk exceeds the limit).
fn windows_for_chapter<'a>(
    chunks: &[&'a crate::chunker::DocumentChunk],
    max_chars: usize,
) -> Vec<Vec<&'a crate::chunker::DocumentChunk>> {
    let mut windows: Vec<Vec<&crate::chunker::DocumentChunk>> = Vec::new();
    let mut current: Vec<&crate::chunker::DocumentChunk> = Vec::new();
    let mut current_len = 0usize;

    for chunk in chunks {
        let chunk_len = chunk.content.len();
        // If adding this chunk would overflow and we already have something, flush first.
        if !current.is_empty() && current_len + 2 + chunk_len > max_chars {
            windows.push(std::mem::take(&mut current));
            current_len = 0;
        }
        current_len += if current.is_empty() { chunk_len } else { 2 + chunk_len };
        current.push(chunk);
    }
    if !current.is_empty() {
        windows.push(current);
    }
    windows
}

async fn extract_story_for_chapter(
    text: &str,
    doc_title: &str,
    chapter_name: &str,
    llm: &dyn LlmClient,
) -> Result<StoryExtractionResult> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let req = CompletionRequest {
        task: LlmTask::StoryExtraction,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: guide_llm::prompts::story_extraction_system().to_string(),
            },
            Message {
                role: MessageRole::User,
                content: guide_llm::prompts::story_extraction_user_chapter(text, doc_title, chapter_name),
            },
        ],
        model_override: None,
        temperature: Some(0.3),
        max_tokens: Some(8192),
        json_mode: true,
    };

    let resp = llm.complete(req).await?;
    let trimmed = resp.content.trim();
    if let Err(ref e) = serde_json::from_str::<StoryExtractionResult>(trimmed) {
        tracing::warn!(
            chapter = chapter_name,
            error = %e,
            response_preview = &trimmed[..trimmed.floor_char_boundary(500)],
            "Story extraction JSON parse failed"
        );
    }
    serde_json::from_str(trimmed)
        .map_err(|e| GuideError::Internal(format!("Story extraction parse failed for chapter '{chapter_name}': {e}")))
}

fn merge_story_extractions(chapter_results: Vec<(usize, StoryExtractionResult)>) -> StoryExtractionResult {
    use guide_core::models::{ArcPoint, CharacterArcInput, StoryArcInput};
    use std::collections::HashMap;

    let mut arc_order_seq = 1i32;
    let mut arc_seen: Vec<String> = Vec::new();
    let mut arcs: Vec<StoryArcInput> = Vec::new();

    let mut events = Vec::new();
    let mut subplots = Vec::new();
    let mut encounters = Vec::new();

    let mut char_arc_order: HashMap<String, Vec<(String, Vec<ArcPoint>)>> = HashMap::new();
    let mut char_arc_desc: HashMap<String, String> = HashMap::new();
    let mut char_arc_insert_order: Vec<String> = Vec::new();

    for (chapter_idx, result) in chapter_results {
        for mut arc in result.arcs {
            let key = arc.title.trim().to_lowercase();
            if !arc_seen.contains(&key) {
                arc_seen.push(key);
                arc.arc_order = arc_order_seq;
                arc_order_seq += 1;
                arcs.push(arc);
            }
        }

        for mut event in result.events {
            event.event_order += chapter_idx as i32 * 1000;
            events.push(event);
        }

        for subplot in result.subplots {
            subplots.push(subplot);
        }

        for enc in result.encounters {
            encounters.push(enc);
        }

        for char_arc in result.character_arcs {
            let key = char_arc.character_name.trim().to_lowercase();
            if !char_arc_insert_order.contains(&key) {
                char_arc_insert_order.push(key.clone());
                char_arc_desc.insert(key.clone(), char_arc.description);
            }
            char_arc_order
                .entry(key)
                .or_default()
                .push((char_arc.character_name, char_arc.arc_points));
        }
    }

    let character_arcs = char_arc_insert_order
        .into_iter()
        .map(|key| {
            let entries = char_arc_order.remove(&key).unwrap_or_default();
            let character_name = entries.first().map(|(n, _)| n.clone()).unwrap_or_default();
            let description = char_arc_desc.remove(&key).unwrap_or_default();
            let mut order_seq = 1i32;
            let arc_points = entries
                .into_iter()
                .flat_map(|(_, pts)| pts)
                .map(|mut pt| { pt.order = order_seq; order_seq += 1; pt })
                .collect();
            CharacterArcInput { character_name, description, arc_points }
        })
        .collect();

    StoryExtractionResult { arcs, events, subplots, character_arcs, encounters }
}

pub async fn extract_story(
    doc: &CampaignDocument,
    chunks: &[crate::chunker::DocumentChunk],
    llm: Arc<dyn LlmClient>,
    db: &SqlitePool,
) -> Result<()> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};
    use std::collections::HashMap;

    let doc_repo = DocumentRepository::new(db);
    doc_repo
        .update_story_extraction_status(doc.id, "processing", None)
        .await?;

    let chapter_groups = group_chunks_by_chapter(chunks);

    let extraction: StoryExtractionResult = if chapter_groups.len() < 2 {
        // Fallback: original single-call path
        let full_text = join_and_truncate(&chunks.iter().collect::<Vec<_>>(), 40_000);
        let req = CompletionRequest {
            task: LlmTask::StoryExtraction,
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: guide_llm::prompts::story_extraction_system().to_string(),
                },
                Message {
                    role: MessageRole::User,
                    content: guide_llm::prompts::story_extraction_user(&full_text, &doc.filename),
                },
            ],
            model_override: None,
            temperature: Some(0.3),
            max_tokens: Some(8192),
            json_mode: true,
        };
        let resp = llm.complete(req).await?;
        let trimmed = resp.content.trim();
        if let Err(ref e) = serde_json::from_str::<StoryExtractionResult>(trimmed) {
            tracing::warn!(
                error = %e,
                response_preview = &trimmed[..trimmed.floor_char_boundary(500)],
                "Story extraction JSON parse failed"
            );
        }
        serde_json::from_str(trimmed)
            .map_err(|e| GuideError::Internal(format!("Story extraction parse failed: {e}")))?
    } else {
        // Chapter path: one LLM call per window within each chapter.
        // A flat call_idx drives event ordering so windows of the same chapter
        // stay in order relative to windows of other chapters.
        let mut results: Vec<(usize, StoryExtractionResult)> = Vec::new();
        let mut call_idx = 0usize;
        for (chapter_name, chapter_chunks) in &chapter_groups {
            let windows = windows_for_chapter(chapter_chunks, 20_000);
            let window_count = windows.len();
            for (w_idx, window) in windows.into_iter().enumerate() {
                let text = join_chunks(&window);
                let label = if window_count > 1 {
                    format!("{chapter_name} (part {}/{})", w_idx + 1, window_count)
                } else {
                    chapter_name.clone()
                };
                match extract_story_for_chapter(&text, &doc.filename, chapter_name, llm.as_ref()).await {
                    Ok(r) => results.push((call_idx, r)),
                    Err(e) => tracing::warn!("Window '{label}' extraction failed: {e}; skipping"),
                }
                call_idx += 1;
            }
        }
        if results.is_empty() {
            return Err(GuideError::Internal("All chapter extractions failed".into()));
        }
        merge_story_extractions(results)
    };

    if extraction.arcs.is_empty() && extraction.events.is_empty() {
        tracing::warn!(
            doc_id = %doc.id,
            doc_name = %doc.filename,
            "Story extraction completed but returned no arcs or events — \
             check LLM response quality and prompt configuration"
        );
    }

    let story_repo = StoryRepository::new(db);
    story_repo.delete_all_for_doc(doc.id).await?;

    let mut arc_title_to_id: HashMap<String, Uuid> = HashMap::new();
    for arc_input in extraction.arcs {
        let arc = story_repo
            .insert_arc(doc.campaign_id, doc.id, arc_input.clone())
            .await?;
        arc_title_to_id.insert(arc.title.to_lowercase(), arc.id);
    }

    let mut event_title_to_id: HashMap<String, Uuid> = HashMap::new();
    for event_input in extraction.events {
        let arc_id = event_input
            .arc_title
            .as_ref()
            .and_then(|t| arc_title_to_id.get(&t.to_lowercase()))
            .copied();
        let event = story_repo
            .insert_event(doc.campaign_id, doc.id, arc_id, event_input.clone())
            .await?;
        event_title_to_id.insert(event.title.to_lowercase(), event.id);
    }

    for subplot_input in extraction.subplots {
        let arc_id = subplot_input
            .arc_title
            .as_ref()
            .and_then(|t| arc_title_to_id.get(&t.to_lowercase()))
            .copied();
        story_repo
            .insert_subplot(doc.campaign_id, doc.id, arc_id, subplot_input)
            .await?;
    }

    for char_arc_input in extraction.character_arcs {
        story_repo
            .insert_character_arc(doc.campaign_id, doc.id, char_arc_input)
            .await?;
    }

    for enc_input in extraction.encounters {
        let story_event_id = enc_input
            .story_event_title
            .as_ref()
            .and_then(|t| event_title_to_id.get(&t.to_lowercase()))
            .copied();
        story_repo
            .insert_prepopulated_encounter(doc.campaign_id, doc.id, story_event_id, enc_input)
            .await?;
    }

    doc_repo
        .update_story_extraction_status(doc.id, "completed", None)
        .await?;

    Ok(())
}
