use async_trait::async_trait;
use irongate_core::{errors::StoreError, Permission};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgPermissionRepo {
    pub(crate) pool: PgPool,
}

fn row_to_permission(row: &sqlx::postgres::PgRow) -> Result<Permission, StoreError> {
    Ok(Permission {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        resource: row.try_get("resource").map_err(map_row_err)?,
        action: row.try_get("action").map_err(map_row_err)?,
        description: row.try_get("description").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::PermissionRepository for PgPermissionRepo {
    async fn create(&self, permission: Permission) -> Result<Permission, StoreError> {
        let row = sqlx::query(
            "INSERT INTO permissions (id, tenant_id, resource, action, description, created_at)
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING *",
        )
        .bind(permission.id)
        .bind(permission.tenant_id)
        .bind(&permission.resource)
        .bind(&permission.action)
        .bind(&permission.description)
        .bind(permission.created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_permission(&row)
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Permission, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM permissions WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_permission(&row)
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Permission>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM permissions WHERE tenant_id = $1 ORDER BY resource, action",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_permission).collect()
    }

    async fn get_permissions_for_role(
        &self,
        role_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Permission>, StoreError> {
        let rows = sqlx::query(
            "SELECT p.* FROM permissions p
             JOIN role_permissions rp ON rp.permission_id = p.id
             WHERE rp.role_id = $1 AND rp.tenant_id = $2",
        )
        .bind(role_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_permission).collect()
    }

    async fn assign_permission_to_role(
        &self,
        role_id: Uuid,
        permission_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO role_permissions (role_id, permission_id, tenant_id)
             VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(role_id)
        .bind(permission_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM permissions WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
