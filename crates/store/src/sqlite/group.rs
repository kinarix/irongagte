use async_trait::async_trait;
use irongate_core::{errors::StoreError, Group, User};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct SqliteGroupRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_group(row: &sqlx::sqlite::SqliteRow) -> Result<Group, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id =
        Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    Ok(Group {
        id,
        tenant_id,
        display_name: row.try_get("display_name").map_err(map_row_err)?,
        external_id: row.try_get("external_id").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

fn row_to_user(row: &sqlx::sqlite::SqliteRow) -> Result<User, StoreError> {
    use irongate_core::types::UserStatus;

    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id =
        Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let status_str: String = row.try_get("status").map_err(map_row_err)?;
    let status: UserStatus = status_str
        .parse()
        .map_err(|_| StoreError::Database(format!("unknown user status: {status_str}")))?;

    Ok(User {
        id,
        tenant_id,
        email: row.try_get("email").map_err(map_row_err)?,
        email_verified: row.try_get::<bool, _>("email_verified").map_err(map_row_err)?,
        name: row.try_get("name").map_err(map_row_err)?,
        given_name: row.try_get("given_name").map_err(map_row_err)?,
        family_name: row.try_get("family_name").map_err(map_row_err)?,
        picture_url: row.try_get("picture_url").map_err(map_row_err)?,
        status,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
        last_login_at: row.try_get("last_login_at").map_err(map_row_err)?,
        deleted_at: row.try_get("deleted_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::GroupRepository for SqliteGroupRepo {
    async fn create(&self, group: Group) -> Result<Group, StoreError> {
        sqlx::query(
            "INSERT INTO groups
             (id, tenant_id, display_name, external_id, created_at, updated_at)
             VALUES (?,?,?,?,?,?)",
        )
        .bind(group.id.to_string())
        .bind(group.tenant_id.to_string())
        .bind(&group.display_name)
        .bind(&group.external_id)
        .bind(group.created_at)
        .bind(group.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(group.id, group.tenant_id).await
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Group, StoreError> {
        let row =
            sqlx::query("SELECT * FROM groups WHERE id = ? AND tenant_id = ?")
                .bind(id.to_string())
                .bind(tenant_id.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(map_db_err)?;
        row_to_group(&row)
    }

    async fn get_by_display_name(
        &self,
        display_name: &str,
        tenant_id: Uuid,
    ) -> Result<Group, StoreError> {
        let row =
            sqlx::query("SELECT * FROM groups WHERE display_name = ? AND tenant_id = ?")
                .bind(display_name)
                .bind(tenant_id.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(map_db_err)?;
        row_to_group(&row)
    }

    async fn update(&self, group: Group) -> Result<Group, StoreError> {
        sqlx::query(
            "UPDATE groups SET display_name = ?, external_id = ?, updated_at = ?
             WHERE id = ? AND tenant_id = ?",
        )
        .bind(&group.display_name)
        .bind(&group.external_id)
        .bind(group.updated_at)
        .bind(group.id.to_string())
        .bind(group.tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(group.id, group.tenant_id).await
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM groups WHERE id = ? AND tenant_id = ?")
            .bind(id.to_string())
            .bind(tenant_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    async fn list(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Group>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM groups WHERE tenant_id = ? ORDER BY display_name LIMIT ? OFFSET ?",
        )
        .bind(tenant_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_group).collect()
    }

    async fn add_member(
        &self,
        group_id: Uuid,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT OR IGNORE INTO group_members (group_id, user_id, tenant_id) VALUES (?,?,?)",
        )
        .bind(group_id.to_string())
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn remove_member(
        &self,
        group_id: Uuid,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM group_members WHERE group_id = ? AND user_id = ? AND tenant_id = ?",
        )
        .bind(group_id.to_string())
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn list_members(
        &self,
        group_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<User>, StoreError> {
        let rows = sqlx::query(
            "SELECT u.* FROM users u
             JOIN group_members gm ON gm.user_id = u.id
             WHERE gm.group_id = ? AND gm.tenant_id = ? AND u.deleted_at IS NULL",
        )
        .bind(group_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_user).collect()
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Vec<Group>, StoreError> {
        let rows = sqlx::query(
            "SELECT g.* FROM groups g
             JOIN group_members gm ON gm.group_id = g.id
             WHERE gm.user_id = ? AND gm.tenant_id = ?",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_group).collect()
    }
}
