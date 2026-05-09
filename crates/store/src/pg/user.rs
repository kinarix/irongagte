use async_trait::async_trait;
use irongate_core::{errors::StoreError, User, UserStatus};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_parse_err, map_row_err};

pub struct PgUserRepo {
    pub(crate) pool: PgPool,
}

fn row_to_user(row: &sqlx::postgres::PgRow) -> Result<User, StoreError> {
    let status_str: String = row.try_get("status").map_err(map_row_err)?;
    let status: UserStatus = status_str
        .parse()
        .map_err(|_| map_parse_err("status"))?;
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
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
        last_login_at: row.try_get("last_login_at").map_err(map_row_err)?,
        deleted_at: row.try_get("deleted_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::UserRepository for PgUserRepo {
    async fn create(&self, user: User) -> Result<User, StoreError> {
        let row = sqlx::query(
            "INSERT INTO users (id, tenant_id, email, email_verified, name, given_name,
             family_name, picture_url, status, created_at, updated_at, last_login_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
             RETURNING *",
        )
        .bind(user.id)
        .bind(user.tenant_id)
        .bind(&user.email)
        .bind(user.email_verified)
        .bind(&user.name)
        .bind(&user.given_name)
        .bind(&user.family_name)
        .bind(&user.picture_url)
        .bind(user.status.to_string())
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(user.last_login_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_user(&row)
    }

    async fn get_by_id(&self, id: Uuid, tenant_id: Uuid) -> Result<User, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM users WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_user(&row)
    }

    async fn get_by_email(&self, email: &str, tenant_id: Uuid) -> Result<User, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM users WHERE email = $1 AND tenant_id = $2 AND deleted_at IS NULL",
        )
        .bind(email)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_user(&row)
    }

    async fn update(&self, user: User) -> Result<User, StoreError> {
        let row = sqlx::query(
            "UPDATE users SET email = $1, email_verified = $2, name = $3, given_name = $4,
             family_name = $5, picture_url = $6, status = $7, updated_at = $8, last_login_at = $9
             WHERE id = $10 AND tenant_id = $11 AND deleted_at IS NULL
             RETURNING *",
        )
        .bind(&user.email)
        .bind(user.email_verified)
        .bind(&user.name)
        .bind(&user.given_name)
        .bind(&user.family_name)
        .bind(&user.picture_url)
        .bind(user.status.to_string())
        .bind(user.updated_at)
        .bind(user.last_login_at)
        .bind(user.id)
        .bind(user.tenant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_user(&row)
    }

    async fn soft_delete(&self, id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        sqlx::query(
            "UPDATE users SET deleted_at = $1
             WHERE id = $2 AND tenant_id = $3 AND deleted_at IS NULL",
        )
        .bind(now)
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
    ) -> Result<Vec<User>, StoreError> {
        let rows = sqlx::query(
            "SELECT * FROM users WHERE tenant_id = $1 AND deleted_at IS NULL
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        rows.iter().map(row_to_user).collect()
    }
}
