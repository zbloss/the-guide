use std::path::Path;
use std::sync::Arc;

use guide_core::{
    models::{
        CampaignDocument, DocSummary, DocumentKind, GlobalDocument, MetaIndex,
        StoryExtractionResult, StoryFactionInput, StoryLocationInput, StoryNpcInput,
    },
    AppConfig, GuideError, Result,
};
use guide_db::{
    documents::{DocumentRepository, GlobalDocumentRepository},
    story::StoryRepository,
    vectors::{self, LoreChunkInsert},
    DuckDbPool,
};
use guide_llm::{EmbeddingHint, EmbeddingRequest, LlmClient};
use uuid::Uuid;

use crate::{chunker, extractor};

const META_INDEX_DIR: &str = "data/indexes";

// ── Document pre-analysis context ─────────────────────────────────────────────

/// Lightweight campaign context extracted before story extraction.
/// All fields default to empty strings/vecs so failures are non-fatal.
#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct DocumentContext {
    #[serde(default)]
    pub campaign_setting: String,
    #[serde(default)]
    pub campaign_title: String,
    #[serde(default)]
    pub themes: Vec<String>,
    #[serde(default)]
    pub chapter_names: Vec<String>,
    #[serde(default)]
    pub major_npcs: Vec<String>,
    #[serde(default)]
    pub major_locations: Vec<String>,
    #[serde(default)]
    pub tone: String,
}

impl DocumentContext {
    pub fn themes_str(&self) -> String {
        if self.themes.is_empty() {
            "unknown".to_string()
        } else {
            self.themes.join(", ")
        }
    }

    pub fn setting_str(&self) -> &str {
        if self.campaign_setting.is_empty() { "Unknown" } else { &self.campaign_setting }
    }

    pub fn tone_str(&self) -> &str {
        if self.tone.is_empty() { "adventure" } else { &self.tone }
    }

    /// Compact summary of this chapter's story result for passing to the next chapter.
    pub fn chapter_summary_from(result: &StoryExtractionResult, chapter_name: &str) -> String {
        let arcs: Vec<&str> = result.arcs.iter().map(|a| a.title.as_str()).collect();
        let major_events: Vec<&str> = result
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e.significance,
                    guide_core::models::StorySignificance::Major
                        | guide_core::models::StorySignificance::Pivotal
                        | guide_core::models::StorySignificance::Climax
                )
            })
            .map(|e| e.title.as_str())
            .collect();
        let npcs: Vec<&str> = result.npcs.iter().map(|n| n.name.as_str()).collect();

        let mut summary = format!("Chapter '{chapter_name}':");
        if !arcs.is_empty() {
            summary.push_str(&format!(" arcs=[{}]", arcs.join(", ")));
        }
        if !major_events.is_empty() {
            summary.push_str(&format!("; major events=[{}]", major_events.join(", ")));
        }
        if !npcs.is_empty() {
            summary.push_str(&format!("; NPCs=[{}]", npcs.join(", ")));
        }
        // Cap at 800 chars to avoid excessive context bloat.
        summary.chars().take(800).collect()
    }
}

/// Make a quick LLM call on the first ~5K chars of document text to extract
/// campaign context (setting, themes, chapter names, major NPCs).
/// Returns `DocumentContext::default()` on any error — this is best-effort only.
async fn analyze_document_structure(
    chunks: &[crate::chunker::DocumentChunk],
    doc_title: &str,
    llm: &dyn LlmClient,
    max_chars: usize,
) -> DocumentContext {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let excerpt: String = chunks
        .iter()
        .take(4)
        .map(|c| c.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
        .chars()
        .take(max_chars)
        .collect();

    if excerpt.trim().is_empty() {
        return DocumentContext::default();
    }

    let req = CompletionRequest {
        task: LlmTask::General,
        messages: vec![
            Message {
                role: MessageRole::System,
                content: guide_llm::prompts::document_preanalysis_system().to_string(),
            },
            Message {
                role: MessageRole::User,
                content: guide_llm::prompts::document_preanalysis_user(&excerpt, doc_title),
            },
        ],
        model_override: None,
        temperature: Some(0.2),
        max_tokens: Some(1024),
        json_mode: true,
    };

    match llm.complete(req).await {
        Ok(resp) => {
            tracing::info!(
                prompt_tokens = resp.prompt_tokens,
                completion_tokens = resp.completion_tokens,
                model = %resp.model,
                phase = "pre_analysis",
                "LLM token usage"
            );
            let cleaned = guide_llm::prompts::strip_think_block(&resp.content);
            serde_json::from_str::<DocumentContext>(cleaned).unwrap_or_else(|e| {
                tracing::debug!("Pre-analysis parse failed: {e}");
                DocumentContext::default()
            })
        }
        Err(e) => {
            tracing::debug!("Pre-analysis LLM call failed: {e}");
            DocumentContext::default()
        }
    }
}

pub async fn ingest_campaign_document(
    pdf_path: &Path,
    doc: &CampaignDocument,
    llm: Arc<dyn LlmClient>,
    config: &AppConfig,
    db: &DuckDbPool,
) -> Result<usize> {
    let doc_repo = DocumentRepository::new(db);
    doc_repo
        .update_status(doc.id, &guide_core::models::IngestionStatus::Processing, None)
        .await?;
    doc_repo
        .update_story_extraction_status(doc.id, "ocr_processing", None)
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
    let page_tuples: Vec<(u32, String)> = pages
        .iter()
        .map(|p| (p.page_num, p.raw_text.clone()))
        .collect();
    doc_repo.save_page_ocr(doc.id, &page_tuples).await?;

    doc_repo
        .update_story_extraction_status(doc.id, "chunking", None)
        .await?;
    doc_repo
        .update_story_extraction_progress(doc.id, &format!("Extracted {page_count} pages"))
        .await?;

    let chunks = chunker::chunk_document(&pages, config.chunk_max_chars, config.chunk_overlap_chars)
        .await?;

    let chunk_count = chunks.len();

    if config.enable_rag {
        let mut lore_chunks = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            let embed_text = truncate_for_embed(&chunk.content);
            let embedding = match llm.embed(EmbeddingRequest { text: embed_text, model_override: None, hint: EmbeddingHint::Document }).await {
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
                page_range: chunk.page_range,
                section_path: chunk.section_path.clone(),
                doc_title: doc.filename.clone(),
                vector: embedding,
            });
        }
        vectors::upsert_chunks(db, lore_chunks).await?;
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
        if let Err(e) = extract_story(doc, &chunks, &pages, llm.clone(), db, config).await {
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
    db: &DuckDbPool,
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

    let chunks = chunker::chunk_document(&pages, config.chunk_max_chars, config.chunk_overlap_chars)
        .await?;

    let chunk_count = chunks.len();

    if config.enable_rag {
        let mut lore_chunks = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            let embed_text = truncate_for_embed(&chunk.content);
            let embedding = match llm.embed(EmbeddingRequest { text: embed_text, model_override: None, hint: EmbeddingHint::Document }).await {
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
                page_range: chunk.page_range,
                section_path: chunk.section_path.clone(),
                doc_title: doc.title.clone(),
                vector: embedding,
            });
        }
        vectors::upsert_chunks(db, lore_chunks).await?;
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
    llm: &dyn LlmClient,
    db: &DuckDbPool,
) -> Result<Vec<guide_core::models::RankedChunk>> {
    let embedding = llm
        .embed(EmbeddingRequest {
            text: query.to_string(),
            model_override: None,
            hint: EmbeddingHint::Query,
        })
        .await?;

    let mut results = Vec::new();

    if let Some(cid) = campaign_id {
        let mut campaign_chunks =
            vectors::query_chunks(db, Some(cid), &embedding, 5).await?;
        results.append(&mut campaign_chunks);
    }

    let mut global_chunks = vectors::query_chunks(db, None, &embedding, 3).await?;
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
    doc_ctx: &DocumentContext,
    prev_chapter_summary: Option<&str>,
    llm: &dyn LlmClient,
    max_output_tokens: u32,
) -> Result<StoryExtractionResult> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let system = guide_llm::prompts::story_extraction_system_v2(
        doc_ctx.setting_str(),
        &doc_ctx.themes_str(),
        doc_ctx.tone_str(),
    );
    let user = guide_llm::prompts::story_extraction_user_chapter_v2(
        text,
        doc_title,
        chapter_name,
        prev_chapter_summary,
    );

    let req = CompletionRequest {
        task: LlmTask::StoryExtraction,
        messages: vec![
            Message { role: MessageRole::System, content: system },
            Message { role: MessageRole::User, content: user },
        ],
        model_override: None,
        temperature: Some(0.3),
        max_tokens: Some(max_output_tokens),
        json_mode: true,
    };

    let resp = llm.complete(req).await?;
    tracing::info!(
        chapter = chapter_name,
        prompt_tokens = resp.prompt_tokens,
        completion_tokens = resp.completion_tokens,
        model = %resp.model,
        phase = "chapter_extraction",
        "LLM token usage"
    );
    let cleaned = guide_llm::prompts::strip_think_block(&resp.content);
    if let Err(ref e) = serde_json::from_str::<StoryExtractionResult>(cleaned) {
        tracing::warn!(
            chapter = chapter_name,
            error = %e,
            response_preview = &cleaned[..cleaned.floor_char_boundary(500)],
            "Story extraction JSON parse failed"
        );
    }
    serde_json::from_str(cleaned)
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

    // Deduplicate NPCs, locations, and factions by lowercase name.
    let mut npc_seen: Vec<String> = Vec::new();
    let mut npcs: Vec<StoryNpcInput> = Vec::new();
    let mut location_seen: Vec<String> = Vec::new();
    let mut locations: Vec<StoryLocationInput> = Vec::new();
    let mut faction_seen: Vec<String> = Vec::new();
    let mut factions: Vec<StoryFactionInput> = Vec::new();

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

        for npc in result.npcs {
            let key = npc.name.trim().to_lowercase();
            if !npc_seen.contains(&key) {
                npc_seen.push(key);
                npcs.push(npc);
            }
        }

        for location in result.locations {
            let key = location.name.trim().to_lowercase();
            if !location_seen.contains(&key) {
                location_seen.push(key);
                locations.push(location);
            }
        }

        for faction in result.factions {
            let key = faction.name.trim().to_lowercase();
            if !faction_seen.contains(&key) {
                faction_seen.push(key);
                factions.push(faction);
            }
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

    StoryExtractionResult { arcs, events, subplots, character_arcs, encounters, npcs, locations, factions }
}

pub async fn extract_story(
    doc: &CampaignDocument,
    chunks: &[crate::chunker::DocumentChunk],
    pages: &[extractor::PageExtraction],
    llm: Arc<dyn LlmClient>,
    db: &DuckDbPool,
    config: &AppConfig,
) -> Result<()> {
    use guide_llm::{CompletionRequest, LlmTask, Message, MessageRole};

    let doc_repo = DocumentRepository::new(db);
    doc_repo
        .update_story_extraction_status(doc.id, "processing", None)
        .await?;

    let max_input_chars = config.max_input_chars();
    let max_output_tokens = config.max_output_tokens();

    // Phase 1: Quick document pre-analysis for campaign context.
    let doc_ctx = analyze_document_structure(
        chunks,
        &doc.filename,
        llm.as_ref(),
        config.max_preanalysis_chars(),
    )
    .await;
    tracing::info!(
        doc_id = %doc.id,
        setting = doc_ctx.setting_str(),
        tone = doc_ctx.tone_str(),
        themes = %doc_ctx.themes_str(),
        "Document pre-analysis complete"
    );

    let chapter_groups = group_chunks_by_chapter(chunks);

    // Build DocumentData for the agent (only when a ToC is detected).
    let agent_doc_data: Option<Arc<crate::agent::DocumentData>> = {
        use crate::navigation::{detect_document_navigation, detect_page_offset};
        use crate::agent::DocumentData;

        let (toc_entries, index_entries) = detect_document_navigation(pages);
        if toc_entries.is_empty() {
            None
        } else {
            let pages_map: std::collections::HashMap<u32, String> =
                pages.iter().map(|p| (p.page_num, p.raw_text.clone())).collect();
            let page_offset = detect_page_offset(&pages_map, &toc_entries);
            let index_map: std::collections::HashMap<String, Vec<u32>> = index_entries
                .into_iter()
                .map(|e| (e.term.to_lowercase(), e.pages))
                .collect();
            tracing::info!(
                doc_id = %doc.id,
                toc_entries = toc_entries.len(),
                index_terms = index_map.len(),
                page_offset,
                "Document navigation detected — agent path enabled"
            );
            Some(Arc::new(DocumentData {
                pages: pages_map,
                toc: toc_entries,
                index: index_map,
                page_offset,
            }))
        }
    };

    let extraction: StoryExtractionResult = if chapter_groups.len() < 2 {
        // Fallback: single-call path with v2 prompt and context.
        doc_repo.update_story_extraction_progress(doc.id, "Analyzing document (1 section)").await.ok();
        let full_text = join_and_truncate(&chunks.iter().collect::<Vec<_>>(), max_input_chars);
        let system = guide_llm::prompts::story_extraction_system_v2(
            doc_ctx.setting_str(),
            &doc_ctx.themes_str(),
            doc_ctx.tone_str(),
        );
        let req = CompletionRequest {
            task: LlmTask::StoryExtraction,
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: system,
                },
                Message {
                    role: MessageRole::User,
                    content: guide_llm::prompts::story_extraction_user(&full_text, &doc.filename),
                },
            ],
            model_override: None,
            temperature: Some(0.3),
            max_tokens: Some(max_output_tokens),
            json_mode: true,
        };
        let resp = llm.complete(req).await?;
        tracing::info!(
            prompt_tokens = resp.prompt_tokens,
            completion_tokens = resp.completion_tokens,
            model = %resp.model,
            phase = "single_call_extraction",
            "LLM token usage"
        );
        let cleaned = guide_llm::prompts::strip_think_block(&resp.content);
        if let Err(ref e) = serde_json::from_str::<StoryExtractionResult>(cleaned) {
            tracing::warn!(
                error = %e,
                response_preview = &cleaned[..cleaned.floor_char_boundary(500)],
                "Story extraction JSON parse failed"
            );
        }
        serde_json::from_str(cleaned)
            .map_err(|e| GuideError::Internal(format!("Story extraction parse failed: {e}")))?
    } else {
        // Chapter path: one LLM call per window, with cross-chapter continuity.
        let mut results: Vec<(usize, StoryExtractionResult)> = Vec::new();
        let mut call_idx = 0usize;
        let mut prev_chapter_summary: Option<String> = None;
        let total_chapters = chapter_groups.len();
        let mut chapter_idx = 0usize;

        // Emit total upfront so the frontend can show a scaled progress bar
        // from the very first poll, before any chapter completes.
        doc_repo
            .update_story_extraction_progress(
                doc.id,
                &format!("Chapter 0 of {total_chapters}"),
            )
            .await
            .ok();

        for (chapter_name, chapter_chunks) in &chapter_groups {
            chapter_idx += 1;
            let windows = windows_for_chapter(chapter_chunks, max_input_chars);
            let window_count = windows.len();
            for (w_idx, window) in windows.into_iter().enumerate() {
                let text = join_chunks(&window);
                let label = if window_count > 1 {
                    format!("{chapter_name} (part {}/{})", w_idx + 1, window_count)
                } else {
                    chapter_name.clone()
                };

                // Try the agent path first when navigation data is available.
                let agent_result: Option<StoryExtractionResult> =
                    if let Some(ref doc_data) = agent_doc_data {
                        crate::agent::extract_chapter_with_agent(
                            &text,
                            chapter_name,
                            &doc.filename,
                            &doc_ctx,
                            prev_chapter_summary.as_deref(),
                            doc_data.clone(),
                            config,
                        )
                        .await
                        .unwrap_or(None)
                    } else {
                        None
                    };

                let extraction_result: Result<StoryExtractionResult> =
                    if let Some(r) = agent_result {
                        tracing::info!(chapter = %label, "Agent story extraction succeeded");
                        Ok(r)
                    } else {
                        extract_story_for_chapter(
                            &text,
                            &doc.filename,
                            chapter_name,
                            &doc_ctx,
                            prev_chapter_summary.as_deref(),
                            llm.as_ref(),
                            max_output_tokens,
                        )
                        .await
                    };

                match extraction_result {
                    Ok(r) => {
                        // Build summary for next chapter only on the last window of each chapter.
                        if w_idx + 1 == window_count {
                            let summary = DocumentContext::chapter_summary_from(&r, chapter_name);
                            prev_chapter_summary = Some(match prev_chapter_summary {
                                Some(existing) => format!("{existing}\n{summary}"),
                                None => summary,
                            });
                            // Cap total summary at ~2000 chars.
                            if let Some(ref mut s) = prev_chapter_summary {
                                if s.len() > 2000 {
                                    *s = s.chars().take(2000).collect();
                                }
                            }
                            doc_repo
                                .update_story_extraction_progress(
                                    doc.id,
                                    &format!("Chapter {chapter_idx} of {total_chapters}"),
                                )
                                .await
                                .ok();
                        }
                        results.push((call_idx, r));
                    }
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
    story_repo
        .persist_story_batch(doc.campaign_id, doc.id, extraction)
        .await?;

    doc_repo
        .update_story_extraction_status(doc.id, "completed", None)
        .await?;

    Ok(())
}

/// Extract the first complete `{...}` JSON object from an LLM response string.
/// Returns the extracted slice, or the original string if no braces are found.
pub fn extract_first_json_object_pub(s: &str) -> &str {
    let bytes = s.as_bytes();
    let Some(start) = bytes.iter().position(|&b| b == b'{') else {
        return s;
    };
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes[start..].iter().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        match b {
            b'\\' if in_str => escape = true,
            b'"' => in_str = !in_str,
            b'{' if !in_str => depth += 1,
            b'}' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return &s[start..start + i + 1];
                }
            }
            _ => {}
        }
    }
    s
}

/// Re-run story extraction on an already-ingested document without repeating OCR.
///
/// Loads the stored page OCR text from the database, re-chunks it, and runs the
/// full story extraction pipeline. Useful for re-testing with a different LLM model
/// without re-processing the PDF.
pub async fn re_extract_story_from_stored_ocr(
    doc: &CampaignDocument,
    llm: Arc<dyn LlmClient>,
    db: &DuckDbPool,
    config: &AppConfig,
) -> Result<()> {
    let doc_repo = DocumentRepository::new(db);

    let rows = doc_repo.list_page_ocr(doc.id).await?;
    if rows.is_empty() {
        return Err(GuideError::Internal(
            "No stored OCR text found — run a full ingest first".to_string(),
        ));
    }

    // Reconstruct PageExtraction from stored rows (headings are not persisted; chunker doesn't need them).
    let pages: Vec<extractor::PageExtraction> = rows
        .into_iter()
        .map(|r| extractor::PageExtraction {
            page_num: r.page_num as u32,
            raw_text: r.raw_text,
            headings: Vec::new(),
        })
        .collect();

    let chunks = chunker::chunk_document(&pages, config.chunk_max_chars, config.chunk_overlap_chars).await?;

    extract_story(doc, &chunks, &pages, llm, db, config).await
}
