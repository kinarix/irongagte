use async_trait::async_trait;
use irongate_core::{errors::StoreError, UserCredentials};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgUserCredentialsRepo {
    pub(crate) pool: PgPool,
}

fn row_to_creds(row: &sqlx::postgres::PgRow) -> Result<UserCredentials, StoreError> {
    Ok(UserCredentials {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        user_id: row.try_get("user_id").map_err(map_row_err)?,
        password_hash: row.try_get("password_hash").map_err(map_row_err)?,
        totp_secret: row.try_get("totp_secret").map_err(map_row_err)?,
        totp_enabled: row.try_get("totp_enabled").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        updated_at: row.try_get("updated_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::UserCredentialsRepository for PgUserCredentialsRepo {
    async fn create(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError> {
        let row = sqlx::query(
            "INSERT INTO user_credentials
             (id, tenant_id, user_id, password_hash, totp_secret, totp_enabled, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             RETURNING *",
        )
        .bind(creds.id)
        .bind(creds.tenant_id)
        .bind(creds.user_id)
        .bind(&creds.password_hash)
        .bind(&creds.totp_secret)
        .bind(creds.totp_enabled)
        .bind(creds.created_at)
        .bind(creds.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_creds(&row)
    }

    async fn get_by_user_id(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<UserCredentials, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM user_credentials WHERE user_id = $1 AND tenant_id = $2",
        )
        .bind(user_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        match row {
            Some(r) => row_to_creds(&r),
            None => Err(StoreError::NotFound(format!("credentials for user {user_id}"))),
        }
    }

    async fn update(&self, creds: UserCredentials) -> Result<UserCredentials, StoreError> {
        let row = sqlx::query(
            "UPDATE user_credentials
             SET password_hash = $1, totp_secret = $2, totp_enabled = $3, updated_at = $4
             WHERE user_id = $5 AND tenant_id = $6
             RETURNING *",
        )
        .bind(&creds.password_hash)
        .bind(&creds.totp_secret)
        .bind(creds.totp_enabled)
        .bind(creds.updated_at)
        .bind(creds.user_id)
        .bind(creds.tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        match row {
            Some(r) => row_to_creds(&r),
            None => Err(StoreError::NotFound(format!(
                "credentials for user {}",
                creds.user_id
            ))),
        }
    }

    async fn delete(&self, user_id: Uuid, tenant_id: Uuid) -> Result<(), StoreError> {
        sqlx::query(
            "DELETE FROM user_credentials WHERE user_id = $1 AND tenant_id = $2",
        )
        .bind(user_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }
}
