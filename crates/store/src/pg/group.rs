use async_trait::async_trait;
use irongate_core::{errors::StoreError, Group, User};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgGroupRepo {
    pub(crate) pool: PgPool,
}

fn row_to_group(row: &sqlx::postgres::PgRow) -> Result<Group, StoreError> {
    Ok(Group {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        display_name: row.try_get("display_name").map_err(map_row_err)?,
        external_id: row.try_get("external_id").map_err(map_row_err)?,
        priority: row.try_get("priority").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

fn row_to_user(row: &sqlx::postgres::PgRow) -> Result<User, StoreError> {
    use irongate_core::types::UserStatus;
    let status_str: String = row.try_get("status").map_err(map_row_err)?;
    let status: UserStatus = status_str.parse().map_err(|_| {
        StoreError::Database(format!("unknown user status: {status_str}"))
    })?;
    Ok(User {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        email: row.try_get("email").map_err(map_row_err)?,
        email_verified: row.try_get("email_verified").map_err(map_row_err)?,
        name: row.try_get("name").map_err(map_row_err)?,
        given_name: row.try_get("given_name").map_err(map_row_err)?,
        family_name: row.try_get("family_name").map_err(map_row_err)?,
        picture_url: row.try_get("picture_url").map_err(map_row_err)?,
        status,
        attributes: row.try_get("attributes").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
        last_login_at: row.try_get("last_login_at").map_err(map_row_err)?,
        deleted_at: row.try_get("deleted_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::GroupRepository for PgGroupRepo {
    async fn create(&self, group: Group) -> Result<Group, StoreError> {
        let row = sqlx::query(
            "INSERT INTO groups (id, tenant_id, display_name, external_id, priority, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *",
        )
        .bind(group.id)
        .bind(group.tenant_id)
        .bind(&group.display_name)
        .bind(&group.external_id)
        .bind(group.priority)
        .bind(group.created_at)
        .bind(group.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_group(&row)
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<Group, StoreError> {
        let row = sqlx::query("SELECT * FROM groups WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
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
            sqlx::query("SELECT * FROM groups WHERE display_name = $1 AND tenant_id = $2")
                .bind(display_name)
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_db_err)?;
        row_to_group(&row)
    }

    async fn update(&self, group: Group) -> Result<Group, StoreError> {
        let row = sqlx::query(
            "UPDATE groups SET display_name = $1, external_id = $2, priority = $3, updated_at = $4
             WHERE id = $5 AND tenant_id = $6 RETURNING *",
        )
        .bind(&group.display_name)
        .bind(&group.external_id)
        .bind(group.priority)
        .bind(group.updated_at)
        .bind(group.id)
        .bind(group.tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_group(&row)
    }

    async fn delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM groups WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
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
            "SELECT * FROM groups WHERE tenant_id = $1 ORDER BY display_name LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
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
            "INSERT INTO group_members (group_id, user_id, tenant_id)
             VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
        )
        .bind(group_id)
        .bind(user_id)
        .bind(tenant_id)
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
            "DELETE FROM group_members WHERE group_id = $1 AND user_id = $2 AND tenant_id = $3",
        )
        .bind(group_id)
        .bind(user_id)
        .bind(tenant_id)
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
             WHERE gm.group_id = $1 AND gm.tenant_id = $2 AND u.deleted_at IS NULL",
        )
        .bind(group_id)
        .bind(tenant_id)
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
             WHERE gm.user_id = $1 AND gm.tenant_id = $2",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_group).collect()
    }
}
