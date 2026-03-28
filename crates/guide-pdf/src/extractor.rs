use std::path::Path;
use std::sync::OnceLock;

use guide_core::{models::{character::ParsedSheetResult, DocumentKind}, GuideError, Result};
use guide_llm::{prompts, LlmClient, LlmTask, VisionRequest};
use serde::Deserialize;

// ── AcroForm field extraction (fillable PDFs) ────────────────────────────────

/// Extract all AcroForm field values from a PDF.
/// Returns a map of field_name → value string, skipping empty/Off fields.
/// This handles fillable character sheets (D&D Beyond exports, etc.).
pub fn extract_form_fields(pdf_bytes: &[u8]) -> std::collections::HashMap<String, String> {
    use lopdf::Document;

    let mut fields = std::collections::HashMap::new();

    let doc = match Document::load_mem(pdf_bytes) {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!("lopdf load failed: {e}");
            return fields;
        }
    };

    collect_form_fields_from_doc(&doc, &mut fields);
    fields
}

/// Decode a PDF string object to a Rust String.
/// Handles UTF-16BE (BOM \xfe\xff) and UTF-16LE (BOM \xff\xfe) as well as plain UTF-8/Latin-1.
fn pdf_str_to_string(obj: &lopdf::Object) -> Option<String> {
    let raw = match obj.as_str() {
        Ok(b) => b,
        Err(_) => return None,
    };

    if raw.is_empty() {
        return None;
    }

    // Detect UTF-16BE (PDF standard) or UTF-16LE
    if raw.len() >= 2 {
        if raw[0] == 0xfe && raw[1] == 0xff {
            // UTF-16BE with BOM
            let pairs: Vec<u16> = raw[2..]
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            return String::from_utf16(&pairs).ok().map(|s| s.trim().to_string());
        }
        if raw[0] == 0xff && raw[1] == 0xfe {
            // UTF-16LE with BOM
            let pairs: Vec<u16> = raw[2..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            return String::from_utf16(&pairs).ok().map(|s| s.trim().to_string());
        }
    }

    // Default: try UTF-8, then fall back to Latin-1
    match std::str::from_utf8(raw) {
        Ok(s) => Some(s.trim().to_string()),
        Err(_) => {
            // Latin-1 fallback — every byte is a valid codepoint
            Some(raw.iter().map(|&b| b as char).collect::<String>().trim().to_string())
        }
    }
}

fn collect_form_fields_from_doc(
    doc: &lopdf::Document,
    out: &mut std::collections::HashMap<String, String>,
) {
    // Scan ALL objects in the document for PDF field widgets.
    // A field widget is a Dictionary that has both a "T" (field name) key and
    // a "V" (field value) key.  This approach is robust against non-standard
    // AcroForm tree structures (e.g. compressed XRef streams, object streams).
    for (_id, object) in doc.objects.iter() {
        let dict = match object.as_dict() {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Must have a field name
        let name_obj = match dict.get(b"T") {
            Ok(o) => o,
            Err(_) => continue,
        };
        let name = match pdf_str_to_string(name_obj) {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };

        // Must have a value
        let value_obj = match dict.get(b"V") {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Try string value first, then Name (for checkboxes/radio)
        let value = match pdf_str_to_string(value_obj) {
            Some(s) if !s.is_empty() => s,
            _ => match value_obj.as_name_str() {
                Ok(s) if s != "Off" && s != "No" && !s.is_empty() => s.to_string(),
                _ => continue,
            },
        };

        if !value.is_empty() {
            out.insert(name, value);
        }
    }
}

/// Format AcroForm fields as readable key:value text for LLM parsing.
/// Long values (> 400 chars) are truncated to keep total size manageable.
pub fn format_form_fields(fields: &std::collections::HashMap<String, String>) -> String {
    let mut field_names: Vec<&str> = fields.keys().map(|s| s.as_str()).collect();
    field_names.sort();
    tracing::info!("Form field names: {:?}", field_names);

    let mut lines: Vec<String> = fields
        .iter()
        .map(|(k, v)| {
            let v_trimmed = if v.len() > 400 {
                format!("{}…", &v[..400])
            } else {
                v.clone()
            };
            format!("{k}: {v_trimmed}")
        })
        .collect();
    lines.sort();
    lines.join("\n")
}

// ── JSON type-coercion helpers for LLM output tolerance ──────────────────────

/// Accept any JSON value and return a String (integers become their decimal form, null → "").
fn val_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// Accept a JSON number or numeric string and return i32 (null / bad type → 0).
fn val_i32(v: &serde_json::Value) -> i32 {
    match v {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0) as i32,
        serde_json::Value::String(s) => s.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

/// Same as val_i32 but wraps in Some; returns None only for JSON null.
fn val_i32_opt(v: &serde_json::Value) -> Option<i32> {
    match v {
        serde_json::Value::Null => None,
        other => Some(val_i32(other)),
    }
}

fn val_f32(v: &serde_json::Value) -> f32 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) as f32,
        serde_json::Value::String(s) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Convert a JSON array of strings (or nulls/numbers) to Vec<String>.
/// Arrays of objects are skipped. Non-array values become empty vec.
fn val_str_vec(v: &serde_json::Value) -> Vec<String> {
    match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| match item {
                serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

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

    let path = dir.join(env!("PDFIUM_LIB_NAME"));
    if !path.exists() {
        std::fs::write(&path, PDFIUM_BYTES)
            .map_err(|e| GuideError::PdfProcessing(format!("failed to write pdfium lib: {e}")))?;
    }

    Ok(PDFIUM_LIB_PATH.get_or_init(|| path))
}

/// Extract raw text from PDF bytes using pdfium's text layer (no vision LLM).
/// Writes bytes to a temp file, extracts text from each page, then cleans up.
pub fn extract_text_sync(pdf_bytes: &[u8]) -> Result<String> {
    use pdfium_render::prelude::*;

    // Write to temp file
    let temp_path = std::env::temp_dir().join(format!(
        "guide-sheet-{}.pdf",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::write(&temp_path, pdf_bytes)
        .map_err(|e| GuideError::PdfProcessing(format!("failed to write temp pdf: {e}")))?;

    let lib_path = pdfium_lib_path()?;
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(lib_path)
            .map_err(|e| GuideError::PdfProcessing(format!("pdfium load failed: {e}")))?,
    );

    let doc = pdfium
        .load_pdf_from_file(&temp_path, None)
        .map_err(|e| GuideError::PdfProcessing(format!("PDF open failed: {e}")))?;

    let mut all_text = String::new();
    for page in doc.pages().iter() {
        if let Ok(text_obj) = page.text() {
            all_text.push_str(&text_obj.all());
            all_text.push('\n');
        }
    }

    let _ = std::fs::remove_file(&temp_path);
    Ok(all_text)
}

pub struct PageExtraction {
    pub page_num: u32,
    pub raw_text: String,
    pub headings: Vec<String>,
}

/// Normalise raw text from the PDF text layer or OCR:
/// - Windows/old-Mac line endings → `\n`
/// - Tabs → single space
/// - Multiple consecutive spaces on a line → single space
/// - Trailing whitespace trimmed from each line
pub(crate) fn normalize_page_text(s: &str) -> String {
    let normalized = s.replace("\r\n", "\n").replace('\r', "\n").replace('\t', " ");
    normalized
        .lines()
        .map(|line| {
            // Collapse runs of spaces within each line.
            let mut out = String::with_capacity(line.len());
            let mut prev_space = false;
            for ch in line.chars() {
                if ch == ' ' {
                    if !prev_space {
                        out.push(' ');
                    }
                    prev_space = true;
                } else {
                    out.push(ch);
                    prev_space = false;
                }
            }
            out.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Deserialize)]
struct PageOcrResponse {
    raw_text: String,
    #[serde(default)]
    headings: Vec<String>,
}

pub async fn extract_pages(
    pdf_path: &Path,
    document_kind: &DocumentKind,
    llm: &dyn LlmClient,
    ocr_model: &str,
    delay_ms: u64,
) -> Result<Vec<PageExtraction>> {
    let is_campaign_doc = matches!(
        document_kind,
        DocumentKind::Campaign | DocumentKind::Supplemental
    );

    let path_buf = pdf_path.to_path_buf();

    // For campaign documents: render pages with per-page layout detection.
    // Text-layer pages (>= 80 words in the native text layer) skip vision OCR entirely,
    // with column-ordered text extraction for multi-column layouts.
    // Pages with sparse or no text layer fall back to vision OCR with a layout-aware prompt.
    if is_campaign_doc {
        let pages = tokio::task::spawn_blocking(move || render_pages_with_layout(&path_buf))
            .await
            .map_err(|e| GuideError::PdfProcessing(format!("render task panicked: {e}")))??;

        if pages.is_empty() {
            return Err(GuideError::PdfProcessing(format!(
                "pdfium rendered 0 pages from {:?}",
                pdf_path
            )));
        }

        tracing::info!(
            "Rendered {} pages (with layout detection) from {:?}",
            pages.len(),
            pdf_path
        );
        ocr_pages_campaign(pages, llm, ocr_model, delay_ms).await
    } else {
        // Non-campaign documents (character sheets, rulebooks) use the simple prompt.
        let path_buf2 = pdf_path.to_path_buf();
        let pages = tokio::task::spawn_blocking(move || render_pages_to_jpeg(&path_buf2))
            .await
            .map_err(|e| GuideError::PdfProcessing(format!("render task panicked: {e}")))??;

        if pages.is_empty() {
            return Err(GuideError::PdfProcessing(format!(
                "pdfium rendered 0 pages from {:?}",
                pdf_path
            )));
        }

        tracing::info!("Rendered {} pages via pdfium from {:?}", pages.len(), pdf_path);
        ocr_pages(pages, llm, ocr_model, prompts::ocr_simple_prompt(), delay_ms).await
    }
}

/// t_patch_size=2 from GLM-OCR config, 14=ViT spatial patch → align to 14×2=28px
const VISION_PATCH_SIZE: u32 = 28;
const OCR_MIN_PIXELS: u32 = 12_544;    // 112 * 112
const OCR_MAX_PIXELS: u32 = 71_372_800; // 14 * 14 * 4 * 1280

fn align_to_patch(n: u32) -> u32 {
    n.div_ceil(VISION_PATCH_SIZE) * VISION_PATCH_SIZE
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

    // 200 DPI for letter page (8.5×11in) → 1700×2200px, within GLM-OCR pixel bounds
    let render_config = PdfRenderConfig::new()
        .set_target_width(1700)
        .set_maximum_height(2400)
        .rotate_if_landscape(PdfPageRenderRotation::None, true);

    let mut result = Vec::with_capacity(page_count as usize);

    for (index, page) in doc.pages().iter().enumerate() {
        let page_num = (index + 1) as u32;

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| GuideError::PdfProcessing(format!("render page {page_num}: {e}")))?;

        let image = bitmap.as_image();
        let pixel_area = image.width() * image.height();
        let image = if pixel_area < OCR_MIN_PIXELS {
            let scale = (OCR_MIN_PIXELS as f64 / pixel_area as f64).sqrt();
            let new_w = (image.width() as f64 * scale).round() as u32;
            let new_h = (image.height() as f64 * scale).round() as u32;
            image.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
        } else if pixel_area > OCR_MAX_PIXELS {
            let scale = (OCR_MAX_PIXELS as f64 / pixel_area as f64).sqrt();
            let new_w = (image.width() as f64 * scale).round() as u32;
            let new_h = (image.height() as f64 * scale).round() as u32;
            image.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
        } else {
            image
        };
        let aligned_w = align_to_patch(image.width());
        let aligned_h = align_to_patch(image.height());
        let image = if aligned_w != image.width() || aligned_h != image.height() {
            image.resize_exact(aligned_w, aligned_h, image::imageops::FilterType::Lanczos3)
        } else {
            image
        };
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

// ── Column layout detection ────────────────────────────────────────────────────

/// A page that has been either rendered (needs vision OCR) or extracted from the text layer.
enum PageData {
    /// Page rendered to JPEG bytes + detected layout hint for vision OCR
    NeedsOcr { page_num: u32, jpeg: Vec<u8>, layout: &'static str },
    /// Page text extracted directly from the PDF text layer (no LLM needed)
    TextLayer { page_num: u32, text: String, headings: Vec<String> },
}

/// Detect the column layout of a PDF page by analyzing the x-coordinate distribution
/// of text characters.  Returns one of: "single-column", "two-column", "three-column",
/// or "unknown layout".
fn detect_page_layout(page: &pdfium_render::prelude::PdfPage<'_>) -> &'static str {
    let text_obj = match page.text() {
        Ok(t) => t,
        Err(_) => return "unknown layout",
    };

    let page_width = page.page_size().width().value;
    if page_width < 50.0 {
        return "unknown layout";
    }

    // Collect x-center positions of non-whitespace characters.
    let mut x_positions: Vec<f32> = Vec::new();
    for c in text_obj.chars().iter() {
        if let Some(ch) = c.unicode_char() {
            if ch.is_whitespace() || ch.is_control() || ch == '\0' {
                continue;
            }
            if let Ok(bounds) = c.loose_bounds() {
                let cx = (bounds.left().value + bounds.right().value) / 2.0;
                x_positions.push(cx);
            }
        }
    }

    if x_positions.len() < 40 {
        return "unknown layout";
    }

    // Normalize to the observed text extent (not the full page width, to handle margins).
    let min_x = x_positions.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_x = x_positions.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = max_x - min_x;
    if range < 20.0 {
        return "single-column";
    }

    // Bucket into 24 histogram bins.
    const BINS: usize = 24;
    let mut hist = [0u32; BINS];
    for &x in &x_positions {
        let bin = (((x - min_x) / range) * (BINS as f32 - 1.0)) as usize;
        hist[bin.min(BINS - 1)] += 1;
    }

    let peak = *hist.iter().max().unwrap_or(&1);
    if peak == 0 {
        return "single-column";
    }
    let valley_threshold = (peak as f32 * 0.15) as u32; // < 15% of peak = valley bin

    // Search for contiguous low-density "gap" regions in the middle portion (bins 8..16).
    let search_start = BINS / 3;
    let search_end = 2 * BINS / 3;

    let mut in_valley = false;
    let mut valley_count = 0usize;
    for &bin_count in hist.iter().take(search_end).skip(search_start) {
        if bin_count <= valley_threshold {
            if !in_valley {
                in_valley = true;
                valley_count += 1;
            }
        } else {
            in_valley = false;
        }
    }

    match valley_count {
        0 => "single-column",
        1 => "two-column",
        _ => "three-column",
    }
}

/// Extract column-ordered text from a page's text layer.
/// For single-column pages, returns `text_obj.all()` directly.
/// For two-column pages, extracts left-column characters first, then right-column,
/// each sorted top-to-bottom (PDF y-axis is inverted: higher value = higher on page).
fn extract_column_ordered_text(
    page: &pdfium_render::prelude::PdfPage<'_>,
    layout: &str,
) -> String {
    let text_obj = match page.text() {
        Ok(t) => t,
        Err(_) => return String::new(),
    };

    match layout {
        "two-column" => {
            let page_width = page.page_size().width().value;
            let mid = page_width / 2.0;

            // Collect (y_desc, x, char) for sorting: negative y so higher page position sorts first.
            let mut left: Vec<(f32, f32, char)> = Vec::new();
            let mut right: Vec<(f32, f32, char)> = Vec::new();

            for c in text_obj.chars().iter() {
                if let Some(ch) = c.unicode_char() {
                    if let Ok(bounds) = c.loose_bounds() {
                        let cx = (bounds.left().value + bounds.right().value) / 2.0;
                        let cy = bounds.top().value; // higher = higher on page
                        if cx < mid {
                            left.push((-cy, cx, ch));
                        } else {
                            right.push((-cy, cx, ch));
                        }
                    }
                }
            }

            // Sort each column: top-to-bottom, then left-to-right within same line.
            left.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            });
            right.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            });

            let left_text: String = left.iter().map(|&(_, _, c)| c).collect();
            let right_text: String = right.iter().map(|&(_, _, c)| c).collect();
            format!("{left_text}\n\n{right_text}")
        }
        _ => text_obj.all(),
    }
}

/// Render campaign PDF pages to JPEG AND detect each page's column layout.
/// Returns a `PageData` per page — either `TextLayer` (if native text is sufficient)
/// or `NeedsOcr` (JPEG + layout hint for vision OCR).
fn render_pages_with_layout(pdf_path: &std::path::PathBuf) -> Result<Vec<PageData>> {
    use image::ImageFormat;
    use pdfium_render::prelude::*;

    const TEXT_LAYER_MIN_WORDS: usize = 80;

    let lib_path = pdfium_lib_path()?;
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(lib_path)
            .map_err(|e| GuideError::PdfProcessing(format!("pdfium load failed: {e}")))?,
    );

    let doc = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| GuideError::PdfProcessing(format!("PDF open failed: {e}")))?;

    let page_count = doc.pages().len();
    tracing::info!("Processing {page_count} pages with layout detection from {:?}", pdf_path);

    let render_config = PdfRenderConfig::new()
        .set_target_width(1700)
        .set_maximum_height(2400)
        .rotate_if_landscape(PdfPageRenderRotation::None, true);

    let mut result = Vec::with_capacity(page_count as usize);

    for (index, page) in doc.pages().iter().enumerate() {
        let page_num = (index + 1) as u32;

        // Detect column layout from text layer (always available, fast).
        let layout = detect_page_layout(&page);

        // Check native text layer word count.
        let native_text = page.text().map(|t| t.all()).unwrap_or_default();
        let word_count = native_text.split_whitespace().count();

        if word_count >= TEXT_LAYER_MIN_WORDS {
            // Sufficient text layer — extract column-ordered text, skip vision OCR.
            let text = if layout != "unknown layout" {
                extract_column_ordered_text(&page, layout)
            } else {
                native_text
            };

            tracing::debug!(
                "Page {page_num}: text-layer path ({word_count} words, layout={layout})"
            );

            // Basic heading detection from the text layer.
            let headings: Vec<String> = text
                .lines()
                .filter(|l| {
                    let l = l.trim();
                    !l.is_empty()
                        && l.len() < 80
                        && (l.starts_with('#') || l.chars().filter(|c| c.is_uppercase()).count() > l.len() / 2)
                })
                .take(10)
                .map(|l| l.trim_start_matches('#').trim().to_string())
                .collect();

            result.push(PageData::TextLayer {
                page_num,
                text,
                headings,
            });
            continue;
        }

        // Sparse or no text layer — render to JPEG for vision OCR.
        tracing::debug!(
            "Page {page_num}: vision OCR path ({word_count} words, layout={layout})"
        );

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| GuideError::PdfProcessing(format!("render page {page_num}: {e}")))?;

        let image = bitmap.as_image();
        let pixel_area = image.width() * image.height();
        let image = if pixel_area < OCR_MIN_PIXELS {
            let scale = (OCR_MIN_PIXELS as f64 / pixel_area as f64).sqrt();
            let new_w = (image.width() as f64 * scale).round() as u32;
            let new_h = (image.height() as f64 * scale).round() as u32;
            image.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
        } else if pixel_area > OCR_MAX_PIXELS {
            let scale = (OCR_MAX_PIXELS as f64 / pixel_area as f64).sqrt();
            let new_w = (image.width() as f64 * scale).round() as u32;
            let new_h = (image.height() as f64 * scale).round() as u32;
            image.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
        } else {
            image
        };
        let aligned_w = align_to_patch(image.width());
        let aligned_h = align_to_patch(image.height());
        let image = if aligned_w != image.width() || aligned_h != image.height() {
            image.resize_exact(aligned_w, aligned_h, image::imageops::FilterType::Lanczos3)
        } else {
            image
        };

        let mut jpeg_bytes: Vec<u8> = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg_bytes),
                ImageFormat::Jpeg,
            )
            .map_err(|e| GuideError::PdfProcessing(format!("encode page {page_num}: {e}")))?;

        result.push(PageData::NeedsOcr { page_num, jpeg: jpeg_bytes, layout });
    }

    Ok(result)
}

/// OCR pipeline for campaign documents: uses text-layer extractions directly and
/// calls the vision model only for pages that need it (sparse/no text layer).
async fn ocr_pages_campaign(
    pages: Vec<PageData>,
    llm: &dyn LlmClient,
    ocr_model: &str,
    delay_ms: u64,
) -> Result<Vec<PageExtraction>> {
    let mut extractions = Vec::with_capacity(pages.len());

    for page_data in pages {
        match page_data {
            PageData::TextLayer { page_num, text, headings } => {
                extractions.push(PageExtraction { page_num, raw_text: normalize_page_text(&text), headings });
            }
            PageData::NeedsOcr { page_num, jpeg, layout } => {
                tracing::info!("Vision OCR page {page_num} ({} bytes, layout={layout})", jpeg.len());
                let prompt = prompts::ocr_campaign_prompt(layout);
                let vision_result = llm
                    .complete_with_vision(VisionRequest {
                        task: LlmTask::OcrExtraction,
                        prompt,
                        image_bytes: jpeg,
                        image_mime_type: "image/jpeg".into(),
                        model_override: Some(ocr_model.to_string()),
                        max_tokens: Some(4096),
                        temperature: Some(0.8),
                        top_p: Some(0.9),
                    })
                    .await;

                match vision_result {
                    Ok(resp) => {
                        let page_ocr: PageOcrResponse =
                            serde_json::from_str(resp.content.trim()).unwrap_or_else(|_| {
                                PageOcrResponse {
                                    raw_text: resp.content.clone(),
                                    headings: Vec::new(),
                                }
                            });
                        extractions.push(PageExtraction {
                            page_num,
                            raw_text: normalize_page_text(&page_ocr.raw_text),
                            headings: page_ocr.headings,
                        });
                    }
                    Err(e) => {
                        tracing::warn!("OCR failed for page {page_num}, skipping: {e}");
                        extractions.push(PageExtraction {
                            page_num,
                            raw_text: String::new(),
                            headings: Vec::new(),
                        });
                    }
                }

                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    Ok(extractions)
}

/// Directly parse a D&D Beyond fillable PDF character sheet into a ParsedSheetResult
/// WITHOUT needing an LLM call.  Returns `None` if the PDF doesn't appear to be a
/// standard D&D Beyond sheet (< 5 recognizable fields).
pub fn parse_character_sheet_from_form(
    pdf_bytes: &[u8],
) -> Option<guide_core::models::character::ParsedSheetResult> {
    use guide_core::models::character::{
        AbilityScores, ParsedSheetResult, SavingThrows, SkillScores, SpellSlot,
    };

    let fields = extract_form_fields(pdf_bytes);
    if fields.len() < 5 {
        return None;
    }

    let g = |key: &str| -> String { fields.get(key).cloned().unwrap_or_default() };
    let gi = |key: &str| -> i32 { g(key).parse().unwrap_or(0) };
    let gi_opt = |key: &str| -> Option<i32> {
        let v = g(key);
        if v.is_empty() { None } else { v.parse().ok() }
    };
    let gs = |key: &str| -> Option<String> {
        let v = g(key);
        if v.is_empty() { None } else { Some(v) }
    };

    // --- Character name ---
    let name = gs("CharacterName")
        .or_else(|| gs("CharacterName2"))
        .or_else(|| gs("CharacterName4"))
        .unwrap_or_else(|| "Unknown".into());

    // --- Class & Level: field is e.g. "Ranger 5" or just "Ranger" ---
    let class_level_raw = gs("CLASS  LEVEL")
        .or_else(|| gs("CLASS  LEVEL2"))
        .or_else(|| gs("ClassLevel"))
        .unwrap_or_default();
    let (class, level) = parse_class_level(&class_level_raw);

    // --- Race / Species ---
    let race = gs("RACE")
        .or_else(|| gs("RACE2"))
        .or_else(|| gs("Race"))
        .or_else(|| gs("Species"));

    // --- Background ---
    let background = gs("BACKGROUND")
        .or_else(|| gs("BACKGROUND2"))
        .or_else(|| gs("Background"));

    // --- XP ---
    let experience_points = gi_opt("EXPERIENCE POINTS")
        .or_else(|| gi_opt("EXPERIENCE POINTS2"))
        .or_else(|| gi_opt("XP"));

    // --- Ability scores ---
    let ability_scores = AbilityScores {
        strength:     gi("STR"),
        dexterity:    gi("DEX"),
        constitution: gi("CON"),
        intelligence: gi("INT"),
        wisdom:       gi("WIS"),
        charisma:     gi("CHA"),
    };

    // --- Core stats ---
    let max_hp = gi("MaxHP").max(gi("HPMax")).max(1);
    let armor_class = {
        let ac = gi("AC");
        if ac == 0 { 10 } else { ac }
    };
    let speed = {
        // Speed field may be "35 ft. (Walking)" — extract first integer
        let raw = g("Speed");
        let parsed: i32 = raw.split(|c: char| !c.is_ascii_digit())
            .find_map(|s| s.parse().ok())
            .unwrap_or(0);
        if parsed == 0 { 30 } else { parsed }
    };

    // --- Hit Dice ---
    let hit_dice = gs("Total") // "Total" field often holds "1d10" hit dice string
        .filter(|s| s.contains('d'))
        .or_else(|| gs("HDTotal"));

    // --- Saving Throws ---
    let has_saves = fields.contains_key("ST Strength") || fields.contains_key("ST Dexterity");
    let saving_throws = if has_saves {
        Some(SavingThrows {
            strength:     gi_opt("ST Strength"),
            dexterity:    gi_opt("ST Dexterity"),
            constitution: gi_opt("ST Constitution"),
            intelligence: gi_opt("ST Intelligence"),
            wisdom:       gi_opt("ST Wisdom"),
            charisma:     gi_opt("ST Charisma"),
        })
    } else {
        None
    };

    // --- Skills ---
    let has_skills = fields.contains_key("Acrobatics") || fields.contains_key("Perception");
    let skills = if has_skills {
        Some(SkillScores {
            acrobatics:      gi_opt("Acrobatics"),
            animal_handling: gi_opt("Animal"),
            arcana:          gi_opt("Arcana"),
            athletics:       gi_opt("Athletics"),
            deception:       gi_opt("Deception"),
            history:         gi_opt("History"),
            insight:         gi_opt("Insight"),
            intimidation:    gi_opt("Intimidation"),
            investigation:   gi_opt("Investigation"),
            medicine:        gi_opt("Medicine"),
            nature:          gi_opt("Nature"),
            perception:      gi_opt("Perception"),
            performance:     gi_opt("Performance"),
            persuasion:      gi_opt("Persuasion"),
            religion:        gi_opt("Religion"),
            sleight_of_hand: gi_opt("SleightofHand"),
            stealth:         gi_opt("Stealth"),
            survival:        gi_opt("Survival"),
        })
    } else {
        None
    };

    // --- Features & Traits ---
    let mut features: Vec<String> = Vec::new();
    for key in &["FeaturesTraits1", "FeaturesTraits2", "FeaturesTraits3", "FeaturesTraits"] {
        if let Some(v) = gs(key) {
            if !v.is_empty() { features.push(v); }
        }
    }

    // --- Proficiencies & Languages ---
    let (proficiencies, languages) = split_proficiencies_languages(
        &gs("ProficienciesLang").unwrap_or_default()
    );

    // --- Personality fields ---
    let personality_traits = gs("PersonalityTraits");
    let ideals = gs("Ideals");
    let bonds = gs("Bonds");
    let flaws = gs("Flaws");

    // --- Backstory ---
    let backstory_text = gs("Backstory").or_else(|| gs("CharacterBackstory"));

    // --- Spell slots ---
    // Two sets of fields: spellHeader{n} ("=== 1st LEVEL ===") and
    // spellSlotHeader{n} ("2 Slots OO" or "(At Will)").
    // Correlate them by index.
    let spell_slots: Vec<SpellSlot> = (0..10_usize).filter_map(|idx| {
        // Get the level from spellHeader (e.g. "=== 1st LEVEL ===")
        let section_header = gs(&format!("spellHeader{idx}")).unwrap_or_default();
        let slot_header = gs(&format!("spellSlotHeader{idx}")).unwrap_or_default();

        // Skip cantrips / "(At Will)" / empty
        if slot_header.is_empty()
            || slot_header.contains("At Will")
            || section_header.to_lowercase().contains("cantrip")
        {
            return None;
        }

        // Parse level from section header "=== 1st LEVEL ===" or "=== 2nd LEVEL ==="
        let slot_level = parse_spell_level_from_header(&section_header)
            .or_else(|| parse_spell_level_from_header(&slot_header))?;

        // Parse total from slot_header like "2 Slots OO" or "4 Slots OOOO"
        // First integer token is the count
        let total: i32 = slot_header
            .split_whitespace()
            .find_map(|tok| tok.parse().ok())
            .unwrap_or(0);

        if total <= 0 { return None; }

        // Count remaining from 'O' characters (filled circles = available slots)
        let remaining = slot_header.chars().filter(|&c| c == 'O').count() as i32;
        let remaining = remaining.min(total);

        Some(SpellSlot { level: slot_level, total, remaining })
    }).collect();

    // --- Confidence: based on how many key fields were populated ---
    let key_count = [
        !name.is_empty() && name != "Unknown",
        class.is_some(),
        race.is_some(),
        max_hp > 1,
        armor_class != 10,
        ability_scores.strength != 0,
        ability_scores.dexterity != 0,
    ].iter().filter(|&&b| b).count();
    let parse_confidence = (key_count as f32) / 7.0;

    tracing::info!(
        "Direct D&D Beyond parse: name={:?} class={:?} race={:?} level={} hp={} ac={} conf={:.2}",
        name, class, race, level, max_hp, armor_class, parse_confidence
    );

    Some(ParsedSheetResult {
        name,
        class,
        race,
        level,
        max_hp,
        armor_class,
        speed,
        ability_scores,
        backstory_text,
        raw_extracted_text: format_form_fields(&fields),
        parse_confidence,
        background,
        experience_points,
        hit_dice,
        saving_throws,
        skills,
        proficiencies,
        languages,
        features_and_traits: features,
        equipment: Vec::new(), // equipment is in a complex table format
        personality_traits,
        ideals,
        bonds,
        flaws,
        spell_slots,
    })
}

/// Parse "Ranger 5" → (Some("Ranger"), 5)  or  "Fighter" → (Some("Fighter"), 1)
fn parse_class_level(raw: &str) -> (Option<String>, i32) {
    let raw = raw.trim();
    if raw.is_empty() {
        return (None, 1);
    }
    // Try to split on last whitespace-separated token that's a number
    let parts: Vec<&str> = raw.splitn(10, ' ').collect();
    if let Some(last) = parts.last() {
        if let Ok(level) = last.parse::<i32>() {
            let class = parts[..parts.len() - 1].join(" ").trim().to_string();
            let class = if class.is_empty() { None } else { Some(class) };
            return (class, level.max(1));
        }
    }
    (Some(raw.to_string()), 1)
}

/// Parse the D&D Beyond `ProficienciesLang` field which looks like:
/// ```text
/// === ARMOR ===
/// Light Armor, Medium Armor, Shields
///
/// === WEAPONS ===
/// Martial Weapons, Simple Weapons
///
/// === TOOLS ===
/// Dragonchess Set
///
/// === LANGUAGES ===
/// Common, Elvish
/// ```
fn split_proficiencies_languages(text: &str) -> (Vec<String>, Vec<String>) {
    let mut proficiencies = Vec::new();
    let mut languages = Vec::new();
    let mut current_section = "proficiencies"; // default bucket

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        // Section header like "=== LANGUAGES ===" or "Languages:"
        if line.starts_with("===") && line.ends_with("===") {
            let section = line.trim_matches('=').trim().to_lowercase();
            if section.contains("language") {
                current_section = "languages";
            } else {
                current_section = "proficiencies";
            }
            continue;
        }

        // "Languages: Common, Elvish" inline format
        let lower = line.to_lowercase();
        if lower.starts_with("language") && line.contains(':') {
            let after = line.split_once(':').map(|x| x.1).unwrap_or("").trim();
            for item in after.split(',') {
                let s = item.trim().to_string();
                if !s.is_empty() { languages.push(s); }
            }
            continue;
        }

        // Regular content line — split by comma, add to current section
        for item in line.split(',') {
            let s = item.trim().to_string();
            if s.is_empty() || s.starts_with("===") { continue; }
            match current_section {
                "languages" => languages.push(s),
                _ => proficiencies.push(s),
            }
        }
    }

    (proficiencies, languages)
}

fn parse_spell_level_from_header(header: &str) -> Option<i32> {
    let header = header.to_lowercase();
    if header.contains("1st") || header.contains("cantrip") { return Some(1); }
    if header.contains("2nd") { return Some(2); }
    if header.contains("3rd") { return Some(3); }
    if header.contains("4th") { return Some(4); }
    if header.contains("5th") { return Some(5); }
    if header.contains("6th") { return Some(6); }
    if header.contains("7th") { return Some(7); }
    if header.contains("8th") { return Some(8); }
    if header.contains("9th") { return Some(9); }
    None
}

/// Extract raw text from a character sheet PDF using the best available method.
///
/// Strategy:
/// 1. Try AcroForm field extraction (fillable PDFs like D&D Beyond exports).
///    If >= 5 non-empty fields are found, format them as key: value text.
/// 2. Try pdfium text layer extraction (digital PDFs with embedded text).
/// 3. If text is sparse (< 50 words), render each page to JPEG and run the
///    vision OCR model with the simple OCR prompt to get plain text.
///    This handles scanned / image-only PDFs.
///
/// Returns the combined raw text from all pages.
pub async fn extract_character_sheet_text(
    pdf_bytes: &[u8],
    llm: &dyn LlmClient,
    ocr_model: &str,
) -> String {
    // Step 1: AcroForm field extraction (fillable PDFs)
    let form_fields = extract_form_fields(pdf_bytes);
    tracing::info!("AcroForm fields found: {}", form_fields.len());
    if form_fields.len() >= 5 {
        let form_text = format_form_fields(&form_fields);
        tracing::info!("Using AcroForm fields ({} fields)", form_fields.len());
        return form_text;
    }

    // Step 2: pdfium text layer
    let pdfium_text = extract_text_sync(pdf_bytes).unwrap_or_default();
    let word_count = pdfium_text.split_whitespace().count();
    tracing::info!("pdfium text extraction: {word_count} words");

    if word_count >= 50 {
        return pdfium_text;
    }

    tracing::info!("pdfium text insufficient ({word_count} words), falling back to vision OCR");

    // Step 2: render pages and OCR with simple text-extraction prompt
    let temp_path = std::env::temp_dir().join(format!(
        "guide-cs-text-{}.pdf",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    if std::fs::write(&temp_path, pdf_bytes).is_err() {
        return pdfium_text;
    }

    let path_buf = temp_path.clone();
    let pages = match tokio::task::spawn_blocking(move || render_pages_to_jpeg(&path_buf))
        .await
        .ok()
        .and_then(|r| r.ok())
    {
        Some(p) => p,
        None => return pdfium_text,
    };
    let _ = std::fs::remove_file(&temp_path);

    let prompt = prompts::ocr_simple_prompt().to_string();
    let mut combined = String::new();

    for (page_num, image_bytes) in pages {
        tracing::info!("Vision OCR page {page_num} for text extraction");
        if let Ok(resp) = llm
            .complete_with_vision(VisionRequest {
                task: LlmTask::OcrExtraction,
                prompt: prompt.clone(),
                image_bytes,
                image_mime_type: "image/jpeg".into(),
                model_override: Some(ocr_model.to_string()),
                max_tokens: Some(4096),
                temperature: Some(0.3),
                top_p: None,
            })
            .await
        {
            if !combined.is_empty() {
                combined.push_str("\n\n--- Page ");
                combined.push_str(&page_num.to_string());
                combined.push_str(" ---\n");
            }
            combined.push_str(&resp.content);
        }
    }

    if combined.trim().is_empty() { pdfium_text } else { combined }
}

/// Use GLM-OCR vision to parse a D&D character sheet PDF into a structured result.
/// Renders each page to JPEG and sends it to the OCR model with a structured JSON prompt.
/// Results from multiple pages are merged (scalars: first non-empty wins; lists: concat+dedup).
pub async fn ocr_character_sheet(
    pdf_bytes: &[u8],
    llm: &dyn LlmClient,
) -> Result<ParsedSheetResult> {
    use guide_core::models::character::{AbilityScores, SavingThrows, SkillScores, SpellSlot};

    // Write PDF to a temp file so render_pages_to_jpeg can load it
    let temp_path = std::env::temp_dir().join(format!(
        "guide-ocr-sheet-{}.pdf",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::write(&temp_path, pdf_bytes)
        .map_err(|e| GuideError::PdfProcessing(format!("failed to write temp pdf: {e}")))?;

    let path_buf = temp_path.clone();
    let pages = tokio::task::spawn_blocking(move || render_pages_to_jpeg(&path_buf))
        .await
        .map_err(|e| GuideError::PdfProcessing(format!("render task panicked: {e}")))??;

    let _ = std::fs::remove_file(&temp_path);

    if pages.is_empty() {
        return Err(GuideError::PdfProcessing("pdfium rendered 0 pages".into()));
    }

    let prompt = prompts::character_sheet_ocr_prompt().to_string();

    // Use serde_json::Value as intermediate to tolerate LLM type mismatches
    // (e.g. model returns integer 0 for a string field, or array for an object field).
    struct OcrSheet {
        name: String,
        class_name: String,
        race: String,
        background: String,
        level: i32,
        experience_points: i32,
        ability_scores: AbilityScores,
        saving_throws: SavingThrows,
        skills: SkillScores,
        max_hp: i32,
        armor_class: i32,
        speed: i32,
        hit_dice: String,
        proficiencies: Vec<String>,
        languages: Vec<String>,
        features_and_traits: Vec<String>,
        equipment: Vec<String>,
        personality_traits: String,
        ideals: String,
        bonds: String,
        flaws: String,
        spell_slots: Vec<SpellSlot>,
        backstory_text: String,
        parse_confidence: f32,
    }

    fn parse_ability_scores(v: &serde_json::Value) -> AbilityScores {
        if let serde_json::Value::Object(m) = v {
            AbilityScores {
                strength:     val_i32(m.get("strength").unwrap_or(&serde_json::Value::Null)),
                dexterity:    val_i32(m.get("dexterity").unwrap_or(&serde_json::Value::Null)),
                constitution: val_i32(m.get("constitution").unwrap_or(&serde_json::Value::Null)),
                intelligence: val_i32(m.get("intelligence").unwrap_or(&serde_json::Value::Null)),
                wisdom:       val_i32(m.get("wisdom").unwrap_or(&serde_json::Value::Null)),
                charisma:     val_i32(m.get("charisma").unwrap_or(&serde_json::Value::Null)),
            }
        } else {
            AbilityScores::default()
        }
    }

    fn parse_saving_throws(v: &serde_json::Value) -> SavingThrows {
        if let serde_json::Value::Object(m) = v {
            SavingThrows {
                strength:     val_i32_opt(m.get("strength").unwrap_or(&serde_json::Value::Null)),
                dexterity:    val_i32_opt(m.get("dexterity").unwrap_or(&serde_json::Value::Null)),
                constitution: val_i32_opt(m.get("constitution").unwrap_or(&serde_json::Value::Null)),
                intelligence: val_i32_opt(m.get("intelligence").unwrap_or(&serde_json::Value::Null)),
                wisdom:       val_i32_opt(m.get("wisdom").unwrap_or(&serde_json::Value::Null)),
                charisma:     val_i32_opt(m.get("charisma").unwrap_or(&serde_json::Value::Null)),
            }
        } else {
            SavingThrows::default()
        }
    }

    fn parse_skill_scores(v: &serde_json::Value) -> SkillScores {
        if let serde_json::Value::Object(m) = v {
            SkillScores {
                acrobatics:     val_i32_opt(m.get("acrobatics").unwrap_or(&serde_json::Value::Null)),
                animal_handling:val_i32_opt(m.get("animal_handling").unwrap_or(&serde_json::Value::Null)),
                arcana:         val_i32_opt(m.get("arcana").unwrap_or(&serde_json::Value::Null)),
                athletics:      val_i32_opt(m.get("athletics").unwrap_or(&serde_json::Value::Null)),
                deception:      val_i32_opt(m.get("deception").unwrap_or(&serde_json::Value::Null)),
                history:        val_i32_opt(m.get("history").unwrap_or(&serde_json::Value::Null)),
                insight:        val_i32_opt(m.get("insight").unwrap_or(&serde_json::Value::Null)),
                intimidation:   val_i32_opt(m.get("intimidation").unwrap_or(&serde_json::Value::Null)),
                investigation:  val_i32_opt(m.get("investigation").unwrap_or(&serde_json::Value::Null)),
                medicine:       val_i32_opt(m.get("medicine").unwrap_or(&serde_json::Value::Null)),
                nature:         val_i32_opt(m.get("nature").unwrap_or(&serde_json::Value::Null)),
                perception:     val_i32_opt(m.get("perception").unwrap_or(&serde_json::Value::Null)),
                performance:    val_i32_opt(m.get("performance").unwrap_or(&serde_json::Value::Null)),
                persuasion:     val_i32_opt(m.get("persuasion").unwrap_or(&serde_json::Value::Null)),
                religion:       val_i32_opt(m.get("religion").unwrap_or(&serde_json::Value::Null)),
                sleight_of_hand:val_i32_opt(m.get("sleight_of_hand").unwrap_or(&serde_json::Value::Null)),
                stealth:        val_i32_opt(m.get("stealth").unwrap_or(&serde_json::Value::Null)),
                survival:       val_i32_opt(m.get("survival").unwrap_or(&serde_json::Value::Null)),
            }
        } else {
            SkillScores::default()
        }
    }

    fn parse_spell_slots(v: &serde_json::Value) -> Vec<SpellSlot> {
        match v {
            serde_json::Value::Array(arr) => arr
                .iter()
                .filter_map(|item| {
                    if let serde_json::Value::Object(m) = item {
                        Some(SpellSlot {
                            level:     val_i32(m.get("level").unwrap_or(&serde_json::Value::Null)),
                            total:     val_i32(m.get("total").unwrap_or(&serde_json::Value::Null)),
                            remaining: val_i32(m.get("remaining").unwrap_or(&serde_json::Value::Null)),
                        })
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn parse_ocr_sheet(v: serde_json::Value) -> OcrSheet {
        let null = serde_json::Value::Null;
        OcrSheet {
            name:              val_str(v.get("name").unwrap_or(&null)),
            class_name:        val_str(v.get("class").unwrap_or(&null)),
            race:              val_str(v.get("race").unwrap_or(&null)),
            background:        val_str(v.get("background").unwrap_or(&null)),
            level:             val_i32(v.get("level").unwrap_or(&null)),
            experience_points: val_i32(v.get("experience_points").unwrap_or(&null)),
            ability_scores:    parse_ability_scores(v.get("ability_scores").unwrap_or(&null)),
            saving_throws:     parse_saving_throws(v.get("saving_throws").unwrap_or(&null)),
            skills:            parse_skill_scores(v.get("skills").unwrap_or(&null)),
            max_hp:            val_i32(v.get("max_hp").unwrap_or(&null)),
            armor_class:       val_i32(v.get("armor_class").unwrap_or(&null)),
            speed:             val_i32(v.get("speed").unwrap_or(&null)),
            hit_dice:          val_str(v.get("hit_dice").unwrap_or(&null)),
            proficiencies:     val_str_vec(v.get("proficiencies").unwrap_or(&null)),
            languages:         val_str_vec(v.get("languages").unwrap_or(&null)),
            features_and_traits: val_str_vec(v.get("features_and_traits").unwrap_or(&null)),
            equipment:         val_str_vec(v.get("equipment").unwrap_or(&null)),
            personality_traits:val_str(v.get("personality_traits").unwrap_or(&null)),
            ideals:            val_str(v.get("ideals").unwrap_or(&null)),
            bonds:             val_str(v.get("bonds").unwrap_or(&null)),
            flaws:             val_str(v.get("flaws").unwrap_or(&null)),
            spell_slots:       parse_spell_slots(v.get("spell_slots").unwrap_or(&null)),
            backstory_text:    val_str(v.get("backstory_text").unwrap_or(&null)),
            parse_confidence:  val_f32(v.get("parse_confidence").unwrap_or(&null)),
        }
    }

    let mut page_results: Vec<OcrSheet> = Vec::new();

    for (page_num, image_bytes) in pages {
        tracing::info!("OCR character sheet page {page_num} ({} bytes)", image_bytes.len());
        match llm
            .complete_with_vision(VisionRequest {
                task: LlmTask::CharacterSheetOcr,
                prompt: prompt.clone(),
                image_bytes,
                image_mime_type: "image/jpeg".into(),
                model_override: None,
                max_tokens: Some(4096),
                temperature: Some(0.8),
                top_p: Some(0.9),
            })
            .await
        {
            Ok(resp) => {
                let raw = resp.content.trim();
                let json_str = raw
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                match serde_json::from_str::<serde_json::Value>(json_str) {
                    Ok(val) => page_results.push(parse_ocr_sheet(val)),
                    Err(e) => {
                        tracing::warn!("Failed to parse OCR sheet JSON for page {page_num}: {e}");
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Vision OCR failed for page {page_num}: {e}");
            }
        }
    }

    if page_results.is_empty() {
        return Err(GuideError::PdfProcessing(
            "OCR produced no parseable results from any page".into(),
        ));
    }

    // Merge: scalars take first non-empty value; lists concatenate and dedup
    fn first_str(pages: &[OcrSheet], f: impl Fn(&OcrSheet) -> &str) -> Option<String> {
        pages.iter().map(f).find(|s| !s.is_empty()).map(|s| s.to_string())
    }
    fn first_i32(pages: &[OcrSheet], f: impl Fn(&OcrSheet) -> i32) -> i32 {
        pages.iter().map(f).find(|&v| v != 0).unwrap_or(0)
    }
    fn merge_list(pages: &[OcrSheet], f: impl Fn(&OcrSheet) -> &[String]) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for page in pages {
            for item in f(page) {
                if seen.insert(item.clone()) {
                    out.push(item.clone());
                }
            }
        }
        out
    }

    let confidence_sum: f32 = page_results.iter().map(|p| p.parse_confidence).sum();
    let confidence_avg = confidence_sum / page_results.len() as f32;

    // Ability scores: take from first page that has non-zero values
    let ability_scores = page_results
        .iter()
        .find(|p| p.ability_scores.strength != 0 || p.ability_scores.dexterity != 0)
        .map(|p| p.ability_scores.clone())
        .unwrap_or_default();

    // Saving throws: take from first page that has any non-null value
    let saving_throws = page_results
        .iter()
        .find(|p| {
            p.saving_throws.strength.is_some()
                || p.saving_throws.dexterity.is_some()
                || p.saving_throws.constitution.is_some()
        })
        .map(|p| SavingThrows {
            strength: p.saving_throws.strength,
            dexterity: p.saving_throws.dexterity,
            constitution: p.saving_throws.constitution,
            intelligence: p.saving_throws.intelligence,
            wisdom: p.saving_throws.wisdom,
            charisma: p.saving_throws.charisma,
        });

    // Skills: take from first page that has any non-null value
    let skills = page_results
        .iter()
        .find(|p| p.skills.acrobatics.is_some() || p.skills.perception.is_some())
        .map(|p| SkillScores {
            acrobatics: p.skills.acrobatics,
            animal_handling: p.skills.animal_handling,
            arcana: p.skills.arcana,
            athletics: p.skills.athletics,
            deception: p.skills.deception,
            history: p.skills.history,
            insight: p.skills.insight,
            intimidation: p.skills.intimidation,
            investigation: p.skills.investigation,
            medicine: p.skills.medicine,
            nature: p.skills.nature,
            perception: p.skills.perception,
            performance: p.skills.performance,
            persuasion: p.skills.persuasion,
            religion: p.skills.religion,
            sleight_of_hand: p.skills.sleight_of_hand,
            stealth: p.skills.stealth,
            survival: p.skills.survival,
        });

    Ok(ParsedSheetResult {
        name: first_str(&page_results, |p| &p.name).unwrap_or_else(|| "Unknown".into()),
        class: first_str(&page_results, |p| &p.class_name),
        race: first_str(&page_results, |p| &p.race),
        level: first_i32(&page_results, |p| p.level).max(1),
        max_hp: first_i32(&page_results, |p| p.max_hp).max(1),
        armor_class: {
            let ac = first_i32(&page_results, |p| p.armor_class);
            if ac == 0 { 10 } else { ac }
        },
        speed: {
            let sp = first_i32(&page_results, |p| p.speed);
            if sp == 0 { 30 } else { sp }
        },
        ability_scores,
        backstory_text: first_str(&page_results, |p| &p.backstory_text),
        raw_extracted_text: String::new(),
        parse_confidence: confidence_avg,
        background: first_str(&page_results, |p| &p.background),
        experience_points: {
            let xp = page_results.iter().map(|p| p.experience_points).find(|&v| v != 0);
            xp
        },
        hit_dice: first_str(&page_results, |p| &p.hit_dice),
        saving_throws,
        skills,
        proficiencies: merge_list(&page_results, |p| &p.proficiencies),
        languages: merge_list(&page_results, |p| &p.languages),
        features_and_traits: merge_list(&page_results, |p| &p.features_and_traits),
        equipment: merge_list(&page_results, |p| &p.equipment),
        personality_traits: first_str(&page_results, |p| &p.personality_traits),
        ideals: first_str(&page_results, |p| &p.ideals),
        bonds: first_str(&page_results, |p| &p.bonds),
        flaws: first_str(&page_results, |p| &p.flaws),
        spell_slots: page_results
            .iter()
            .find(|p| !p.spell_slots.is_empty())
            .map(|p| p.spell_slots.clone())
            .unwrap_or_default(),
    })
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
                max_tokens: Some(4096),
                temperature: Some(0.8),
                top_p: Some(0.9),
            })
            .await;

        match vision_result {
            Ok(vision_resp) => {
                let page_ocr: PageOcrResponse = serde_json::from_str(vision_resp.content.trim())
                    .unwrap_or_else(|_| PageOcrResponse {
                        raw_text: vision_resp.content.clone(),
                        headings: Vec::new(),
                    });
                extractions.push(PageExtraction {
                    page_num,
                    raw_text: normalize_page_text(&page_ocr.raw_text),
                    headings: page_ocr.headings,
                });
            }
            Err(e) => {
                tracing::warn!("OCR failed for page {page_num}, skipping: {e}");
                extractions.push(PageExtraction {
                    page_num,
                    raw_text: String::new(),
                    headings: Vec::new(),
                });
            }
        }

        if delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
    }

    Ok(extractions)
}
