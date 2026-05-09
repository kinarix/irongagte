use async_trait::async_trait;
use irongate_core::{errors::StoreError, User, UserStatus};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_parse_err, map_row_err};

pub struct SqliteUserRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_user(row: &sqlx::sqlite::SqliteRow) -> Result<User, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let status_str: String = row.try_get("status").map_err(map_row_err)?;
    let status: UserStatus = status_str.parse().map_err(|_| map_parse_err("status"))?;

    let email_verified: i64 = row.try_get("email_verified").map_err(map_row_err)?;

    Ok(User {
        id,
        tenant_id,
        email: row.try_get("email").map_err(map_row_err)?,
        email_verified: email_verified != 0,
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
impl irongate_core::repositories::UserRepository for SqliteUserRepo {
    async fn create(&self, user: User) -> Result<User, StoreError> {
        sqlx::query(
            "INSERT INTO users
             (id, tenant_id, email, email_verified, name, given_name,
              family_name, picture_url, status, created_at, updated_at, last_login_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(user.id.to_string())
        .bind(user.tenant_id.to_string())
        .bind(&user.email)
        .bind(user.email_verified as i64)
        .bind(&user.name)
        .bind(&user.given_name)
        .bind(&user.family_name)
        .bind(&user.picture_url)
        .bind(user.status.to_string())
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(user.last_login_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(user.id, user.tenant_id).await
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<User, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM users WHERE id = ? AND tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(id.to_string())
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_user(&row)
    }

    async fn get_by_email(&self, email: &str, tenant_id: Uuid) -> Result<User, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM users WHERE email = ? AND tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(email)
        .bind(tenant_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_user(&row)
    }

    async fn update(&self, user: User) -> Result<User, StoreError> {
        sqlx::query(
            "UPDATE users
             SET email = ?, email_verified = ?, name = ?, given_name = ?,
                 family_name = ?, picture_url = ?, status = ?, updated_at = ?, last_login_at = ?
             WHERE id = ? AND tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(&user.email)
        .bind(user.email_verified as i64)
        .bind(&user.name)
        .bind(&user.given_name)
        .bind(&user.family_name)
        .bind(&user.picture_url)
        .bind(user.status.to_string())
        .bind(user.updated_at)
        .bind(user.last_login_at)
        .bind(user.id.to_string())
        .bind(user.tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_id(user.id, user.tenant_id).await
    }

    async fn soft_delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        sqlx::query(
            "UPDATE users SET deleted_at = ?
             WHERE id = ? AND tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(now)
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
    ) -> Result<Vec<User>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM users WHERE tenant_id = ? AND deleted_at IS NULL
             ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(tenant_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_user).collect()
    }
}
