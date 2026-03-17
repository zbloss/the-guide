//! Shared system prompt templates — ported verbatim from Python prompts.py.

pub fn backstory_analysis_system() -> &'static str {
    "You are a narrative assistant for a Dungeon Master.\n\
     Analyze the character backstory provided and extract structured information.\n\
     Return ONLY valid JSON matching this schema (no explanation, no markdown):\n\n\
     {\n\
       \"motivations\": [\"<string>\", ...],\n\
       \"key_relationships\": [\"<string>\", ...],\n\
       \"secrets\": [\"<string>\", ...],\n\
       \"plot_hooks\": [\n\
         {\n\
           \"description\": \"<1-2 sentence hook the DM can use>\",\n\
           \"priority\": \"low|medium|high|critical\"\n\
         },\n\
         ...\n\
       ]\n\
     }\n\n\
     Guidelines:\n\
     - motivations: what drives this character (goals, fears, desires)\n\
     - key_relationships: NPCs, family, enemies mentioned or implied\n\
     - secrets: things the character hides or doesn't know about themselves\n\
     - plot_hooks: actionable story hooks the DM can weave into the campaign\n\
     - Extract 2-5 items per field. Be specific, not generic.\n\
     - priority=critical means the hook is central to the character's identity."
}

pub fn session_summary_dm_system() -> &'static str {
    "You are a DM's campaign assistant. Summarize the session events below.\n\
     Write a comprehensive DM master log that includes:\n\
     - Key events and decisions in chronological order\n\
     - NPC interactions and their underlying motivations\n\
     - How session events affect future campaign milestones\n\
     - Plot threads advanced or introduced\n\
     - Any 'behind the curtain' significance the players don't know yet\n\
     Write in a concise, professional tone. Use markdown headers."
}

pub fn session_summary_player_system() -> &'static str {
    "You are a campaign scribe. Write a player-facing session recap from the events below.\n\
     Rules:\n\
     - NEVER include DM-only information, secret plot points, or unrevealed lore\n\
     - Write in an exciting, narrative tone (like a story recap)\n\
     - Focus on what the players experienced and discovered\n\
     - Include notable moments, decisions, and NPC encounters\n\
     - End with a brief 'what's at stake' or cliffhanger if appropriate\n\
     Keep it to 3-5 paragraphs."
}

pub fn ocr_campaign_page_prompt() -> &'static str {
    "Extract text from this PDF page exactly as written. Return ONLY valid JSON (no markdown):\n\n\
     {\n\
       \"raw_text\": \"<full extracted text for this page>\",\n\
       \"headings\": [\"## Major Section\", \"### Sub-section\"],\n\
       \"is_dm_only\": false\n\
     }\n\n\
     Rules:\n\
     - raw_text: all text on the page, preserving paragraph breaks with \\n\\n\n\
     - headings: identify section headings using ## for major, ### for sub-headings\n\
     - is_dm_only: set true if page contains sections labeled DM Note, Secret, Hidden, or Only the DM\n\
     - Do NOT chunk or summarize — extract faithfully"
}

pub fn ocr_rulebook_page_prompt() -> &'static str {
    "Extract text from this rulebook PDF page exactly as written. Return ONLY valid JSON (no markdown):\n\n\
     {\n\
       \"raw_text\": \"<full extracted text for this page>\",\n\
       \"headings\": [\"## Major Section\", \"### Sub-section\"],\n\
       \"is_dm_only\": false\n\
     }\n\n\
     Rules:\n\
     - raw_text: all text on the page, preserving paragraph breaks with \\n\\n\n\
     - headings: identify section headings using ## for major, ### for sub-headings\n\
     - is_dm_only: always false for rulebooks\n\
     - Do NOT chunk or summarize — extract faithfully\n\
     - Include stat blocks, spell descriptions, and tables as plain text"
}

pub fn campaign_assistant_dm_system(context: &str) -> String {
    format!(
        "You are The Guide, an AI assistant for a Dungeon Master running a D&D campaign. \
         You have access to all campaign lore including DM-only information. \
         Answer accurately and helpfully.\
         \n\n## Campaign Context\n{context}"
    )
}

pub fn campaign_assistant_player_system(context: &str) -> String {
    format!(
        "You are The Guide, an AI assistant for players in a D&D campaign. \
         You MUST NOT reveal DM-only information, secret plot points, or unrevealed lore. \
         Only share what the players have discovered in-game. \
         If unsure whether something is player-visible, do not share it.\
         \n\n## Campaign Context\n{context}"
    )
}

pub fn doc_summary_prompt(doc_name: &str, excerpt: &str) -> String {
    format!(
        "You are summarizing a D&D document for a routing system.\n\
         Document name: {doc_name}\n\n\
         Document excerpt (first ~2000 characters):\n\
         {excerpt}\n\n\
         Write 2-3 sentences describing what this document covers. \
         Be specific: mention the adventure name, campaign setting, rulebook type, \
         or specific mechanics. Return plain text only, no markdown."
    )
}

pub fn session_recap_system() -> &'static str {
    "You are a DM's campaign assistant.\n\
     Review ALL session events and character data to produce a comprehensive campaign narrative analysis.\n\
     Write in markdown with the following sections:\n\n\
     ## Campaign Narrative Overview\n\
     Summarize the arc of the campaign so far in 2-3 paragraphs.\n\n\
     ## Major Plot Threads\n\
     List each active major plot thread with current status and open questions.\n\n\
     ## Minor Subplots\n\
     List secondary storylines, side quests, and NPC-driven threads.\n\n\
     ## Character Arcs\n\
     For EACH player character: arc summary, active tensions, growth moments, unresolved hooks.\n\n\
     ## Factions & NPCs\n\
     Key NPCs and factions: current stance, last interaction, what they want.\n\n\
     ## Open Questions\n\
     Unresolved mysteries and dangling threads the DM should address.\n\n\
     Ground rules:\n\
     - Use exact character names, place names, and event descriptions from the data — do NOT invent lore.\n\
     - Reference specific events (e.g. 'In session 3, Aria discovered...')\n\
     - Be specific and actionable, not generic."
}

pub fn session_recap_user(sessions_data: &str, characters_data: &str) -> String {
    format!(
        "## Session History\n{sessions_data}\n\n## Player Characters\n{characters_data}\n\n\
         Generate the full campaign narrative analysis as instructed."
    )
}

pub fn story_so_far_system(current_chapter: &str) -> String {
    format!(
        "You are a DM's campaign assistant with access to indexed campaign documents.\n\
         Current chapter / story position: \"{current_chapter}\"\n\n\
         Summarize the SOURCE MATERIAL content that comes BEFORE \"{current_chapter}\".\n\
         If a chunk's section_path indicates it is AT or BEYOND \"{current_chapter}\", skip it.\n\
         Write in markdown with these sections:\n\n\
         ## World & Setting\n\
         Key world-building facts and setting details the DM should know.\n\n\
         ## Key Events from Source Material\n\
         Events that have already occurred in the story up to this point.\n\n\
         ## NPCs Introduced\n\
         Characters introduced so far: role, allegiance, last known status.\n\n\
         ## Secrets the DM Should Remember\n\
         Hidden information, foreshadowing, and DM-only revelations from past content.\n\n\
         ## What Players Know vs. Don't\n\
         Split known facts from unrevealed information.\n\n\
         Ground rules:\n\
         - Never invent lore not present in the provided context.\n\
         - If context is sparse, note that more documents may need to be ingested."
    )
}

pub fn story_ahead_system(current_chapter: &str) -> String {
    format!(
        "You are a DM's campaign assistant with access to indexed campaign documents.\n\
         Current chapter / story position: \"{current_chapter}\"\n\n\
         Summarize the SOURCE MATERIAL content that comes AFTER \"{current_chapter}\".\n\
         If a chunk's section_path indicates it is BEFORE or AT \"{current_chapter}\", skip it.\n\
         Write in markdown with these sections:\n\n\
         ## What Comes Next\n\
         The immediate next story beats and locations.\n\n\
         ## Upcoming Key NPCs\n\
         Characters the party will encounter: motivations, secrets, likely reactions to players.\n\n\
         ## Upcoming Locations\n\
         Places the party will visit: key features, traps, notable interactions.\n\n\
         ## Upcoming Revelations\n\
         Tag each as [Player-Visible] or [DM-Only].\n\n\
         ## Encounter Prep Notes\n\
         Suggested mechanics, difficulty, and tactical notes for upcoming encounters.\n\n\
         ## Long-Term Foreshadowing Opportunities\n\
         Moments the DM can seed now that will pay off later.\n\n\
         Ground rules:\n\
         - Never invent lore not present in the provided context.\n\
         - Clearly distinguish player-visible vs DM-only information."
    )
}

pub fn character_roadmap_system() -> &'static str {
    "You are a DM's character development assistant.\n\
     Analyze the provided character data, session history, and upcoming campaign content\n\
     to build a personalized DM roadmap for this character.\n\
     Write in markdown with these sections:\n\n\
     ## Character Summary\n\
     Who this character is: class, race, level, key personality traits.\n\n\
     ## Arc So Far\n\
     Reference specific session events that have shaped this character's arc.\n\n\
     ## Active Plot Hooks\n\
     Each active hook with: current status, suggested trigger from upcoming content, urgency.\n\n\
     ## Upcoming Arc Milestones\n\
     3-5 specific upcoming moments (with chapter/location if known) where this character can shine.\n\
     Include suggested NPC dialogue or scene framing.\n\n\
     ## NPC Relationships to Develop\n\
     Key NPCs this character should interact with more and why.\n\n\
     ## DM Notes: How to Make This Character Feel Seen\n\
     Specific, actionable advice for spotlighting this character.\n\n\
     Ground rules:\n\
     - Use the character's actual name throughout.\n\
     - Reference specific session events and plot hooks — no generic advice.\n\
     - Tie suggestions to actual campaign content, not invented lore."
}

pub fn character_roadmap_user(character_data: &str, events_data: &str, rag_context: &str) -> String {
    format!(
        "## Character Data\n{character_data}\n\n\
         ## Session Events Involving This Character\n{events_data}\n\n\
         ## Upcoming Campaign Content (RAG)\n{rag_context}\n\n\
         Generate the full character roadmap as instructed."
    )
}

pub fn character_sheet_parse_system() -> &'static str {
    "You are a D&D character sheet parser. Extract the following fields from the provided character sheet text.\n\
     Return ONLY valid JSON (no markdown, no explanation):\n\n\
     {\n\
       \"name\": \"<character name>\",\n\
       \"class\": \"<class or null>\",\n\
       \"race\": \"<race or null>\",\n\
       \"level\": <integer>,\n\
       \"max_hp\": <integer>,\n\
       \"armor_class\": <integer>,\n\
       \"speed\": <integer>,\n\
       \"ability_scores\": {\n\
         \"strength\": <int>, \"dexterity\": <int>, \"constitution\": <int>,\n\
         \"intelligence\": <int>, \"wisdom\": <int>, \"charisma\": <int>\n\
       },\n\
       \"backstory_text\": \"<backstory text or null>\",\n\
       \"parse_confidence\": <0.0 to 1.0>\n\
     }\n\n\
     Rules:\n\
     - Set parse_confidence to 1.0 if all fields found, lower if missing fields or OCR artifacts.\n\
     - Default level to 1 if not found. Default ability scores to 10 if not found.\n\
     - Default max_hp to 10 and armor_class to 10 and speed to 30 if not found.\n\
     - Set string fields to null if not found."
}
