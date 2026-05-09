use async_trait::async_trait;
use irongate_core::{errors::StoreError, UserCredentials};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct SqliteUserCredentialsRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_creds(row: &sqlx::sqlite::SqliteRow) -> Result<UserCredentials, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let tenant_id_str: String = row.try_get("tenant_id").map_err(map_row_err)?;
    let tenant_id = Uuid::parse_str(&tenant_id_str)
        .map_err(|_| StoreError::Database("bad uuid: tenant_id".into()))?;

    let user_id_str: String = row.try_get("user_id").map_err(map_row_err)?;
    let user_id = Uuid::parse_str(&user_id_str)
        .map_err(|_| StoreError::Database("bad uuid: user_id".into()))?;

    let totp_enabled: i64 = row.try_get("totp_enabled").map_err(map_row_err)?;

    Ok(UserCredentials {
        id,
        tenant_id,
        user_id,
        password_hash: row.try_get("password_hash").map_err(map_row_err)?,
        totp_secret: row.try_get("totp_secret").map_err(map_row_err)?,
        totp_enabled: totp_enabled != 0,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::UserCredentialsRepository for SqliteUserCredentialsRepo {
    async fn create(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError> {
        sqlx::query(
            "INSERT INTO user_credentials
             (id, tenant_id, user_id, password_hash, totp_secret, totp_enabled, created_at, updated_at)
             VALUES (?,?,?,?,?,?,?,?)",
        )
        .bind(creds.id.to_string())
        .bind(creds.tenant_id.to_string())
        .bind(creds.user_id.to_string())
        .bind(&creds.password_hash)
        .bind(&creds.totp_secret)
        .bind(creds.totp_enabled as i64)
        .bind(creds.created_at)
        .bind(creds.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(creds)
    }

    async fn get_by_user_id(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<UserCredentials, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM user_credentials WHERE user_id = ? AND tenant_id = ?",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        match row {
            Some(r) => row_to_creds(&r),
            None => Err(StoreError::NotFound(format!("credentials for user {user_id}"))),
        }
    }

    async fn update(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError> {
        let rows_affected = sqlx::query(
            "UPDATE user_credentials
             SET password_hash = ?, totp_secret = ?, totp_enabled = ?, updated_at = ?
             WHERE user_id = ? AND tenant_id = ?",
        )
        .bind(&creds.password_hash)
        .bind(&creds.totp_secret)
        .bind(creds.totp_enabled as i64)
        .bind(creds.updated_at)
        .bind(creds.user_id.to_string())
        .bind(creds.tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?
        .rows_affected();

        if rows_affected == 0 {
            return Err(StoreError::NotFound(format!(
                "credentials for user {}",
                creds.user_id
            )));
        }
        Ok(creds)
    }

    async fn delete(&self, user_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM user_credentials WHERE user_id = ? AND tenant_id = ?",
        )
        .bind(user_id.to_string())
        .bind(tenant_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }
}
