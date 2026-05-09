use async_trait::async_trait;
use irongate_core::{errors::StoreError, Tenant};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_json_err, map_row_err};

pub struct SqliteTenantRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_tenant(row: &sqlx::sqlite::SqliteRow) -> Result<Tenant, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let settings_json: String = row.try_get("settings").map_err(map_row_err)?;
    let settings: serde_json::Value =
        serde_json::from_str(&settings_json).map_err(map_json_err("settings"))?;

    Ok(Tenant {
        id,
        name: row.try_get("name").map_err(map_row_err)?,
        slug: row.try_get("slug").map_err(map_row_err)?,
        settings,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
        deleted_at: row.try_get("deleted_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::TenantRepository for SqliteTenantRepo {
    async fn create(&self, tenant: Tenant) -> Result<Tenant, StoreError> {
        let settings =
            serde_json::to_string(&tenant.settings).map_err(map_json_err("settings"))?;
        sqlx::query(
            "INSERT INTO tenants (id, name, slug, settings, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant.id.to_string())
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&settings)
        .bind(tenant.created_at)
        .bind(tenant.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(tenant.id).await
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Tenant, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM tenants WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_tenant(&row)
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Tenant, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM tenants WHERE slug = ? AND deleted_at IS NULL",
        )
        .bind(slug)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_tenant(&row)
    }

    async fn update(&self, tenant: Tenant) -> Result<Tenant, StoreError> {
        let settings =
            serde_json::to_string(&tenant.settings).map_err(map_json_err("settings"))?;
        sqlx::query(
            "UPDATE tenants SET name = ?, slug = ?, settings = ?, updated_at = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&settings)
        .bind(tenant.updated_at)
        .bind(tenant.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(tenant.id).await
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        sqlx::query(
            "UPDATE tenants SET deleted_at = ? WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(now)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM tenants WHERE deleted_at IS NULL
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_tenant).collect()
    }
}
