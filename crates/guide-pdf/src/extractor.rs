//! Stage 1 of the ingestion pipeline: extract per-page text from a PDF.
//!
//! Uses pdfium-render to render each page to a small JPEG, then sends it to
//! GLM-OCR via the vision API. Falls back to whole-document mode if pdfium
//! fails to load at runtime.

use std::path::{Path, PathBuf};

use guide_core::{GuideError, Result, models::DocumentKind};
use guide_llm::{LlmClient, LlmTask, VisionRequest, prompts};
use image::ImageFormat;
use serde::Deserialize;

/// Structured extraction result for one page.
pub struct PageExtraction {
    /// 1-based page index (0 in whole-doc fallback mode)
    pub page_num: u32,
    pub raw_text: String,
    pub headings: Vec<String>,
    pub is_dm_only: bool,
}

#[derive(Deserialize)]
struct PageOcrResponse {
    raw_text: String,
    #[serde(default)]
    headings: Vec<String>,
    #[serde(default)]
    is_dm_only: bool,
}

/// Extract text from a PDF using pdfium per-page rendering + GLM-OCR.
/// Falls back to whole-document mode if pdfium cannot load.
pub async fn extract_pages(
    pdf_path: &Path,
    document_kind: &DocumentKind,
    llm: &dyn LlmClient,
    ocr_model: &str,
    delay_ms: u64,
) -> Result<Vec<PageExtraction>> {
    let prompt = match document_kind {
        DocumentKind::Rulebook => prompts::ocr_rulebook_page_prompt(),
        DocumentKind::Campaign => prompts::ocr_campaign_page_prompt(),
    };

    // Render all pages to JPEG in a blocking thread (pdfium types are not Send)
    let path_buf = pdf_path.to_path_buf();
    let page_images: Result<Vec<(u32, Vec<u8>)>> =
        tokio::task::spawn_blocking(move || render_pages_to_jpeg(&path_buf))
            .await
            .map_err(|e| GuideError::PdfProcessing(format!("Render task panicked: {e}")))?;

    match page_images {
        Ok(pages) if !pages.is_empty() => {
            tracing::info!("Rendered {} pages via pdfium from {:?}", pages.len(), pdf_path);
            ocr_pages(pages, llm, ocr_model, prompt, delay_ms).await
        }
        Ok(_) => {
            tracing::warn!("pdfium returned 0 pages; falling back to whole-document mode");
            whole_doc_fallback(pdf_path, llm, ocr_model, prompt).await
        }
        Err(e) => {
            tracing::warn!("pdfium rendering failed ({e}); falling back to whole-document mode");
            whole_doc_fallback(pdf_path, llm, ocr_model, prompt).await
        }
    }
}

/// Render every page to a small JPEG. Runs in a blocking thread.
/// 600px width targets ~100KB per page, keeping images well within GLM-OCR's
/// context window even for image-heavy or unusually-proportioned pages.
fn render_pages_to_jpeg(pdf_path: &PathBuf) -> Result<Vec<(u32, Vec<u8>)>> {
    use pdfium_render::prelude::*;

    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| GuideError::PdfProcessing(format!("pdfium load failed: {e}")))?,
    );

    let doc = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| GuideError::PdfProcessing(format!("PDF open failed: {e}")))?;

    let page_count = doc.pages().len();
    tracing::info!("Rendering {page_count} pages from {:?}", pdf_path);

    let render_config = PdfRenderConfig::new()
        .set_target_width(600)
        .set_maximum_height(850)
        .rotate_if_landscape(PdfPageRenderRotation::None, true);

    let mut result = Vec::with_capacity(page_count as usize);

    for (index, page) in doc.pages().iter().enumerate() {
        let page_num = (index + 1) as u32;

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| GuideError::PdfProcessing(format!("Render page {page_num}: {e}")))?;

        let image = bitmap.as_image();
        let mut jpeg_bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut jpeg_bytes), ImageFormat::Jpeg)
            .map_err(|e| GuideError::PdfProcessing(format!("Encode page {page_num}: {e}")))?;

        tracing::debug!("Page {page_num}/{page_count}: {} JPEG bytes", jpeg_bytes.len());
        result.push((page_num, jpeg_bytes));
    }

    Ok(result)
}

/// Send each rendered page image to GLM-OCR and collect extractions.
async fn ocr_pages(
    pages: Vec<(u32, Vec<u8>)>,
    llm: &dyn LlmClient,
    ocr_model: &str,
    prompt: &str,
    delay_ms: u64,
) -> Result<Vec<PageExtraction>> {
    let mut extractions = Vec::with_capacity(pages.len());

    for (page_num, image_bytes) in pages {
        tracing::info!("OCR page {page_num} ({} bytes)", image_bytes.len());
        let vision_result = llm
            .complete_with_vision(VisionRequest {
                task: LlmTask::OcrExtraction,
                prompt: prompt.to_string(),
                image_bytes,
                image_mime_type: "image/jpeg".into(),
                model_override: Some(ocr_model.to_string()),
            })
            .await;

        match vision_result {
            Ok(vision_resp) => {
                let page_ocr: PageOcrResponse = serde_json::from_str(vision_resp.content.trim())
                    .unwrap_or_else(|_| PageOcrResponse {
                        raw_text: vision_resp.content.clone(),
                        headings: Vec::new(),
                        is_dm_only: false,
                    });
                extractions.push(PageExtraction {
                    page_num,
                    raw_text: page_ocr.raw_text,
                    headings: page_ocr.headings,
                    is_dm_only: page_ocr.is_dm_only,
                });
            }
            Err(e) => {
                tracing::warn!("OCR failed for page {page_num}, skipping: {e}");
                extractions.push(PageExtraction {
                    page_num,
                    raw_text: String::new(),
                    headings: Vec::new(),
                    is_dm_only: false,
                });
            }
        }

        if delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
    }

    Ok(extractions)
}

/// Whole-document fallback: not supported.
///
/// Sending a raw PDF file to a vision model (which expects JPEG/PNG) will
/// produce a malformed prompt of tens of millions of tokens and crash the
/// inference engine. Return a clear error so the operator knows to install
/// the pdfium library instead.
async fn whole_doc_fallback(
    pdf_path: &Path,
    _llm: &dyn LlmClient,
    _ocr_model: &str,
    _prompt: &str,
) -> Result<Vec<PageExtraction>> {
    Err(GuideError::PdfProcessing(format!(
        "pdfium failed to render pages from {:?}. \
        Ensure the pdfium shared library is present (libpdfium.so on Linux, \
        pdfium.dll on Windows) in the working directory or system library path.",
        pdf_path
    )))
}
