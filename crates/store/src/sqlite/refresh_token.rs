use async_trait::async_trait;
use irongate_core::{errors::StoreError, RefreshToken};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct SqliteRefreshTokenRepo {
    pub(crate) pool: SqlitePool,
}

fn row_to_token(row: &sqlx::sqlite::SqliteRow) -> Result<RefreshToken, StoreError> {
    let id_str: String = row.try_get("id").map_err(map_row_err)?;
    let id = Uuid::parse_str(&id_str).map_err(|_| StoreError::Database("bad uuid: id".into()))?;

    let session_id_str: String = row.try_get("session_id").map_err(map_row_err)?;
    let session_id = Uuid::parse_str(&session_id_str)
        .map_err(|_| StoreError::Database("bad uuid: session_id".into()))?;

    let application_id_str: String = row.try_get("application_id").map_err(map_row_err)?;
    let application_id = Uuid::parse_str(&application_id_str)
        .map_err(|_| StoreError::Database("bad uuid: application_id".into()))?;

    let previous_id: Option<String> = row.try_get("previous_id").map_err(map_row_err)?;
    let previous_id = previous_id
        .map(|s| Uuid::parse_str(&s).map_err(|_| StoreError::Database("bad uuid: previous_id".into())))
        .transpose()?;

    Ok(RefreshToken {
        id,
        session_id,
        application_id,
        token_hash: row.try_get("token_hash").map_err(map_row_err)?,
        scope: row.try_get("scope").map_err(map_row_err)?,
        previous_id,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
        expires_at: row.try_get("expires_at").map_err(map_row_err)?,
        revoked_at: row.try_get("revoked_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::RefreshTokenRepository for SqliteRefreshTokenRepo {
    async fn create(&self, token: RefreshToken) -> Result<RefreshToken, StoreError> {
        sqlx::query(
            "INSERT INTO refresh_tokens
             (id, session_id, application_id, token_hash, scope, previous_id,
              created_at, expires_at)
             VALUES (?,?,?,?,?,?,?,?)",
        )
        .bind(token.id.to_string())
        .bind(token.session_id.to_string())
        .bind(token.application_id.to_string())
        .bind(&token.token_hash)
        .bind(&token.scope)
        .bind(token.previous_id.map(|u| u.to_string()))
        .bind(token.created_at)
        .bind(token.expires_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;

        self.get_by_hash(&token.token_hash).await
    }

    async fn get_by_hash(&self, token_hash: &str) -> Result<RefreshToken, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM refresh_tokens WHERE token_hash = ? AND revoked_at IS NULL",
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
            "UPDATE refresh_tokens SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(())
    }

    async fn revoke_all_for_session(&self, session_id: Uuid) -> Result<u64, StoreError> {
        let now = time::OffsetDateTime::now_utc();
        let result = sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = ?
             WHERE session_id = ? AND revoked_at IS NULL",
        )
        .bind(now)
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(result.rows_affected())
    }
}
