use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{
        ActionBudget, CombatParticipant, CreateEncounterRequest, Encounter,
        EncounterStatus,
    },
    GuideError, Result,
};

pub struct EncounterRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> EncounterRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, campaign_id: Uuid, req: CreateEncounterRequest) -> Result<Encounter> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO encounters \
             (id, session_id, campaign_id, name, description, status, round, \
              current_turn_index, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, 'pending', 0, 0, ?, ?)",
        )
        .bind(id.to_string())
        .bind(req.session_id.to_string())
        .bind(campaign_id.to_string())
        .bind(&req.name)
        .bind(&req.description)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Encounter> {
        let row = sqlx::query(
            "SELECT id, session_id, campaign_id, name, description, status, round, \
             current_turn_index, created_at, updated_at \
             FROM encounters WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("Encounter {id}")))?;

        let mut encounter = row_to_encounter(row)?;
        encounter.participants = self.list_participants(id).await?;
        Ok(encounter)
    }

    pub async fn list_by_session(&self, session_id: Uuid) -> Result<Vec<Encounter>> {
        let rows = sqlx::query(
            "SELECT id, session_id, campaign_id, name, description, status, round, \
             current_turn_index, created_at, updated_at \
             FROM encounters WHERE session_id = ? ORDER BY created_at ASC",
        )
        .bind(session_id.to_string())
        .fetch_all(self.pool)
        .await?;

        let mut encounters = Vec::new();
        for row in rows {
            let mut enc = row_to_encounter(row)?;
            enc.participants = self.list_participants(enc.id).await?;
            encounters.push(enc);
        }
        Ok(encounters)
    }

    /// Atomically persist encounter status/round/turn and all participant states.
    pub async fn save_state(&self, encounter: &Encounter) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let status_str = status_to_str(&encounter.status);
        sqlx::query(
            "UPDATE encounters SET status = ?, round = ?, current_turn_index = ?, \
             updated_at = ? WHERE id = ?",
        )
        .bind(status_str)
        .bind(encounter.round)
        .bind(encounter.current_turn_index)
        .bind(Utc::now().to_rfc3339())
        .bind(encounter.id.to_string())
        .execute(&mut *tx)
        .await?;

        for p in &encounter.participants {
            let conditions_json = serde_json::to_string(&p.conditions)?;
            let budget_json = serde_json::to_string(&p.action_budget)?;

            sqlx::query(
                "UPDATE combat_participants SET initiative_roll = ?, initiative_modifier = ?, \
                 initiative_total = ?, current_hp = ?, conditions = ?, action_budget = ?, \
                 has_taken_turn = ?, is_defeated = ? WHERE id = ?",
            )
            .bind(p.initiative_roll)
            .bind(p.initiative_modifier)
            .bind(p.initiative_total)
            .bind(p.current_hp)
            .bind(&conditions_json)
            .bind(&budget_json)
            .bind(p.has_taken_turn as i32)
            .bind(p.is_defeated as i32)
            .bind(p.id.to_string())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn add_participant(&self, participant: &CombatParticipant) -> Result<()> {
        let conditions_json = serde_json::to_string(&participant.conditions)?;
        let budget_json = serde_json::to_string(&participant.action_budget)?;

        sqlx::query(
            "INSERT INTO combat_participants \
             (id, encounter_id, character_id, name, initiative_roll, initiative_modifier, \
              initiative_total, current_hp, max_hp, armor_class, conditions, action_budget, \
              has_taken_turn, is_defeated) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(participant.id.to_string())
        .bind(participant.encounter_id.to_string())
        .bind(participant.character_id.to_string())
        .bind(&participant.name)
        .bind(participant.initiative_roll)
        .bind(participant.initiative_modifier)
        .bind(participant.initiative_total)
        .bind(participant.current_hp)
        .bind(participant.max_hp)
        .bind(participant.armor_class)
        .bind(&conditions_json)
        .bind(&budget_json)
        .bind(participant.has_taken_turn as i32)
        .bind(participant.is_defeated as i32)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM encounters WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("Encounter {id}")));
        }
        Ok(())
    }

    async fn list_participants(&self, encounter_id: Uuid) -> Result<Vec<CombatParticipant>> {
        let rows = sqlx::query(
            "SELECT id, encounter_id, character_id, name, initiative_roll, initiative_modifier, \
             initiative_total, current_hp, max_hp, armor_class, conditions, action_budget, \
             has_taken_turn, is_defeated \
             FROM combat_participants WHERE encounter_id = ? \
             ORDER BY initiative_total DESC, initiative_modifier DESC",
        )
        .bind(encounter_id.to_string())
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_participant).collect()
    }
}

fn row_to_encounter(row: SqliteRow) -> Result<Encounter> {
    let id_str: String = row.try_get("id")?;
    let session_id_str: String = row.try_get("session_id")?;
    let campaign_id_str: String = row.try_get("campaign_id")?;
    let status_str: String = row.try_get("status")?;
    let created_at_str: String = row.try_get("created_at")?;
    let updated_at_str: String = row.try_get("updated_at")?;

    Ok(Encounter {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        session_id: Uuid::parse_str(&session_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        status: parse_status(&status_str),
        round: row.try_get("round")?,
        current_turn_index: row.try_get("current_turn_index")?,
        participants: Vec::new(),
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn row_to_participant(row: SqliteRow) -> Result<CombatParticipant> {
    let id_str: String = row.try_get("id")?;
    let enc_id_str: String = row.try_get("encounter_id")?;
    let char_id_str: String = row.try_get("character_id")?;
    let conditions_json: String = row.try_get("conditions")?;
    let budget_json: String = row.try_get("action_budget")?;
    let has_taken_turn_int: i32 = row.try_get("has_taken_turn")?;
    let is_defeated_int: i32 = row.try_get("is_defeated")?;

    Ok(CombatParticipant {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        encounter_id: Uuid::parse_str(&enc_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        character_id: Uuid::parse_str(&char_id_str)
            .map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        initiative_roll: row.try_get("initiative_roll")?,
        initiative_modifier: row.try_get("initiative_modifier")?,
        initiative_total: row.try_get("initiative_total")?,
        current_hp: row.try_get("current_hp")?,
        max_hp: row.try_get("max_hp")?,
        armor_class: row.try_get("armor_class")?,
        conditions: serde_json::from_str(&conditions_json).unwrap_or_default(),
        action_budget: serde_json::from_str(&budget_json)
            .unwrap_or_else(|_| ActionBudget::new(30)),
        has_taken_turn: has_taken_turn_int != 0,
        is_defeated: is_defeated_int != 0,
    })
}

fn status_to_str(s: &EncounterStatus) -> &'static str {
    match s {
        EncounterStatus::Pending => "pending",
        EncounterStatus::Active => "active",
        EncounterStatus::Completed => "completed",
        EncounterStatus::Fled => "fled",
    }
}

fn parse_status(s: &str) -> EncounterStatus {
    match s {
        "active" => EncounterStatus::Active,
        "completed" => EncounterStatus::Completed,
        "fled" => EncounterStatus::Fled,
        _ => EncounterStatus::Pending,
    }
}
