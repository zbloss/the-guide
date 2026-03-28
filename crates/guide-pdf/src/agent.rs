/// AI-agent-based story extraction using rig-core.
///
/// When a campaign document contains a detectable Table of Contents, this module
/// runs a rig `Agent` that navigates the document page by page — exactly as a
/// human DM would — instead of relying purely on pre-chunked text windows.
///
/// ## Tool inventory
/// | Tool              | Purpose |
/// |-------------------|---------|
/// | `get_page`        | Fetch the raw text of a single page by page number |
/// | `get_page_range`  | Fetch concatenated text for a range of pages |
/// | `get_toc`         | Return the Table of Contents as a formatted list |
/// | `lookup_index`    | Return page numbers where a given term appears |
use std::collections::HashMap;
use std::sync::Arc;

use guide_core::{models::StoryExtractionResult, AppConfig, Result};
use rig::{
    completion::{Prompt, ToolDefinition},
    prelude::CompletionClient,
    providers::openai::CompletionsClient,
    tool::Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ── Custom tool error ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ToolErr(pub String);

impl std::fmt::Display for ToolErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolErr {}

// ── Shared document data ──────────────────────────────────────────────────────

/// All document data needed by the agent tools, preloaded before the agent runs.
#[derive(Debug)]
pub struct DocumentData {
    /// page_num (1-based) → raw OCR text
    pub pages: HashMap<u32, String>,
    /// Table of Contents entries in document order
    pub toc: Vec<guide_core::models::TocEntry>,
    /// Lowercase term → list of page numbers (from the back-of-book index)
    pub index: HashMap<String, Vec<u32>>,
    /// Offset to add to printed page numbers from the ToC to get PDF page indices.
    /// offset = pdf_page_index - printed_page_number  (usually ≥ 0).
    pub page_offset: i32,
}

// ── Tool: get_page ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
struct GetPageTool {
    #[serde(skip)]
    data: Option<Arc<DocumentData>>,
}

impl GetPageTool {
    fn new(data: Arc<DocumentData>) -> Self {
        Self { data: Some(data) }
    }
    fn data(&self) -> &DocumentData {
        self.data.as_deref().expect("DocumentData not set")
    }
}

#[derive(Deserialize)]
struct GetPageArgs {
    page_num: u32,
}

impl Tool for GetPageTool {
    const NAME: &'static str = "get_page";
    type Error = ToolErr;
    type Args = GetPageArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Return the raw text of a single page from the campaign document. \
                          Use this after finding a page number via lookup_index or get_toc."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "page_num": {
                        "type": "integer",
                        "description": "1-based page number to retrieve"
                    }
                },
                "required": ["page_num"]
            }),
        }
    }

    async fn call(&self, args: GetPageArgs) -> std::result::Result<String, ToolErr> {
        let data = self.data();
        // Apply page offset: the agent may use printed page numbers from the ToC,
        // but our pages HashMap is indexed by PDF page number (1-based physical index).
        let pdf_page = (args.page_num as i32 + data.page_offset).max(1) as u32;
        data.pages
            .get(&pdf_page)
            .cloned()
            .ok_or_else(|| ToolErr(format!("Page {} not found in document (pdf_page={})", args.page_num, pdf_page)))
    }
}

// ── Tool: get_page_range ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
struct GetPageRangeTool {
    #[serde(skip)]
    data: Option<Arc<DocumentData>>,
}

impl GetPageRangeTool {
    fn new(data: Arc<DocumentData>) -> Self {
        Self { data: Some(data) }
    }
    fn data(&self) -> &DocumentData {
        self.data.as_deref().expect("DocumentData not set")
    }
}

#[derive(Deserialize)]
struct GetPageRangeArgs {
    start: u32,
    end: u32,
}

impl Tool for GetPageRangeTool {
    const NAME: &'static str = "get_page_range";
    type Error = ToolErr;
    type Args = GetPageRangeArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Return the concatenated text of a range of consecutive pages. \
                          Use this to read an entire chapter at once after finding its \
                          start page from get_toc. Limit to 30 pages per call."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "start": {
                        "type": "integer",
                        "description": "First page number (inclusive, 1-based)"
                    },
                    "end": {
                        "type": "integer",
                        "description": "Last page number (inclusive, 1-based, max start+29)"
                    }
                },
                "required": ["start", "end"]
            }),
        }
    }

    async fn call(&self, args: GetPageRangeArgs) -> std::result::Result<String, ToolErr> {
        let data = self.data();
        // Apply page offset to convert printed page numbers to PDF page indices.
        let pdf_start = (args.start as i32 + data.page_offset).max(1) as u32;
        let pdf_end = (args.end as i32 + data.page_offset).max(1) as u32;
        let pdf_end = pdf_end.min(pdf_start.saturating_add(29));
        let mut parts: Vec<&str> = Vec::new();
        for page_num in pdf_start..=pdf_end {
            if let Some(text) = data.pages.get(&page_num) {
                parts.push(text.as_str());
            }
        }
        if parts.is_empty() {
            Err(ToolErr(format!("No pages found in range {}-{} (pdf {}-{})", args.start, args.end, pdf_start, pdf_end)))
        } else {
            Ok(parts.join("\n\n---\n\n"))
        }
    }
}

// ── Tool: get_toc ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
struct GetTocTool {
    #[serde(skip)]
    data: Option<Arc<DocumentData>>,
}

impl GetTocTool {
    fn new(data: Arc<DocumentData>) -> Self {
        Self { data: Some(data) }
    }
    fn data(&self) -> &DocumentData {
        self.data.as_deref().expect("DocumentData not set")
    }
}

#[derive(Deserialize)]
struct GetTocArgs {}

impl Tool for GetTocTool {
    const NAME: &'static str = "get_toc";
    type Error = ToolErr;
    type Args = GetTocArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Return the Table of Contents for this document as a formatted list \
                          showing chapter titles and their starting page numbers."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: GetTocArgs) -> std::result::Result<String, ToolErr> {
        let toc = &self.data().toc;
        if toc.is_empty() {
            return Ok("No Table of Contents available for this document.".to_string());
        }
        let lines: Vec<String> = toc
            .iter()
            .map(|e| {
                let indent = "  ".repeat(e.depth as usize);
                format!("{}{} — page {}", indent, e.title, e.page)
            })
            .collect();
        Ok(lines.join("\n"))
    }
}

// ── Tool: lookup_index ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
struct LookupIndexTool {
    #[serde(skip)]
    data: Option<Arc<DocumentData>>,
}

impl LookupIndexTool {
    fn new(data: Arc<DocumentData>) -> Self {
        Self { data: Some(data) }
    }
    fn data(&self) -> &DocumentData {
        self.data.as_deref().expect("DocumentData not set")
    }
}

#[derive(Deserialize)]
struct LookupIndexArgs {
    term: String,
}

impl Tool for LookupIndexTool {
    const NAME: &'static str = "lookup_index";
    type Error = ToolErr;
    type Args = LookupIndexArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Look up a term in the document's back-of-book Index and return \
                          the page numbers where it appears. Use this when you need more \
                          information about a specific NPC, location, faction, or concept."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "term": {
                        "type": "string",
                        "description": "The term to look up (case-insensitive)"
                    }
                },
                "required": ["term"]
            }),
        }
    }

    async fn call(&self, args: LookupIndexArgs) -> std::result::Result<String, ToolErr> {
        let key = args.term.to_lowercase();
        match self.data().index.get(&key) {
            Some(pages) if !pages.is_empty() => {
                let page_list: Vec<String> = pages.iter().map(|p| p.to_string()).collect();
                Ok(format!(
                    "'{}' appears on page(s): {}",
                    args.term,
                    page_list.join(", ")
                ))
            }
            _ => Ok(format!("'{}' was not found in the document index.", args.term)),
        }
    }
}

// ── Agent runner ──────────────────────────────────────────────────────────────

/// Run the rig agent to extract the story from a single chapter's text.
///
/// Returns `Ok(None)` if the agent produces output that can't be parsed as
/// `StoryExtractionResult` (the caller should fall back to the legacy LLM path).
pub async fn extract_chapter_with_agent(
    chapter_text: &str,
    chapter_name: &str,
    doc_title: &str,
    doc_ctx: &crate::pipeline::DocumentContext,
    prev_summary: Option<&str>,
    data: Arc<DocumentData>,
    config: &AppConfig,
) -> Result<Option<StoryExtractionResult>> {
    // Route to cloud or local Ollama based on config.story_provider.
    let (base_url, api_key, model) = if config.story_provider == "cloud" {
        let url = match config.cloud_fallback.as_deref() {
            Some("gemini") => {
                Some("https://generativelanguage.googleapis.com/v1beta/openai".to_string())
            }
            _ => None, // openai-compatible: use default base url
        };
        let key = config.cloud_api_key.clone().unwrap_or_default();
        let mdl = config
            .cloud_model
            .clone()
            .unwrap_or_else(|| "gemini-2.0-flash".to_string());
        (url, key, mdl)
    } else {
        (
            Some(config.ollama_base_url.clone()),
            "ollama".to_string(),
            config.default_model.clone(),
        )
    };

    let mut builder = CompletionsClient::builder().api_key(&api_key);
    if let Some(ref url) = base_url {
        builder = builder.base_url(url);
    }
    let client = match builder.build() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to create rig client: {e}");
            return Ok(None);
        }
    };

    tracing::debug!(
        provider = if config.story_provider == "cloud" { "cloud" } else { "ollama" },
        model = %model,
        "Agent story extraction using provider"
    );

    let system = agent_system_prompt(doc_title, doc_ctx);

    let agent = client
        .agent(&model)
        .preamble(&system)
        .max_tokens(config.max_output_tokens() as u64)
        .default_max_turns(20)
        .tool(GetPageTool::new(data.clone()))
        .tool(GetPageRangeTool::new(data.clone()))
        .tool(GetTocTool::new(data.clone()))
        .tool(LookupIndexTool::new(data.clone()))
        .build();

    let user_msg = agent_chapter_prompt(chapter_text, chapter_name, doc_title, prev_summary);

    let response = match agent.prompt(user_msg).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(chapter = chapter_name, "Agent chapter extraction failed: {e}");
            return Ok(None);
        }
    };

    let cleaned = guide_llm::prompts::strip_think_block(&response);
    let json_str = crate::pipeline::extract_first_json_object_pub(cleaned);

    match serde_json::from_str::<StoryExtractionResult>(json_str) {
        Ok(result) => Ok(Some(result)),
        Err(e) => {
            tracing::warn!(chapter = chapter_name, "Agent response JSON parse failed: {e}");
            Ok(None)
        }
    }
}

// ── Prompts ───────────────────────────────────────────────────────────────────

fn agent_system_prompt(doc_title: &str, doc_ctx: &crate::pipeline::DocumentContext) -> String {
    format!(
        "You are a specialist story extraction agent for the D&D campaign document '{doc_title}'.\n\
         Setting: {setting}. Tone: {tone}. Themes: {themes}.\n\n\
         You have tools to navigate the document by page number:\n\
         - get_toc(): See the Table of Contents with chapter pages.\n\
         - get_page(page_num): Read a single page.\n\
         - get_page_range(start, end): Read a range of pages (max 30 at once).\n\
         - lookup_index(term): Find which pages mention a specific term.\n\n\
         Use these tools to gather information, then return your findings as a single JSON object.\n\n\
         STRICT RULES:\n\
         - arcs is an array of OBJECTS, not strings. Each arc: \
           {{\"title\": \"<name>\", \"description\": \"<2-3 sentences>\", \"arc_order\": <int>}}.\n\
           arc_order MUST be a positive integer (1, 2, 3...). NEVER null.\n\
         - event_type MUST be EXACTLY ONE value from: combat, social, revelation, travel, rest, \
           discovery, puzzle, trap, boss, quest_given, npc_interaction. \
           Do NOT use | as a separator. Do NOT invent new types.\n\
         - Return ONLY the JSON object — no prose before or after, no markdown fences.\n\n\
         JSON schema:\n\
         {{\n\
           \"arcs\": [\n\
             {{\"title\": \"<arc name>\", \"description\": \"<2-3 sentences>\", \"arc_order\": 1}}\n\
           ],\n\
           \"events\": [\n\
             {{\"title\": \"<name>\", \"description\": \"<2-3 sentences>\", \
               \"event_type\": \"<combat|social|revelation|...>\", \
               \"significance\": \"<trivial|minor|moderate|major|pivotal|climax>\", \
               \"location\": \"<name or null>\", \"involved_characters\": [\"<name>\"], \
               \"event_order\": 1, \"arc_title\": \"<arc title or null>\"}}\n\
           ],\n\
           \"subplots\": [{{\"title\": \"<title>\", \"description\": \"<desc>\", \"arc_title\": \"<arc or null>\"}}],\n\
           \"character_arcs\": [{{\"character_name\": \"<name>\", \"description\": \"<desc>\", \
             \"arc_points\": [{{\"description\": \"<point>\", \"order\": 1}}]}}],\n\
           \"encounters\": [{{\"name\": \"<name>\", \"description\": \"<desc>\", \
             \"location\": \"<loc or null>\", \"difficulty_hint\": \"<easy|medium|hard|deadly|null>\", \
             \"monsters\": [{{\"name\": \"<name>\", \"count\": 1, \"cr\": \"<cr or null>\"}}], \
             \"story_event_title\": \"<event title or null>\"}}],\n\
           \"npcs\": [{{\"name\": \"<name>\", \"role\": \"<villain|ally|neutral|questgiver|boss>\", \
             \"description\": \"<1-2 sentences>\", \"location\": \"<loc or null>\"}}],\n\
           \"locations\": [{{\"name\": \"<name>\", \"description\": \"<1-2 sentences>\", \
             \"location_type\": \"<dungeon|town|wilderness|stronghold|planar>\"}}],\n\
           \"factions\": [{{\"name\": \"<name>\", \"description\": \"<1-2 sentences>\", \
             \"alignment_hint\": \"<e.g. lawful evil or null>\"}}]\n\
         }}\n\
         All arrays may be empty if not applicable.",
        doc_title = doc_title,
        setting = doc_ctx.setting_str(),
        tone = doc_ctx.tone_str(),
        themes = doc_ctx.themes_str(),
    )
}

fn agent_chapter_prompt(
    chapter_text: &str,
    chapter_name: &str,
    doc_title: &str,
    prev_summary: Option<&str>,
) -> String {
    let mut msg = format!("Document: {doc_title}\nChapter: {chapter_name}\n");
    if let Some(prev) = prev_summary {
        msg.push_str(&format!("\nPrevious chapters summary:\n{prev}\n"));
    }
    msg.push_str(&format!(
        "\n# {chapter_name}\n{chapter_text}\n\n\
         Extract the complete story structure for this chapter. \
         Use lookup_index or get_page if you need more detail on any element. \
         Return the StoryExtractionResult JSON."
    ));
    msg
}
