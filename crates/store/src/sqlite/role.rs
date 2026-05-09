use async_trait::async_trait;
use irongate_core::{errors::StoreError, Role};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct SqliteRoleRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_role(row: &sqlx::sqlite::SqliteRow) -> Result<Role, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let parent_role_id: Option<String> = row.try_get("parent_role_id").map_err(map_row_err)?;
    let parent_role_id = parent_role_id
        .map(|s| {
            Uuid::parse_str(&s)
                .map_err(|_| StoreError::Database("bad uuid: parent_role_id".into()))
        })
        .transpose()?;

    Ok(Role {
        id,
        tenant_id,
        name: row.try_get("name").map_err(map_row_err)?,
        description: row.try_get("description").map_err(map_row_err)?,
        parent_role_id,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::RoleRepository for SqliteRoleRepo {
    async fn create(&self, role: Role) -> Result<Role, StoreError> {
        sqlx::query(
            "INSERT INTO roles
             (id, tenant_id, name, description, parent_role_id, created_at, updated_at)
             VALUES (?,?,?,?,?,?,?)",
        )
        .bind(role.id.to_string())
        .bind(role.tenant_id.to_string())
        .bind(&role.name)
        .bind(&role.description)
        .bind(role.parent_role_id.map(|u| u.to_string()))
        .bind(role.created_at)
        .bind(role.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(role.id, role.tenant_id).await
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Role, StoreError> {
        let row = sqlx::query("SELECT * FROM roles WHERE id = ? AND tenant_id = ?")
            .bind(id.to_string())
            .bind(tenant_id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        row_to_role(&row)
    }

    async fn get_by_name(&self, name: &str, tenant_id: Uuid) -> Result<Role, StoreError> {
        let row = sqlx::query("SELECT * FROM roles WHERE name = ? AND tenant_id = ?")
            .bind(name)
            .bind(tenant_id.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_err)?;
        row_to_role(&row)
    }

    async fn update(&self, role: Role) -> Result<Role, StoreError> {
        sqlx::query(
            "UPDATE roles
             SET name = ?, description = ?, parent_role_id = ?, updated_at = ?
             WHERE id = ? AND tenant_id = ?",
        )
        .bind(&role.name)
        .bind(&role.description)
        .bind(role.parent_role_id.map(|u| u.to_string()))
        .bind(role.updated_at)
        .bind(role.id.to_string())
        .bind(role.tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(role.id, role.tenant_id).await
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM roles WHERE id = ? AND tenant_id = ?")
            .bind(id.to_string())
            .bind(tenant_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Role>, StoreError> {
        let rows = sqlx::query("SELECT * FROM roles WHERE tenant_id = ? ORDER BY name")
            .bind(tenant_id.to_string())
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
             WHERE ur.user_id = ? AND ur.tenant_id = ?",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
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
            "INSERT OR IGNORE INTO user_roles (user_id, role_id, tenant_id) VALUES (?,?,?)",
        )
        .bind(user_id.to_string())
        .bind(role_id.to_string())
        .bind(tenant_id.to_string())
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
            "DELETE FROM user_roles WHERE user_id = ? AND role_id = ? AND tenant_id = ?",
        )
        .bind(user_id.to_string())
        .bind(role_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }
}
