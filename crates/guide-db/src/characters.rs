use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{
        Backstory, Character, CharacterType, CreateCharacterRequest, UpdateCharacterRequest,
    },
    GuideError, Result,
};

pub struct CharacterRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CharacterRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, campaign_id: Uuid, req: CreateCharacterRequest) -> Result<Character> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let char_type_str = char_type_to_str(&req.character_type);
        let ability_scores = req.ability_scores.unwrap_or_default();
        let ability_json = serde_json::to_string(&ability_scores)?;
        let level = req.level.unwrap_or(1);
        let speed = req.speed.unwrap_or(30);

        sqlx::query(
            "INSERT INTO characters \
             (id, campaign_id, name, character_type, class, race, level, max_hp, current_hp, \
              armor_class, speed, ability_scores, conditions, backstory, is_alive, \
              created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(campaign_id.to_string())
        .bind(&req.name)
        .bind(char_type_str)
        .bind(&req.class)
        .bind(&req.race)
        .bind(level)
        .bind(req.max_hp)
        .bind(req.max_hp)
        .bind(req.armor_class)
        .bind(speed)
        .bind(&ability_json)
        .bind("[]")
        .bind(Option::<String>::None)
        .bind(1i32)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Character> {
        let row = sqlx::query(
            "SELECT id, campaign_id, name, character_type, class, race, level, max_hp, \
             current_hp, armor_class, speed, ability_scores, conditions, backstory, \
             is_alive, created_at, updated_at \
             FROM characters WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("Character {id}")))?;

        row_to_character(row)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<Character>> {
        let rows = sqlx::query(
            "SELECT id, campaign_id, name, character_type, class, race, level, max_hp, \
             current_hp, armor_class, speed, ability_scores, conditions, backstory, \
             is_alive, created_at, updated_at \
             FROM characters WHERE campaign_id = ? ORDER BY name ASC",
        )
        .bind(campaign_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_character).collect()
    }

    pub async fn update(&self, id: Uuid, req: UpdateCharacterRequest) -> Result<Character> {
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();

        if let Some(hp) = req.current_hp {
            sqlx::query("UPDATE characters SET current_hp = ?, updated_at = ? WHERE id = ?")
                .bind(hp)
                .bind(&now)
                .bind(&id_str)
                .execute(self.pool)
                .await?;
        }

        if let Some(conditions) = &req.conditions {
            let json = serde_json::to_string(conditions)?;
            sqlx::query("UPDATE characters SET conditions = ?, updated_at = ? WHERE id = ?")
                .bind(&json)
                .bind(&now)
                .bind(&id_str)
                .execute(self.pool)
                .await?;
        }

        if let Some(alive) = req.is_alive {
            sqlx::query("UPDATE characters SET is_alive = ?, updated_at = ? WHERE id = ?")
                .bind(alive as i32)
                .bind(&now)
                .bind(&id_str)
                .execute(self.pool)
                .await?;
        }

        self.get_by_id(id).await
    }

    pub async fn update_backstory(&self, id: Uuid, backstory: &Backstory) -> Result<Character> {
        let now = Utc::now().to_rfc3339();
        let json = serde_json::to_string(backstory)?;
        sqlx::query("UPDATE characters SET backstory = ?, updated_at = ? WHERE id = ?")
            .bind(&json)
            .bind(&now)
            .bind(id.to_string())
            .execute(self.pool)
            .await?;
        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM characters WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("Character {id}")));
        }
        Ok(())
    }
}

fn row_to_character(row: SqliteRow) -> Result<Character> {
    let id_str: String = row.try_get("id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let char_type_str: String = row.try_get("character_type")?;
    let ability_json: String = row.try_get("ability_scores")?;
    let conditions_json: String = row.try_get("conditions")?;
    let backstory_json: Option<String> = row.try_get("backstory")?;
    let is_alive_int: i32 = row.try_get("is_alive")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(Character {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        character_type: parse_char_type(&char_type_str),
        class: row.try_get("class")?,
        race: row.try_get("race")?,
        level: row.try_get("level")?,
        max_hp: row.try_get("max_hp")?,
        current_hp: row.try_get("current_hp")?,
        armor_class: row.try_get("armor_class")?,
        speed: row.try_get("speed")?,
        ability_scores: serde_json::from_str(&ability_json).unwrap_or_default(),
        conditions: serde_json::from_str(&conditions_json).unwrap_or_default(),
        backstory: backstory_json.as_deref().and_then(|s| serde_json::from_str(s).ok()),
        is_alive: is_alive_int != 0,
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
