use async_trait::async_trait;
use irongate_core::{errors::StoreError, RefreshToken};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgRefreshTokenRepo {
    pub(crate) pool: PgPool,
}

fn row_to_token(row: &sqlx::postgres::PgRow) -> Result<RefreshToken, StoreError> {
    Ok(RefreshToken {
        id: row.try_get("id").map_err(map_row_err)?,
        session_id: row.try_get("session_id").map_err(map_row_err)?,
        application_id: row.try_get("application_id").map_err(map_row_err)?,
        token_hash: row.try_get("token_hash").map_err(map_row_err)?,
        scope: row.try_get("scope").map_err(map_row_err)?,
        previous_id: row.try_get("previous_id").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        expires_at: row.try_get("expires_at").map_err(map_row_err)?,
        revoked_at: row.try_get("revoked_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::RefreshTokenRepository for PgRefreshTokenRepo {
    async fn create(&self, token: RefreshToken) -> Result<RefreshToken, StoreError> {
        let row = sqlx::query(
            "INSERT INTO refresh_tokens
             (id, session_id, application_id, token_hash, scope, previous_id, created_at, expires_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             RETURNING *",
        )
        .bind(token.id)
        .bind(token.session_id)
        .bind(token.application_id)
        .bind(&token.token_hash)
        .bind(&token.scope)
        .bind(token.previous_id)
        .bind(token.created_at)
        .bind(token.expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_token(&row)
    }

    async fn get_by_hash(&self, token_hash: &str) -> Result<RefreshToken, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM refresh_tokens WHERE token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_token(&row)
    }

    async fn revoke(&self, id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = $1 WHERE id = $2 AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn revoke_all_for_session(&self, session_id: Uuid) -> Result<u64, StoreError> {
        let now = time::OffsetDateTime::now_utc();
        let result = sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = $1
             WHERE session_id = $2 AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(result.rows_affected())
    }
}
