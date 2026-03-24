use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{ActionBudget, CombatParticipant, CreateEncounterRequest, Encounter, EncounterStatus},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct EncounterRepository {
    pool: DuckDbPool,
}

impl EncounterRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn create(&self, campaign_id: Uuid, req: CreateEncounterRequest) -> Result<Encounter> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let id_str = id.to_string();
        let campaign_id_str = campaign_id.to_string();
        let session_id_str = req.session_id.map(|s| s.to_string());
        let name = req.name.clone();
        let desc = req.description.clone();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO encounters \
                 (id, session_id, campaign_id, name, description, status, round, \
                  current_turn_index, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, 'pending', 0, 0, ?, ?)",
                duckdb::params![
                    id_str, session_id_str, campaign_id_str, name, desc, now, now
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await?;

        self.get_by_id(id).await
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Encounter> {
        let id_str = id.to_string();
        let mut encounter = with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, session_id, campaign_id, name, description, status, round, \
                 current_turn_index, created_at, updated_at \
                 FROM encounters WHERE id = ?",
                [&id_str],
                row_to_encounter,
                format!("Encounter {id}"),
            )
        })
        .await?;
        encounter.participants = self.list_participants(id).await?;
        Ok(encounter)
    }

    pub async fn list_by_campaign(&self, campaign_id: Uuid) -> Result<Vec<Encounter>> {
        let id_str = campaign_id.to_string();
        let encounters = with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, session_id, campaign_id, name, description, status, round, \
                 current_turn_index, created_at, updated_at \
                 FROM encounters WHERE campaign_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_encounter,
            )
        })
        .await?;

        let mut result = Vec::with_capacity(encounters.len());
        for mut enc in encounters {
            enc.participants = self.list_participants(enc.id).await?;
            result.push(enc);
        }
        Ok(result)
    }

    pub async fn list_by_session(&self, session_id: Uuid) -> Result<Vec<Encounter>> {
        let id_str = session_id.to_string();
        let encounters = with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, session_id, campaign_id, name, description, status, round, \
                 current_turn_index, created_at, updated_at \
                 FROM encounters WHERE session_id = ? ORDER BY created_at ASC",
                [&id_str],
                row_to_encounter,
            )
        })
        .await?;

        let mut result = Vec::with_capacity(encounters.len());
        for mut enc in encounters {
            enc.participants = self.list_participants(enc.id).await?;
            result.push(enc);
        }
        Ok(result)
    }

    pub async fn save_state(&self, encounter: &Encounter) -> Result<()> {
        let status_str = status_to_str(&encounter.status).to_string();
        let enc_id_str = encounter.id.to_string();
        let round = encounter.round;
        let turn_idx = encounter.current_turn_index;
        let now = Utc::now().to_rfc3339();

        #[allow(clippy::type_complexity)]
        let mut participant_data: Vec<(String, i32, i32, i32, i32, String, String, i32, i32)> =
            Vec::with_capacity(encounter.participants.len());
        for p in &encounter.participants {
            let conditions_json = serde_json::to_string(&p.conditions)?;
            let budget_json = serde_json::to_string(&p.action_budget)?;
            participant_data.push((
                p.id.to_string(),
                p.initiative_roll,
                p.initiative_modifier,
                p.initiative_total,
                p.current_hp,
                conditions_json,
                budget_json,
                p.has_taken_turn as i32,
                p.is_defeated as i32,
            ));
        }

        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = pool.get().map_err(|e| GuideError::Internal(e.to_string()))?;
            let tx = conn.transaction().map_err(|e| GuideError::Internal(e.to_string()))?;

            tx.execute(
                "UPDATE encounters SET status = ?, round = ?, current_turn_index = ?, \
                 updated_at = ? WHERE id = ?",
                duckdb::params![status_str, round, turn_idx, now, enc_id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;

            for (pid, roll, modi, total, hp, cond_json, budget_json, taken, defeated) in
                &participant_data
            {
                tx.execute(
                    "UPDATE combat_participants SET initiative_roll = ?, initiative_modifier = ?, \
                     initiative_total = ?, current_hp = ?, conditions = ?, action_budget = ?, \
                     has_taken_turn = ?, is_defeated = ? WHERE id = ?",
                    duckdb::params![roll, modi, total, hp, cond_json, budget_json, taken, defeated, pid],
                )
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            }

            tx.commit().map_err(|e| GuideError::Internal(e.to_string()))
        })
        .await
        .map_err(|e| GuideError::Internal(e.to_string()))?
    }

    pub async fn add_participant(&self, participant: &CombatParticipant) -> Result<()> {
        let conditions_json = serde_json::to_string(&participant.conditions)?;
        let budget_json = serde_json::to_string(&participant.action_budget)?;
        let id_str = participant.id.to_string();
        let enc_id_str = participant.encounter_id.to_string();
        let char_id_str = participant.character_id.to_string();
        let name = participant.name.clone();
        let roll = participant.initiative_roll;
        let modi = participant.initiative_modifier;
        let total = participant.initiative_total;
        let hp = participant.current_hp;
        let max_hp = participant.max_hp;
        let ac = participant.armor_class;
        let taken = participant.has_taken_turn as i32;
        let defeated = participant.is_defeated as i32;

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO combat_participants \
                 (id, encounter_id, character_id, name, initiative_roll, initiative_modifier, \
                  initiative_total, current_hp, max_hp, armor_class, conditions, action_budget, \
                  has_taken_turn, is_defeated) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    id_str, enc_id_str, char_id_str, name, roll, modi, total,
                    hp, max_hp, ac, conditions_json, budget_json, taken, defeated
                ],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn update_participant_name(&self, participant_id: Uuid, name: &str) -> Result<()> {
        let id_str = participant_id.to_string();
        let name = name.to_string();
        with_db(&self.pool, move |conn| {
            conn.execute(
                "UPDATE combat_participants SET name = ? WHERE id = ?",
                duckdb::params![name, id_str],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM encounters WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("Encounter {id}")));
            }
            Ok(())
        })
        .await
    }

    pub async fn record_turn_snapshot(&self, encounter: &Encounter, turn_number: i32) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let snapshot_json = serde_json::to_string(encounter)?;
        let enc_id_str = encounter.id.to_string();
        let round = encounter.round;
        let now = Utc::now().to_rfc3339();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO encounter_turns \
                 (id, encounter_id, turn_number, round_number, snapshot_json, recorded_at) \
                 VALUES (?, ?, ?, ?, ?, ?) \
                 ON CONFLICT(encounter_id, turn_number) DO UPDATE SET \
                 snapshot_json = excluded.snapshot_json, recorded_at = excluded.recorded_at",
                duckdb::params![id, enc_id_str, turn_number, round, snapshot_json, now],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn count_turn_snapshots(&self, encounter_id: Uuid) -> Result<i64> {
        let id_str = encounter_id.to_string();
        with_db(&self.pool, move |conn| {
            conn.query_row(
                "SELECT COUNT(*) AS cnt FROM encounter_turns WHERE encounter_id = ?",
                [&id_str],
                |r| r.get("cnt"),
            )
            .map_err(|e| GuideError::Internal(e.to_string()))
        })
        .await
    }

    pub async fn list_turn_snapshots(
        &self,
        encounter_id: Uuid,
    ) -> Result<Vec<guide_core::models::EncounterTurnSnapshot>> {
        use guide_core::models::EncounterTurnSnapshot;
        let id_str = encounter_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, encounter_id, turn_number, round_number, snapshot_json, recorded_at \
                 FROM encounter_turns WHERE encounter_id = ? ORDER BY turn_number ASC",
                [&id_str],
                |row| {
                    let id_str: String = row.get("id")?;
                    let enc_id_str: String = row.get("encounter_id")?;
                    let snapshot_json: String = row.get("snapshot_json")?;
                    let recorded_at_str: String = row.get("recorded_at")?;
                    let snapshot: Encounter = serde_json::from_str(&snapshot_json)
                        .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?;
                    Ok(EncounterTurnSnapshot {
                        id: Uuid::parse_str(&id_str)
                            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
                        encounter_id: Uuid::parse_str(&enc_id_str)
                            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
                        turn_number: row.get("turn_number")?,
                        round_number: row.get("round_number")?,
                        snapshot,
                        recorded_at: recorded_at_str.parse().unwrap_or_else(|_| Utc::now()),
                    })
                },
            )
        })
        .await
    }

    async fn list_participants(&self, encounter_id: Uuid) -> Result<Vec<CombatParticipant>> {
        let id_str = encounter_id.to_string();
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, encounter_id, character_id, name, initiative_roll, initiative_modifier, \
                 initiative_total, current_hp, max_hp, armor_class, conditions, action_budget, \
                 has_taken_turn, is_defeated \
                 FROM combat_participants WHERE encounter_id = ? \
                 ORDER BY initiative_total DESC, initiative_modifier DESC",
                [&id_str],
                row_to_participant,
            )
        })
        .await
    }
}

fn row_to_encounter(row: &duckdb::Row) -> duckdb::Result<Encounter> {
    let id_str: String = row.get("id")?;
    let session_id_str: Option<String> = row.get("session_id")?;
    let campaign_id_str: String = row.get("campaign_id")?;
    let status_str: String = row.get("status")?;
    let created_at_str: String = row.get("created_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    let session_id = session_id_str
        .as_deref()
        .map(|s| {
            Uuid::parse_str(s).map_err(|e| {
                duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e))
            })
        })
        .transpose()?;

    Ok(Encounter {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        session_id,
        campaign_id: Uuid::parse_str(&campaign_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(2, duckdb::types::Type::Text, Box::new(e)))?,
        name: row.get("name")?,
        description: row.get("description")?,
        status: parse_status(&status_str),
        round: row.get("round")?,
        current_turn_index: row.get("current_turn_index")?,
        participants: Vec::new(),
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}

fn row_to_participant(row: &duckdb::Row) -> duckdb::Result<CombatParticipant> {
    let id_str: String = row.get("id")?;
    let enc_id_str: String = row.get("encounter_id")?;
    let char_id_str: String = row.get("character_id")?;
    let conditions_json: String = row.get("conditions")?;
    let budget_json: String = row.get("action_budget")?;
    let has_taken_turn_int: i32 = row.get("has_taken_turn")?;
    let is_defeated_int: i32 = row.get("is_defeated")?;

    Ok(CombatParticipant {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        encounter_id: Uuid::parse_str(&enc_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?,
        character_id: Uuid::parse_str(&char_id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(2, duckdb::types::Type::Text, Box::new(e)))?,
        name: row.get("name")?,
        initiative_roll: row.get("initiative_roll")?,
        initiative_modifier: row.get("initiative_modifier")?,
        initiative_total: row.get("initiative_total")?,
        current_hp: row.get("current_hp")?,
        max_hp: row.get("max_hp")?,
        armor_class: row.get("armor_class")?,
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
