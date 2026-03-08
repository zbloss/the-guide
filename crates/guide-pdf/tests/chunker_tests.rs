use guide_pdf::{chunker::chunk_document, extractor::PageExtraction};

fn page(num: u32, text: &str, dm_only: bool) -> PageExtraction {
    PageExtraction {
        page_num: num,
        raw_text: text.to_string(),
        headings: vec![],
        is_dm_only: dm_only,
    }
}

#[tokio::test]
async fn test_empty_pages_returns_empty() {
    let chunks = chunk_document(vec![], 1600, 200).await.unwrap();
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn test_single_page_no_headings() {
    let pages = vec![page(1, "This is some content without headings.", false)];
    let chunks = chunk_document(pages, 1600, 200).await.unwrap();
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("This is some content"));
    assert!(chunks[0].is_player_visible);
    assert_eq!(chunks[0].section_path, "");
}

#[tokio::test]
async fn test_section_split_on_headings() {
    let text = "## Chapter One\nContent of chapter one.\n### Section A\nSection A content.\n## Chapter Two\nContent of chapter two.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(pages, 1600, 0).await.unwrap();
    // Each section becomes a chunk
    assert!(chunks.len() >= 3);
}

#[tokio::test]
async fn test_section_path_set_correctly() {
    let text = "## Main Chapter\nIntro text.\n### Sub Section\nSub content here.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(pages, 1600, 0).await.unwrap();

    // Find chunk with section path containing sub section
    let sub_chunk = chunks.iter().find(|c| c.section_path.contains("Sub Section"));
    assert!(sub_chunk.is_some(), "Expected a chunk with Sub Section in path");
    assert!(sub_chunk.unwrap().section_path.contains("Main Chapter"));
}

#[tokio::test]
async fn test_dm_only_pages_not_player_visible() {
    let pages = vec![page(1, "DM only content here.", true)];
    let chunks = chunk_document(pages, 1600, 200).await.unwrap();
    assert_eq!(chunks.len(), 1);
    assert!(!chunks[0].is_player_visible);
}

#[tokio::test]
async fn test_long_section_gets_split() {
    // Create a section that's much longer than chunk_max
    let long_text = "## Long Section\n".to_string()
        + &(0..50).map(|i| format!("Sentence number {i} with some content.")).collect::<Vec<_>>().join(" ");
    let pages = vec![page(1, &long_text, false)];
    let chunks = chunk_document(pages, 200, 50).await.unwrap();
    // Should produce multiple sub-chunks
    assert!(chunks.len() > 1, "Expected multiple chunks for long section");
}

#[tokio::test]
async fn test_overlap_prepended_to_subsequent_chunks() {
    let long_sentence = "A".repeat(300);
    let text = format!("## Section\n{long_sentence}. {long_sentence}. {long_sentence}.");
    let pages = vec![page(1, &text, false)];
    let chunks = chunk_document(pages, 400, 100).await.unwrap();

    // Chunks after the first should have overlap marker
    if chunks.len() > 1 {
        assert!(
            chunks[1].content.contains("[continued from previous section]"),
            "Expected overlap marker in second chunk"
        );
    }
}

#[tokio::test]
async fn test_multi_page_document() {
    let pages = vec![
        page(1, "## Chapter One\nFirst page content.", false),
        page(2, "Continuation of chapter.", false),
        page(3, "## Chapter Two\nSecond chapter.", false),
    ];
    let chunks = chunk_document(pages, 1600, 0).await.unwrap();
    assert!(chunks.len() >= 2);

    // Check that page ranges are tracked
    let ch2 = chunks.iter().find(|c| c.section_path.contains("Chapter Two"));
    assert!(ch2.is_some());
    assert_eq!(ch2.unwrap().page_range.0, 3);
}

#[tokio::test]
async fn test_section_path_resets_on_level2_heading() {
    let text = "## Chapter A\nContent A.\n### Sub of A\nSub content.\n## Chapter B\nContent B.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(pages, 1600, 0).await.unwrap();

    let ch_b = chunks.iter().find(|c| c.section_path == "Chapter B");
    assert!(
        ch_b.is_some(),
        "Chapter B should have clean path without Sub of A. Got: {:?}",
        chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>()
    );
}
