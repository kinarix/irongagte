use async_trait::async_trait;
use irongate_core::{errors::StoreError, MagicLink};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::util::{map_db_err, map_row_err};

pub struct PgMagicLinkRepo {
    pub(crate) pool: PgPool,
}

fn row_to_magic_link(row: &sqlx::postgres::PgRow) -> Result<MagicLink, StoreError> {
    Ok(MagicLink {
        id: row.try_get("id").map_err(map_row_err)?,
        tenant_id: row.try_get("tenant_id").map_err(map_row_err)?,
        user_id: row.try_get("user_id").map_err(map_row_err)?,
        token_hash: row.try_get("token_hash").map_err(map_row_err)?,
        expires_at: row.try_get("expires_at").map_err(map_row_err)?,
        used_at: row.try_get("used_at").map_err(map_row_err)?,
        created_at: row.try_get("created_at").map_err(map_row_err)?,
    })
}

#[async_trait]
impl irongate_core::repositories::MagicLinkRepository for PgMagicLinkRepo {
    async fn create(&self, link: MagicLink) -> Result<MagicLink, StoreError> {
        let row = sqlx::query(
            "INSERT INTO magic_links (id, tenant_id, user_id, token_hash, expires_at, used_at, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             RETURNING *",
        )
        .bind(link.id)
        .bind(link.tenant_id)
        .bind(link.user_id)
        .bind(&link.token_hash)
        .bind(link.expires_at)
        .bind(link.used_at)
        .bind(link.created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        row_to_magic_link(&row)
    }

    async fn get_by_token_hash(
        &self,
        token_hash: &str,
        tenant_id: Uuid,
    ) -> Result<MagicLink, StoreError> {
        let row = sqlx::query(
            "SELECT * FROM magic_links WHERE token_hash = $1 AND tenant_id = $2 AND used_at IS NULL",
        )
        .bind(token_hash)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        match row {
            Some(r) => row_to_magic_link(&r),
            None => Err(StoreError::NotFound("magic link".into())),
        }
    }

    async fn mark_used(&self, id: Uuid) -> Result<(), StoreError> {
        let now = time::OffsetDateTime::now_utc();
        let row = sqlx::query(
            "UPDATE magic_links SET used_at = $1 WHERE id = $2 AND used_at IS NULL RETURNING id",
        )
        .bind(now)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?;

        if row.is_none() {
            return Err(StoreError::NotFound(format!("magic link {id}")));
        }
        Ok(())
    }

    async fn delete_expired(&self, tenant_id: Uuid) -> Result<u64, StoreError> {
        let now = time::OffsetDateTime::now_utc();
        let result = sqlx::query(
            "DELETE FROM magic_links WHERE tenant_id = $1 AND expires_at < $2",
        )
        .bind(tenant_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(result.rows_affected())
    }
}
