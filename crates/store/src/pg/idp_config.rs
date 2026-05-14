use async_trait::async_trait;
use irongate_core::{errors::StoreError, IdpConfig, IdpType};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_parse_err, map_row_err};

pub struct PgIdpConfigRepo {
    pub(crate) pool: PgPool,
}

fn row_to_idp_config(row: &sqlx::postgres::PgRow) -> Result<IdpConfig, StoreError> {
    let provider_type_str: String = row.try_get("provider_type").map_err(map_row_err)?;
    let provider_type: IdpType = provider_type_str
        .parse()
        .map_err(|_| map_parse_err("provider_type"))?;
    Ok(IdpConfig {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        provider_type,
        name: row.try_get("name").map_err(map_row_err)?,
        enabled: row.try_get("enabled").map_err(map_row_err)?,
        config: row.try_get("config").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::IdpConfigRepository for PgIdpConfigRepo {
    async fn create(&self, config: IdpConfig) -> Result<IdpConfig, StoreError> {
        let config_json = serde_json::to_string(&config.config).unwrap_or_else(|_| "{}".into());
        let row = sqlx::query(
            "INSERT INTO idp_configs
             (id, tenant_id, provider_type, name, enabled, config, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6::jsonb,$7,$8) RETURNING *",
        )
        .bind(config.id)
        .bind(config.tenant_id)
        .bind(config.provider_type.to_string())
        .bind(&config.name)
        .bind(config.enabled)
        .bind(&config_json)
        .bind(config.created_at)
        .bind(config.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_idp_config(&row)
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<IdpConfig, StoreError> {
        let row = sqlx::query("SELECT * FROM idp_configs WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        row_to_idp_config(&row)
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<IdpConfig>, StoreError> {
        let rows = sqlx::query("SELECT * FROM idp_configs WHERE tenant_id = $1 ORDER BY name")
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_err)?;
        rows.iter().map(row_to_idp_config).collect()
    }

    async fn update(&self, config: IdpConfig) -> Result<IdpConfig, StoreError> {
        let config_json = serde_json::to_string(&config.config).unwrap_or_else(|_| "{}".into());
        let row = sqlx::query(
            "UPDATE idp_configs
             SET name = $1, enabled = $2, config = $3::jsonb, updated_at = $4
             WHERE id = $5 AND tenant_id = $6 RETURNING *",
        )
        .bind(&config.name)
        .bind(config.enabled)
        .bind(&config_json)
        .bind(config.updated_at)
        .bind(config.id)
        .bind(config.tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_idp_config(&row)
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM idp_configs WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
