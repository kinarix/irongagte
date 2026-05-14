use async_trait::async_trait;
use irongate_core::{errors::StoreError, Tenant};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_json_err, map_row_err};

pub struct PgTenantRepo {
    pub(crate) pool: PgPool,
}

fn row_to_tenant(row: &sqlx::postgres::PgRow) -> Result<Tenant, StoreError> {
    Ok(Tenant {
        id: row.try_get("id").map_err(map_row_err)?,
        name: row.try_get("name").map_err(map_row_err)?,
        slug: row.try_get("slug").map_err(map_row_err)?,
        settings: row.try_get("settings").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
        deleted_at: row.try_get("deleted_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::TenantRepository for PgTenantRepo {
    async fn create(&self, tenant: Tenant) -> Result<Tenant, StoreError> {
        let settings = serde_json::to_string(&tenant.settings).map_err(map_json_err("settings"))?;
        let row = sqlx::query(
            "INSERT INTO tenants (id, name, slug, settings, created_at, updated_at)
             VALUES ($1, $2, $3, $4::jsonb, $5, $6)
             RETURNING *",
        )
        .bind(tenant.id)
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&settings)
        .bind(tenant.created_at)
        .bind(tenant.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_tenant(&row)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Tenant, StoreError> {
        let row = sqlx::query("SELECT * FROM tenants WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        row_to_tenant(&row)
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Tenant, StoreError> {
        let row = sqlx::query("SELECT * FROM tenants WHERE slug = $1 AND deleted_at IS NULL")
            .bind(slug)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        row_to_tenant(&row)
    }

    async fn update(&self, tenant: Tenant) -> Result<Tenant, StoreError> {
        let settings = serde_json::to_string(&tenant.settings).map_err(map_json_err("settings"))?;
        let row = sqlx::query(
            "UPDATE tenants SET name = $1, slug = $2, settings = $3::jsonb, updated_at = $4
             WHERE id = $5 AND deleted_at IS NULL
             RETURNING *",
        )
        .bind(&tenant.name)
        .bind(&tenant.slug)
        .bind(&settings)
        .bind(tenant.updated_at)
        .bind(tenant.id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_tenant(&row)
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        sqlx::query("UPDATE tenants SET deleted_at = $1 WHERE id = $2 AND deleted_at IS NULL")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM tenants WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_tenant).collect()
    }
}
