use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{
        ArcPoint, ArcStatus, CharacterArc, CharacterArcInput, MonsterHint, PrepopulatedEncounter,
        PrepopulatedEncounterInput, StoryArc, StoryArcInput, StoryEvent, StoryEventInput,
        StoryEventType, StoryFaction, StoryFactionInput, StoryLocation, StoryLocationInput,
        StoryNpc, StoryNpcInput, StorySignificance, StorySubplot, StorySubplotInput, SubplotStatus,
    },
    GuideError, Result,
};

pub struct StoryRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> StoryRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    // ─── Story Arcs ────────────────────────────────────────────────────────────

    pub async fn insert_arc(
        &self,
        campaign_id: Uuid,
        source_doc_id: Uuid,
        input: StoryArcInput,
    ) -> Result<StoryArc> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO story_arcs \
             (id, campaign_id, source_doc_id, title, description, arc_order, status, dm_notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, 'open', NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(source_doc_id.to_string())
        .bind(&input.title)
        .bind(&input.description)
        .bind(input.arc_order)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        self.get_arc(id).await
    }

    pub async fn list_arcs(&self, campaign_id: Uuid) -> Result<Vec<StoryArc>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, source_doc_id, title, description, arc_order, status, \
             dm_notes, created_at, updated_at \
             FROM story_arcs WHERE campaign_id = ? ORDER BY arc_order ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_arc).collect()
    }

    pub async fn get_arc(&self, arc_id: Uuid) -> Result<StoryArc> {
        let row = sqlx::query(
            "SELECT id, campaign_id, source_doc_id, title, description, arc_order, status, \
             dm_notes, created_at, updated_at \
             FROM story_arcs WHERE id = ?",
        )
        .bind(arc_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("StoryArc {arc_id}")))?;

        row_to_arc(row)
    }

    pub async fn update_arc_notes(&self, arc_id: Uuid, notes: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE story_arcs SET dm_notes = ?, updated_at = ? WHERE id = ?",
        )
        .bind(notes)
        .bind(Utc::now().to_rfc3339())
        .bind(arc_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_arc_status(&self, arc_id: Uuid, status: ArcStatus) -> Result<()> {
        let status_str = match status {
            ArcStatus::Open => "open",
            ArcStatus::Resolved => "resolved",
            ArcStatus::Abandoned => "abandoned",
        };
        sqlx::query(
            "UPDATE story_arcs SET status = ?, updated_at = ? WHERE id = ?",
        )
        .bind(status_str)
        .bind(Utc::now().to_rfc3339())
        .bind(arc_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ─── Story Events ──────────────────────────────────────────────────────────

    pub async fn insert_event(
        &self,
        campaign_id: Uuid,
        source_doc_id: Uuid,
        arc_id: Option<Uuid>,
        input: StoryEventInput,
    ) -> Result<StoryEvent> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let event_type_str = match input.event_type {
            StoryEventType::Combat => "combat",
            StoryEventType::Social => "social",
            StoryEventType::Revelation => "revelation",
            StoryEventType::Travel => "travel",
            StoryEventType::Rest => "rest",
            StoryEventType::Discovery => "discovery",
            StoryEventType::Puzzle => "puzzle",
            StoryEventType::Trap => "trap",
            StoryEventType::Boss => "boss",
            StoryEventType::QuestGiven => "quest_given",
            StoryEventType::NpcInteraction => "npc_interaction",
        };
        let significance_str = match input.significance {
            StorySignificance::Minor => "minor",
            StorySignificance::Major => "major",
            StorySignificance::Trivial => "trivial",
            StorySignificance::Moderate => "moderate",
            StorySignificance::Pivotal => "pivotal",
            StorySignificance::Climax => "climax",
        };
        let involved_json = serde_json::to_string(&input.involved_characters)
            .map_err(|e| GuideError::Internal(e.to_string()))?;

        sqlx::query(
            "INSERT INTO story_events \
             (id, campaign_id, arc_id, source_doc_id, title, description, event_type, \
              significance, location, involved_characters, event_order, is_dm_only, \
              dm_notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(arc_id.map(|a| a.to_string()))
        .bind(source_doc_id.to_string())
        .bind(&input.title)
        .bind(&input.description)
        .bind(event_type_str)
        .bind(significance_str)
        .bind(input.location.as_deref())
        .bind(&involved_json)
        .bind(input.event_order)
        .bind(input.is_dm_only as i32)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        self.get_event(id).await
    }

    pub async fn list_events(&self, campaign_id: Uuid) -> Result<Vec<StoryEvent>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, arc_id, source_doc_id, title, description, event_type, \
             significance, location, involved_characters, event_order, is_dm_only, \
             dm_notes, created_at, updated_at \
             FROM story_events WHERE campaign_id = ? ORDER BY event_order ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_event).collect()
    }

    pub async fn list_events_by_arc(&self, arc_id: Uuid) -> Result<Vec<StoryEvent>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, arc_id, source_doc_id, title, description, event_type, \
             significance, location, involved_characters, event_order, is_dm_only, \
             dm_notes, created_at, updated_at \
             FROM story_events WHERE arc_id = ? ORDER BY event_order ASC",
        )
        .bind(arc_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_event).collect()
    }

    pub async fn get_event(&self, event_id: Uuid) -> Result<StoryEvent> {
        let row = sqlx::query(
            "SELECT id, campaign_id, arc_id, source_doc_id, title, description, event_type, \
             significance, location, involved_characters, event_order, is_dm_only, \
             dm_notes, created_at, updated_at \
             FROM story_events WHERE id = ?",
        )
        .bind(event_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("StoryEvent {event_id}")))?;

        row_to_event(row)
    }

    pub async fn update_event_notes(&self, event_id: Uuid, notes: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE story_events SET dm_notes = ?, updated_at = ? WHERE id = ?",
        )
        .bind(notes)
        .bind(Utc::now().to_rfc3339())
        .bind(event_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ─── Story Subplots ────────────────────────────────────────────────────────

    pub async fn insert_subplot(
        &self,
        campaign_id: Uuid,
        source_doc_id: Uuid,
        arc_id: Option<Uuid>,
        input: StorySubplotInput,
    ) -> Result<StorySubplot> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO story_subplots \
             (id, campaign_id, arc_id, source_doc_id, title, description, status, \
              dm_notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, 'open', NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(arc_id.map(|a| a.to_string()))
        .bind(source_doc_id.to_string())
        .bind(&input.title)
        .bind(&input.description)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        self.get_subplot(id).await
    }

    async fn get_subplot(&self, subplot_id: Uuid) -> Result<StorySubplot> {
        let row = sqlx::query(
            "SELECT id, campaign_id, arc_id, source_doc_id, title, description, status, \
             dm_notes, created_at, updated_at \
             FROM story_subplots WHERE id = ?",
        )
        .bind(subplot_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("StorySubplot {subplot_id}")))?;

        row_to_subplot(row)
    }

    pub async fn list_subplots(&self, campaign_id: Uuid) -> Result<Vec<StorySubplot>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, arc_id, source_doc_id, title, description, status, \
             dm_notes, created_at, updated_at \
             FROM story_subplots WHERE campaign_id = ? ORDER BY created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_subplot).collect()
    }

    pub async fn update_subplot_notes(&self, subplot_id: Uuid, notes: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE story_subplots SET dm_notes = ?, updated_at = ? WHERE id = ?",
        )
        .bind(notes)
        .bind(Utc::now().to_rfc3339())
        .bind(subplot_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_subplot_status(
        &self,
        subplot_id: Uuid,
        status: SubplotStatus,
    ) -> Result<()> {
        let status_str = match status {
            SubplotStatus::Open => "open",
            SubplotStatus::Resolved => "resolved",
            SubplotStatus::Abandoned => "abandoned",
        };
        sqlx::query(
            "UPDATE story_subplots SET status = ?, updated_at = ? WHERE id = ?",
        )
        .bind(status_str)
        .bind(Utc::now().to_rfc3339())
        .bind(subplot_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ─── Character Arcs ────────────────────────────────────────────────────────

    pub async fn insert_character_arc(
        &self,
        campaign_id: Uuid,
        source_doc_id: Uuid,
        input: CharacterArcInput,
    ) -> Result<CharacterArc> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let arc_points_json = serde_json::to_string(&input.arc_points)
            .map_err(|e| GuideError::Internal(e.to_string()))?;

        sqlx::query(
            "INSERT INTO character_arcs \
             (id, campaign_id, character_name, character_id, source_doc_id, description, \
              arc_points, dm_notes, created_at, updated_at) \
             VALUES (?, ?, ?, NULL, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(&input.character_name)
        .bind(source_doc_id.to_string())
        .bind(&input.description)
        .bind(&arc_points_json)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        self.get_character_arc(id).await
    }

    async fn get_character_arc(&self, arc_id: Uuid) -> Result<CharacterArc> {
        let row = sqlx::query(
            "SELECT id, campaign_id, character_name, character_id, source_doc_id, description, \
             arc_points, dm_notes, created_at, updated_at \
             FROM character_arcs WHERE id = ?",
        )
        .bind(arc_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("CharacterArc {arc_id}")))?;

        row_to_character_arc(row)
    }

    pub async fn list_character_arcs(&self, campaign_id: Uuid) -> Result<Vec<CharacterArc>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, character_name, character_id, source_doc_id, description, \
             arc_points, dm_notes, created_at, updated_at \
             FROM character_arcs WHERE campaign_id = ? ORDER BY created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_character_arc).collect()
    }

    pub async fn update_character_arc_notes(
        &self,
        arc_id: Uuid,
        notes: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE character_arcs SET dm_notes = ?, updated_at = ? WHERE id = ?",
        )
        .bind(notes)
        .bind(Utc::now().to_rfc3339())
        .bind(arc_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // ─── Prepopulated Encounters ───────────────────────────────────────────────

    pub async fn insert_prepopulated_encounter(
        &self,
        campaign_id: Uuid,
        source_doc_id: Uuid,
        story_event_id: Option<Uuid>,
        input: PrepopulatedEncounterInput,
    ) -> Result<PrepopulatedEncounter> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let monsters_json = serde_json::to_string(&input.monsters)
            .map_err(|e| GuideError::Internal(e.to_string()))?;

        sqlx::query(
            "INSERT INTO prepopulated_encounters \
             (id, campaign_id, story_event_id, source_doc_id, name, description, location, \
              difficulty_hint, monsters, dm_notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(story_event_id.map(|s| s.to_string()))
        .bind(source_doc_id.to_string())
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.location.as_deref())
        .bind(input.difficulty_hint.as_deref())
        .bind(&monsters_json)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        self.get_prepopulated_encounter(id).await
    }

    pub async fn get_prepopulated_encounter(
        &self,
        encounter_id: Uuid,
    ) -> Result<PrepopulatedEncounter> {
        let row = sqlx::query(
            "SELECT id, campaign_id, story_event_id, source_doc_id, name, description, location, \
             difficulty_hint, monsters, dm_notes, created_at, updated_at \
             FROM prepopulated_encounters WHERE id = ?",
        )
        .bind(encounter_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("PrepopulatedEncounter {encounter_id}")))?;

        row_to_prepopulated_encounter(row)
    }

    pub async fn list_prepopulated_encounters(
        &self,
        campaign_id: Uuid,
    ) -> Result<Vec<PrepopulatedEncounter>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, story_event_id, source_doc_id, name, description, location, \
             difficulty_hint, monsters, dm_notes, created_at, updated_at \
             FROM prepopulated_encounters WHERE campaign_id = ? ORDER BY created_at ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_prepopulated_encounter).collect()
    }

    pub async fn update_encounter_notes(
        &self,
        encounter_id: Uuid,
        notes: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE prepopulated_encounters SET dm_notes = ?, updated_at = ? WHERE id = ?",
        )
        .bind(notes)
        .bind(Utc::now().to_rfc3339())
        .bind(encounter_id.to_string())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Delete all story data sourced from a specific document.
    pub async fn delete_all_for_doc(&self, doc_id: Uuid) -> Result<()> {
        let doc_id_str = doc_id.to_string();
        sqlx::query("DELETE FROM story_npcs WHERE source_doc_id = ?")
            .bind(&doc_id_str)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM story_locations WHERE source_doc_id = ?")
            .bind(&doc_id_str)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM story_factions WHERE source_doc_id = ?")
            .bind(&doc_id_str)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM prepopulated_encounters WHERE source_doc_id = ?")
            .bind(&doc_id_str)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM character_arcs WHERE source_doc_id = ?")
            .bind(&doc_id_str)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM story_subplots WHERE source_doc_id = ?")
            .bind(&doc_id_str)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM story_events WHERE source_doc_id = ?")
            .bind(&doc_id_str)
            .execute(self.pool)
            .await?;
        sqlx::query("DELETE FROM story_arcs WHERE source_doc_id = ?")
            .bind(&doc_id_str)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // ─── Story NPCs ────────────────────────────────────────────────────────────

    pub async fn insert_npc(
        &self,
        campaign_id: Uuid,
        source_doc_id: Uuid,
        input: StoryNpcInput,
    ) -> Result<StoryNpc> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO story_npcs \
             (id, campaign_id, source_doc_id, name, role, description, location, \
              is_dm_only, dm_notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(source_doc_id.to_string())
        .bind(&input.name)
        .bind(&input.role)
        .bind(&input.description)
        .bind(input.location.as_deref())
        .bind(input.is_dm_only as i32)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        self.get_npc(id).await
    }

    async fn get_npc(&self, npc_id: Uuid) -> Result<StoryNpc> {
        let row = sqlx::query(
            "SELECT id, campaign_id, source_doc_id, name, role, description, location, \
             is_dm_only, dm_notes, created_at, updated_at \
             FROM story_npcs WHERE id = ?",
        )
        .bind(npc_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("StoryNpc {npc_id}")))?;

        row_to_npc(row)
    }

    pub async fn list_npcs(&self, campaign_id: Uuid) -> Result<Vec<StoryNpc>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, source_doc_id, name, role, description, location, \
             is_dm_only, dm_notes, created_at, updated_at \
             FROM story_npcs WHERE campaign_id = ? ORDER BY name ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_npc).collect()
    }

    // ─── Story Locations ───────────────────────────────────────────────────────

    pub async fn insert_location(
        &self,
        campaign_id: Uuid,
        source_doc_id: Uuid,
        input: StoryLocationInput,
    ) -> Result<StoryLocation> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO story_locations \
             (id, campaign_id, source_doc_id, name, description, location_type, \
              dm_notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(source_doc_id.to_string())
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.location_type)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        self.get_location(id).await
    }

    async fn get_location(&self, location_id: Uuid) -> Result<StoryLocation> {
        let row = sqlx::query(
            "SELECT id, campaign_id, source_doc_id, name, description, location_type, \
             dm_notes, created_at, updated_at \
             FROM story_locations WHERE id = ?",
        )
        .bind(location_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("StoryLocation {location_id}")))?;

        row_to_location(row)
    }

    pub async fn list_locations(&self, campaign_id: Uuid) -> Result<Vec<StoryLocation>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, source_doc_id, name, description, location_type, \
             dm_notes, created_at, updated_at \
             FROM story_locations WHERE campaign_id = ? ORDER BY name ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_location).collect()
    }

    // ─── Story Factions ────────────────────────────────────────────────────────

    pub async fn insert_faction(
        &self,
        campaign_id: Uuid,
        source_doc_id: Uuid,
        input: StoryFactionInput,
    ) -> Result<StoryFaction> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO story_factions \
             (id, campaign_id, source_doc_id, name, description, alignment_hint, \
              dm_notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(source_doc_id.to_string())
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.alignment_hint.as_deref())
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        self.get_faction(id).await
    }

    async fn get_faction(&self, faction_id: Uuid) -> Result<StoryFaction> {
        let row = sqlx::query(
            "SELECT id, campaign_id, source_doc_id, name, description, alignment_hint, \
             dm_notes, created_at, updated_at \
             FROM story_factions WHERE id = ?",
        )
        .bind(faction_id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("StoryFaction {faction_id}")))?;

        row_to_faction(row)
    }

    pub async fn list_factions(&self, campaign_id: Uuid) -> Result<Vec<StoryFaction>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, source_doc_id, name, description, alignment_hint, \
             dm_notes, created_at, updated_at \
             FROM story_factions WHERE campaign_id = ? ORDER BY name ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_faction).collect()
    }
}

// ─── Row mapping helpers ───────────────────────────────────────────────────────

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn row_to_arc(row: sqlx::sqlite::SqliteRow) -> Result<StoryArc> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let source_doc_id_str: String = row.try_get("source_doc_id")?;
    let status_str: String = row.try_get("status").unwrap_or_else(|_| "open".to_string());
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let status = match status_str.as_str() {
        "resolved" => ArcStatus::Resolved,
        "abandoned" => ArcStatus::Abandoned,
        _ => ArcStatus::Open,
    };

    Ok(StoryArc {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        source_doc_id: Uuid::parse_str(&source_doc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        arc_order: row.try_get("arc_order").unwrap_or(0),
        status,
        dm_notes: row.try_get("dm_notes").ok().flatten(),
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_event(row: sqlx::sqlite::SqliteRow) -> Result<StoryEvent> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let arc_id_str: Option<String> = row.try_get("arc_id").ok().flatten();
    let source_doc_id_str: String = row.try_get("source_doc_id")?;
    let event_type_str: String = row
        .try_get("event_type")
        .unwrap_or_else(|_| "combat".to_string());
    let significance_str: String = row
        .try_get("significance")
        .unwrap_or_else(|_| "minor".to_string());
    let involved_json: String = row
        .try_get("involved_characters")
        .unwrap_or_else(|_| "[]".to_string());
    let is_dm_only_int: i32 = row.try_get("is_dm_only").unwrap_or(0);
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let event_type = match event_type_str.as_str() {
        "social" => StoryEventType::Social,
        "revelation" => StoryEventType::Revelation,
        "travel" => StoryEventType::Travel,
        "rest" => StoryEventType::Rest,
        "discovery" => StoryEventType::Discovery,
        "puzzle" => StoryEventType::Puzzle,
        "trap" => StoryEventType::Trap,
        "boss" => StoryEventType::Boss,
        "quest_given" => StoryEventType::QuestGiven,
        "npc_interaction" => StoryEventType::NpcInteraction,
        _ => StoryEventType::Combat,
    };
    let significance = match significance_str.as_str() {
        "major" => StorySignificance::Major,
        "trivial" => StorySignificance::Trivial,
        "moderate" => StorySignificance::Moderate,
        "pivotal" => StorySignificance::Pivotal,
        "climax" => StorySignificance::Climax,
        _ => StorySignificance::Minor,
    };
    let involved_characters: Vec<String> =
        serde_json::from_str(&involved_json).unwrap_or_default();

    Ok(StoryEvent {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        arc_id: arc_id_str
            .as_deref()
            .map(|s| Uuid::parse_str(s).map_err(|e| GuideError::Internal(e.to_string())))
            .transpose()?,
        source_doc_id: Uuid::parse_str(&source_doc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        event_type,
        significance,
        location: row.try_get("location").ok().flatten(),
        involved_characters,
        event_order: row.try_get("event_order").unwrap_or(0),
        is_dm_only: is_dm_only_int != 0,
        dm_notes: row.try_get("dm_notes").ok().flatten(),
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_subplot(row: sqlx::sqlite::SqliteRow) -> Result<StorySubplot> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let arc_id_str: Option<String> = row.try_get("arc_id").ok().flatten();
    let source_doc_id_str: String = row.try_get("source_doc_id")?;
    let status_str: String = row.try_get("status").unwrap_or_else(|_| "open".to_string());
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let status = match status_str.as_str() {
        "resolved" => SubplotStatus::Resolved,
        "abandoned" => SubplotStatus::Abandoned,
        _ => SubplotStatus::Open,
    };

    Ok(StorySubplot {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        arc_id: arc_id_str
            .as_deref()
            .map(|s| Uuid::parse_str(s).map_err(|e| GuideError::Internal(e.to_string())))
            .transpose()?,
        source_doc_id: Uuid::parse_str(&source_doc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        status,
        dm_notes: row.try_get("dm_notes").ok().flatten(),
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_character_arc(row: sqlx::sqlite::SqliteRow) -> Result<CharacterArc> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let character_id_str: Option<String> = row.try_get("character_id").ok().flatten();
    let source_doc_id_str: String = row.try_get("source_doc_id")?;
    let arc_points_json: String = row
        .try_get("arc_points")
        .unwrap_or_else(|_| "[]".to_string());
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let arc_points: Vec<ArcPoint> = serde_json::from_str(&arc_points_json).unwrap_or_default();

    Ok(CharacterArc {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        character_name: row.try_get("character_name")?,
        character_id: character_id_str
            .as_deref()
            .map(|s| Uuid::parse_str(s).map_err(|e| GuideError::Internal(e.to_string())))
            .transpose()?,
        source_doc_id: Uuid::parse_str(&source_doc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        description: row.try_get("description")?,
        arc_points,
        dm_notes: row.try_get("dm_notes").ok().flatten(),
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_prepopulated_encounter(row: sqlx::sqlite::SqliteRow) -> Result<PrepopulatedEncounter> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let story_event_id_str: Option<String> = row.try_get("story_event_id").ok().flatten();
    let source_doc_id_str: String = row.try_get("source_doc_id")?;
    let monsters_json: String = row
        .try_get("monsters")
        .unwrap_or_else(|_| "[]".to_string());
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    let monsters: Vec<MonsterHint> = serde_json::from_str(&monsters_json).unwrap_or_default();

    Ok(PrepopulatedEncounter {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        story_event_id: story_event_id_str
            .as_deref()
            .map(|s| Uuid::parse_str(s).map_err(|e| GuideError::Internal(e.to_string())))
            .transpose()?,
        source_doc_id: Uuid::parse_str(&source_doc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        location: row.try_get("location").ok().flatten(),
        difficulty_hint: row.try_get("difficulty_hint").ok().flatten(),
        monsters,
        dm_notes: row.try_get("dm_notes").ok().flatten(),
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_npc(row: sqlx::sqlite::SqliteRow) -> Result<StoryNpc> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let source_doc_id_str: String = row.try_get("source_doc_id")?;
    let is_dm_only_int: i32 = row.try_get("is_dm_only").unwrap_or(0);
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(StoryNpc {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        source_doc_id: Uuid::parse_str(&source_doc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        role: row.try_get("role").unwrap_or_else(|_| "neutral".to_string()),
        description: row.try_get("description").unwrap_or_default(),
        location: row.try_get("location").ok().flatten(),
        is_dm_only: is_dm_only_int != 0,
        dm_notes: row.try_get("dm_notes").ok().flatten(),
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_location(row: sqlx::sqlite::SqliteRow) -> Result<StoryLocation> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let source_doc_id_str: String = row.try_get("source_doc_id")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(StoryLocation {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        source_doc_id: Uuid::parse_str(&source_doc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        description: row.try_get("description").unwrap_or_default(),
        location_type: row.try_get("location_type").unwrap_or_else(|_| "dungeon".to_string()),
        dm_notes: row.try_get("dm_notes").ok().flatten(),
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_faction(row: sqlx::sqlite::SqliteRow) -> Result<StoryFaction> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let source_doc_id_str: String = row.try_get("source_doc_id")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(StoryFaction {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        source_doc_id: Uuid::parse_str(&source_doc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        description: row.try_get("description").unwrap_or_default(),
        alignment_hint: row.try_get("alignment_hint").ok().flatten(),
        dm_notes: row.try_get("dm_notes").ok().flatten(),
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}
