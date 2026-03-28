/// Deterministic pre-pass that detects Table of Contents and back-of-book Index
/// pages from the raw page text produced by the PDF extractor.
///
/// Neither detection path requires an LLM call — both rely on typographic patterns
/// that are highly consistent across published D&D campaign books:
///
/// **ToC**: dotted-leader lines of the form `<Title> .... <page_number>` found
///   in the first 15 pages of the document.
///
/// **Index**: alphabetical term entries followed by comma-separated page numbers
///   found in the last 25 pages of the document.
use guide_core::models::{IndexEntry, TocEntry};

use crate::extractor::PageExtraction;

// ── constants ─────────────────────────────────────────────────────────────────

/// How many pages from the start to scan for a ToC.
const TOC_SCAN_PAGES: usize = 15;
/// Minimum dotted-leader lines for a page to qualify as a ToC page.
const TOC_MIN_LINES: usize = 4;

/// How many pages from the end to scan for a back-of-book Index.
const INDEX_SCAN_PAGES: usize = 25;
/// Minimum index-looking lines for a page to qualify as an Index page.
const INDEX_MIN_LINES: usize = 8;

// ── public entry point ────────────────────────────────────────────────────────

/// Scan extracted page text and return any ToC and Index entries found.
/// Both results may be empty if the document has neither section.
pub fn detect_document_navigation(
    pages: &[PageExtraction],
) -> (Vec<TocEntry>, Vec<IndexEntry>) {
    let toc = detect_toc(pages);
    let index = detect_index(pages);
    (toc, index)
}

/// Detect the offset between printed page numbers (as stored in ToC entries) and
/// PDF page indices (1-based physical page position).
///
/// Most campaign books start with a cover, blank page, or front matter, so the
/// physical PDF page 1 corresponds to printed page 0 (or "i").  The offset
/// `pdf_page - printed_page` must be added to any printed page number from the
/// ToC before calling `get_page`.
///
/// Strategy: scan the first 20 page texts for a lone small integer (1–30) in a
/// footer or header position (short line at the start or end of the text).  When
/// a candidate printed number matches a ToC entry's page, we compute and return
/// the offset.  Returns 0 if no reliable calibration can be made.
///
/// `pages` is a mapping of `pdf_page_num (1-based) → raw_text`.
pub fn detect_page_offset(
    pages: &std::collections::HashMap<u32, String>,
    toc: &[TocEntry],
) -> i32 {
    if toc.is_empty() {
        return 0;
    }

    // Build a quick set of printed page numbers from the ToC for fast lookup.
    let toc_pages: std::collections::HashSet<u32> = toc.iter().map(|e| e.page).collect();

    // Scan the first 20 PDF pages (sorted by page number).
    let mut sorted_pages: Vec<u32> = pages.keys().copied().collect();
    sorted_pages.sort_unstable();

    for pdf_page in sorted_pages.into_iter().take(20) {
        let raw_text = match pages.get(&pdf_page) {
            Some(t) => t,
            None => continue,
        };
        // Check the first and last non-empty lines for a lone small page number.
        let lines: Vec<&str> = raw_text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();

        let candidates = lines.first().into_iter().chain(lines.last());
        for line in candidates {
            if let Ok(printed) = line.parse::<u32>() {
                if printed > 0 && printed <= 30 && toc_pages.contains(&printed) {
                    let offset = pdf_page as i32 - printed as i32;
                    if offset >= 0 {
                        tracing::debug!(
                            pdf_page,
                            printed_page = printed,
                            offset,
                            "Detected page number offset from footer"
                        );
                        return offset;
                    }
                }
            }
        }
    }

    0
}

// ── ToC detection ─────────────────────────────────────────────────────────────

fn detect_toc(pages: &[PageExtraction]) -> Vec<TocEntry> {
    let scan_end = pages.len().min(TOC_SCAN_PAGES);
    let mut entries: Vec<TocEntry> = Vec::new();

    for page in &pages[..scan_end] {
        let candidates: Vec<(String, u32, u8)> = page
            .raw_text
            .lines()
            .filter_map(parse_toc_line)
            .collect();

        if candidates.len() >= TOC_MIN_LINES {
            for (title, page_num, depth) in candidates {
                entries.push(TocEntry { title, page: page_num, depth });
            }
        }
    }

    // Deduplicate: keep the first occurrence of each (title, page) pair.
    let mut seen = std::collections::HashSet::new();
    entries.retain(|e| seen.insert((e.title.clone(), e.page)));

    entries
}

/// Try to parse a ToC dotted-leader line.
/// Expected form: `<indent><title><dots><space><page_number>`
/// where `<dots>` is 3 or more of `.`, `·`, `–`, or `-`.
fn parse_toc_line(line: &str) -> Option<(String, u32, u8)> {
    // Count leading whitespace for depth estimation.
    let indent_len = line.len() - line.trim_start().len();
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return None;
    }

    // Extract the trailing page number by scanning from the right for the first
    // non-digit character.  This is robust to tabs anywhere in the title text
    // (e.g. "Leveling\tUp.........6" from multi-column OCR).
    let digit_start = trimmed
        .rfind(|c: char| !c.is_ascii_digit())
        .map(|i| i + trimmed[i..].chars().next().map_or(1, |c| c.len_utf8()))
        .unwrap_or(0);
    let page_str = trimmed[digit_start..].trim();
    let page: u32 = page_str.parse().ok().filter(|&p| p > 0)?;
    let rest = trimmed[..digit_start].trim_end();

    // The part before the page number must contain a dotted-leader run of ≥3 chars.
    let leader_chars = ['.', '·', '\u{2026}', '\u{00B7}', '-', '\u{2013}', '\u{2014}'];
    let leader_start = rest.rfind(|c: char| !leader_chars.contains(&c))?;
    let after_content = &rest[leader_start + rest[leader_start..].chars().next().map_or(1, |c| c.len_utf8())..];

    // Count contiguous leader chars at the start of what remains.
    let leader_run: usize = after_content.chars().take_while(|c| leader_chars.contains(c)).count();
    if leader_run < 3 {
        return None;
    }

    // Replace any whitespace characters (including tabs from OCR column fusion) with spaces.
    let raw_title = rest[..=leader_start].trim_end_matches(|c: char| leader_chars.contains(&c)).trim();
    let title: String = raw_title
        .chars()
        .map(|c| if c.is_ascii_whitespace() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if title.is_empty() {
        return None;
    }

    // Depth heuristic: every 3 spaces of leading indent → +1 nesting level (capped at 3).
    let depth = ((indent_len / 3) as u8).min(3);

    Some((title, page, depth))
}

// ── Index detection ───────────────────────────────────────────────────────────

fn detect_index(pages: &[PageExtraction]) -> Vec<IndexEntry> {
    if pages.is_empty() {
        return Vec::new();
    }
    let scan_start = pages.len().saturating_sub(INDEX_SCAN_PAGES);
    let mut entries: Vec<IndexEntry> = Vec::new();

    for page in &pages[scan_start..] {
        let candidates: Vec<(String, Vec<u32>)> = page
            .raw_text
            .lines()
            .filter_map(parse_index_line)
            .collect();

        if candidates.len() >= INDEX_MIN_LINES {
            for (term, pages) in candidates {
                entries.push(IndexEntry { term, pages });
            }
        }
    }

    // Merge duplicate terms (some indexes span multiple columns / pages).
    let mut merged: std::collections::HashMap<String, Vec<u32>> =
        std::collections::HashMap::new();
    for entry in entries {
        let key = entry.term.to_lowercase();
        merged.entry(key).or_default().extend(entry.pages);
    }

    let mut result: Vec<IndexEntry> = merged
        .into_iter()
        .map(|(key, mut pages)| {
            pages.sort_unstable();
            pages.dedup();
            IndexEntry { term: key, pages }
        })
        .collect();

    result.sort_by(|a, b| a.term.cmp(&b.term));
    result
}

/// Try to parse a back-of-book index line.
/// Expected form: `<Term>, <page>[, <page>…]`
/// where `<Term>` starts with a letter and contains no more than ~60 characters,
/// and each `<page>` is a number (ranges like "14–16" are accepted; only the
/// first page of the range is kept).
fn parse_index_line(line: &str) -> Option<(String, Vec<u32>)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Must start with a letter.
    if !line.starts_with(|c: char| c.is_alphabetic()) {
        return None;
    }

    // Split at the first comma to separate term from page list.
    let (term, pages_raw) = line.split_once(',')?;
    let term = term.trim();

    // Term must be ≤60 chars and not contain digits (avoids matching sentence fragments).
    if term.len() > 60 || term.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }

    // Parse page references — may include ranges like "109-111", "109–111".
    let pages: Vec<u32> = pages_raw
        .split(',')
        .filter_map(|token| {
            let token = token.trim();
            // Take only the first number of a range.
            let first = token.split(['-', '\u{2013}']).next()?;
            first.trim().parse::<u32>().ok().filter(|&p| p > 0)
        })
        .collect();

    if term.is_empty() || pages.is_empty() {
        return None;
    }

    Some((term.to_string(), pages))
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_page(page_num: u32, text: &str) -> PageExtraction {
        PageExtraction {
            page_num,
            raw_text: text.to_string(),
            headings: vec![],
        }
    }

    #[test]
    fn detects_toc_dotted_leaders() {
        let toc_text = "\
Chapter 1: Welcome to Barovia .......... 5\n\
Chapter 2: The Village of Barovia ..... 14\n\
Chapter 3: The Village of Krezk ........ 38\n\
Chapter 4: Castle Ravenloft ............. 55\n\
Appendix A: Monsters ................... 230\n";
        let pages = vec![fake_page(2, toc_text)];
        let (toc, _) = detect_document_navigation(&pages);
        assert_eq!(toc.len(), 5);
        assert_eq!(toc[0].title, "Chapter 1: Welcome to Barovia");
        assert_eq!(toc[0].page, 5);
        assert_eq!(toc[0].depth, 0);
    }

    #[test]
    fn toc_handles_tab_separated_ocr() {
        // Multi-column OCR produces tabs within titles like "Leveling\tUp.........6"
        let toc_text = "\
Introduction ..............................3\n\
Campaign\tOverview...........................4\n\
Leveling\tUp.........................................................................6\n\
Chapter 1: The\tVillage ....................................................14\n\
Chapter 2: The\tCastle ....................................................55\n";
        let pages = vec![fake_page(2, toc_text)];
        let (toc, _) = detect_document_navigation(&pages);
        assert_eq!(toc.len(), 5);
        assert_eq!(toc[1].title, "Campaign Overview");
        assert_eq!(toc[1].page, 4);
        assert_eq!(toc[2].title, "Leveling Up");
        assert_eq!(toc[2].page, 6);
    }

    #[test]
    fn toc_not_detected_on_sparse_page() {
        // Only 2 dotted-leader lines — below the threshold.
        let text = "Some random page\nChapter 1 ...... 5\nAnother line\nChapter 2 ...... 14\n";
        let pages = vec![fake_page(1, text)];
        let (toc, _) = detect_document_navigation(&pages);
        assert!(toc.is_empty());
    }

    #[test]
    fn detects_index_entries() {
        let index_text = "\
Barovia, 5, 14, 109\n\
Bats, 23, 45\n\
Beasts, 78\n\
Bones of St. Andral, 55\n\
Burgomaster, 14, 21\n\
Castle Ravenloft, 55, 78, 100\n\
Coffin maker, 30\n\
Crypts, 100\n\
Death house, 7\n";
        let pages = vec![fake_page(240, index_text)];
        let (_, index) = detect_document_navigation(&pages);
        assert!(index.len() >= 8);
        let barovia = index.iter().find(|e| e.term == "barovia").unwrap();
        assert!(barovia.pages.contains(&5));
        assert!(barovia.pages.contains(&14));
    }

    #[test]
    fn index_not_detected_in_early_pages() {
        // Index-looking text on page 3 should NOT be picked up (only last 25 pages scanned).
        let index_text = "\
Alpha, 1, 2\nBeta, 3\nGamma, 4\nDelta, 5\nEpsilon, 6\nZeta, 7\nEta, 8\nTheta, 9\n";
        let pages: Vec<PageExtraction> = (1u32..=300)
            .map(|n| fake_page(n, if n == 3 { index_text } else { "Regular content here.\n" }))
            .collect();
        // Index-looking text only on page 3; last 25 pages contain generic text.
        let (_, index) = detect_document_navigation(&pages);
        assert!(index.is_empty());
    }
}
