use chrono::{DateTime, Utc};
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

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct StoryRepository {
    pool: DuckDbPool,
}

impl StoryRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
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
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let source_doc_id_str = source_doc_id.to_string();
        let title = input.title.clone();
        let description = input.description.clone();
        let arc_order = input.arc_order;

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO story_arcs \
                 (id, campaign_id, source_doc_id, title, description, arc_order, status, dm_notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, 'open', NULL, ?, ?)",
                duckdb::params![id_str, campaign_id_str, source_doc_id_str, title, description, arc_order, now, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_arc(id).await
    }

    pub async fn list_arcs(&self, campaign_id: Uuid) -> Result<Vec<StoryArc>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, source_doc_id, title, description, arc_order, status, \
                 dm_notes, created_at, updated_at \
                 FROM story_arcs WHERE campaign_id = ? ORDER BY arc_order ASC",
                [&id_str],
                row_to_arc,
            )
        })
        .await
    }

    pub async fn get_arc(&self, arc_id: Uuid) -> Result<StoryArc> {
        let id_str = arc_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, source_doc_id, title, description, arc_order, status, \
                 dm_notes, created_at, updated_at \
                 FROM story_arcs WHERE id = ?",
                [&id_str],
                row_to_arc,
                format!("StoryArc {arc_id}"),
            )
        })
        .await
    }

    pub async fn update_arc_notes(&self, arc_id: Uuid, notes: Option<&str>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = arc_id.to_string();
        let notes_owned = notes.map(|s| s.to_string());

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE story_arcs SET dm_notes = ?, updated_at = ? WHERE id = ?",
                duckdb::params![notes_owned, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn update_arc_status(&self, arc_id: Uuid, status: ArcStatus) -> Result<()> {
        let status_str = match status {
            ArcStatus::Open => "open",
            ArcStatus::Resolved => "resolved",
            ArcStatus::Abandoned => "abandoned",
        };
        let now = Utc::now().to_rfc3339();
        let id_str = arc_id.to_string();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE story_arcs SET status = ?, updated_at = ? WHERE id = ?",
                duckdb::params![status_str, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
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

        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let arc_id_str = arc_id.map(|a| a.to_string());
        let source_doc_id_str = source_doc_id.to_string();
        let title = input.title.clone();
        let description = input.description.clone();
        let location = input.location.clone();
        let event_order = input.event_order;
        let is_dm_only = input.is_dm_only as i32;

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO story_events \
                 (id, campaign_id, arc_id, source_doc_id, title, description, event_type, \
                  significance, location, involved_characters, event_order, is_dm_only, \
                  dm_notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, arc_id_str, source_doc_id_str,
                    title, description, event_type_str, significance_str,
                    location, involved_json, event_order, is_dm_only, now, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_event(id).await
    }

    pub async fn list_events(&self, campaign_id: Uuid) -> Result<Vec<StoryEvent>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, arc_id, source_doc_id, title, description, event_type, \
                 significance, location, involved_characters, event_order, is_dm_only, \
                 dm_notes, created_at, updated_at \
                 FROM story_events WHERE campaign_id = ? ORDER BY event_order ASC",
                [&id_str],
                row_to_event,
            )
        })
        .await
    }

    pub async fn list_events_by_arc(&self, arc_id: Uuid) -> Result<Vec<StoryEvent>> {
        let id_str = arc_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, arc_id, source_doc_id, title, description, event_type, \
                 significance, location, involved_characters, event_order, is_dm_only, \
                 dm_notes, created_at, updated_at \
                 FROM story_events WHERE arc_id = ? ORDER BY event_order ASC",
                [&id_str],
                row_to_event,
            )
        })
        .await
    }

    pub async fn get_event(&self, event_id: Uuid) -> Result<StoryEvent> {
        let id_str = event_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, arc_id, source_doc_id, title, description, event_type, \
                 significance, location, involved_characters, event_order, is_dm_only, \
                 dm_notes, created_at, updated_at \
                 FROM story_events WHERE id = ?",
                [&id_str],
                row_to_event,
                format!("StoryEvent {event_id}"),
            )
        })
        .await
    }

    pub async fn update_event_notes(&self, event_id: Uuid, notes: Option<&str>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = event_id.to_string();
        let notes_owned = notes.map(|s| s.to_string());

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE story_events SET dm_notes = ?, updated_at = ? WHERE id = ?",
                duckdb::params![notes_owned, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
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
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let arc_id_str = arc_id.map(|a| a.to_string());
        let source_doc_id_str = source_doc_id.to_string();
        let title = input.title.clone();
        let description = input.description.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO story_subplots \
                 (id, campaign_id, arc_id, source_doc_id, title, description, status, \
                  dm_notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, 'open', NULL, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, arc_id_str, source_doc_id_str,
                    title, description, now, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_subplot(id).await
    }

    async fn get_subplot(&self, subplot_id: Uuid) -> Result<StorySubplot> {
        let id_str = subplot_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, arc_id, source_doc_id, title, description, status, \
                 dm_notes, created_at, updated_at \
                 FROM story_subplots WHERE id = ?",
                [&id_str],
                row_to_subplot,
                format!("StorySubplot {subplot_id}"),
            )
        })
        .await
    }

    pub async fn list_subplots(&self, campaign_id: Uuid) -> Result<Vec<StorySubplot>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, arc_id, source_doc_id, title, description, status, \
                 dm_notes, created_at, updated_at \
                 FROM story_subplots WHERE campaign_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_subplot,
            )
        })
        .await
    }

    pub async fn update_subplot_notes(&self, subplot_id: Uuid, notes: Option<&str>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = subplot_id.to_string();
        let notes_owned = notes.map(|s| s.to_string());

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE story_subplots SET dm_notes = ?, updated_at = ? WHERE id = ?",
                duckdb::params![notes_owned, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
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
        let now = Utc::now().to_rfc3339();
        let id_str = subplot_id.to_string();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE story_subplots SET status = ?, updated_at = ? WHERE id = ?",
                duckdb::params![status_str, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
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
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let source_doc_id_str = source_doc_id.to_string();
        let character_name = input.character_name.clone();
        let description = input.description.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO character_arcs \
                 (id, campaign_id, character_name, character_id, source_doc_id, description, \
                  arc_points, dm_notes, created_at, updated_at) \
                 VALUES (?, ?, ?, NULL, ?, ?, ?, NULL, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, character_name, source_doc_id_str,
                    description, arc_points_json, now, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_character_arc(id).await
    }

    async fn get_character_arc(&self, arc_id: Uuid) -> Result<CharacterArc> {
        let id_str = arc_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, character_name, character_id, source_doc_id, description, \
                 arc_points, dm_notes, created_at, updated_at \
                 FROM character_arcs WHERE id = ?",
                [&id_str],
                row_to_character_arc,
                format!("CharacterArc {arc_id}"),
            )
        })
        .await
    }

    pub async fn list_character_arcs(&self, campaign_id: Uuid) -> Result<Vec<CharacterArc>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, character_name, character_id, source_doc_id, description, \
                 arc_points, dm_notes, created_at, updated_at \
                 FROM character_arcs WHERE campaign_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_character_arc,
            )
        })
        .await
    }

    pub async fn update_character_arc_notes(
        &self,
        arc_id: Uuid,
        notes: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = arc_id.to_string();
        let notes_owned = notes.map(|s| s.to_string());

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE character_arcs SET dm_notes = ?, updated_at = ? WHERE id = ?",
                duckdb::params![notes_owned, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
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
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let story_event_id_str = story_event_id.map(|s| s.to_string());
        let source_doc_id_str = source_doc_id.to_string();
        let name = input.name.clone();
        let description = input.description.clone();
        let location = input.location.clone();
        let difficulty_hint = input.difficulty_hint.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO prepopulated_encounters \
                 (id, campaign_id, story_event_id, source_doc_id, name, description, location, \
                  difficulty_hint, monsters, dm_notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, story_event_id_str, source_doc_id_str,
                    name, description, location, difficulty_hint, monsters_json, now, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_prepopulated_encounter(id).await
    }

    pub async fn get_prepopulated_encounter(
        &self,
        encounter_id: Uuid,
    ) -> Result<PrepopulatedEncounter> {
        let id_str = encounter_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, story_event_id, source_doc_id, name, description, location, \
                 difficulty_hint, monsters, dm_notes, created_at, updated_at \
                 FROM prepopulated_encounters WHERE id = ?",
                [&id_str],
                row_to_prepopulated_encounter,
                format!("PrepopulatedEncounter {encounter_id}"),
            )
        })
        .await
    }

    pub async fn list_prepopulated_encounters(
        &self,
        campaign_id: Uuid,
    ) -> Result<Vec<PrepopulatedEncounter>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, story_event_id, source_doc_id, name, description, location, \
                 difficulty_hint, monsters, dm_notes, created_at, updated_at \
                 FROM prepopulated_encounters WHERE campaign_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_prepopulated_encounter,
            )
        })
        .await
    }

    pub async fn update_encounter_notes(
        &self,
        encounter_id: Uuid,
        notes: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = encounter_id.to_string();
        let notes_owned = notes.map(|s| s.to_string());

        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE prepopulated_encounters SET dm_notes = ?, updated_at = ? WHERE id = ?",
                duckdb::params![notes_owned, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    /// Delete all story data sourced from a specific document.
    pub async fn delete_all_for_doc(&self, doc_id: Uuid) -> Result<()> {
        let doc_id_str = doc_id.to_string();
        with_db(&self.pool, move |conn| {
            for table in &[
                "story_npcs",
                "story_locations",
                "story_factions",
                "prepopulated_encounters",
                "character_arcs",
                "story_subplots",
                "story_events",
                "story_arcs",
            ] {
                conn.execute(
                    &format!("DELETE FROM {table} WHERE source_doc_id = ?"),
                    [&doc_id_str],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            }
            Ok(())
        })
        .await
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
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let source_doc_id_str = source_doc_id.to_string();
        let name = input.name.clone();
        let role = input.role.clone();
        let description = input.description.clone();
        let location = input.location.clone();
        let is_dm_only = input.is_dm_only as i32;

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO story_npcs \
                 (id, campaign_id, source_doc_id, name, role, description, location, \
                  is_dm_only, dm_notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, source_doc_id_str,
                    name, role, description, location, is_dm_only, now, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_npc(id).await
    }

    async fn get_npc(&self, npc_id: Uuid) -> Result<StoryNpc> {
        let id_str = npc_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, source_doc_id, name, role, description, location, \
                 is_dm_only, dm_notes, created_at, updated_at \
                 FROM story_npcs WHERE id = ?",
                [&id_str],
                row_to_npc,
                format!("StoryNpc {npc_id}"),
            )
        })
        .await
    }

    pub async fn list_npcs(&self, campaign_id: Uuid) -> Result<Vec<StoryNpc>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, source_doc_id, name, role, description, location, \
                 is_dm_only, dm_notes, created_at, updated_at \
                 FROM story_npcs WHERE campaign_id = ? ORDER BY name ASC",
                [&id_str],
                row_to_npc,
            )
        })
        .await
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
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let source_doc_id_str = source_doc_id.to_string();
        let name = input.name.clone();
        let description = input.description.clone();
        let location_type = input.location_type.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO story_locations \
                 (id, campaign_id, source_doc_id, name, description, location_type, \
                  dm_notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, source_doc_id_str,
                    name, description, location_type, now, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_location(id).await
    }

    async fn get_location(&self, location_id: Uuid) -> Result<StoryLocation> {
        let id_str = location_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, source_doc_id, name, description, location_type, \
                 dm_notes, created_at, updated_at \
                 FROM story_locations WHERE id = ?",
                [&id_str],
                row_to_location,
                format!("StoryLocation {location_id}"),
            )
        })
        .await
    }

    pub async fn list_locations(&self, campaign_id: Uuid) -> Result<Vec<StoryLocation>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, source_doc_id, name, description, location_type, \
                 dm_notes, created_at, updated_at \
                 FROM story_locations WHERE campaign_id = ? ORDER BY name ASC",
                [&id_str],
                row_to_location,
            )
        })
        .await
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
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let source_doc_id_str = source_doc_id.to_string();
        let name = input.name.clone();
        let description = input.description.clone();
        let alignment_hint = input.alignment_hint.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO story_factions \
                 (id, campaign_id, source_doc_id, name, description, alignment_hint, \
                  dm_notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, source_doc_id_str,
                    name, description, alignment_hint, now, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_faction(id).await
    }

    async fn get_faction(&self, faction_id: Uuid) -> Result<StoryFaction> {
        let id_str = faction_id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, source_doc_id, name, description, alignment_hint, \
                 dm_notes, created_at, updated_at \
                 FROM story_factions WHERE id = ?",
                [&id_str],
                row_to_faction,
                format!("StoryFaction {faction_id}"),
            )
        })
        .await
    }

    pub async fn list_factions(&self, campaign_id: Uuid) -> Result<Vec<StoryFaction>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, source_doc_id, name, description, alignment_hint, \
                 dm_notes, created_at, updated_at \
                 FROM story_factions WHERE campaign_id = ? ORDER BY name ASC",
                [&id_str],
                row_to_faction,
            )
        })
        .await
    }
}

// ─── Row mapping helpers ───────────────────────────────────────────────────────

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_uuid(s: &str, idx: usize) -> duckdb::Result<Uuid> {
    Uuid::parse_str(s)
        .map_err(|e| duckdb::Error::FromSqlConversionFailure(idx, duckdb::types::Type::Text, Box::new(e)))
}

fn row_to_arc(row: &duckdb::Row) -> duckdb::Result<StoryArc> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let source_doc_id_str: String = row.get("source_doc_id")?;
    let status_str: String = row.get::<_, Option<String>>("status")?.unwrap_or_else(|| "open".to_string());
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let status = match status_str.as_str() {
        "resolved" => ArcStatus::Resolved,
        "abandoned" => ArcStatus::Abandoned,
        _ => ArcStatus::Open,
    };

    Ok(StoryArc {
        id: parse_uuid(&id_str, 0)?,
        campaign_id: parse_uuid(&campaign_id_str, 1)?,
        source_doc_id: parse_uuid(&source_doc_id_str, 2)?,
        title: row.get("title")?,
        description: row.get("description")?,
        arc_order: row.get::<_, Option<i32>>("arc_order")?.unwrap_or(0),
        status,
        dm_notes: row.get("dm_notes")?,
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_event(row: &duckdb::Row) -> duckdb::Result<StoryEvent> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let arc_id_str: Option<String> = row.get("arc_id")?;
    let source_doc_id_str: String = row.get("source_doc_id")?;
    let event_type_str: String = row.get::<_, Option<String>>("event_type")?.unwrap_or_else(|| "combat".to_string());
    let significance_str: String = row.get::<_, Option<String>>("significance")?.unwrap_or_else(|| "minor".to_string());
    let involved_json: String = row.get::<_, Option<String>>("involved_characters")?.unwrap_or_else(|| "[]".to_string());
    let is_dm_only_int: i32 = row.get::<_, Option<i32>>("is_dm_only")?.unwrap_or(0);
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

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
        id: parse_uuid(&id_str, 0)?,
        campaign_id: parse_uuid(&campaign_id_str, 1)?,
        arc_id: arc_id_str
            .as_deref()
            .map(|s| parse_uuid(s, 2))
            .transpose()?,
        source_doc_id: parse_uuid(&source_doc_id_str, 3)?,
        title: row.get("title")?,
        description: row.get("description")?,
        event_type,
        significance,
        location: row.get("location")?,
        involved_characters,
        event_order: row.get::<_, Option<i32>>("event_order")?.unwrap_or(0),
        is_dm_only: is_dm_only_int != 0,
        dm_notes: row.get("dm_notes")?,
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_subplot(row: &duckdb::Row) -> duckdb::Result<StorySubplot> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let arc_id_str: Option<String> = row.get("arc_id")?;
    let source_doc_id_str: String = row.get("source_doc_id")?;
    let status_str: String = row.get::<_, Option<String>>("status")?.unwrap_or_else(|| "open".to_string());
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let status = match status_str.as_str() {
        "resolved" => SubplotStatus::Resolved,
        "abandoned" => SubplotStatus::Abandoned,
        _ => SubplotStatus::Open,
    };

    Ok(StorySubplot {
        id: parse_uuid(&id_str, 0)?,
        campaign_id: parse_uuid(&campaign_id_str, 1)?,
        arc_id: arc_id_str.as_deref().map(|s| parse_uuid(s, 2)).transpose()?,
        source_doc_id: parse_uuid(&source_doc_id_str, 3)?,
        title: row.get("title")?,
        description: row.get("description")?,
        status,
        dm_notes: row.get("dm_notes")?,
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_character_arc(row: &duckdb::Row) -> duckdb::Result<CharacterArc> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let character_id_str: Option<String> = row.get("character_id")?;
    let source_doc_id_str: String = row.get("source_doc_id")?;
    let arc_points_json: String = row.get::<_, Option<String>>("arc_points")?.unwrap_or_else(|| "[]".to_string());
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let arc_points: Vec<ArcPoint> = serde_json::from_str(&arc_points_json).unwrap_or_default();

    Ok(CharacterArc {
        id: parse_uuid(&id_str, 0)?,
        campaign_id: parse_uuid(&campaign_id_str, 1)?,
        character_name: row.get("character_name")?,
        character_id: character_id_str.as_deref().map(|s| parse_uuid(s, 2)).transpose()?,
        source_doc_id: parse_uuid(&source_doc_id_str, 3)?,
        description: row.get("description")?,
        arc_points,
        dm_notes: row.get("dm_notes")?,
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_prepopulated_encounter(row: &duckdb::Row) -> duckdb::Result<PrepopulatedEncounter> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let story_event_id_str: Option<String> = row.get("story_event_id")?;
    let source_doc_id_str: String = row.get("source_doc_id")?;
    let monsters_json: String = row.get::<_, Option<String>>("monsters")?.unwrap_or_else(|| "[]".to_string());
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let monsters: Vec<MonsterHint> = serde_json::from_str(&monsters_json).unwrap_or_default();

    Ok(PrepopulatedEncounter {
        id: parse_uuid(&id_str, 0)?,
        campaign_id: parse_uuid(&campaign_id_str, 1)?,
        story_event_id: story_event_id_str.as_deref().map(|s| parse_uuid(s, 2)).transpose()?,
        source_doc_id: parse_uuid(&source_doc_id_str, 3)?,
        name: row.get("name")?,
        description: row.get("description")?,
        location: row.get("location")?,
        difficulty_hint: row.get("difficulty_hint")?,
        monsters,
        dm_notes: row.get("dm_notes")?,
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_npc(row: &duckdb::Row) -> duckdb::Result<StoryNpc> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let source_doc_id_str: String = row.get("source_doc_id")?;
    let is_dm_only_int: i32 = row.get::<_, Option<i32>>("is_dm_only")?.unwrap_or(0);
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    Ok(StoryNpc {
        id: parse_uuid(&id_str, 0)?,
        campaign_id: parse_uuid(&campaign_id_str, 1)?,
        source_doc_id: parse_uuid(&source_doc_id_str, 2)?,
        name: row.get("name")?,
        role: row.get::<_, Option<String>>("role")?.unwrap_or_else(|| "neutral".to_string()),
        description: row.get::<_, Option<String>>("description")?.unwrap_or_default(),
        location: row.get("location")?,
        is_dm_only: is_dm_only_int != 0,
        dm_notes: row.get("dm_notes")?,
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_location(row: &duckdb::Row) -> duckdb::Result<StoryLocation> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let source_doc_id_str: String = row.get("source_doc_id")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    Ok(StoryLocation {
        id: parse_uuid(&id_str, 0)?,
        campaign_id: parse_uuid(&campaign_id_str, 1)?,
        source_doc_id: parse_uuid(&source_doc_id_str, 2)?,
        name: row.get("name")?,
        description: row.get::<_, Option<String>>("description")?.unwrap_or_default(),
        location_type: row.get::<_, Option<String>>("location_type")?.unwrap_or_else(|| "dungeon".to_string()),
        dm_notes: row.get("dm_notes")?,
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}

fn row_to_faction(row: &duckdb::Row) -> duckdb::Result<StoryFaction> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let source_doc_id_str: String = row.get("source_doc_id")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    Ok(StoryFaction {
        id: parse_uuid(&id_str, 0)?,
        campaign_id: parse_uuid(&campaign_id_str, 1)?,
        source_doc_id: parse_uuid(&source_doc_id_str, 2)?,
        name: row.get("name")?,
        description: row.get::<_, Option<String>>("description")?.unwrap_or_default(),
        alignment_hint: row.get("alignment_hint")?,
        dm_notes: row.get("dm_notes")?,
        created_at: parse_dt(&created_at_str),
        updated_at: parse_dt(&updated_at_str),
    })
}
