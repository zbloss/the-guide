//! guide-pdf — PDF ingestion pipeline
//!
//! Pipeline: PDF file → base64 → GLM-OCR via Ollama vision API
//! → JSON chunks → embed → upsert Qdrant.

use std::path::Path;
use std::sync::Arc;

use guide_core::{GuideError, Result};
use guide_db::{
    documents::DocumentRepository,
    qdrant::{upsert_lore_chunk, LoreChunkInsert},
    SqlitePool,
};
use guide_llm::{EmbeddingRequest, LlmClient, LlmTask, VisionRequest, prompts};
use serde::Deserialize;
use uuid::Uuid;

/// Run the full ingestion pipeline for a single document:
/// PDF bytes → GLM-OCR → chunk → embed → Qdrant upsert.
///
/// Returns the number of lore chunks created.
pub async fn ingest_document(
    pdf_path: &Path,
    campaign_id: Uuid,
    doc_id: Uuid,
    is_player_visible_default: bool,
    llm: Arc<dyn LlmClient>,
    qdrant: &qdrant_client::Qdrant,
    db: &SqlitePool,
) -> Result<usize> {
    // 1. Read PDF bytes
    let pdf_bytes = tokio::fs::read(pdf_path)
        .await
        .map_err(|e| GuideError::PdfProcessing(e.to_string()))?;

    tracing::info!(
        "Ingesting PDF {:?} ({} bytes) for campaign {}",
        pdf_path,
        pdf_bytes.len(),
        campaign_id
    );

    // 2. Send raw PDF bytes to GLM-OCR via the vision API
    let vision_resp = llm
        .complete_with_vision(VisionRequest {
            task: LlmTask::OcrExtraction,
            prompt: prompts::ocr_extraction_prompt().to_string(),
            image_bytes: pdf_bytes,
            image_mime_type: "application/pdf".into(),
            model_override: None,
        })
        .await?;

    // 3. Parse JSON response
    let ocr_response: OcrResponse = serde_json::from_str(&vision_resp.content)
        .map_err(|e| GuideError::PdfProcessing(format!("Failed to parse OCR JSON: {e}")))?;

    tracing::info!(
        "OCR returned {} chunks for doc {}",
        ocr_response.chunks.len(),
        doc_id
    );

    // 4. Embed and upsert each chunk
    let campaign_id_str = campaign_id.to_string();
    let mut count = 0;

    for chunk in ocr_response.chunks {
        let vector = llm
            .embed(EmbeddingRequest { text: chunk.content.clone(), model_override: None })
            .await?;

        let is_visible = chunk.is_player_visible.unwrap_or(is_player_visible_default);

        upsert_lore_chunk(
            qdrant,
            &campaign_id_str,
            LoreChunkInsert {
                id: Uuid::new_v4(),
                campaign_id,
                source_document_id: doc_id,
                content: chunk.content,
                lore_type: chunk.lore_type,
                significance: chunk.significance,
                entities: chunk.entities,
                is_player_visible: is_visible,
                vector,
            },
        )
        .await?;

        count += 1;
    }

    // 5. Mark document as ingested in SQLite
    let doc_repo = DocumentRepository::new(db);
    doc_repo.update_ingested(doc_id, None).await?;

    tracing::info!("Ingested {count} lore chunks for doc {doc_id}");
    Ok(count)
}

#[derive(Deserialize)]
struct OcrResponse {
    chunks: Vec<OcrChunk>,
}

#[derive(Deserialize)]
struct OcrChunk {
    content: String,
    lore_type: String,
    significance: String,
    #[serde(default)]
    entities: Vec<String>,
    is_player_visible: Option<bool>,
}
