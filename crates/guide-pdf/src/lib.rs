//! guide-pdf — PDF ingestion pipeline (two-stage)
//!
//! Stage 1 (extractor): PDF → per-page text/headings via GLM-OCR vision API
//! Stage 2 (chunker):   Pages → heading-split + sentence-bounded chunks
//! Stage 3 (embed):     Chunks → Vec<f32> → Qdrant upsert

pub mod chunker;
pub mod extractor;

use std::path::Path;
use std::sync::Arc;

use guide_core::{GuideError, Result, config::IngestionConfig, models::DocumentKind};
use guide_db::{
    documents::{DocumentRepository, GlobalDocumentRepository},
    qdrant::{
        LoreChunkInsert, campaign_collection_name, create_campaign_collection,
        create_global_collection, global_collection_name, upsert_lore_chunk,
    },
    SqlitePool,
};
use guide_llm::{EmbeddingRequest, LlmClient};
use uuid::Uuid;

/// Run the full two-stage ingestion pipeline for a single document.
///
/// Returns the number of lore chunks created.
pub async fn ingest_document(
    pdf_path: &Path,
    document_kind: DocumentKind,
    campaign_id: Option<Uuid>,
    doc_id: Uuid,
    doc_title: &str,
    is_player_visible_default: bool,
    llm: Arc<dyn LlmClient>,
    ocr_model: &str,
    ingestion: &IngestionConfig,
    qdrant: &qdrant_client::Qdrant,
    db: &SqlitePool,
) -> Result<usize> {
    // ── Stage 1: Page extraction ───────────────────────────────────────────────
    let pages = extractor::extract_pages(
        pdf_path,
        &document_kind,
        llm.as_ref(),
        ocr_model,
        ingestion.ocr_batch_delay_ms,
    )
    .await?;

    let page_count = pages.len() as i32;

    tracing::info!(
        "Extracted {} page(s) from {:?} (doc {})",
        page_count,
        pdf_path,
        doc_id
    );

    // ── Stage 2: Intelligent chunking ─────────────────────────────────────────
    let chunks = chunker::chunk_document(
        pages,
        ingestion.chunk_max_tokens * 4, // convert token estimate to char budget
        ingestion.chunk_overlap_chars,
    )
    .await?;

    tracing::info!("Produced {} chunks for doc {}", chunks.len(), doc_id);

    // ── Stage 3: Embed & upsert ───────────────────────────────────────────────
    let collection = match &document_kind {
        DocumentKind::Rulebook => {
            create_global_collection(qdrant, 768).await?;
            global_collection_name().to_string()
        }
        DocumentKind::Campaign => {
            let cid = campaign_id
                .ok_or_else(|| GuideError::PdfProcessing("campaign_id required for Campaign documents".into()))?;
            create_campaign_collection(qdrant, &cid.to_string(), 768).await?;
            campaign_collection_name(&cid.to_string())
        }
    };

    let mut count = 0;

    for chunk in chunks {
        let vector = match llm
            .embed(EmbeddingRequest { text: chunk.content.clone(), model_override: None })
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    "Embedding failed for chunk on pages {:?}, skipping: {e}",
                    chunk.page_range
                );
                continue;
            }
        };

        let is_visible = if matches!(document_kind, DocumentKind::Rulebook) {
            true // rulebook content is always player-visible
        } else {
            chunk.is_player_visible && is_player_visible_default
        };

        upsert_lore_chunk(
            qdrant,
            &collection,
            LoreChunkInsert {
                id: Uuid::new_v4(),
                campaign_id,
                source_document_id: doc_id,
                document_kind: document_kind.clone(),
                content: chunk.content,
                lore_type: infer_lore_type(&document_kind),
                significance: "minor".to_string(),
                entities: Vec::new(),
                is_player_visible: is_visible,
                page_range: chunk.page_range,
                section_path: chunk.section_path,
                doc_title: doc_title.to_string(),
                vector,
            },
        )
        .await?;

        count += 1;
    }

    // ── Update DB status ───────────────────────────────────────────────────────
    match document_kind {
        DocumentKind::Campaign => {
            DocumentRepository::new(db).update_ingested(doc_id, Some(page_count)).await?;
        }
        DocumentKind::Rulebook => {
            GlobalDocumentRepository::new(db).update_ingested(doc_id, Some(page_count)).await?;
        }
    }

    tracing::info!("Ingested {count} lore chunks for doc {doc_id}");
    Ok(count)
}

fn infer_lore_type(kind: &DocumentKind) -> String {
    match kind {
        DocumentKind::Rulebook => "mechanic".to_string(),
        DocumentKind::Campaign => "plot".to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pdfium_loads_and_renders() {
        use pdfium_render::prelude::*;
        use image::ImageFormat;

        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .expect("pdfium should load"),
        );

        let pdf_path = "/mnt/c/Users/altoz/Projects/the-guide/data/documents/global/b159dc64-0286-43e9-9447-4de88a655334.pdf";
        let doc = pdfium.load_pdf_from_file(pdf_path, None).expect("PDF should open");
        let page_count = doc.pages().len();
        println!("PDF has {page_count} pages");

        let config = PdfRenderConfig::new()
            .set_target_width(800)
            .set_maximum_height(1100);

        let page = doc.pages().get(0).expect("page 0 should exist");
        let bitmap = page.render_with_config(&config).expect("render should succeed");
        let image = bitmap.as_image();

        let mut jpeg_bytes: Vec<u8> = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut jpeg_bytes), ImageFormat::Jpeg)
            .expect("encode should succeed");

        println!("Page 1 JPEG size: {} bytes", jpeg_bytes.len());
        assert!(!jpeg_bytes.is_empty());
    }
}
