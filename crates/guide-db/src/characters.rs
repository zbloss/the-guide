use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{
        Backstory, Character, CharacterType, CreateCharacterRequest, SpellSlot,
        UpdateCharacterRequest,
    },
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct CharacterRepository {
    pool: DuckDbPool,
}

impl CharacterRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(&self, campaign_id: Uuid, req: CreateCharacterRequest) -> Result<Character> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let char_type_str = char_type_to_str(&req.character_type).to_string();
        let ability_scores = req.ability_scores.unwrap_or_default();
        let ability_json = serde_json::to_string(&ability_scores)?;
        let level = req.level.unwrap_or(1);
        let speed = req.speed.unwrap_or(30);
        let spell_slots = req.spell_slots.unwrap_or_default();
        let spell_slots_json = serde_json::to_string(&spell_slots)?;

        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let name = req.name.clone();
        let class = req.class.clone();
        let race = req.race.clone();
        let max_hp = req.max_hp;
        let ac = req.armor_class;
        let now2 = now.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO characters \
                 (id, campaign_id, name, character_type, class, race, level, max_hp, current_hp, \
                  armor_class, speed, ability_scores, conditions, spell_slots, backstory, is_alive, \
                  created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, 1, ?, ?)",
                duckdb::params![
                    id_str, campaign_id_str, name, char_type_str, class, race,
                    level, max_hp, max_hp, ac, speed, ability_json, "[]",
                    spell_slots_json, now2, now2
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        if let Some(backstory_text) = req.backstory_text {
            let backstory = Backstory {
                raw_text: backstory_text,
                extracted_hooks: vec![],
                motivations: vec![],
                key_relationships: vec![],
                secrets: vec![],
            };
            let json = serde_json::to_string(&backstory)?;
            let id_str = id.to_string();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE characters SET backstory = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![json, now, id_str],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Character> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, campaign_id, name, character_type, class, race, level, max_hp, \
                 current_hp, armor_class, speed, ability_scores, conditions, spell_slots, \
                 portrait_url, backstory, is_alive, created_at, updated_at \
                 FROM characters WHERE id = ?",
                [&id_str],
                row_to_character,
                format!("Character {id}"),
            )
        })
        .await
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<Character>> {
        let id_str = campaign_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, campaign_id, name, character_type, class, race, level, max_hp, \
                 current_hp, armor_class, speed, ability_scores, conditions, spell_slots, \
                 portrait_url, backstory, is_alive, created_at, updated_at \
                 FROM characters WHERE campaign_id = ? ORDER BY name ASC",
                [&id_str],
                row_to_character,
            )
        })
        .await
    }

    pub async fn update(&self, id: Uuid, req: UpdateCharacterRequest) -> Result<Character> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();

        macro_rules! update_field {
            ($field:literal, $val:expr) => {{
                let val = $val.clone();
                let now2 = now.clone();
                let id2 = id_str.clone();
                with_db(&self.pool, move |conn| {
                    conn.execute(
                        concat!("UPDATE characters SET ", $field, " = ?, updated_at = ? WHERE id = ?"),
                        duckdb::params![val, now2, id2],
                    )
                    .map_err(|e| GuideError::Internal(e.to_string()))?;
                    Ok(())
                })
                .await?;
            }};
        }

        if let Some(v) = &req.name { update_field!("name", v); }
        if let Some(v) = &req.class { update_field!("class", v); }
        if let Some(v) = &req.race { update_field!("race", v); }
        if let Some(v) = req.level { update_field!("level", v); }
        if let Some(v) = req.max_hp { update_field!("max_hp", v); }
        if let Some(v) = req.current_hp { update_field!("current_hp", v); }
        if let Some(v) = req.armor_class { update_field!("armor_class", v); }
        if let Some(v) = req.speed { update_field!("speed", v); }

        if let Some(ability_scores) = &req.ability_scores {
            let json = serde_json::to_string(ability_scores)?;
            let now2 = now.clone();
            let id2 = id_str.clone();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE characters SET ability_scores = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![json, now2, id2],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        if let Some(conditions) = &req.conditions {
            let json = serde_json::to_string(conditions)?;
            let now2 = now.clone();
            let id2 = id_str.clone();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE characters SET conditions = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![json, now2, id2],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        if let Some(alive) = req.is_alive {
            let val = alive as i32;
            let now2 = now.clone();
            let id2 = id_str.clone();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE characters SET is_alive = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![val, now2, id2],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        if let Some(backstory_text) = &req.backstory_text {
            let char = self.get_by_id(id).await?;
            let backstory = match char.backstory {
                Some(mut b) => {
                    b.raw_text = backstory_text.clone();
                    b
                }
                None => Backstory {
                    raw_text: backstory_text.clone(),
                    extracted_hooks: vec![],
                    motivations: vec![],
                    key_relationships: vec![],
                    secrets: vec![],
                },
            };
            let json = serde_json::to_string(&backstory)?;
            let now2 = now.clone();
            let id2 = id_str.clone();
            with_db(&self.pool, move |conn| {
                conn.execute(
                    "UPDATE characters SET backstory = ?, updated_at = ? WHERE id = ?",
                    duckdb::params![json, now2, id2],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
                Ok(())
            })
            .await?;
        }

        self.get_by_id(id).await
    }

    pub async fn update_backstory(&self, id: Uuid, backstory: &Backstory) -> Result<Character> {
        let now = Utc::now().to_rfc3339();
        let json = serde_json::to_string(backstory)?;
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE characters SET backstory = ?, updated_at = ? WHERE id = ?",
                duckdb::params![json, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;
        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM characters WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Character {id}")));
            }
            Ok(())
        })
        .await
    }

    pub async fn update_portrait_url(&self, id: Uuid, url: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let url = url.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE characters SET portrait_url = ?, updated_at = ? WHERE id = ?",
                duckdb::params![url, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn spend_spell_slot(&self, char_id: Uuid, level: i32) -> Result<Character> {
        let now = Utc::now().to_rfc3339();
        let mut character = self.get_by_id(char_id).await?;
        let slot = character
            .spell_slots
            .iter_mut()
            .find(|s| s.level == level)
            .ok_or_else(|| GuideError::InvalidInput(format!("No spell slot at level {level}")))?;
        slot.remaining = (slot.remaining - 1).max(0);
        let json = serde_json::to_string(&character.spell_slots)?;
        let id_str = char_id.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE characters SET spell_slots = ?, updated_at = ? WHERE id = ?",
                duckdb::params![json, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;
        self.get_by_id(char_id).await
    }

    pub async fn restore_spell_slots(&self, char_id: Uuid, level: Option<i32>) -> Result<Character> {
        let now = Utc::now().to_rfc3339();
        let mut character = self.get_by_id(char_id).await?;
        match level {
            None => {
                for slot in character.spell_slots.iter_mut() {
                    slot.remaining = slot.total;
                }
            }
            Some(lvl) => {
                let slot = character
                    .spell_slots
                    .iter_mut()
                    .find(|s| s.level == lvl)
                    .ok_or_else(|| {
                        GuideError::InvalidInput(format!("No spell slot at level {lvl}"))
                    })?;
                slot.remaining = slot.total;
            }
        }
        let json = serde_json::to_string(&character.spell_slots)?;
        let id_str = char_id.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE characters SET spell_slots = ?, updated_at = ? WHERE id = ?",
                duckdb::params![json, now, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;
        self.get_by_id(char_id).await
    }
}

fn row_to_character(row: &duckdb::Row) -> duckdb::Result<Character> {
    let id_str: String = row.get("id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let char_type_str: String = row.get("character_type")?;
    let ability_json: String = row.get("ability_scores")?;
    let conditions_json: String = row.get("conditions")?;
    let backstory_json: Option<String> = row.get("backstory")?;
    let is_alive_int: i32 = row.get("is_alive")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;
    let spell_slots_json: Option<String> = row.get("spell_slots").ok();
    let portrait_url: Option<String> = row.get("portrait_url").ok();
    let spell_slots: Vec<SpellSlot> = spell_slots_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    Ok(Character {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        name: row.get("name")?,
        character_type: parse_char_type(&char_type_str),
        class: row.get("class")?,
        race: row.get("race")?,
        level: row.get("level")?,
        max_hp: row.get("max_hp")?,
        current_hp: row.get("current_hp")?,
        armor_class: row.get("armor_class")?,
        speed: row.get("speed")?,
        ability_scores: serde_json::from_str(&ability_json).unwrap_or_default(),
        conditions: serde_json::from_str(&conditions_json).unwrap_or_default(),
        backstory: backstory_json.as_deref().and_then(|s| serde_json::from_str(s).ok()),
        is_alive: is_alive_int != 0,
        portrait_url,
        spell_slots,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn char_type_to_str(t: &CharacterType) -> &'static str {
    match t {
        CharacterType::Pc => "pc",
        CharacterType::Npc => "npc",
        CharacterType::Monster => "monster",
    }
}

fn parse_char_type(s: &str) -> CharacterType {
    match s {
        "pc" => CharacterType::Pc,
        "monster" => CharacterType::Monster,
        _ => CharacterType::Npc,
    }
}
