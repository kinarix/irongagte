use async_trait::async_trait;
use irongate_core::{errors::StoreError, Permission};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct SqlitePermissionRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_permission(row: &sqlx::sqlite::SqliteRow) -> Result<Permission, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    Ok(Permission {
        id,
        tenant_id,
        resource: row.try_get("resource").map_err(map_row_err)?,
        action: row.try_get("action").map_err(map_row_err)?,
        description: row.try_get("description").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::PermissionRepository for SqlitePermissionRepo {
    async fn create(&self, permission: Permission) -> Result<Permission, StoreError> {
        sqlx::query(
            "INSERT INTO permissions (id, tenant_id, resource, action, description, created_at)
             VALUES (?,?,?,?,?,?)",
        )
        .bind(permission.id.to_string())
        .bind(permission.tenant_id.to_string())
        .bind(&permission.resource)
        .bind(&permission.action)
        .bind(&permission.description)
        .bind(permission.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(permission.id, permission.tenant_id).await
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Permission, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM permissions WHERE id = ? AND tenant_id = ?",
        )
        .bind(id.to_string())
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_permission(&row)
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Permission>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM permissions WHERE tenant_id = ? ORDER BY resource, action",
        )
        .bind(tenant_id.to_string())
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
             WHERE rp.role_id = ? AND rp.tenant_id = ?",
        )
        .bind(role_id.to_string())
        .bind(tenant_id.to_string())
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
            "INSERT OR IGNORE INTO role_permissions (role_id, permission_id, tenant_id)
             VALUES (?,?,?)",
        )
        .bind(role_id.to_string())
        .bind(permission_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM permissions WHERE id = ? AND tenant_id = ?")
            .bind(id.to_string())
            .bind(tenant_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }
}
