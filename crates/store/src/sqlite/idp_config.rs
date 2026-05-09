use async_trait::async_trait;
use irongate_core::{errors::StoreError, IdpConfig, IdpType};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_json_err, map_parse_err, map_row_err};

pub struct SqliteIdpConfigRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_idp_config(row: &sqlx::sqlite::SqliteRow) -> Result<IdpConfig, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let provider_type_str: String = row.try_get("provider_type").map_err(map_row_err)?;
    let provider_type: IdpType =
        provider_type_str.parse().map_err(|_| map_parse_err("provider_type"))?;

    let enabled: i64 = row.try_get("enabled").map_err(map_row_err)?;

    let config_json: String = row.try_get("config").map_err(map_row_err)?;
    let config: serde_json::Value =
        serde_json::from_str(&config_json).map_err(map_json_err("config"))?;

    Ok(IdpConfig {
        id,
        tenant_id,
        provider_type,
        name: row.try_get("name").map_err(map_row_err)?,
        enabled: enabled != 0,
        config,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::IdpConfigRepository for SqliteIdpConfigRepo {
    async fn create(&self, config: IdpConfig) -> Result<IdpConfig, StoreError> {
        let config_json =
            serde_json::to_string(&config.config).unwrap_or_else(|_| "{}".into());

        sqlx::query(
            "INSERT INTO idp_configs
             (id, tenant_id, provider_type, name, enabled, config, created_at, updated_at)
             VALUES (?,?,?,?,?,?,?,?)",
        )
        .bind(config.id.to_string())
        .bind(config.tenant_id.to_string())
        .bind(config.provider_type.to_string())
        .bind(&config.name)
        .bind(config.enabled as i64)
        .bind(&config_json)
        .bind(config.created_at)
        .bind(config.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(config.id, config.tenant_id).await
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<IdpConfig, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM idp_configs WHERE id = ? AND tenant_id = ?",
        )
        .bind(id.to_string())
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_idp_config(&row)
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<IdpConfig>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM idp_configs WHERE tenant_id = ? ORDER BY name",
        )
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_idp_config).collect()
    }

    async fn update(&self, config: IdpConfig) -> Result<IdpConfig, StoreError> {
        let config_json =
            serde_json::to_string(&config.config).unwrap_or_else(|_| "{}".into());

        sqlx::query(
            "UPDATE idp_configs
             SET name = ?, enabled = ?, config = ?, updated_at = ?
             WHERE id = ? AND tenant_id = ?",
        )
        .bind(&config.name)
        .bind(config.enabled as i64)
        .bind(&config_json)
        .bind(config.updated_at)
        .bind(config.id.to_string())
        .bind(config.tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(config.id, config.tenant_id).await
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM idp_configs WHERE id = ? AND tenant_id = ?")
            .bind(id.to_string())
            .bind(tenant_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
