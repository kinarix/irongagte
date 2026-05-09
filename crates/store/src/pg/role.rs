use async_trait::async_trait;
use irongate_core::{errors::StoreError, Role};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgRoleRepo {
    pub(crate) pool: PgPool,
}

fn row_to_role(row: &sqlx::postgres::PgRow) -> Result<Role, StoreError> {
    Ok(Role {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        name: row.try_get("name").map_err(map_row_err)?,
        description: row.try_get("description").map_err(map_row_err)?,
        parent_role_id: row.try_get("parent_role_id").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::RoleRepository for PgRoleRepo {
    async fn create(&self, role: Role) -> Result<Role, StoreError> {
        let row = sqlx::query(
            "INSERT INTO roles (id, tenant_id, name, description, parent_role_id, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *",
        )
        .bind(role.id)
        .bind(role.tenant_id)
        .bind(&role.name)
        .bind(&role.description)
        .bind(role.parent_role_id)
        .bind(role.created_at)
        .bind(role.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_role(&row)
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Role, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM roles WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_role(&row)
    }

    async fn get_by_name(&self, name: &str, tenant_id: Uuid) -> Result<Role, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM roles WHERE name = $1 AND tenant_id = $2",
        )
        .bind(name)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_role(&row)
    }

    async fn update(&self, role: Role) -> Result<Role, StoreError> {
        let row = sqlx::query(
            "UPDATE roles SET name = $1, description = $2, parent_role_id = $3, updated_at = $4
             WHERE id = $5 AND tenant_id = $6 RETURNING *",
        )
        .bind(&role.name)
        .bind(&role.description)
        .bind(role.parent_role_id)
        .bind(role.updated_at)
        .bind(role.id)
        .bind(role.tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_role(&row)
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM roles WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Role>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM roles WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_role).collect()
    }

    async fn get_roles_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Role>, StoreError> {
        let rows = sqlx::query(
            "SELECT r.* FROM roles r
             JOIN user_roles ur ON ur.role_id = r.id
             WHERE ur.user_id = $1 AND ur.tenant_id = $2",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_role).collect()
    }

    async fn assign_role_to_user(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO user_roles (user_id, role_id, tenant_id)
             VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(role_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn remove_role_from_user(
        &self,
        user_id: Uuid,
        role_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM user_roles WHERE user_id = $1 AND role_id = $2 AND tenant_id = $3",
        )
        .bind(user_id)
        .bind(role_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }
}
