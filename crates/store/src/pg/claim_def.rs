use async_trait::async_trait;
use irongate_core::{errors::StoreError, ClaimDefinition, ClaimType};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

use crate::util::{map_db_err, map_parse_err, map_row_err};

pub struct PgClaimDefinitionRepo {
    pub(crate) pool: PgPool,
}

fn row_to_def(row: &sqlx::postgres::PgRow) -> Result<ClaimDefinition, StoreError> {
    let claim_type_str: String = row.try_get("claim_type").map_err(map_row_err)?;
    let claim_type = ClaimType::from_str(&claim_type_str)
        .map_err(|_| map_parse_err("claim_type"))?;
    Ok(ClaimDefinition {
        id: row.try_get("id").map_err(map_row_err)?,
        application_id: row.try_get("application_id").map_err(map_row_err)?,
        key: row.try_get("key").map_err(map_row_err)?,
        claim_type,
        description: row.try_get("description").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::ClaimDefinitionRepository for PgClaimDefinitionRepo {
    async fn create(&self, def: ClaimDefinition) -> Result<ClaimDefinition, StoreError> {
        let row = sqlx::query(
            "INSERT INTO claim_definitions
                (id, application_id, key, claim_type, description, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             RETURNING *",
        )
        .bind(def.id)
        .bind(def.application_id)
        .bind(&def.key)
        .bind(def.claim_type.as_str())
        .bind(&def.description)
        .bind(def.created_at)
        .bind(def.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_def(&row)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<ClaimDefinition, StoreError> {
        let row = sqlx::query("SELECT * FROM claim_definitions WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        row_to_def(&row)
    }

    async fn get_by_app_and_key(
        &self,
        application_id: Uuid,
        key: &str,
    ) -> Result<ClaimDefinition, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM claim_definitions WHERE application_id = $1 AND key = $2",
        )
        .bind(application_id)
        .bind(key)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_def(&row)
    }

    async fn list_for_app(
        &self,
        application_id: Uuid,
    ) -> Result<Vec<ClaimDefinition>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM claim_definitions WHERE application_id = $1 ORDER BY key",
        )
        .bind(application_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_def).collect()
    }

    async fn list_for_tenant(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<ClaimDefinition>, StoreError> {
        let rows = sqlx::query(
            "SELECT cd.* FROM claim_definitions cd
             JOIN applications a ON a.id = cd.application_id
             WHERE a.tenant_id = $1 AND a.deleted_at IS NULL
             ORDER BY cd.key",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_def).collect()
    }

    async fn update(&self, def: ClaimDefinition) -> Result<ClaimDefinition, StoreError> {
        let row = sqlx::query(
            "UPDATE claim_definitions
             SET key = $1, claim_type = $2, description = $3, updated_at = $4
             WHERE id = $5
             RETURNING *",
        )
        .bind(&def.key)
        .bind(def.claim_type.as_str())
        .bind(&def.description)
        .bind(def.updated_at)
        .bind(def.id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_def(&row)
    }

    async fn delete(&self, id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM claim_definitions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
