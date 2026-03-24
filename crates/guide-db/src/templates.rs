use chrono::Utc;
use uuid::Uuid;

use guide_core::{
    models::{EncounterTemplate, TemplateParticipant},
    GuideError, Result,
};

use crate::{query_all, query_one, with_db, DuckDbPool};

pub struct TemplateRepository {
    pool: DuckDbPool,
}

impl TemplateRepository {
    pub fn new(pool: &DuckDbPool) -> Self {
        Self { pool: pool.clone() }
    }

    pub async fn save_template(&self, template: &EncounterTemplate) -> Result<()> {
        let participant_json = serde_json::to_string(&template.participants)
            .map_err(|e| GuideError::Internal(e.to_string()))?;
        let id_str = template.id.to_string();
        let name = template.name.clone();
        let description = template.description.clone();
        let created_at = template.created_at.to_rfc3339();

        with_db(&self.pool, move |conn| {
            conn.execute(
                "INSERT INTO encounter_templates \
                 (id, name, description, participant_data, created_at) \
                 VALUES (?, ?, ?, ?, ?)",
                duckdb::params![id_str, name, description, participant_json, created_at],
            )
            .map_err(|e| GuideError::Internal(e.to_string()))?;
            Ok(())
        })
        .await
    }

    pub async fn list_templates(&self) -> Result<Vec<EncounterTemplate>> {
        with_db(&self.pool, move |conn| {
            query_all(
                conn,
                "SELECT id, name, description, participant_data, created_at \
                 FROM encounter_templates ORDER BY created_at DESC",
                [],
                row_to_template,
            )
        })
        .await
    }

    pub async fn get_template(&self, id: Uuid) -> Result<EncounterTemplate> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            query_one(
                conn,
                "SELECT id, name, description, participant_data, created_at \
                 FROM encounter_templates WHERE id = ?",
                [&id_str],
                row_to_template,
                format!("EncounterTemplate {id}"),
            )
        })
        .await
    }

    pub async fn delete_template(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        with_db(&self.pool, move |conn| {
            let n = conn
                .execute("DELETE FROM encounter_templates WHERE id = ?", [&id_str])
                .map_err(|e| GuideError::Internal(e.to_string()))?;
            if n == 0 {
                return Err(GuideError::NotFound(format!("EncounterTemplate {id}")));
            }
            Ok(())
        })
        .await
    }
}

fn row_to_template(row: &duckdb::Row) -> duckdb::Result<EncounterTemplate> {
    let id_str: String = row.get("id")?;
    let participant_json: String = row.get("participant_data")?;
    let created_at_str: String = row.get("created_at")?;

    let participants: Vec<TemplateParticipant> =
        serde_json::from_str(&participant_json).unwrap_or_default();

    Ok(EncounterTemplate {
        id: Uuid::parse_str(&id_str)
            .map_err(|e| duckdb::Error::FromSqlConversionFailure(0, duckdb::types::Type::Text, Box::new(e)))?,
        name: row.get("name")?,
        description: row.get("description")?,
        participants,
        created_at: created_at_str.parse().unwrap_or_else(|_| Utc::now()),
    })
}
