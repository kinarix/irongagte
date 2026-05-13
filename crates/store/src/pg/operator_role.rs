use async_trait::async_trait;
use irongate_core::{
    errors::StoreError,
    repositories::{OperatorRoleRepository, OperatorRoleScope},
    OperatorPermission, OperatorRole,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgOperatorRoleRepo {
    pub(crate) pool: PgPool,
}

fn row_to_operator_role(row: &sqlx::postgres::PgRow) -> Result<OperatorRole, StoreError> {
    Ok(OperatorRole {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        name: row.try_get("name").map_err(map_row_err)?,
        description: row.try_get("description").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

fn row_to_operator_permission(row: &sqlx::postgres::PgRow) -> Result<OperatorPermission, StoreError> {
    Ok(OperatorPermission {
        id: row.try_get("id").map_err(map_row_err)?,
        resource: row.try_get("resource").map_err(map_row_err)?,
        action: row.try_get("action").map_err(map_row_err)?,
        description: row.try_get("description").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl OperatorRoleRepository for PgOperatorRoleRepo {
    async fn create(&self, r: OperatorRole) -> Result<OperatorRole, StoreError> {
        let row = sqlx::query(
            "INSERT INTO operator_roles (id, tenant_id, name, description, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *",
        )
        .bind(r.id)
        .bind(r.tenant_id)
        .bind(&r.name)
        .bind(&r.description)
        .bind(r.created_at)
        .bind(r.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_operator_role(&row)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<OperatorRole, StoreError> {
        let row = sqlx::query("SELECT * FROM operator_roles WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        row_to_operator_role(&row)
    }

    async fn get_by_name(
        &self,
        name: &str,
        tenant_id: Option<Uuid>,
    ) -> Result<OperatorRole, StoreError> {
        let row = match tenant_id {
            None => sqlx::query(
                "SELECT * FROM operator_roles WHERE name = $1 AND tenant_id IS NULL",
            )
            .bind(name)
            .fetch_one(&self.pool)
            .await,
            Some(tid) => sqlx::query(
                "SELECT * FROM operator_roles WHERE name = $1 AND tenant_id = $2",
            )
            .bind(name)
            .bind(tid)
            .fetch_one(&self.pool)
            .await,
        }
        .map_err(map_db_err)?;
        row_to_operator_role(&row)
    }

    async fn list(
        &self,
        scope: OperatorRoleScope,
    ) -> Result<Vec<OperatorRole>, StoreError> {
        let rows = match scope {
            OperatorRoleScope::All => {
                sqlx::query("SELECT * FROM operator_roles ORDER BY tenant_id NULLS FIRST, name")
                    .fetch_all(&self.pool)
                    .await
            }
            OperatorRoleScope::Global => {
                sqlx::query("SELECT * FROM operator_roles WHERE tenant_id IS NULL ORDER BY name")
                    .fetch_all(&self.pool)
                    .await
            }
            OperatorRoleScope::Tenant(tid) => sqlx::query(
                "SELECT * FROM operator_roles WHERE tenant_id = $1 ORDER BY name",
            )
            .bind(tid)
            .fetch_all(&self.pool)
            .await,
        }
        .map_err(map_db_err)?;
        rows.iter().map(row_to_operator_role).collect()
    }

    async fn update(&self, r: OperatorRole) -> Result<OperatorRole, StoreError> {
        // tenant_id is immutable after creation — changing scope is a delete + create.
        let row = sqlx::query(
            "UPDATE operator_roles SET name = $1, description = $2, updated_at = $3
             WHERE id = $4
             RETURNING *",
        )
        .bind(&r.name)
        .bind(&r.description)
        .bind(r.updated_at)
        .bind(r.id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_operator_role(&row)
    }

    async fn delete(&self, id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM operator_roles WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    async fn assign_permission(
        &self,
        role_id: Uuid,
        permission_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO operator_role_permissions (operator_role_id, operator_permission_id)
             VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn revoke_permission(
        &self,
        role_id: Uuid,
        permission_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM operator_role_permissions
             WHERE operator_role_id = $1 AND operator_permission_id = $2",
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn list_permissions(&self, role_id: Uuid) -> Result<Vec<OperatorPermission>, StoreError> {
        let rows = sqlx::query(
            "SELECT p.* FROM operator_permissions p
             INNER JOIN operator_role_permissions rp ON p.id = rp.operator_permission_id
             WHERE rp.operator_role_id = $1
             ORDER BY p.resource, p.action",
        )
        .bind(role_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_operator_permission).collect()
    }

    async fn assign_to_operator(
        &self,
        operator_id: Uuid,
        role_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO operator_role_assignments (operator_id, operator_role_id, assigned_at)
             VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(operator_id)
        .bind(role_id)
        .bind(time::OffsetDateTime::now_utc())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn revoke_from_operator(
        &self,
        operator_id: Uuid,
        role_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM operator_role_assignments
             WHERE operator_id = $1 AND operator_role_id = $2",
        )
        .bind(operator_id)
        .bind(role_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn list_for_operator(&self, operator_id: Uuid) -> Result<Vec<OperatorRole>, StoreError> {
        let rows = sqlx::query(
            "SELECT r.* FROM operator_roles r
             INNER JOIN operator_role_assignments a ON r.id = a.operator_role_id
             WHERE a.operator_id = $1
             ORDER BY r.name",
        )
        .bind(operator_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_operator_role).collect()
    }

    async fn list_permissions_for_operator(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<OperatorPermission>, StoreError> {
        let rows = sqlx::query(
            "SELECT DISTINCT p.* FROM operator_permissions p
             INNER JOIN operator_role_permissions rp ON p.id = rp.operator_permission_id
             INNER JOIN operator_role_assignments a ON rp.operator_role_id = a.operator_role_id
             WHERE a.operator_id = $1
             ORDER BY p.resource, p.action",
        )
        .bind(operator_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_operator_permission).collect()
    }

    async fn list_permissions_for_operator_in_tenant(
        &self,
        operator_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<OperatorPermission>, StoreError> {
        let rows = sqlx::query(
            "SELECT DISTINCT p.* FROM operator_permissions p
             INNER JOIN operator_role_permissions rp ON p.id = rp.operator_permission_id
             INNER JOIN operator_role_assignments a ON rp.operator_role_id = a.operator_role_id
             INNER JOIN operator_roles r ON r.id = a.operator_role_id
             WHERE a.operator_id = $1
               AND (r.tenant_id IS NULL OR r.tenant_id = $2)
             ORDER BY p.resource, p.action",
        )
        .bind(operator_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_operator_permission).collect()
    }

    async fn list_permissions_for_operator_global(
        &self,
        operator_id: Uuid,
    ) -> Result<Vec<OperatorPermission>, StoreError> {
        let rows = sqlx::query(
            "SELECT DISTINCT p.* FROM operator_permissions p
             INNER JOIN operator_role_permissions rp ON p.id = rp.operator_permission_id
             INNER JOIN operator_role_assignments a ON rp.operator_role_id = a.operator_role_id
             INNER JOIN operator_roles r ON r.id = a.operator_role_id
             WHERE a.operator_id = $1 AND r.tenant_id IS NULL
             ORDER BY p.resource, p.action",
        )
        .bind(operator_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_operator_permission).collect()
    }
}
