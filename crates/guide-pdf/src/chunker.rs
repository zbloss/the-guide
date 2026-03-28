use guide_core::Result;

use crate::extractor::PageExtraction;

pub struct DocumentChunk {
    pub content: String,
    pub page_range: (u32, u32),
    pub section_path: String,
}

pub async fn chunk_document(
    pages: &[PageExtraction],
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

    for page in pages {
        let page_num = page.page_num;

        for line in page.raw_text.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("## ") || trimmed.starts_with("### ") || looks_like_heading(trimmed) {
                if !current_lines.is_empty() {
                    candidates.push(SectionCandidate {
                        content: current_lines.join("\n"),
                        page_range: (current_page_start, current_page_end),
                        section_path: section_path(&heading_stack),
                    });
                    current_lines.clear();
                }

                if trimmed.starts_with("## ") {
                    heading_stack.clear();
                    heading_stack.push(trimmed.trim_start_matches("## ").to_string());
                } else if trimmed.starts_with("### ") {
                    if heading_stack.len() > 1 {
                        heading_stack.truncate(1);
                    }
                    heading_stack.push(trimmed.trim_start_matches("### ").to_string());
                } else {
                    // Plain-text heading detected via looks_like_heading
                    heading_stack.clear();
                    heading_stack.push(trimmed.to_string());
                }

                current_page_start = page_num;
                current_page_end = page_num;
            } else {
                current_lines.push(line.to_string());
                current_page_end = page_num;
            }
        }
    }

    if !current_lines.is_empty() {
        candidates.push(SectionCandidate {
            content: current_lines.join("\n"),
            page_range: (current_page_start, current_page_end),
            section_path: section_path(&heading_stack),
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
}

fn looks_like_heading(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || t.len() > 80 {
        return false;
    }
    // Already handled upstream (guard kept for standalone-call safety)
    if t.starts_with("## ") || t.starts_with("### ") {
        return false;
    }
    let lower = t.to_lowercase();

    // "Chapter N / Part N / Act N / Scene N" — require a digit or uppercase letter immediately
    // after the keyword space so we don't match ordinary sentences like
    // "Chapter contains…" or "Act accordingly." or "part of the dungeon".
    let section_keyword = ["chapter ", "part ", "act ", "scene "].iter().any(|prefix| {
        if !lower.starts_with(prefix) {
            return false;
        }
        t[prefix.len()..].chars().next().is_some_and(|c| c.is_ascii_digit() || c.is_uppercase())
    });

    // All-caps title (e.g., "THE AMBER TEMPLE", "CASTLE RAVENLOFT"):
    // • Every alphabetic character in the line must be uppercase.
    // • At least one whitespace-separated word must have ≥5 consecutive uppercase alpha chars.
    //   This excludes common D&D abbreviations (STR, DEX, CON, NPC, ATK, …) and
    //   single-word stat labels that appear inline in stat blocks.
    let has_long_caps_word = t.split_whitespace().any(|word| {
        let alpha: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
        alpha.len() >= 5 && alpha.iter().all(|c| c.is_uppercase())
    });
    let all_caps_title = has_long_caps_word
        && t.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase());

    section_keyword || all_caps_title
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
        let start = content.len() - overlap;
        let start = content.floor_char_boundary(start);
        Some(content[start..].to_string())
    }
}
