use guide_core::Result;

use crate::extractor::PageExtraction;

pub struct DocumentChunk {
    pub content: String,
    pub page_range: (u32, u32),
    pub section_path: String,
    pub is_player_visible: bool,
}

pub async fn chunk_document(
    pages: Vec<PageExtraction>,
    chunk_max_chars: usize,
    chunk_overlap_chars: usize,
) -> Result<Vec<DocumentChunk>> {
    const DEFAULT_MAX: usize = 1_600;
    const DEFAULT_OVERLAP: usize = 200;

    let chunk_max = if chunk_max_chars == 0 { DEFAULT_MAX } else { chunk_max_chars };
    let overlap = if chunk_overlap_chars == 0 { DEFAULT_OVERLAP } else { chunk_overlap_chars };

    // Pass 1: structural split on heading markers
    let mut candidates: Vec<SectionCandidate> = Vec::new();
    let mut heading_stack: Vec<String> = Vec::new();
    let mut current_lines: Vec<String> = Vec::new();
    let mut current_page_start: u32 = 0;
    let mut current_page_end: u32 = 0;
    let mut current_is_dm_only = false;

    for page in &pages {
        let page_num = page.page_num;
        let is_dm_only = page.is_dm_only;

        for line in page.raw_text.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
                if !current_lines.is_empty() {
                    candidates.push(SectionCandidate {
                        content: current_lines.join("\n"),
                        page_range: (current_page_start, current_page_end),
                        section_path: section_path(&heading_stack),
                        is_player_visible: !current_is_dm_only,
                    });
                    current_lines.clear();
                }

                if trimmed.starts_with("## ") {
                    heading_stack.clear();
                    heading_stack.push(trimmed.trim_start_matches("## ").to_string());
                } else {
                    if heading_stack.len() > 1 {
                        heading_stack.truncate(1);
                    }
                    heading_stack.push(trimmed.trim_start_matches("### ").to_string());
                }

                current_page_start = page_num;
                current_page_end = page_num;
                current_is_dm_only = is_dm_only;
            } else {
                current_lines.push(line.to_string());
                current_page_end = page_num;
                if is_dm_only {
                    current_is_dm_only = true;
                }
            }
        }
    }

    if !current_lines.is_empty() {
        candidates.push(SectionCandidate {
            content: current_lines.join("\n"),
            page_range: (current_page_start, current_page_end),
            section_path: section_path(&heading_stack),
            is_player_visible: !current_is_dm_only,
        });
    }

    if candidates.is_empty() && !pages.is_empty() {
        let all_text: String = pages
            .iter()
            .map(|p| p.raw_text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let last_page = pages.last().map(|p| p.page_num).unwrap_or(0);
        candidates.push(SectionCandidate {
            content: all_text,
            page_range: (0, last_page),
            section_path: String::new(),
            is_player_visible: !pages.iter().any(|p| p.is_dm_only),
        });
    }

    // Pass 2: token-budget sub-chunking with overlap
    let mut chunks: Vec<DocumentChunk> = Vec::new();
    let mut prev_tail: Option<String> = None;

    for candidate in candidates {
        if candidate.content.len() <= chunk_max {
            let content = prepend_overlap(&candidate.content, prev_tail.take());
            prev_tail = tail_overlap(&candidate.content, overlap);
            chunks.push(DocumentChunk {
                content,
                page_range: candidate.page_range,
                section_path: candidate.section_path,
                is_player_visible: candidate.is_player_visible,
            });
        } else {
            let sub_chunks = split_at_sentences(&candidate.content, chunk_max);
            for (i, sub) in sub_chunks.into_iter().enumerate() {
                let tail = if i == 0 {
                    prev_tail.take()
                } else {
                    chunks.last().and_then(|c| tail_overlap(&c.content, overlap))
                };
                let content = prepend_overlap(&sub, tail);
                prev_tail = tail_overlap(&content, overlap);
                chunks.push(DocumentChunk {
                    content,
                    page_range: candidate.page_range,
                    section_path: candidate.section_path.clone(),
                    is_player_visible: candidate.is_player_visible,
                });
            }
        }
    }

    Ok(chunks)
}

struct SectionCandidate {
    content: String,
    page_range: (u32, u32),
    section_path: String,
    is_player_visible: bool,
}

fn section_path(stack: &[String]) -> String {
    stack.join(" > ")
}

fn split_at_sentences(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for sentence in text.split_inclusive(['.', '?', '!']) {
        if current.len() + sentence.len() > max_chars && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current = sentence.to_string();
        } else {
            current.push_str(sentence);
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }

    if chunks.is_empty() {
        chunks = text
            .as_bytes()
            .chunks(max_chars)
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .collect();
    }

    chunks
}

fn prepend_overlap(content: &str, tail: Option<String>) -> String {
    match tail {
        Some(t) if !t.is_empty() => {
            format!("[continued from previous section]\n{t}\n\n{content}")
        }
        _ => content.to_string(),
    }
}

fn tail_overlap(content: &str, overlap: usize) -> Option<String> {
    if content.is_empty() {
        None
    } else if content.len() <= overlap {
        Some(content.to_string())
    } else {
        Some(content[content.len() - overlap..].to_string())
    }
}
