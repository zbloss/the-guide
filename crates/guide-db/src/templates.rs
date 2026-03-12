use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use uuid::Uuid;

use guide_core::{
    models::{EncounterTemplate, TemplateParticipant},
    GuideError, Result,
};

pub struct TemplateRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> TemplateRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn save_template(&self, template: &EncounterTemplate) -> Result<()> {
        let participant_json = serde_json::to_string(&template.participants)?;
        sqlx::query(
            "INSERT INTO encounter_templates (id, name, description, participant_data, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(template.id.to_string())
        .bind(&template.name)
        .bind(&template.description)
        .bind(&participant_json)
        .bind(template.created_at.to_rfc3339())
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_templates(&self) -> Result<Vec<EncounterTemplate>> {
        let rows = sqlx::query(
            "SELECT id, name, description, participant_data, created_at \
             FROM encounter_templates ORDER BY created_at DESC",
        )
        .fetch_all(self.pool)
        .await?;

        rows.into_iter().map(row_to_template).collect()
    }

    pub async fn get_template(&self, id: Uuid) -> Result<EncounterTemplate> {
        let row = sqlx::query(
            "SELECT id, name, description, participant_data, created_at \
             FROM encounter_templates WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| GuideError::NotFound(format!("EncounterTemplate {id}")))?;

        row_to_template(row)
    }

    pub async fn delete_template(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM encounter_templates WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(GuideError::NotFound(format!("EncounterTemplate {id}")));
        }
        Ok(())
    }
}

fn row_to_template(row: SqliteRow) -> Result<EncounterTemplate> {
    let id_str: String = row.try_get("id")?;
    let participant_json: String = row.try_get("participant_data")?;
    let created_at_str: String = row.try_get("created_at")?;

    let participants: Vec<TemplateParticipant> =
        serde_json::from_str(&participant_json).unwrap_or_default();

    Ok(EncounterTemplate {
        id: Uuid::parse_str(&id_str).map_err(|e| GuideError::Internal(e.to_string()))?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        participants,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
