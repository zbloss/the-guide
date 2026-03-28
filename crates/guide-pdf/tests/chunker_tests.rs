use guide_pdf::{chunker::chunk_document, extractor::PageExtraction};

fn page(num: u32, text: &str, _dm_only: bool) -> PageExtraction {
    PageExtraction {
        page_num: num,
        raw_text: text.to_string(),
        headings: vec![],
    }
}

#[tokio::test]
async fn test_empty_pages_returns_empty() {
    let chunks = chunk_document(&[], 1600, 200).await.unwrap();
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn test_single_page_no_headings() {
    let pages = vec![page(1, "This is some content without headings.", false)];
    let chunks = chunk_document(&pages, 1600, 200).await.unwrap();
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("This is some content"));
    assert_eq!(chunks[0].section_path, "");
}

#[tokio::test]
async fn test_section_split_on_headings() {
    let text = "## Chapter One\nContent of chapter one.\n### Section A\nSection A content.\n## Chapter Two\nContent of chapter two.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();
    // Each section becomes a chunk
    assert!(chunks.len() >= 3);
}

#[tokio::test]
async fn test_section_path_set_correctly() {
    let text = "## Main Chapter\nIntro text.\n### Sub Section\nSub content here.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();

    // Find chunk with section path containing sub section
    let sub_chunk = chunks.iter().find(|c| c.section_path.contains("Sub Section"));
    assert!(sub_chunk.is_some(), "Expected a chunk with Sub Section in path");
    assert!(sub_chunk.unwrap().section_path.contains("Main Chapter"));
}

#[tokio::test]
async fn test_long_section_gets_split() {
    // Create a section that's much longer than chunk_max
    let long_text = "## Long Section\n".to_string()
        + &(0..50).map(|i| format!("Sentence number {i} with some content.")).collect::<Vec<_>>().join(" ");
    let pages = vec![page(1, &long_text, false)];
    let chunks = chunk_document(&pages, 200, 50).await.unwrap();
    // Should produce multiple sub-chunks
    assert!(chunks.len() > 1, "Expected multiple chunks for long section");
}

#[tokio::test]
async fn test_overlap_prepended_to_subsequent_chunks() {
    let long_sentence = "A".repeat(300);
    let text = format!("## Section\n{long_sentence}. {long_sentence}. {long_sentence}.");
    let pages = vec![page(1, &text, false)];
    let chunks = chunk_document(&pages, 400, 100).await.unwrap();

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
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();
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
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();

    let ch_b = chunks.iter().find(|c| c.section_path == "Chapter B");
    assert!(
        ch_b.is_some(),
        "Chapter B should have clean path without Sub of A. Got: {:?}",
        chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>()
    );
}

// ── looks_like_heading integration tests ──────────────────────────────────────

#[tokio::test]
async fn test_plain_text_chapter_heading_splits_sections() {
    // Plain "Chapter N: ..." lines (no ## prefix) should create separate chunks
    let text = "Intro content before any chapter.\nChapter 1: The Village of Barovia\nContent about the village.\nChapter 2: Castle Ravenloft\nContent about the castle.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();

    assert!(
        chunks.len() >= 2,
        "Expected at least 2 chunks from plain-text chapter headings, got {}. Paths: {:?}",
        chunks.len(),
        chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>()
    );
    let has_ch1 = chunks.iter().any(|c| c.section_path.contains("Chapter 1"));
    let has_ch2 = chunks.iter().any(|c| c.section_path.contains("Chapter 2"));
    assert!(has_ch1, "Expected 'Chapter 1' in section paths. Got: {:?}", chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>());
    assert!(has_ch2, "Expected 'Chapter 2' in section paths. Got: {:?}", chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_all_caps_heading_splits_sections() {
    // All-caps short titles should be recognised as section boundaries
    let text = "Some preamble text here.\nTHE AMBER TEMPLE\nDescription of the temple.\nCASTLE RAVENLOFT\nDescription of the castle.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();

    let has_amber = chunks.iter().any(|c| c.section_path.contains("THE AMBER TEMPLE"));
    let has_castle = chunks.iter().any(|c| c.section_path.contains("CASTLE RAVENLOFT"));
    assert!(has_amber, "Expected 'THE AMBER TEMPLE' section. Paths: {:?}", chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>());
    assert!(has_castle, "Expected 'CASTLE RAVENLOFT' section. Paths: {:?}", chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_act_and_scene_headings_split_sections() {
    let text = "Prologue text.\nAct 1: A Dark Beginning\nAct content here.\nAct 2: The Long Road\nMore act content.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();

    let has_act1 = chunks.iter().any(|c| c.section_path.contains("Act 1"));
    let has_act2 = chunks.iter().any(|c| c.section_path.contains("Act 2"));
    assert!(has_act1, "Expected 'Act 1' section. Paths: {:?}", chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>());
    assert!(has_act2, "Expected 'Act 2' section. Paths: {:?}", chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_short_dnd_abbreviations_not_treated_as_headings() {
    // 3-char all-caps D&D abbreviations must NOT split sections
    let text = "## Monster Stat Block\nSTR DEX CON INT WIS CHA\n10 10 10 10 10 10\nNPC encounters in this area.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();

    // All content after the ## heading should be in one chunk (not split by STR/NPC/etc.)
    let stat_block_chunks: Vec<_> = chunks.iter().filter(|c| c.section_path.contains("Monster Stat Block")).collect();
    assert!(
        !stat_block_chunks.is_empty(),
        "Expected chunks under 'Monster Stat Block' section"
    );
    let combined: String = stat_block_chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join("\n");
    assert!(
        combined.contains("STR DEX CON"),
        "STR DEX CON should be content, not a heading separator"
    );
    assert!(
        combined.contains("NPC encounters"),
        "NPC line should be content, not a heading separator"
    );
}

#[tokio::test]
async fn test_chapter_keyword_without_trailing_space_not_a_heading() {
    // "Chapter" alone (no trailing space) must NOT trigger heading detection
    let text = "## Section One\nThis chapter contains information.\nThe chapter describes the area.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();

    // "This chapter contains..." and "The chapter describes..." should NOT create new sections
    let section_one_chunks: Vec<_> = chunks.iter().filter(|c| c.section_path.contains("Section One")).collect();
    assert!(
        !section_one_chunks.is_empty(),
        "Expected content under Section One"
    );
    let combined: String = section_one_chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join("\n");
    assert!(
        combined.contains("This chapter contains"),
        "'This chapter contains' should be body text, not a section split. Content: {combined}"
    );
}

#[tokio::test]
async fn test_part_heading_splits_sections() {
    let text = "Introduction text.\nPart I: The Beginning\nContent of part one.\nPart II: The Middle\nContent of part two.";
    let pages = vec![page(1, text, false)];
    let chunks = chunk_document(&pages, 1600, 0).await.unwrap();

    let has_part1 = chunks.iter().any(|c| c.section_path.contains("Part I"));
    let has_part2 = chunks.iter().any(|c| c.section_path.contains("Part II"));
    assert!(has_part1, "Expected 'Part I' section. Paths: {:?}", chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>());
    assert!(has_part2, "Expected 'Part II' section. Paths: {:?}", chunks.iter().map(|c| &c.section_path).collect::<Vec<_>>());
}
