use std::path::Path;
use std::sync::OnceLock;

use guide_core::{models::DocumentKind, GuideError, Result};
use guide_llm::{prompts, LlmClient, LlmTask, VisionRequest};
use serde::Deserialize;

// Embed libpdfium.so at compile time (downloaded by build.rs into OUT_DIR).
static PDFIUM_BYTES: &[u8] = include_bytes!(env!("PDFIUM_LIB_PATH"));

static PDFIUM_LIB_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Writes the embedded pdfium binary to a temp directory on first call,
/// then returns the stable path for subsequent calls.
fn pdfium_lib_path() -> Result<&'static std::path::PathBuf> {
    if let Some(p) = PDFIUM_LIB_PATH.get() {
        return Ok(p);
    }

    let dir = std::env::temp_dir().join("guide-pdfium");
    std::fs::create_dir_all(&dir)
        .map_err(|e| GuideError::PdfProcessing(format!("failed to create pdfium temp dir: {e}")))?;

    let path = dir.join("libpdfium.so");
    if !path.exists() {
        std::fs::write(&path, PDFIUM_BYTES)
            .map_err(|e| GuideError::PdfProcessing(format!("failed to write pdfium lib: {e}")))?;
    }

    Ok(PDFIUM_LIB_PATH.get_or_init(|| path))
}

pub struct PageExtraction {
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

    let path_buf = pdf_path.to_path_buf();
    let pages = tokio::task::spawn_blocking(move || render_pages_to_jpeg(&path_buf))
        .await
        .map_err(|e| GuideError::PdfProcessing(format!("render task panicked: {e}")))??;

    if pages.is_empty() {
        return Err(GuideError::PdfProcessing(format!(
            "pdfium rendered 0 pages from {:?}",
            pdf_path
        )));
    }

    tracing::info!("Rendered {} pages via pdfium from {:?}", pages.len(), pdf_path);
    ocr_pages(pages, llm, ocr_model, prompt, delay_ms).await
}

fn render_pages_to_jpeg(pdf_path: &std::path::PathBuf) -> Result<Vec<(u32, Vec<u8>)>> {
    use image::ImageFormat;
    use pdfium_render::prelude::*;

    let lib_path = pdfium_lib_path()?;
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(lib_path)
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
            .map_err(|e| GuideError::PdfProcessing(format!("render page {page_num}: {e}")))?;

        let image = bitmap.as_image();
        let mut jpeg_bytes: Vec<u8> = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg_bytes),
                ImageFormat::Jpeg,
            )
            .map_err(|e| GuideError::PdfProcessing(format!("encode page {page_num}: {e}")))?;

        result.push((page_num, jpeg_bytes));
    }

    Ok(result)
}

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
